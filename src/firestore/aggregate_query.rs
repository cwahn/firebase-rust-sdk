//! Firestore Aggregation Query support
//!
//! # C++ Reference
//! - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/aggregate_query.h:36`
//! - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/aggregate_query_snapshot.h:35`

use super::field_value::proto;
use super::query::QueryState;
use crate::error::FirebaseError;
use crate::firestore::firestore::FirestoreInterceptor;
use proto::google::firestore::v1::firestore_client::FirestoreClient as GrpcClient;
use std::collections::HashMap;

// Type aliases to reduce verbosity
use proto::google::firestore::v1 as firestore_proto;
use firestore_proto::structured_aggregation_query::aggregation::{Count, Sum, Avg, Operator as AggOp};
use firestore_proto::structured_aggregation_query::Aggregation;
use firestore_proto::structured_query::{CollectionSelector, FieldReference, Order};
use firestore_proto::{RunAggregationQueryRequest, StructuredAggregationQuery, StructuredQuery};

/// Type of aggregation operation
///
/// # C++ Reference
/// - `aggregate_query.h:58` - Count(), Sum(), Average()
#[derive(Debug, Clone)]
pub enum AggregationType {
    /// Count the number of documents
    Count,
    /// Sum a numeric field across documents
    Sum(String),
    /// Average a numeric field across documents
    Average(String),
}

/// Field specification for aggregation
///
/// # C++ Reference
/// - `aggregate_query.h:58` - Aggregation field specification
#[derive(Debug, Clone)]
pub struct AggregateField {
    /// Alias for the aggregation result (optional)
    pub alias: Option<String>,
    /// Type of aggregation
    pub aggregation_type: AggregationType,
}

impl AggregateField {
    /// Create a count aggregation
    ///
    /// # C++ Reference
    /// - `aggregate_query.h:69` - Count()
    pub fn count() -> Self {
        Self {
            alias: None,
            aggregation_type: AggregationType::Count,
        }
    }

    /// Create a count aggregation with alias
    pub fn count_with_alias(alias: impl Into<String>) -> Self {
        Self {
            alias: Some(alias.into()),
            aggregation_type: AggregationType::Count,
        }
    }

    /// Create a sum aggregation on a field
    ///
    /// # C++ Reference
    /// - `aggregate_query.h:79` - Sum(field_path)
    pub fn sum(field: impl Into<String>) -> Self {
        Self {
            alias: None,
            aggregation_type: AggregationType::Sum(field.into()),
        }
    }

    /// Create a sum aggregation with alias
    pub fn sum_with_alias(field: impl Into<String>, alias: impl Into<String>) -> Self {
        Self {
            alias: Some(alias.into()),
            aggregation_type: AggregationType::Sum(field.into()),
        }
    }

    /// Create an average aggregation on a field
    ///
    /// # C++ Reference
    /// - `aggregate_query.h:89` - Average(field_path)
    pub fn average(field: impl Into<String>) -> Self {
        Self {
            alias: None,
            aggregation_type: AggregationType::Average(field.into()),
        }
    }

    /// Create an average aggregation with alias
    pub fn average_with_alias(field: impl Into<String>, alias: impl Into<String>) -> Self {
        Self {
            alias: Some(alias.into()),
            aggregation_type: AggregationType::Average(field.into()),
        }
    }

    /// Set an alias for this aggregation field
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

/// Aggregation query for performing aggregate operations on collections
///
/// # C++ Reference
/// - `aggregate_query.h:36` - AggregateQuery class
#[derive(Clone)]
pub struct AggregateQuery {
    /// Base query state
    pub(crate) query_state: QueryState,
    /// Aggregations to perform
    pub(crate) aggregations: Vec<AggregateField>,
}

impl AggregateQuery {
    /// Create a new aggregation query
    pub(crate) fn new(query_state: QueryState, aggregations: Vec<AggregateField>) -> Self {
        Self {
            query_state,
            aggregations,
        }
    }

    /// Execute the aggregation query and return results
    ///
    /// # C++ Reference
    /// - `aggregate_query.h:117` - Get()
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("project-id", "(default)", None).await?;
    ///
    /// let result = firestore
    ///     .collection("users")
    ///     .count()
    ///     .get()
    ///     .await?;
    ///
    /// println!("Total users: {}", result.count().unwrap_or(0));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self) -> Result<AggregateQuerySnapshot, FirebaseError> {
        let database_path = format!(
            "projects/{}/databases/{}",
            self.query_state.firestore.project_id, self.query_state.firestore.database_id
        );
        let parent = format!("{}/documents", database_path);

        // Convert query state to StructuredQuery
        let structured_query = query_state_to_structured_query(&self.query_state);

        // Build aggregations
        let mut aggregations_proto = Vec::new();

        for agg_field in &self.aggregations {
            let aggregation = match &agg_field.aggregation_type {
                AggregationType::Count => {
                    let count = Count {
                        up_to: None, // No limit on count
                    };
                    Aggregation {
                        alias: agg_field.alias.clone().unwrap_or_else(|| "count".to_string()),
                        operator: Some(AggOp::Count(count)),
                    }
                }
                AggregationType::Sum(field) => {
                    let sum = Sum {
                        field: Some(FieldReference {
                            field_path: field.clone(),
                        }),
                    };
                    Aggregation {
                        alias: agg_field.alias.clone().unwrap_or_else(|| format!("sum_{}", field)),
                        operator: Some(AggOp::Sum(sum)),
                    }
                }
                AggregationType::Average(field) => {
                    let avg = Avg {
                        field: Some(FieldReference {
                            field_path: field.clone(),
                        }),
                    };
                    Aggregation {
                        alias: agg_field
                            .alias
                            .clone()
                            .unwrap_or_else(|| format!("average_{}", field)),
                        operator: Some(AggOp::Avg(avg)),
                    }
                }
            };

            aggregations_proto.push(aggregation);
        }

        let structured_aggregation_query = StructuredAggregationQuery {
            query_type: Some(
                firestore_proto::structured_aggregation_query::QueryType::StructuredQuery(
                    structured_query,
                ),
            ),
            aggregations: aggregations_proto,
        };

        let request = RunAggregationQueryRequest {
            parent,
            consistency_selector: None,
            explain_options: None,
            query_type: Some(
                firestore_proto::run_aggregation_query_request::QueryType::StructuredAggregationQuery(
                    structured_aggregation_query,
                ),
            ),
        };

        let interceptor = FirestoreInterceptor {
            auth_data: self.query_state.firestore.auth_data.clone(),
        };
        let mut client = GrpcClient::with_interceptor(
            self.query_state.firestore.channel.clone(),
            interceptor,
        );

        let mut response = client.run_aggregation_query(request).await.map_err(|e| {
            crate::error::FirestoreError::Connection(format!("Aggregation query failed: {}", e))
        })?;

        let stream = response.get_mut();

        // Get first (and typically only) response
        use futures::stream::StreamExt;
        let result = stream.next().await;

        match result {
            Some(Ok(response)) => {
                let mut results = HashMap::new();

                if let Some(result) = response.result {
                    for (alias, value) in result.aggregate_fields {
                        results.insert(alias, value);
                    }
                }

                Ok(AggregateQuerySnapshot { results })
            }
            Some(Err(e)) => Err(crate::error::FirestoreError::Connection(format!(
                "Aggregation query stream error: {}",
                e
            ))
            .into()),
            None => Ok(AggregateQuerySnapshot {
                results: HashMap::new(),
            }),
        }
    }
}

/// Snapshot of aggregation query results
///
/// # C++ Reference
/// - `aggregate_query_snapshot.h:35` - AggregateQuerySnapshot class
#[derive(Debug, Clone)]
pub struct AggregateQuerySnapshot {
    /// Aggregation results keyed by alias
    results: HashMap<String, proto::google::firestore::v1::Value>,
}

impl AggregateQuerySnapshot {
    /// Get count result (convenience method for count aggregations)
    ///
    /// # C++ Reference
    /// - `aggregate_query_snapshot.h:58` - count()
    pub fn count(&self) -> Option<i64> {
        self.get("count")
            .and_then(|v| v.value_type.as_ref())
            .and_then(|vt| match vt {
                proto::google::firestore::v1::value::ValueType::IntegerValue(i) => Some(*i),
                _ => None,
            })
    }

    /// Get a specific aggregation result by alias
    ///
    /// # C++ Reference
    /// - `aggregate_query_snapshot.h:68` - get(field_path)
    pub fn get(&self, alias: &str) -> Option<&proto::google::firestore::v1::Value> {
        self.results.get(alias)
    }

    /// Get integer value from aggregation result
    pub fn get_int(&self, alias: &str) -> Option<i64> {
        self.get(alias)
            .and_then(|v| v.value_type.as_ref())
            .and_then(|vt| match vt {
                proto::google::firestore::v1::value::ValueType::IntegerValue(i) => Some(*i),
                _ => None,
            })
    }

    /// Get double value from aggregation result
    pub fn get_double(&self, alias: &str) -> Option<f64> {
        self.get(alias)
            .and_then(|v| v.value_type.as_ref())
            .and_then(|vt| match vt {
                proto::google::firestore::v1::value::ValueType::DoubleValue(d) => Some(*d),
                proto::google::firestore::v1::value::ValueType::IntegerValue(i) => {
                    Some(*i as f64)
                }
                _ => None,
            })
    }

    /// Get all aggregation results
    pub fn results(&self) -> &HashMap<String, proto::google::firestore::v1::Value> {
        &self.results
    }
}

/// Convert QueryState to StructuredQuery for aggregation
fn query_state_to_structured_query(
    state: &QueryState,
) -> proto::google::firestore::v1::StructuredQuery {
    let collection_id = state
        .collection_path
        .split('/')
        .last()
        .unwrap_or("documents")
        .to_string();

    let mut query = StructuredQuery {
        select: None,
        from: vec![CollectionSelector {
            collection_id,
            all_descendants: false,
        }],
        r#where: None,
        order_by: Vec::new(),
        start_at: None,
        end_at: None,
        offset: 0,
        limit: None,
        find_nearest: None,
    };

    // Add filters (simplified - real implementation would handle all filter types)
    if !state.filters.is_empty() {
        // For now, just log that filters exist - full implementation would convert them
        // This matches the listener.rs implementation
    }

    // Add order by
    for (field, direction) in &state.orders {
        query.order_by.push(Order {
            field: Some(FieldReference {
                field_path: field.clone(),
            }),
            direction: *direction as i32,
        });
    }

    // Add limit
    if let Some(limit) = state.limit_value {
        query.limit = Some(limit);
    }

    query
}
