//! Firestore CollectionReference type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/collection_reference.h:38`

use super::document_reference::DocumentReference;
use super::field_value::MapValue;
use super::query::{Query, QueryState};
use crate::error::FirebaseError;
use crate::firestore::firestore::FirestoreInner;

/// Reference to a Firestore collection
///
/// Implements the Query trait to support filtering, ordering, and pagination.
/// Following C++ SDK pattern where CollectionReference inherits from Query.
///
/// # C++ Reference
/// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.h:38`
/// - `query.h:61` - CollectionReference inherits from Query in C++
#[derive(Clone)]
pub struct CollectionReference {
    /// Internal query state
    pub(crate) state: QueryState,
}

impl CollectionReference {
    /// Create a new collection reference
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:28`
    pub(crate) fn new(path: impl Into<String>, firestore: std::sync::Arc<FirestoreInner>) -> Self {
        Self { 
            state: QueryState::new(path.into(), firestore),
        }
    }

    /// Get collection ID (last segment of path)
    pub fn id(&self) -> &str {
        self.state.collection_path.rsplit('/').next().unwrap_or(&self.state.collection_path)
    }

    /// Get a document reference within this collection
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:41` - Document()
    pub fn document(&self, document_id: impl AsRef<str>) -> DocumentReference {
        let path = format!("{}/{}", self.state.collection_path, document_id.as_ref());
        DocumentReference::new(path, std::sync::Arc::clone(&self.state.firestore))
    }

    /// Add a new document with auto-generated ID
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:46` - AddDocument()
    pub async fn add(&self, data: MapValue) -> Result<DocumentReference, FirebaseError> {
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

/// Implement Query trait for CollectionReference
///
/// Following C++ SDK pattern where CollectionReference inherits from Query.
/// All query methods return new CollectionReference instances with modified state.
///
/// # C++ Reference
/// - `query.h:61` - CollectionReference inherits from Query
/// - `collection_reference_main.h` - CollectionReferenceInternal inherits QueryInternal
impl Query for CollectionReference {
    fn query_state(&self) -> &QueryState {
        &self.state
    }

    fn with_state(&self, state: QueryState) -> Self {
        Self { state }
    }
}
