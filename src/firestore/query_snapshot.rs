//! Firestore QuerySnapshot and DocumentChange types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/query_snapshot.h:55`
//! - `firestore/src/include/firebase/firestore/document_change.h:36`

use super::document_snapshot::{DocumentSnapshot, SnapshotMetadata};
use crate::firestore::firestore::FirestoreInner;

/// Query snapshot containing multiple documents
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query_snapshot.h:55`
#[derive(Clone)]
pub struct QuerySnapshot {
    /// Raw document protos from gRPC response
    pub(crate) documents: Vec<super::field_value::proto::google::firestore::v1::Document>,

    /// Reference to Firestore client
    pub(crate) firestore: std::sync::Arc<FirestoreInner>,
}

impl QuerySnapshot {
    /// Get all documents as DocumentSnapshot instances
    pub fn documents(&self) -> Vec<DocumentSnapshot> {
        self.documents
            .iter()
            .map(|doc| {
                let data = if doc.fields.is_empty() {
                    None
                } else {
                    Some(super::field_value::MapValue {
                        fields: doc.fields.clone(),
                    })
                };
                DocumentSnapshot {
                    data,
                    reference: super::document_reference::DocumentReference::new(
                        doc.name.clone(),
                        std::sync::Arc::clone(&self.firestore),
                    ),
                    metadata: SnapshotMetadata::default(),
                }
            })
            .collect()
    }

    /// Check if the query result is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get the number of documents in the snapshot
    pub fn len(&self) -> usize {
        self.documents.len()
    }
}

/// Document change type for snapshot listeners
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_change.h:36`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentChangeType {
    /// Document was added
    Added,
    /// Document was modified
    Modified,
    /// Document was removed
    Removed,
}

/// Represents a change to a document in a query snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_change.h:48`
#[derive(Debug, Clone)]
pub struct DocumentChange {
    /// Type of change
    pub change_type: DocumentChangeType,

    /// The document that changed
    pub document: DocumentSnapshot,

    /// The old index of the document (-1 if added)
    pub old_index: i32,

    /// The new index of the document (-1 if removed)
    pub new_index: i32,
}
