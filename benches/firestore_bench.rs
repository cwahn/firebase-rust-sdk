//! Firestore Performance Benchmarks
//!
//! Measures latency of Auth and Firestore operations under various concurrency levels.
//!
//! ## Benchmark Structure
//! 1. Auth operations (sign in, token refresh)
//! 2. CRUD operations (get, set, update, delete) - auth time excluded
//! 3. Listen operations (document listeners, query listeners) - auth time excluded
//!
//! ## Concurrency Levels
//! Tests with 1, 2, 4, 8, 16, 32, 64, 128, 256 concurrent operations
//!
//! ## Core Configuration
//! Set RAYON_NUM_THREADS to control core count:
//! - Single core: RAYON_NUM_THREADS=1
//! - Dual core: RAYON_NUM_THREADS=2
//! - Quad core: RAYON_NUM_THREADS=4
//!
//! ## Running Benchmarks
//! ```bash
//! # Single core
//! RAYON_NUM_THREADS=1 cargo bench --bench firestore_bench
//!
//! # Dual core
//! RAYON_NUM_THREADS=2 cargo bench --bench firestore_bench
//!
//! # Quad core
//! RAYON_NUM_THREADS=4 cargo bench --bench firestore_bench
//!
//! # Specific benchmark
//! cargo bench --bench firestore_bench -- crud_get/32
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use firebase_rust_sdk::{
    firestore::{Firestore, MapValue, Value, ValueType},
    App, AppOptions, Auth,
};
use futures::future::join_all;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Shared runtime for all benchmarks
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
});

/// Pre-authenticated Firestore instance (auth time excluded from CRUD/listen benchmarks)
static FIRESTORE: Lazy<Arc<Firestore>> = Lazy::new(|| {
    RUNTIME.block_on(async {
        let api_key = env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY not set");
        let project_id = env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID not set");
        let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
        let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

        let app = App::create(AppOptions {
            api_key: api_key.clone(),
            project_id: project_id.clone(),
            app_name: None,
        })
        .await
        .expect("Failed to create app");

        let auth = Auth::get_auth(&app).await.expect("Failed to get auth");
        auth.sign_in_with_email_and_password(&email, &password)
            .await
            .expect("Failed to sign in");

        let user = auth.current_user().await.expect("No current user");
        let id_token = user.get_id_token(false).await.expect("Failed to get token");

        Arc::new(
            Firestore::new(&project_id, "(default)", Some(id_token))
                .await
                .expect("Failed to create Firestore"),
        )
    })
});

/// Concurrency levels to test
const CONCURRENCY_LEVELS: &[usize] = &[1, 2, 4, 8, 16, 32, 64, 128, 256];

// ============================================================================
// Auth Benchmarks
// ============================================================================

fn bench_auth_sign_in(c: &mut Criterion) {
    let api_key = env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY not set");
    let project_id = env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID not set");
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let mut group = c.benchmark_group("auth");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("sign_in", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|_| async {
                            let app = App::create(AppOptions {
                                api_key: api_key.clone(),
                                project_id: project_id.clone(),
                                app_name: None,
                            })
                            .await
                            .expect("Failed to create app");

                            let auth = Auth::get_auth(&app).await.expect("Failed to get auth");
                            auth.sign_in_with_email_and_password(&email, &password)
                                .await
                                .expect("Failed to sign in");
                            
                            black_box(auth)
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_auth_token_refresh(c: &mut Criterion) {
    let api_key = env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY not set");
    let project_id = env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID not set");
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let mut group = c.benchmark_group("auth");
    group.measurement_time(Duration::from_secs(30));

    // Pre-create authenticated users for token refresh benchmark
    let users = RUNTIME.block_on(async {
        let mut users = Vec::new();
        for _ in 0..256 {
            let app = App::create(AppOptions {
                api_key: api_key.clone(),
                project_id: project_id.clone(),
                app_name: None,
            })
            .await
            .expect("Failed to create app");

            let auth = Auth::get_auth(&app).await.expect("Failed to get auth");
            auth.sign_in_with_email_and_password(&email, &password)
                .await
                .expect("Failed to sign in");
            
            let user = auth.current_user().await.expect("No user");
            users.push(user);
        }
        users
    });

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("token_refresh", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = users
                        .iter()
                        .take(concurrency)
                        .map(|user| async move {
                            let token = user.get_id_token(true).await.expect("Failed to refresh token");
                            black_box(token)
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// CRUD Benchmarks (Auth time excluded)
// ============================================================================

fn bench_crud_get(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);
    
    // Pre-create documents for get benchmark
    let doc_paths: Vec<_> = RUNTIME.block_on(async {
        let mut paths = Vec::new();
        for i in 0..256 {
            let path = format!("benchmark_get/doc_{}", i);
            let mut data = HashMap::new();
            data.insert(
                "value".to_string(),
                Value {
                    value_type: Some(ValueType::IntegerValue(i as i64)),
                },
            );
            
            firestore
                .collection("benchmark_get")
                .document(&format!("doc_{}", i))
                .set(MapValue { fields: data })
                .await
                .expect("Failed to create doc");
            
            paths.push(path);
        }
        paths
    });

    let mut group = c.benchmark_group("crud");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("get", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let doc_id = format!("doc_{}", i % 256);
                                let snapshot = firestore
                                    .collection("benchmark_get")
                                    .document(&doc_id)
                                    .get()
                                    .await
                                    .expect("Failed to get doc");
                                black_box(snapshot)
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_crud_set(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);

    let mut group = c.benchmark_group("crud");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("set", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let doc_id = format!("doc_{}", i);
                                let mut data = HashMap::new();
                                data.insert(
                                    "value".to_string(),
                                    Value {
                                        value_type: Some(ValueType::IntegerValue(i as i64)),
                                    },
                                );
                                
                                firestore
                                    .collection("benchmark_set")
                                    .document(&doc_id)
                                    .set(MapValue { fields: data })
                                    .await
                                    .expect("Failed to set doc");
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_crud_update(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);
    
    // Pre-create documents for update benchmark
    RUNTIME.block_on(async {
        for i in 0..256 {
            let mut data = HashMap::new();
            data.insert(
                "value".to_string(),
                Value {
                    value_type: Some(ValueType::IntegerValue(0)),
                },
            );
            
            firestore
                .collection("benchmark_update")
                .document(&format!("doc_{}", i))
                .set(MapValue { fields: data })
                .await
                .expect("Failed to create doc");
        }
    });

    let mut group = c.benchmark_group("crud");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("update", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let doc_id = format!("doc_{}", i % 256);
                                let mut data = HashMap::new();
                                data.insert(
                                    "value".to_string(),
                                    Value {
                                        value_type: Some(ValueType::IntegerValue(i as i64)),
                                    },
                                );
                                
                                firestore
                                    .collection("benchmark_update")
                                    .document(&doc_id)
                                    .update(MapValue { fields: data })
                                    .await
                                    .expect("Failed to update doc");
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_crud_delete(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);

    let mut group = c.benchmark_group("crud");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("delete", concurrency),
            &concurrency,
            |b, &concurrency| {
                // Pre-create documents before each iteration
                b.iter_batched(
                    || {
                        RUNTIME.block_on(async {
                            for i in 0..concurrency {
                                let mut data = HashMap::new();
                                data.insert(
                                    "value".to_string(),
                                    Value {
                                        value_type: Some(ValueType::IntegerValue(i as i64)),
                                    },
                                );
                                
                                firestore
                                    .collection("benchmark_delete")
                                    .document(&format!("doc_{}", i))
                                    .set(MapValue { fields: data })
                                    .await
                                    .expect("Failed to create doc");
                            }
                        })
                    },
                    |_| {
                        RUNTIME.block_on(async {
                            let futures: Vec<_> = (0..concurrency)
                                .map(|i| {
                                    let firestore = Arc::clone(&firestore);
                                    async move {
                                        let doc_id = format!("doc_{}", i);
                                        firestore
                                            .collection("benchmark_delete")
                                            .document(&doc_id)
                                            .delete()
                                            .await
                                            .expect("Failed to delete doc");
                                    }
                                })
                                .collect();

                            join_all(futures).await
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ============================================================================
// Query Benchmarks (Auth time excluded)
// ============================================================================

fn bench_query_simple(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);
    
    // Pre-create documents for query benchmark
    RUNTIME.block_on(async {
        for i in 0..1000 {
            let mut data = HashMap::new();
            data.insert(
                "value".to_string(),
                Value {
                    value_type: Some(ValueType::IntegerValue(i)),
                },
            );
            data.insert(
                "category".to_string(),
                Value {
                    value_type: Some(ValueType::StringValue(format!("cat_{}", i % 10))),
                },
            );
            
            firestore
                .collection("benchmark_query")
                .document(&format!("doc_{}", i))
                .set(MapValue { fields: data })
                .await
                .expect("Failed to create doc");
        }
    });

    let mut group = c.benchmark_group("query");
    group.measurement_time(Duration::from_secs(30));

    for &concurrency in CONCURRENCY_LEVELS {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("simple_filter", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let category = format!("cat_{}", i % 10);
                                let snapshot = firestore
                                    .collection("benchmark_query")
                                    .where_equal_to(
                                        "category",
                                        Value {
                                            value_type: Some(ValueType::StringValue(category)),
                                        },
                                    )
                                    .get()
                                    .await
                                    .expect("Failed to query");
                                black_box(snapshot)
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Listen Benchmarks (Auth time excluded)
// ============================================================================

fn bench_listen_document(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);
    
    // Pre-create documents for listen benchmark
    RUNTIME.block_on(async {
        for i in 0..256 {
            let mut data = HashMap::new();
            data.insert(
                "value".to_string(),
                Value {
                    value_type: Some(ValueType::IntegerValue(0)),
                },
            );
            
            firestore
                .collection("benchmark_listen")
                .document(&format!("doc_{}", i))
                .set(MapValue { fields: data })
                .await
                .expect("Failed to create doc");
        }
    });

    let mut group = c.benchmark_group("listen");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20); // Fewer samples for listener benchmarks

    for &concurrency in &[1, 2, 4, 8, 16, 32] {
        // Limit concurrency for listeners
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("document", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    use futures::StreamExt;

                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let doc_id = format!("doc_{}", i % 256);
                                let mut stream = firestore
                                    .collection("benchmark_listen")
                                    .document(&doc_id)
                                    .listen(None);

                                // Wait for first snapshot (connection established)
                                if let Some(result) = stream.next().await {
                                    black_box(result.expect("Listener error"));
                                }
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_listen_query(c: &mut Criterion) {
    let firestore = Arc::clone(&FIRESTORE);
    
    // Use existing benchmark_query documents

    let mut group = c.benchmark_group("listen");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20);

    for &concurrency in &[1, 2, 4, 8, 16, 32] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("query", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&*RUNTIME).iter(|| async {
                    use futures::StreamExt;

                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let firestore = Arc::clone(&firestore);
                            async move {
                                let category = format!("cat_{}", i % 10);
                                let mut stream = firestore
                                    .collection("benchmark_query")
                                    .where_equal_to(
                                        "category",
                                        Value {
                                            value_type: Some(ValueType::StringValue(category)),
                                        },
                                    )
                                    .listen(None);

                                // Wait for first snapshot
                                if let Some(result) = stream.next().await {
                                    black_box(result.expect("Listener error"));
                                }
                            }
                        })
                        .collect();

                    join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    auth_benches,
    bench_auth_sign_in,
    bench_auth_token_refresh
);

criterion_group!(
    crud_benches,
    bench_crud_get,
    bench_crud_set,
    bench_crud_update,
    bench_crud_delete
);

criterion_group!(
    query_benches,
    bench_query_simple
);

criterion_group!(
    listen_benches,
    bench_listen_document,
    bench_listen_query
);

criterion_main!(auth_benches, crud_benches, query_benches, listen_benches);
