//! Cloud Firestore module
//!
//! # C++ SDK Structure Mapping
//! Following the C++ SDK's header file organization:
//! - `field_value.h` → `field_value.rs` (Value, MapValue, FilterCondition)
//! - `timestamp.h` → `timestamp.rs`
//! - `geo_point.h` → `geo_point.rs`
//! - `settings.h` → `settings.rs`
//! - `document_reference.h` → `document_reference.rs`
//! - `document_snapshot.h` → `document_snapshot.rs`
//! - `collection_reference.h` → `collection_reference.rs`
//! - `query_snapshot.h` → `query_snapshot.rs`
//! - `write_batch.h` → `write_batch.rs`
//! - `transaction.h` → `transaction.rs`
//! - `listener.h` → `listener.rs`

// Individual type modules (following C++ SDK structure)
pub mod collection_reference;
pub mod document_reference;
pub mod document_snapshot;
pub mod field_value;
pub mod geo_point;
/// Metadata change tracking for real-time listeners
pub mod metadata_changes;
pub mod query;
pub mod query_snapshot;
pub mod settings;
/// Stream utilities for document snapshots
pub mod snapshot_stream;
pub mod timestamp;
pub mod write_batch;

// Core modules
/// Core Firestore client and gRPC implementation
pub mod firestore;
pub mod listener;
/// Transaction support for atomic read-write operations
pub mod transaction;

// Legacy types module for backwards compatibility
// TODO: Remove this once all code is updated to use direct module imports
// pub mod types;

// Re-export main Firestore client
pub use firestore::Firestore;

// Re-export from field_value module
pub use field_value::{FilterCondition, MapValue, OrderDirection, Value, ValueType};

// Re-export proto types for internal use
pub(crate) use field_value::proto;

// Re-export commonly used proto types
pub use field_value::proto::google::firestore::v1::{ArrayValue, Document, StructuredQuery};

// Re-export from query module
pub use query::{Direction, Query};

// Re-export from timestamp module
pub use timestamp::Timestamp;

// Re-export from geo_point module
pub use geo_point::GeoPoint;

// Re-export from settings module
pub use settings::{Settings, Source};

// Re-export from document_reference module
pub use document_reference::DocumentReference;

// Re-export from document_snapshot module
pub use document_snapshot::{DocumentSnapshot, SnapshotMetadata};

// Re-export from collection_reference module
pub use collection_reference::CollectionReference;

// Re-export from query_snapshot module
pub use query_snapshot::{DocumentChange, DocumentChangeType, QuerySnapshot};

// Re-export from write_batch module
pub use write_batch::{WriteBatch, WriteOperation};

// Re-export from metadata_changes module
pub use metadata_changes::MetadataChanges;

// Re-export from snapshot_stream module
pub use snapshot_stream::{DocumentSnapshotStream, QuerySnapshotStream};

// Re-export from listener module
pub use listener::{listen_document, ListenerOptions};

// Re-export from transaction module
pub use transaction::Transaction;
