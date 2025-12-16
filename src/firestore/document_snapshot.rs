//! Firestore DocumentSnapshot and SnapshotMetadata types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/document_snapshot.h:58`
//! - `firestore/src/include/firebase/firestore/snapshot_metadata.h:35`

use super::document_reference::DocumentReference;
use super::field_value::{MapValue, Value};

/// Firestore document snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_snapshot.h:58`
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    /// Document reference
    pub reference: DocumentReference,

    /// Document data (None if document doesn't exist)
    /// Contains protobuf MapValue
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:186` - GetData() returns MapValue
    pub data: Option<MapValue>,

    /// Document metadata
    pub metadata: SnapshotMetadata,
}

impl DocumentSnapshot {
    /// Check if document exists
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:172`
    pub fn exists(&self) -> bool {
        self.data.is_some()
    }

    /// Get a field value by path
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:196`
    pub fn get(&self, field: &str) -> Option<&Value> {
        let Some(data) = &self.data else {
            return None;
        };
        data.fields.get(field)
    }

    /// Get document ID
    pub fn id(&self) -> &str {
        self.reference.id()
    }
}

/// Metadata about a document snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/snapshot_metadata.h:35`
#[derive(Debug, Clone, Copy)]
pub struct SnapshotMetadata {
    /// Whether the snapshot contains pending writes
    pub has_pending_writes: bool,

    /// Whether the data came from cache
    pub is_from_cache: bool,
}

impl Default for SnapshotMetadata {
    fn default() -> Self {
        Self {
            has_pending_writes: false,
            is_from_cache: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_snapshot_metadata_default() {
        let metadata = SnapshotMetadata::default();
        assert!(!metadata.has_pending_writes);
        assert!(!metadata.is_from_cache);
    }

    #[test]
    fn test_document_snapshot_metadata_pending_writes() {
        let metadata = SnapshotMetadata {
            has_pending_writes: true,
            is_from_cache: false,
        };
        assert!(metadata.has_pending_writes);
    }

    #[test]
    fn test_document_snapshot_metadata_from_cache() {
        let metadata = SnapshotMetadata {
            has_pending_writes: false,
            is_from_cache: true,
        };
        assert!(metadata.is_from_cache);
    }

    #[test]
    fn test_document_snapshot_metadata_all_flags() {
        let metadata = SnapshotMetadata {
            has_pending_writes: true,
            is_from_cache: true,
        };
        assert!(metadata.has_pending_writes);
        assert!(metadata.is_from_cache);
    }
}
