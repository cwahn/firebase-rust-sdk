//! Firestore types - Backwards compatibility re-exports
//!
//! This module exists for backwards compatibility only.
//! New code should import directly from `firestore::*` instead of `firestore::types::*`

// Re-export everything from parent module
pub use super::{
    Value, MapValue, ValueType, FilterCondition, OrderDirection,
    Timestamp, GeoPoint,
    Settings, Source,
    DocumentReference, DocumentSnapshot, SnapshotMetadata,
    CollectionReference,
    QuerySnapshot, DocumentChange, DocumentChangeType,
    WriteBatch, WriteOperation
};

// Proto is pub(crate) only
pub(crate) use super::proto;
