//! Integration tests for Firestore
//!
//! These tests interact with real Firestore and require:
//! 1. A Firebase project with Firestore enabled
//! 2. Environment variables set in .env file
//! 3. Run with: cargo test --features integration-tests -- --test-threads=1
//!
//! See INTEGRATION_TESTS.md for setup instructions.

#![cfg(feature = "integration-tests")]

use firebase_rust_sdk::{Auth, firestore::Firestore};
use serde_json::json;
use std::env;

/// Load environment variables from .env file
fn load_env() {
    dotenvy::dotenv().ok();
}

/// Get test credentials and sign in
async fn get_authenticated_firestore() -> (Auth, Firestore, String) {
    load_env();
    
    let api_key = env::var("FIREBASE_API_KEY")
        .expect("FIREBASE_API_KEY must be set in .env file");
    let project_id = env::var("FIREBASE_PROJECT_ID")
        .expect("FIREBASE_PROJECT_ID must be set in .env file");
    let email = env::var("TEST_USER_EMAIL")
        .expect("TEST_USER_EMAIL must be set in .env file");
    let password = env::var("TEST_USER_PASSWORD")
        .expect("TEST_USER_PASSWORD must be set in .env file");
    
    // Sign in to get authentication
    let auth = Auth::get_auth(&api_key).await
        .expect("Failed to get Auth instance");
    
    auth.sign_in_with_email_and_password(&email, &password).await
        .expect("Failed to sign in");
    
    // Get Firestore instance
    let firestore = Firestore::get_firestore(&project_id).await
        .expect("Failed to get Firestore instance");
    
    (auth, firestore, api_key)
}

/// Generate unique collection name for this test run
fn test_collection(test_name: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    format!("test_{}_{}_{}", test_name, timestamp, rand::random::<u32>())
}

/// Clean up: delete all documents in a collection
async fn cleanup_collection(firestore: &Firestore, collection_path: &str) {
    let query = firestore.collection(collection_path).query();
    
    if let Ok(docs) = query.get().await {
        for doc in docs {
            let _ = firestore.delete_document(&doc.reference.path).await;
        }
    }
}

/// Test: Create and read document
#[tokio::test]
async fn test_create_read_document() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("create_read");
    
    let doc_path = format!("{}/test_doc", collection);
    
    // Create document
    let data = json!({
        "name": "Alice",
        "age": 30,
        "active": true
    });
    
    firestore.set_document(&doc_path, data.clone()).await
        .expect("Failed to create document");
    
    // Read document
    let doc = firestore.get_document(&doc_path).await
        .expect("Failed to read document");
    
    assert!(doc.exists());
    assert_eq!(doc.get("name").and_then(|v| v.as_str()), Some("Alice"));
    assert_eq!(doc.get("age").and_then(|v| v.as_i64()), Some(30));
    assert_eq!(doc.get("active").and_then(|v| v.as_bool()), Some(true));
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Create/read document works!");
}

/// Test: Update document
#[tokio::test]
async fn test_update_document() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("update");
    
    let doc_path = format!("{}/test_doc", collection);
    
    // Create document
    firestore.set_document(&doc_path, json!({"count": 0})).await
        .expect("Failed to create document");
    
    // Update document
    firestore.update_document(&doc_path, json!({"count": 42})).await
        .expect("Failed to update document");
    
    // Read updated document
    let doc = firestore.get_document(&doc_path).await
        .expect("Failed to read document");
    
    assert_eq!(doc.get("count").and_then(|v| v.as_i64()), Some(42));
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Update document works!");
}

/// Test: Delete document
#[tokio::test]
async fn test_delete_document() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("delete");
    
    let doc_path = format!("{}/test_doc", collection);
    
    // Create document
    firestore.set_document(&doc_path, json!({"test": true})).await
        .expect("Failed to create document");
    
    // Delete document
    firestore.delete_document(&doc_path).await
        .expect("Failed to delete document");
    
    // Verify it's gone
    let doc = firestore.get_document(&doc_path).await
        .expect("Failed to read document");
    
    assert!(!doc.exists());
    
    // Clean up collection
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Delete document works!");
}

/// Test: Query with filters
#[tokio::test]
async fn test_query_filters() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("query");
    
    // Create test documents
    for i in 1..=5 {
        let doc_path = format!("{}/doc{}", collection, i);
        firestore.set_document(&doc_path, json!({
            "name": format!("User {}", i),
            "age": 20 + i,
            "active": i % 2 == 0
        })).await.expect("Failed to create document");
    }
    
    // Query: age > 22
    use firebase_rust_sdk::firestore::FilterCondition;
    let results = firestore.collection(&collection)
        .query()
        .where_filter(FilterCondition::GreaterThan("age".into(), json!(22)))
        .get()
        .await
        .expect("Failed to query");
    
    assert_eq!(results.len(), 3); // ages 23, 24, 25
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Query with filters works!");
}

/// Test: Query with pagination
#[tokio::test]
async fn test_query_pagination() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("pagination");
    
    // Create test documents
    for i in 1..=10 {
        let doc_path = format!("{}/doc{:02}", collection, i);
        firestore.set_document(&doc_path, json!({
            "index": i,
            "name": format!("Item {}", i)
        })).await.expect("Failed to create document");
    }
    
    // Get first 3 documents
    use firebase_rust_sdk::firestore::OrderDirection;
    let page1 = firestore.collection(&collection)
        .query()
        .order_by("index", OrderDirection::Ascending)
        .limit(3)
        .get()
        .await
        .expect("Failed to query page 1");
    
    assert_eq!(page1.len(), 3);
    
    // Get next 3 documents
    let last_doc = &page1[2];
    let page2 = firestore.collection(&collection)
        .query()
        .order_by("index", OrderDirection::Ascending)
        .start_after(last_doc)
        .limit(3)
        .get()
        .await
        .expect("Failed to query page 2");
    
    assert_eq!(page2.len(), 3);
    assert_ne!(page1[0].reference.path, page2[0].reference.path);
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Query pagination works!");
}

/// Test: Batch writes
#[tokio::test]
async fn test_batch_writes() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("batch");
    
    // Create batch
    use firebase_rust_sdk::firestore::WriteBatch;
    let mut batch = WriteBatch::new();
    
    // Add multiple writes
    for i in 1..=3 {
        let doc_path = format!("{}/doc{}", collection, i);
        batch.set(&doc_path, json!({"index": i, "batch": true}));
    }
    
    // Commit batch
    firestore.commit_batch(batch).await
        .expect("Failed to commit batch");
    
    // Verify all documents exist
    for i in 1..=3 {
        let doc_path = format!("{}/doc{}", collection, i);
        let doc = firestore.get_document(&doc_path).await
            .expect("Failed to read document");
        
        assert!(doc.exists());
        assert_eq!(doc.get("index").and_then(|v| v.as_i64()), Some(i));
    }
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Batch writes work!");
}

/// Test: Transactions
#[tokio::test]
async fn test_transactions() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("transaction");
    
    let counter_path = format!("{}/counter", collection);
    
    // Create counter document
    firestore.set_document(&counter_path, json!({"count": 0})).await
        .expect("Failed to create counter");
    
    // Increment counter in transaction
    firestore.run_transaction(|mut txn| {
        let path = counter_path.clone();
        async move {
            let doc = txn.get(&path).await?;
            let count = doc.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            txn.set(&path, json!({"count": count + 1}));
            Ok(())
        }
    }).await.expect("Failed to run transaction");
    
    // Verify counter was incremented
    let doc = firestore.get_document(&counter_path).await
        .expect("Failed to read counter");
    
    assert_eq!(doc.get("count").and_then(|v| v.as_i64()), Some(1));
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Transactions work!");
}

/// Test: Add document with auto-generated ID
#[tokio::test]
async fn test_add_document() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("add");
    
    // Add document with auto-generated ID
    let doc_ref = firestore.collection(&collection)
        .add(json!({
            "message": "Auto-generated ID",
            "timestamp": chrono::Utc::now().timestamp()
        }))
        .await
        .expect("Failed to add document");
    
    // Verify document was created
    assert!(doc_ref.path.starts_with(&collection));
    assert!(!doc_ref.id().is_empty());
    
    let doc = firestore.get_document(&doc_ref.path).await
        .expect("Failed to read document");
    
    assert!(doc.exists());
    assert_eq!(doc.get("message").and_then(|v| v.as_str()), Some("Auto-generated ID"));
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Add document with auto ID works!");
}

/// Test: Nested collections
#[tokio::test]
async fn test_nested_collections() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("nested");
    
    let parent_path = format!("{}/parent_doc", collection);
    let child_path = format!("{}/subcollection/child_doc", parent_path);
    
    // Create parent document
    firestore.set_document(&parent_path, json!({"type": "parent"})).await
        .expect("Failed to create parent");
    
    // Create child document in subcollection
    firestore.set_document(&child_path, json!({"type": "child"})).await
        .expect("Failed to create child");
    
    // Read child document
    let child = firestore.get_document(&child_path).await
        .expect("Failed to read child");
    
    assert!(child.exists());
    assert_eq!(child.get("type").and_then(|v| v.as_str()), Some("child"));
    
    // Clean up (delete child and parent)
    firestore.delete_document(&child_path).await.ok();
    firestore.delete_document(&parent_path).await.ok();
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Nested collections work!");
}

/// Test: Real-time snapshot listener
#[tokio::test]
async fn test_snapshot_listener() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("listener");
    
    let doc_path = format!("{}/watched_doc", collection);
    
    // Create initial document
    firestore.set_document(&doc_path, json!({"value": 0})).await
        .expect("Failed to create document");
    
    // Set up listener
    use futures::stream::StreamExt;
    let mut stream = firestore.listen_to_document(&doc_path).await
        .expect("Failed to create listener");
    
    // Spawn task to update document after short delay
    let fs = firestore.clone();
    let path = doc_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = fs.update_document(&path, json!({"value": 42})).await;
    });
    
    // Wait for updates (with timeout)
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(5));
    tokio::pin!(timeout);
    
    let mut updates = 0;
    let mut found_update = false;
    
    loop {
        tokio::select! {
            Some(Ok(doc)) = stream.next() => {
                updates += 1;
                if doc.get("value").and_then(|v| v.as_i64()) == Some(42) {
                    found_update = true;
                    break;
                }
                if updates >= 3 {
                    break;
                }
            }
            _ = &mut timeout => {
                break;
            }
        }
    }
    
    assert!(found_update, "Should have received update notification");
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Snapshot listener works! (received {} updates)", updates);
}

/// Test: Compound filters (And/Or)
#[tokio::test]
async fn test_compound_filters() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("compound");
    
    // Create test documents
    firestore.set_document(&format!("{}/doc1", collection), json!({
        "age": 25,
        "active": true
    })).await.expect("Failed to create doc1");
    
    firestore.set_document(&format!("{}/doc2", collection), json!({
        "age": 30,
        "active": false
    })).await.expect("Failed to create doc2");
    
    firestore.set_document(&format!("{}/doc3", collection), json!({
        "age": 35,
        "active": true
    })).await.expect("Failed to create doc3");
    
    // Query: age > 20 AND active == true
    use firebase_rust_sdk::firestore::{FilterCondition, Filter};
    let and_filter = Filter::And(vec![
        Filter::Field(FilterCondition::GreaterThan("age".into(), json!(20))),
        Filter::Field(FilterCondition::Equal("active".into(), json!(true))),
    ]);
    
    let results = firestore.collection(&collection)
        .query()
        .where_composite(and_filter)
        .get()
        .await
        .expect("Failed to query");
    
    assert_eq!(results.len(), 2); // doc1 and doc3
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ Compound filters work!");
}
