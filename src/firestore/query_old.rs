//! Firestore Query trait and implementation
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/query.h:61`

use super::document_snapshot::DocumentSnapshot;
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
/// # C++ Reference
/// - `query.h:61` - Query class
pub trait Query {
    /// Get the internal query state
    fn query_state(&self) -> &QueryState;

    /// Execute the query and return results
    ///
    /// # C++ Reference
    /// - `query.h:642` - Query::Get()
    fn get(&self) -> impl std::future::Future<Output = Result<QuerySnapshot, FirebaseError>> + Send {
        async {
            self.get_with_source(Source::Default).await
        }
    }

    /// Execute the query with specified source
    ///
    /// # C++ Reference
    /// - `query.h:656` - Query::Get(Source source)
    fn get_with_source(
        &self,
        source: Source,
    ) -> impl std::future::Future<Output = Result<QuerySnapshot, FirebaseError>> + Send;

    /// Filter documents where field equals value
    ///
    /// # C++ Reference
    /// - `query.h:177` - Query::WhereEqualTo(field, value)
    fn where_equal_to(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::EqualTo, value));
        WhereQuery { state }
    }

    /// Filter documents where field does not equal value
    ///
    /// # C++ Reference
    /// - `query.h:194` - Query::WhereNotEqualTo(field, value)
    fn where_not_equal_to(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::NotEqualTo, value));
        WhereQuery { state }
    }

    /// Filter documents where field is less than value
    ///
    /// # C++ Reference
    /// - `query.h:222` - Query::WhereLessThan(field, value)
    fn where_less_than(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::LessThan, value));
        WhereQuery { state }
    }

    /// Filter documents where field is less than or equal to value
    ///
    /// # C++ Reference
    /// - `query.h:247` - Query::WhereLessThanOrEqualTo(field, value)
    fn where_less_than_or_equal_to(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::LessThanOrEqualTo, value));
        WhereQuery { state }
    }

    /// Filter documents where field is greater than value
    ///
    /// # C++ Reference
    /// - `query.h:272` - Query::WhereGreaterThan(field, value)
    fn where_greater_than(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::GreaterThan, value));
        WhereQuery { state }
    }

    /// Filter documents where field is greater than or equal to value
    ///
    /// # C++ Reference
    /// - `query.h:297` - Query::WhereGreaterThanOrEqualTo(field, value)
    fn where_greater_than_or_equal_to(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::GreaterThanOrEqualTo, value));
        WhereQuery { state }
    }

    /// Filter documents where field array contains value
    ///
    /// # C++ Reference
    /// - `query.h:322` - Query::WhereArrayContains(field, value)
    fn where_array_contains(&self, field: impl Into<String>, value: Value) -> WhereQuery {
        let mut state = self.query_state().clone();
        state.filters.push((field.into(), FilterOperator::ArrayContains, value));
        WhereQuery { state }
    }

    /// Filter documents where field array contains any of the values
    ///
    /// # C++ Reference
    /// - `query.h:347` - Query::WhereArrayContainsAny(field, values)
    fn where_array_contains_any(&self, field: impl Into<String>, values: Vec<Value>) -> WhereQuery {
        let mut state = self.query_state().clone();
        use super::field_value::proto;
        state.filters.push((
            field.into(),
            FilterOperator::ArrayContainsAny,
            Value::ArrayValue(proto::google::firestore::v1::ArrayValue {
                values: values.iter().map(|v| proto::value_to_proto(v)).collect(),
            }),
        ));
        WhereQuery { state }
    }

    /// Filter documents where field equals any of the values
    ///
    /// # C++ Reference
    /// - `query.h:382` - Query::WhereIn(field, values)
    fn where_in(&self, field: impl Into<String>, values: Vec<Value>) -> WhereQuery {
        let mut state = self.query_state().clone();
        use super::field_value::proto;
        state.filters.push((
            field.into(),
            FilterOperator::In,
            Value::ArrayValue(proto::google::firestore::v1::ArrayValue {
                values: values.iter().map(|v| proto::value_to_proto(v)).collect(),
            }),
        ));
        WhereQuery { state }
    }

    /// Filter documents where field does not equal any of the values
    ///
    /// # C++ Reference
    /// - `query.h:417` - Query::WhereNotIn(field, values)
    fn where_not_in(&self, field: impl Into<String>, values: Vec<Value>) -> WhereQuery {
        let mut state = self.query_state().clone();
        use super::field_value::proto;
        state.filters.push((
            field.into(),
            FilterOperator::NotIn,
            Value::ArrayValue(proto::google::firestore::v1::ArrayValue {
                values: values.iter().map(|v| proto::value_to_proto(v)).collect(),
            }),
        ));
        WhereQuery { state }
    }

    /// Sort query results by field
    ///
    /// # C++ Reference
    /// - `query.h:453` - Query::OrderBy(field, direction)
    fn order_by(&self, field: impl Into<String>, direction: Direction) -> OrderByQuery {
        let mut state = self.query_state().clone();
        state.orders.push((field.into(), direction));
        OrderByQuery { state }
    }

    /// Limit query results to specified number
    ///
    /// # C++ Reference
    /// - `query.h:475` - Query::Limit(limit)
    fn limit(&self, limit: i32) -> LimitQuery {
        let mut state = self.query_state().clone();
        state.limit_value = Some(limit);
        LimitQuery { state }
    }

    /// Limit query results to last N documents
    ///
    /// # C++ Reference
    /// - `query.h:486` - Query::LimitToLast(limit)
    fn limit_to_last(&self, limit: i32) -> LimitQuery {
        let mut state = self.query_state().clone();
        state.limit_to_last_value = Some(limit);
        LimitQuery { state }
    }

    /// Start query at field values
    ///
    /// # C++ Reference
    /// - `query.h:510` - Query::StartAt(values)
    fn start_at_values(&self, values: Vec<Value>) -> CursorQuery {
        let mut state = self.query_state().clone();
        state.start_at = Some(values);
        CursorQuery { state }
    }

    /// Start query after field values
    ///
    /// # C++ Reference
    /// - `query.h:535` - Query::StartAfter(values)
    fn start_after_values(&self, values: Vec<Value>) -> CursorQuery {
        let mut state = self.query_state().clone();
        state.start_after = Some(values);
        CursorQuery { state }
    }

    /// End query at field values
    ///
    /// # C++ Reference
    /// - `query.h:560` - Query::EndAt(values)
    fn end_at_values(&self, values: Vec<Value>) -> CursorQuery {
        let mut state = self.query_state().clone();
        state.end_at = Some(values);
        CursorQuery { state }
    }

    /// End query before field values
    ///
    /// # C++ Reference
    /// - `query.h:585` - Query::EndBefore(values)
    fn end_before_values(&self, values: Vec<Value>) -> CursorQuery {
        let mut state = self.query_state().clone();
        state.end_before = Some(values);
        CursorQuery { state }
    }
}

/// Query with where filter applied
#[derive(Clone)]
pub struct WhereQuery {
    pub(crate) state: QueryState,
}

/// Query with order by clause applied
#[derive(Clone)]
pub struct OrderByQuery {
    pub(crate) state: QueryState,
}

/// Query with limit applied
#[derive(Clone)]
pub struct LimitQuery {
    pub(crate) state: QueryState,
}

/// Query with cursor (start/end) applied
#[derive(Clone)]
pub struct CursorQuery {
    pub(crate) state: QueryState,
}

// Implement Query trait for all concrete query types
macro_rules! impl_query_for_type {
    ($type:ty) => {
        impl Query for $type {
            fn query_state(&self) -> &QueryState {
                &self.state
            }

            async fn get_with_source(&self, _source: Source) -> Result<QuerySnapshot, FirebaseError> {
                execute_query(&self.state).await
            }
        }
    };
}

impl_query_for_type!(WhereQuery);
impl_query_for_type!(OrderByQuery);
impl_query_for_type!(LimitQuery);
impl_query_for_type!(CursorQuery);

/// Execute a query with the given state
pub(crate) async fn execute_query(state: &QueryState) -> Result<QuerySnapshot, FirebaseError> {
    use super::field_value::proto;
    use proto::google::firestore::v1 as firestore_proto;
    use firestore_proto::firestore_client::FirestoreClient;

    let project_id = &state.firestore.project_id;
    let database_id = &state.firestore.database_id;
    let parent = format!("projects/{}/databases/{}/documents", project_id, database_id);

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
            .map(|(field, condition, value)| {
                let field_ref = firestore_proto::structured_query::FieldReference {
                    field_path: field.clone(),
                };
                let value_proto = proto::value_to_proto(value);

                use firestore_proto::structured_query::field_filter::Operator;
                use firestore_proto::structured_query::FieldFilter;

                let op = match condition {
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
                            FieldFilter {
                                field: Some(field_ref),
                                op: op as i32,
                                value: Some(value_proto),
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
            .map(|(field, direction)| firestore_proto::structured_query::Order {
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
            })
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
            values: values.iter().map(proto::value_to_proto).collect(),
            before: true,
        });
    }
    if let Some(values) = &state.start_after {
        structured_query.start_at = Some(firestore_proto::Cursor {
            values: values.iter().map(proto::value_to_proto).collect(),
            before: false,
        });
    }
    if let Some(values) = &state.end_at {
        structured_query.end_at = Some(firestore_proto::Cursor {
            values: values.iter().map(proto::value_to_proto).collect(),
            before: false,
        });
    }
    if let Some(values) = &state.end_before {
        structured_query.end_at = Some(firestore_proto::Cursor {
            values: values.iter().map(proto::value_to_proto).collect(),
            before: true,
        });
    }

    let request = firestore_proto::RunQueryRequest {
        parent,
        query_type: Some(firestore_proto::run_query_request::QueryType::StructuredQuery(
            structured_query,
        )),
        ..Default::default()
    };

    let mut client = FirestoreClient::new(state.firestore.channel.clone());
    let mut stream = client.run_query(request).await?.into_inner();

    let mut documents = Vec::new();
    while let Some(response) = stream.message().await? {
        if let Some(doc) = response.document {
            documents.push(doc);
        }
    }

    Ok(QuerySnapshot {
        documents,
        firestore: Arc::clone(&state.firestore),
    })
}
