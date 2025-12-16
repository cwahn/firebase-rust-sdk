//! Firestore DocumentReference type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/document_reference.h:71`

use super::document_snapshot::{DocumentSnapshot, SnapshotMetadata};
use super::field_value::{MapValue, proto};
use crate::firestore::firestore::FirestoreInner;

/// Reference to a Firestore document
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_reference.h:71`
#[derive(Clone)]
pub struct DocumentReference {
    /// Full document path (e.g., "users/alice")
    pub path: String,
    /// Reference to Firestore client (for operations like set/get/update/delete)
    /// 
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.h:129` - std::shared_ptr<Firestore> firestore_
    pub(crate) firestore: std::sync::Arc<FirestoreInner>,
}

impl DocumentReference {
    /// Create a new document reference
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:40`
    pub(crate) fn new(path: impl Into<String>, firestore: std::sync::Arc<FirestoreInner>) -> Self {
        Self { 
            path: path.into(),
            firestore,
        }
    }

    /// Get the document ID (last segment of path)
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:53` - document_id()
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get the parent collection path
    pub fn parent_path(&self) -> Option<&str> {
        self.path.rsplit_once('/').map(|(parent, _)| parent)
    }

    /// Get the full document path with database prefix
    /// Format: projects/{project_id}/databases/{database_id}/documents/{document_path}
    pub(crate) fn full_path(&self) -> String {
        format!("{}/documents/{}", 
            format!("projects/{}/databases/{}", self.firestore.project_id, self.firestore.database_id),
            self.path
        )
    }

    /// Set document data
    ///
    /// # Arguments
    /// * `data` - Document data as protobuf MapValue
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:67` - SetData()
    /// - `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc` - CommitMutationsWithCredentials()
    pub async fn set(&self, data: MapValue) -> Result<(), crate::error::FirebaseError> {
        use proto::google::firestore::v1::{CommitRequest, Write, write::Operation};
        
        let database_path = format!("projects/{}/databases/{}", 
            self.firestore.project_id, self.firestore.database_id);
        
        // Create a Write mutation with Update operation (which acts as set)
        let write = Write {
            operation: Some(Operation::Update(proto::google::firestore::v1::Document {
                name: self.full_path(),
                fields: data.fields,
                create_time: None,
                update_time: None,
            })),
            update_mask: None,  // None means replace entire document
            update_transforms: vec![],
            current_document: None,
        };
        
        let request = CommitRequest {
            database: database_path,
            writes: vec![write],
            transaction: vec![],
        };
        
        let mut client = self.firestore.grpc_client.clone();
        let _response = client.commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC commit failed: {}", e)))?;
        
        Ok(())
    }

    /// Update document fields
    ///
    /// # Arguments
    /// * `data` - Fields to update as protobuf MapValue
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:74` - UpdateData()
    pub async fn update(&self, data: MapValue) -> Result<(), crate::error::FirebaseError> {
        use proto::google::firestore::v1::{CommitRequest, Write, write::Operation, DocumentMask};
        
        let database_path = format!("projects/{}/databases/{}", 
            self.firestore.project_id, self.firestore.database_id);
        
        // Create update mask with field paths
        let field_paths: Vec<String> = data.fields.keys().cloned().collect();
        
        let write = Write {
            operation: Some(Operation::Update(proto::google::firestore::v1::Document {
                name: self.full_path(),
                fields: data.fields,
                create_time: None,
                update_time: None,
            })),
            update_mask: Some(DocumentMask { field_paths }),
            update_transforms: vec![],
            current_document: Some(proto::google::firestore::v1::Precondition {
                condition_type: Some(proto::google::firestore::v1::precondition::ConditionType::Exists(true)),
            }),
        };
        
        let request = CommitRequest {
            database: database_path,
            writes: vec![write],
            transaction: vec![],
        };
        
        let mut client = self.firestore.grpc_client.clone();
        let _response = client.commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC commit failed: {}", e)))?;
        
        Ok(())
    }

    /// Delete the document
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:82` - DeleteDocument()
    pub async fn delete(&self) -> Result<(), crate::error::FirebaseError> {
        use proto::google::firestore::v1::{CommitRequest, Write, write::Operation};
        
        let database_path = format!("projects/{}/databases/{}", 
            self.firestore.project_id, self.firestore.database_id);
        
        let write = Write {
            operation: Some(Operation::Delete(self.full_path())),
            update_mask: None,
            update_transforms: vec![],
            current_document: None,
        };
        
        let request = CommitRequest {
            database: database_path,
            writes: vec![write],
            transaction: vec![],
        };
        
        let mut client = self.firestore.grpc_client.clone();
        let _response = client.commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC commit failed: {}", e)))?;
        
        Ok(())
    }

    /// Get the document snapshot
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:87` - GetDocument()
    /// - `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc` - LookupDocumentsWithCredentials()
    pub async fn get(&self) -> Result<DocumentSnapshot, crate::error::FirebaseError> {
        use proto::google::firestore::v1::GetDocumentRequest;
        
        let request = GetDocumentRequest {
            name: self.full_path(),
            consistency_selector: None,
            mask: None,
        };
        
        let mut client = self.firestore.grpc_client.clone();
        let response = client.get_document(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC get_document failed: {}", e)))?;
        
        let doc = response.into_inner();
        
        // Convert Document to DocumentSnapshot
        let data = if doc.fields.is_empty() {
            None
        } else {
            Some(MapValue { fields: doc.fields })
        };
        
        Ok(DocumentSnapshot {
            reference: self.clone(),
            data,
            metadata: SnapshotMetadata::default(),
        })
    }

    /// Listen to real-time updates for this document.
    ///
    /// Returns a stream that yields document snapshots as they change.
    /// The stream automatically cleans up the listener when dropped.
    ///
    /// # Arguments
    /// * `metadata_changes` - Optional parameter to control metadata-only change events.
    ///   Use `Some(MetadataChanges::Include)` to receive metadata-only updates.
    ///   Defaults to `MetadataChanges::Exclude` if `None`.
    ///
    /// # Returns
    /// A stream of `Result<DocumentSnapshot, FirebaseError>` that yields updates.
    ///
    /// # Example
    /// ```no_run
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use firebase_rust_sdk::firestore::MetadataChanges;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("my-project", "(default)", None).await?;
    /// let doc_ref = firestore.collection("cities").document("SF");
    ///
    /// let mut stream = doc_ref.listen(Some(MetadataChanges::Include));
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(snapshot) => println!("Document: {:?}", snapshot.id()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # C++ Reference
    /// - `document_reference.h:265` - `AddSnapshotListener` returns `ListenerRegistration`
    /// - Rust uses async streams with Drop cleanup instead of explicit remove()
    pub fn listen(&self, metadata_changes: Option<super::MetadataChanges>) -> super::DocumentSnapshotStream {
        use tokio::sync::{mpsc, oneshot};
        
        let (_tx, rx) = mpsc::unbounded_channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        
        // Spawn background task to handle the listener
        let _doc_ref = self.clone();
        let _include_metadata = metadata_changes.unwrap_or_default() == super::MetadataChanges::Include;
        
        tokio::spawn(async move {
            // TODO: Integrate with existing listener infrastructure from listener.rs
            // For now, this is a placeholder that will be implemented in the next step
            tokio::select! {
                _ = cancel_rx => {
                    // Stream was dropped, cleanup
                }
            }
        });
        
        super::DocumentSnapshotStream::new(rx, cancel_tx)
    }
}

impl std::fmt::Debug for DocumentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentReference")
            .field("path", &self.path)
            .field("project_id", &self.firestore.project_id)
            .field("database_id", &self.firestore.database_id)
            .finish()
    }
}
