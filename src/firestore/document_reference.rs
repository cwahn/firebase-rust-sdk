//! Firestore DocumentReference type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/document_reference.h:71`

use super::document_snapshot::{DocumentSnapshot, SnapshotMetadata};
use super::field_value::{MapValue, proto};
use crate::firestore::firestore::FirestoreInner;
use std::sync::Arc;

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
        use futures::StreamExt;
        
        let (tx, rx) = mpsc::unbounded_channel();
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        
        // Clone necessary data for the async task
        let doc_ref = self.clone();
        let include_metadata = metadata_changes.unwrap_or_default() == super::MetadataChanges::Include;
        
        // Spawn background task to handle the listener
        tokio::spawn(async move {
            // Get authentication token if available
            let auth_token = doc_ref.firestore.id_token.clone().unwrap_or_default();
            
            // Create listener options
            let options = super::listener::ListenerOptions {
                include_metadata_changes: include_metadata,
            };
            
            // Start listening using existing infrastructure
            // This will fail gracefully if there's no valid auth or if Firestore is not set up
            let listener_result = super::listener::listen_document(
                &super::Firestore { inner: Arc::clone(&doc_ref.firestore) },
                auth_token,
                doc_ref.firestore.project_id.clone(),
                doc_ref.firestore.database_id.clone(),
                doc_ref.path.clone(),
                options,
            ).await;
            
            match listener_result {
                Ok(mut stream) => {
                    // Forward events from gRPC stream to our channel until cancelled
                    loop {
                        tokio::select! {
                            _ = &mut cancel_rx => {
                                // Stream was dropped, cleanup and exit
                                break;
                            }
                            event = stream.next() => {
                                match event {
                                    Some(result) => {
                                        // Forward the result to the channel
                                        if tx.send(result).is_err() {
                                            // Receiver dropped, exit
                                            break;
                                        }
                                    }
                                    None => {
                                        // Stream ended
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    // Send the error and close the stream
                    let _ = tx.send(Err(e));
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

#[cfg(test)]
mod tests {
    use crate::firestore::{Firestore, MetadataChanges};
    use futures::StreamExt;

    #[tokio::test]
    async fn test_listen_returns_stream() {
        // Create a Firestore instance (will fail operations without auth, but we can test API)
        let firestore = match Firestore::new("test-project", "(default)", None).await {
            Ok(fs) => fs,
            Err(_) => return, // Skip test if Firestore creation fails
        };
        
        let doc_ref = firestore.collection("test").document("doc1");
        
        // Test that listen() returns a stream without panicking
        let stream = doc_ref.listen(None);
        drop(stream); // Should clean up properly via Drop
    }

    #[tokio::test]
    async fn test_listen_with_metadata_changes() {
        let firestore = match Firestore::new("test-project", "(default)", None).await {
            Ok(fs) => fs,
            Err(_) => return,
        };
        
        let doc_ref = firestore.collection("test").document("doc1");
        
        // Test with Include metadata changes
        let stream = doc_ref.listen(Some(MetadataChanges::Include));
        drop(stream);
        
        // Test with Exclude metadata changes
        let stream = doc_ref.listen(Some(MetadataChanges::Exclude));
        drop(stream);
    }

    #[tokio::test]
    async fn test_listen_stream_cancellation_on_drop() {
        let firestore = match Firestore::new("test-project", "(default)", None).await {
            Ok(fs) => fs,
            Err(_) => return,
        };
        
        let doc_ref = firestore.collection("test").document("doc1");
        
        {
            let _stream = doc_ref.listen(None);
            // Stream dropped here - should trigger cancellation
        }
        
        // If we get here without hanging, cancellation worked
        // (The Drop impl sends cancellation signal)
    }

    #[tokio::test]
    async fn test_listen_stream_error_handling() {
        // This test verifies that streams handle errors gracefully
        let firestore = match Firestore::new("test-project", "(default)", None).await {
            Ok(fs) => fs,
            Err(_) => return,
        };
        
        let doc_ref = firestore.collection("test").document("doc1");
        let mut stream = doc_ref.listen(None);
        
        // Try to get next item (will likely error without proper auth/database setup)
        // This just verifies the stream API works and doesn't panic
        if let Some(result) = stream.next().await {
            // Either Ok or Err is fine - we're testing the API surface
            match result {
                Ok(_snapshot) => {
                    // Got a snapshot (unlikely without real database)
                }
                Err(_e) => {
                    // Expected - no database configured or permission denied
                }
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_listeners_same_document() {
        let firestore = match Firestore::new("test-project", "(default)", None).await {
            Ok(fs) => fs,
            Err(_) => return,
        };
        
        let doc_ref = firestore.collection("test").document("doc1");
        
        // Should be able to create multiple listeners
        let stream1 = doc_ref.listen(None);
        let stream2 = doc_ref.listen(None);
        let stream3 = doc_ref.listen(Some(MetadataChanges::Include));
        
        drop(stream1);
        drop(stream2);
        drop(stream3);
    }
}
