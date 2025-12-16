//! Firestore CollectionReference type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/collection_reference.h:38`

use super::document_reference::DocumentReference;
use super::field_value::MapValue;
use crate::firestore::firestore::FirestoreInner;

/// Reference to a Firestore collection
///
/// # C++ Reference
/// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.h:38`
///
/// # Architecture Note
/// **TODO**: In the C++ SDK, `CollectionReference` inherits from `Query`. The Rust SDK
/// currently uses `CollectionReference` for both collection references and queries, lacking
/// a separate `Query` type. This should be refactored to match the C++ architecture:
/// - Introduce `struct Query` as the base query type
/// - Make `CollectionReference` contain or convert to `Query`
/// - Update `Firestore::collection_group()` to return `Query` instead of `CollectionReference`
#[derive(Clone)]
pub struct CollectionReference {
    /// Collection path (e.g., "users")
    pub path: String,
    /// Reference to Firestore client
    pub(crate) firestore: std::sync::Arc<FirestoreInner>,
}

impl CollectionReference {
    /// Create a new collection reference
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:28`
    pub(crate) fn new(path: impl Into<String>, firestore: std::sync::Arc<FirestoreInner>) -> Self {
        Self { 
            path: path.into(),
            firestore,
        }
    }

    /// Get collection ID (last segment of path)
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get a document reference within this collection
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:41` - Document()
    pub fn document(&self, document_id: impl AsRef<str>) -> DocumentReference {
        let path = format!("{}/{}", self.path, document_id.as_ref());
        DocumentReference::new(path, std::sync::Arc::clone(&self.firestore))
    }

    /// Add a new document with auto-generated ID
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:46` - AddDocument()
    pub async fn add(&self, data: MapValue) -> Result<DocumentReference, crate::error::FirebaseError> {
        // Generate auto ID
        use rand::Rng;
        let auto_id: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        
        let doc_ref = self.document(&auto_id);
        doc_ref.set(data).await?;
        Ok(doc_ref)
    }
}
