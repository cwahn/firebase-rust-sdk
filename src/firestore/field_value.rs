//! Firestore field value types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/field_value.h`
//! - `firestore/src/include/firebase/firestore/map_field_value.h`

// Re-export protobuf types for public API
// Matches C++ SDK's FieldValue and MapValue pattern
#[allow(clippy::all)]
pub(crate) mod proto {
    include!(concat!(env!("OUT_DIR"), "/proto.rs"));
}

/// Firestore Value type - matches C++ SDK's FieldValue
/// 
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/field_value.h`
pub use proto::google::firestore::v1::Value;

/// Map of field values - matches C++ SDK's MapValue
/// Uses protobuf MapValue which contains HashMap<String, Value>
/// 
/// # C++ Reference  
/// - `firestore/src/include/firebase/firestore/map_field_value.h:30`
pub use proto::google::firestore::v1::MapValue;

/// ValueType enum for creating protobuf Value variants
pub use proto::google::firestore::v1::value::ValueType;

/// Filter operators for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:142` (Filter)
/// - `firestore/src/include/firebase/firestore/filter.h:268` (And)
/// - `firestore/src/include/firebase/firestore/filter.h:308` (Or)
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// field == value
    Equal(String, Value),

    /// field < value
    LessThan(String, Value),

    /// field <= value
    LessThanOrEqual(String, Value),

    /// field > value
    GreaterThan(String, Value),

    /// field >= value
    GreaterThanOrEqual(String, Value),

    /// field array contains value
    ArrayContains(String, Value),

    /// field array contains any value from list
    ArrayContainsAny(String, Vec<Value>),

    /// field value is in list
    In(String, Vec<Value>),

    /// field != value
    NotEqual(String, Value),

    /// field not in list
    NotIn(String, Vec<Value>),

    /// Conjunction of multiple filters (all must match)
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/filter.h:268` - And(filters)
    /// 
    /// A document matches this filter if it matches all the provided filters.
    /// If the vector is empty, it acts as a no-op. If only one filter is provided,
    /// it behaves the same as that filter alone.
    And(Vec<FilterCondition>),

    /// Disjunction of multiple filters (any must match)
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/filter.h:308` - Or(filters)
    /// 
    /// A document matches this filter if it matches any of the provided filters.
    /// If the vector is empty, it acts as a no-op. If only one filter is provided,
    /// it behaves the same as that filter alone.
    Or(Vec<FilterCondition>),
}

impl FilterCondition {
    /// Get the field path for this filter
    /// 
    /// For compound filters (And/Or), returns empty string as they don't have a single field path
    pub fn field_path(&self) -> &str {
        match self {
            FilterCondition::Equal(field, _) => field,
            FilterCondition::LessThan(field, _) => field,
            FilterCondition::LessThanOrEqual(field, _) => field,
            FilterCondition::GreaterThan(field, _) => field,
            FilterCondition::GreaterThanOrEqual(field, _) => field,
            FilterCondition::ArrayContains(field, _) => field,
            FilterCondition::ArrayContainsAny(field, _) => field,
            FilterCondition::In(field, _) => field,
            FilterCondition::NotEqual(field, _) => field,
            FilterCondition::NotIn(field, _) => field,
            FilterCondition::And(_) | FilterCondition::Or(_) => "",
        }
    }

    /// Get the operator string for Firestore REST API
    pub fn operator(&self) -> &'static str {
        match self {
            FilterCondition::Equal(_, _) => "EQUAL",
            FilterCondition::LessThan(_, _) => "LESS_THAN",
            FilterCondition::LessThanOrEqual(_, _) => "LESS_THAN_OR_EQUAL",
            FilterCondition::GreaterThan(_, _) => "GREATER_THAN",
            FilterCondition::GreaterThanOrEqual(_, _) => "GREATER_THAN_OR_EQUAL",
            FilterCondition::ArrayContains(_, _) => "ARRAY_CONTAINS",
            FilterCondition::ArrayContainsAny(_, _) => "ARRAY_CONTAINS_ANY",
            FilterCondition::In(_, _) => "IN",
            FilterCondition::NotEqual(_, _) => "NOT_EQUAL",
            FilterCondition::NotIn(_, _) => "NOT_IN",
            FilterCondition::And(_) => "AND",
            FilterCondition::Or(_) => "OR",
        }
    }
}
