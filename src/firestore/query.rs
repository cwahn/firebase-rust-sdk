//! Firestore Query trait and implementation
//!
//! Following C++ SDK pattern where Query is immutable - each method returns a new Query
//! with modified state, similar to how Iterator adapters work.
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/query.h:61` - Query class
//! - `firestore/src/main/query_main.h:47` - QueryInternal with api::Query

use super::document_snapshot::DocumentSnapshot;
use super::field_value::proto;
use super::field_value::Value;
use super::query_snapshot::QuerySnapshot;
use super::settings::Source;
use crate::error::FirebaseError;
use crate::firestore::firestore::FirestoreInner;
use std::sync::Arc;

/// Sort direction for query ordering
///
/// # C++ Reference
/// - `query.h:69` - Query::Direction enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

/// Filter operator type (without the field name and value)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterOperator {
    EqualTo,
    NotEqualTo,
    LessThan,
    LessThanOrEqualTo,
    GreaterThan,
    GreaterThanOrEqualTo,
    ArrayContains,
    ArrayContainsAny,
    In,
    NotIn,
}

/// Internal query state that all query types share
///
/// Similar to C++ QueryInternal which wraps api::Query
///
/// # C++ Reference
/// - `query_main.h:206` - api::Query query_ member
#[derive(Clone)]
pub(crate) struct QueryState {
    pub collection_path: String,
    pub firestore: Arc<FirestoreInner>,
    pub filters: Vec<(String, FilterOperator, Value)>,
    pub orders: Vec<(String, Direction)>,
    pub limit_value: Option<i32>,
    pub limit_to_last_value: Option<i32>,
    pub start_at: Option<Vec<Value>>,
    pub start_after: Option<Vec<Value>>,
    pub end_at: Option<Vec<Value>>,
    pub end_before: Option<Vec<Value>>,
}

impl QueryState {
    pub(crate) fn new(collection_path: String, firestore: Arc<FirestoreInner>) -> Self {
        Self {
            collection_path,
            firestore,
            filters: Vec::new(),
            orders: Vec::new(),
            limit_value: None,
            limit_to_last_value: None,
            start_at: None,
            start_after: None,
            end_at: None,
            end_before: None,
        }
    }
}

/// Base query trait for executing queries
///
/// Following C++ SDK pattern where Query methods return Self (immutable).
/// Each query operation clones the state and returns a new instance.
///
/// # C++ Reference
/// - `query.h:61` - Query class (immutable, methods return new Query)
/// - `query_main.h:47` - QueryInternal
pub trait Query: Clone + Sized {
    /// Get the internal query state
    #[doc(hidden)]
    fn query_state(&self) -> &QueryState;

    /// Create a new instance with modified state
    #[doc(hidden)]
    fn with_state(&self, state: QueryState) -> Self;

    /// Execute the query and return results
    ///
    /// # C++ Reference
    /// - `query.h:642` - Query::Get()
    fn get(&self) -> impl std::future::Future<Output = Result<QuerySnapshot, FirebaseError>> + Send
    where
        Self: Sync,
    {
        async { self.get_with_source(Source::Default).await }
    }

    /// Execute the query with specified source
    ///
    /// # C++ Reference
    /// - `query.h:656` - Query::Get(Source source)
    /// - `query_main.cc:99` - QueryInternal::Get implementation
    fn get_with_source(
        &self,
        _source: Source,
    ) -> impl std::future::Future<Output = Result<QuerySnapshot, FirebaseError>> + Send {
        let state = self.query_state().clone();
        async move { execute_query(&state).await }
    }

    /// Filter documents where field equals value
    ///
    /// # C++ Reference
    /// - `query.h:177` - Query::WhereEqualTo(field, value)
    fn where_equal_to(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::EqualTo, value));
        self.with_state(state)
    }

    /// Filter documents where field does not equal value
    ///
    /// # C++ Reference
    /// - `query.h:197` - Query::WhereNotEqualTo(field, value)
    fn where_not_equal_to(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::NotEqualTo, value));
        self.with_state(state)
    }

    /// Filter documents where field is less than value
    ///
    /// # C++ Reference
    /// - `query.h:218` - Query::WhereLessThan(field, value)
    fn where_less_than(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::LessThan, value));
        self.with_state(state)
    }

    /// Filter documents where field is less than or equal to value
    ///
    /// # C++ Reference
    /// - `query.h:239` - Query::WhereLessThanOrEqualTo(field, value)
    fn where_less_than_or_equal_to(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::LessThanOrEqualTo, value));
        self.with_state(state)
    }

    /// Filter documents where field is greater than value
    ///
    /// # C++ Reference
    /// - `query.h:260` - Query::WhereGreaterThan(field, value)
    fn where_greater_than(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::GreaterThan, value));
        self.with_state(state)
    }

    /// Filter documents where field is greater than or equal to value
    ///
    /// # C++ Reference
    /// - `query.h:281` - Query::WhereGreaterThanOrEqualTo(field, value)
    fn where_greater_than_or_equal_to(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::GreaterThanOrEqualTo, value));
        self.with_state(state)
    }

    /// Filter documents where array field contains value
    ///
    /// # C++ Reference
    /// - `query.h:305` - Query::WhereArrayContains(field, value)
    fn where_array_contains(self, field: impl Into<String>, value: Value) -> Self {
        let mut state = self.query_state().clone();
        state
            .filters
            .push((field.into(), FilterOperator::ArrayContains, value));
        self.with_state(state)
    }

    /// Filter documents where array field contains any of the values
    ///
    /// # C++ Reference
    /// - `query.h:347` - Query::WhereArrayContainsAny(field, values)
    fn where_array_contains_any(self, field: impl Into<String>, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        use proto::google::firestore::v1::value::ValueType;
        use proto::google::firestore::v1::ArrayValue;
        state.filters.push((
            field.into(),
            FilterOperator::ArrayContainsAny,
            Value {
                value_type: Some(ValueType::ArrayValue(ArrayValue { values })),
            },
        ));
        self.with_state(state)
    }

    /// Filter documents where field equals any of the values
    ///
    /// # C++ Reference
    /// - `query.h:382` - Query::WhereIn(field, values)
    fn where_in(self, field: impl Into<String>, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        use proto::google::firestore::v1::value::ValueType;
        use proto::google::firestore::v1::ArrayValue;
        state.filters.push((
            field.into(),
            FilterOperator::In,
            Value {
                value_type: Some(ValueType::ArrayValue(ArrayValue { values })),
            },
        ));
        self.with_state(state)
    }

    /// Filter documents where field does not equal any of the values
    ///
    /// # C++ Reference
    /// - `query.h:416` - Query::WhereNotIn(field, values)
    fn where_not_in(self, field: impl Into<String>, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        use proto::google::firestore::v1::value::ValueType;
        use proto::google::firestore::v1::ArrayValue;
        state.filters.push((
            field.into(),
            FilterOperator::NotIn,
            Value {
                value_type: Some(ValueType::ArrayValue(ArrayValue { values })),
            },
        ));
        self.with_state(state)
    }

    /// Order query results by field
    ///
    /// # C++ Reference
    /// - `query.h:456` - Query::OrderBy(field)
    /// - `query.h:477` - Query::OrderBy(field, direction)
    fn order_by(self, field: impl Into<String>, direction: Direction) -> Self {
        let mut state = self.query_state().clone();
        state.orders.push((field.into(), direction));
        self.with_state(state)
    }

    /// Limit query results to first n documents
    ///
    /// # C++ Reference
    /// - `query.h:496` - Query::Limit(limit)
    fn limit(self, limit: i32) -> Self {
        let mut state = self.query_state().clone();
        state.limit_value = Some(limit);
        self.with_state(state)
    }

    /// Limit query results to last n documents
    ///
    /// # C++ Reference
    /// - `query.h:518` - Query::LimitToLast(limit)
    fn limit_to_last(self, limit: i32) -> Self {
        let mut state = self.query_state().clone();
        state.limit_to_last_value = Some(limit);
        self.with_state(state)
    }

    /// Start query results at document snapshot
    ///
    /// # C++ Reference
    /// - `query.h:546` - Query::StartAt(snapshot)
    fn start_at_document(self, _snapshot: DocumentSnapshot) -> Self {
        // TODO: Extract values from snapshot
        self
    }

    /// Start query results at field values
    ///
    /// # C++ Reference
    /// - `query.h:567` - Query::StartAt(values)
    fn start_at(self, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        state.start_at = Some(values);
        self.with_state(state)
    }

    /// Start query results after document snapshot
    ///
    /// # C++ Reference
    /// - `query.h:582` - Query::StartAfter(snapshot)
    fn start_after_document(self, _snapshot: DocumentSnapshot) -> Self {
        // TODO: Extract values from snapshot
        self
    }

    /// Start query results after field values
    ///
    /// # C++ Reference
    /// - `query.h:602` - Query::StartAfter(values)
    fn start_after(self, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        state.start_after = Some(values);
        self.with_state(state)
    }

    /// End query results before document snapshot
    ///
    /// # C++ Reference
    /// - `query.h:617` - Query::EndBefore(snapshot)
    fn end_before_document(self, _snapshot: DocumentSnapshot) -> Self {
        // TODO: Extract values from snapshot
        self
    }

    /// End query results before field values
    ///
    /// # C++ Reference
    /// - `query.h:637` - Query::EndBefore(values)
    fn end_before(self, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        state.end_before = Some(values);
        self.with_state(state)
    }

    /// End query results at document snapshot
    ///
    /// # C++ Reference
    /// - `query.h:652` - Query::EndAt(snapshot)
    fn end_at_document(self, _snapshot: DocumentSnapshot) -> Self {
        // TODO: Extract values from snapshot
        self
    }

    /// End query results at field values
    ///
    /// # C++ Reference
    /// - `query.h:672` - Query::EndAt(values)
    fn end_at(self, values: Vec<Value>) -> Self {
        let mut state = self.query_state().clone();
        state.end_at = Some(values);
        self.with_state(state)
    }

    /// Listen to real-time updates for this query.
    ///
    /// Returns a stream that yields query snapshots as results change.
    /// The stream automatically cleans up the listener when dropped.
    ///
    /// # Arguments
    /// * `metadata_changes` - Optional parameter to control metadata-only change events.
    ///   Use `Some(MetadataChanges::Include)` to receive metadata-only updates.
    ///   Defaults to `MetadataChanges::Exclude` if `None`.
    ///
    /// # Returns
    /// A stream of `Result<QuerySnapshot, FirebaseError>` that yields updates.
    ///
    /// # Example
    /// ```no_run
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use firebase_rust_sdk::firestore::MetadataChanges;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("my-project", "(default)", None).await?;
    /// let query = firestore.collection("cities").where_equal_to("state", "CA".into());
    ///
    /// let mut stream = query.listen(Some(MetadataChanges::Include));
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(snapshot) => println!("Query results: {} documents", snapshot.documents().len()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # C++ Reference
    /// - `query.h:634` - `AddSnapshotListener` returns `ListenerRegistration`
    /// - Rust uses async streams with Drop cleanup instead of explicit remove()
    fn listen(
        &self,
        metadata_changes: Option<super::MetadataChanges>,
    ) -> super::QuerySnapshotStream {
        use futures::stream::StreamExt;
        use tokio::sync::{mpsc, oneshot};

        let (tx, rx) = mpsc::unbounded_channel();
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        // Clone necessary data for the async task
        let state = self.query_state().clone();
        let firestore = crate::firestore::Firestore {
            inner: state.firestore.clone(),
        };
        let options = super::listener::ListenerOptions {
            include_metadata_changes: metadata_changes.unwrap_or_default()
                == super::MetadataChanges::Include,
        };

        // Spawn background task to handle the listener
        tokio::spawn(async move {
            // Get authentication token if available
            let auth_token = state.firestore.id_token.clone().unwrap_or_default();

            let project_id = state.firestore.project_id.clone();
            let database_id = state.firestore.database_id.clone();

            // Start the query listener
            let listener_result = super::listener::listen_query(
                &firestore,
                auth_token,
                project_id,
                database_id,
                state,
                options,
            )
            .await;

            match listener_result {
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
                Ok(mut stream) => {
                    // Forward stream events until cancellation
                    loop {
                        tokio::select! {
                            snapshot_result = stream.next() => {
                                let Some(result) = snapshot_result else {
                                    break; // Stream ended
                                };
                                if tx.send(result).is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            _ = &mut cancel_rx => {
                                break; // Cancelled
                            }
                        }
                    }
                }
            }

        });

        super::QuerySnapshotStream::new(rx, cancel_tx)
    }
}

/// Execute a query with the given state
///
/// # C++ Reference
/// - `query_main.cc:99` - QueryInternal::Get implementation
pub(crate) async fn execute_query(state: &QueryState) -> Result<QuerySnapshot, FirebaseError> {
    use proto::google::firestore::v1 as firestore_proto;

    let project_id = &state.firestore.project_id;
    let database_id = &state.firestore.database_id;
    let parent = format!(
        "projects/{}/databases/{}/documents",
        project_id, database_id
    );

    // Build structured query
    let mut structured_query = firestore_proto::StructuredQuery {
        from: vec![firestore_proto::structured_query::CollectionSelector {
            collection_id: state.collection_path.clone(),
            all_descendants: false,
        }],
        ..Default::default()
    };

    // Apply filters
    if !state.filters.is_empty() {
        let filter_protos: Vec<_> = state
            .filters
            .iter()
            .map(|(field, operator, value)| {
                use firestore_proto::structured_query::field_filter::Operator;

                let op = match operator {
                    FilterOperator::LessThan => Operator::LessThan,
                    FilterOperator::LessThanOrEqualTo => Operator::LessThanOrEqual,
                    FilterOperator::EqualTo => Operator::Equal,
                    FilterOperator::NotEqualTo => Operator::NotEqual,
                    FilterOperator::GreaterThanOrEqualTo => Operator::GreaterThanOrEqual,
                    FilterOperator::GreaterThan => Operator::GreaterThan,
                    FilterOperator::ArrayContains => Operator::ArrayContains,
                    FilterOperator::ArrayContainsAny => Operator::ArrayContainsAny,
                    FilterOperator::In => Operator::In,
                    FilterOperator::NotIn => Operator::NotIn,
                } as i32;

                firestore_proto::structured_query::Filter {
                    filter_type: Some(
                        firestore_proto::structured_query::filter::FilterType::FieldFilter(
                            firestore_proto::structured_query::FieldFilter {
                                field: Some(firestore_proto::structured_query::FieldReference {
                                    field_path: field.clone(),
                                }),
                                op,
                                value: Some(value.clone()),
                            },
                        ),
                    ),
                }
            })
            .collect();

        if filter_protos.len() == 1 {
            structured_query.r#where = Some(filter_protos.into_iter().next().unwrap());
        } else if filter_protos.len() > 1 {
            structured_query.r#where = Some(firestore_proto::structured_query::Filter {
                filter_type: Some(
                    firestore_proto::structured_query::filter::FilterType::CompositeFilter(
                        firestore_proto::structured_query::CompositeFilter {
                            op: firestore_proto::structured_query::composite_filter::Operator::And
                                as i32,
                            filters: filter_protos,
                        },
                    ),
                ),
            });
        }
    }

    // Apply ordering
    if !state.orders.is_empty() {
        structured_query.order_by = state
            .orders
            .iter()
            .map(
                |(field, direction)| firestore_proto::structured_query::Order {
                    field: Some(firestore_proto::structured_query::FieldReference {
                        field_path: field.clone(),
                    }),
                    direction: match direction {
                        Direction::Ascending => {
                            firestore_proto::structured_query::Direction::Ascending as i32
                        }
                        Direction::Descending => {
                            firestore_proto::structured_query::Direction::Descending as i32
                        }
                    },
                },
            )
            .collect();
    }

    // Apply limit
    if let Some(limit) = state.limit_value {
        structured_query.limit = Some(limit);
    }

    // Apply limit to last
    if let Some(limit) = state.limit_to_last_value {
        structured_query.limit = Some(limit);
        // Reverse order for limit to last
        for order in &mut structured_query.order_by {
            order.direction = match order.direction {
                d if d == firestore_proto::structured_query::Direction::Ascending as i32 => {
                    firestore_proto::structured_query::Direction::Descending as i32
                }
                _ => firestore_proto::structured_query::Direction::Ascending as i32,
            };
        }
    }

    // Apply start/end cursors
    if let Some(values) = &state.start_at {
        structured_query.start_at = Some(firestore_proto::Cursor {
            values: values.clone(),
            before: true,
        });
    }
    if let Some(values) = &state.start_after {
        structured_query.start_at = Some(firestore_proto::Cursor {
            values: values.clone(),
            before: false,
        });
    }
    if let Some(values) = &state.end_at {
        structured_query.end_at = Some(firestore_proto::Cursor {
            values: values.clone(),
            before: false,
        });
    }
    if let Some(values) = &state.end_before {
        structured_query.end_at = Some(firestore_proto::Cursor {
            values: values.clone(),
            before: true,
        });
    }

    let request = firestore_proto::RunQueryRequest {
        parent,
        query_type: Some(
            firestore_proto::run_query_request::QueryType::StructuredQuery(structured_query),
        ),
        ..Default::default()
    };

    let mut client = state.firestore.grpc_client.clone();
    let mut stream = client
        .run_query(request)
        .await
        .map_err(|e| crate::error::FirestoreError::Internal(e.to_string()))?
        .into_inner();

    let mut documents = Vec::new();
    while let Some(response) = stream
        .message()
        .await
        .map_err(|e| crate::error::FirestoreError::Internal(e.to_string()))?
    {
        if let Some(doc) = response.document {
            documents.push(doc);
        }
    }

    Ok(QuerySnapshot {
        documents,
        firestore: Arc::clone(&state.firestore),
    })
}
