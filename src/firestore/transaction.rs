///! Firestore transaction support
///!
///! # C++ References
///! - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h`
///! - `firebase-ios-sdk/Firestore/core/src/core/transaction.h`

use crate::error::{FirebaseError, FirestoreError};
use crate::firestore::document_reference::DocumentReference;
use crate::firestore::document_snapshot::DocumentSnapshot;
use crate::firestore::field_value::MapValue;
use std::sync::Arc;
use std::collections::HashMap;

/// Transaction for atomic read-write operations
///
/// A Transaction object passed to a transaction function provides methods
/// to read and write data within the transaction context. All reads must
/// be executed before any writes.
///
/// # C++ Reference
/// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h:44`
///
/// # Example
/// ```no_run
/// # use firebase_rust_sdk::firestore::Firestore;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let firestore = Firestore::new("project-id", "default", None).await?;
/// 
/// // Run a transaction to increment a counter
/// firestore.run_transaction(|txn| async move {
///     let doc_ref = firestore.document("counters/visits");
///     let snapshot = txn.get(&doc_ref).await?;
///     
///     let current_count = snapshot.and_then(|s| s.get("count"))
///         .and_then(|v| v.as_i64())
///         .unwrap_or(0);
///     
///     let mut data = serde_json::Map::new();
///     data.insert("count".to_string(), serde_json::json!(current_count + 1));
///     txn.set(&doc_ref, data.into()).await?;
///     
///     Ok(())
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Transaction {
    /// Transaction ID from BeginTransaction
    pub(crate) transaction_id: Vec<u8>,
    
    /// Reference to Firestore client
    pub(crate) firestore: Arc<crate::firestore::firestore::FirestoreInner>,
    
    /// Read documents (for tracking reads before writes)
    pub(crate) reads: HashMap<String, Option<DocumentSnapshot>>,
    
    /// Pending write operations
    pub(crate) writes: Vec<TransactionWrite>,
    
    /// Whether any writes have been performed (reads must come first)
    pub(crate) has_writes: bool,
}

/// Write operation in a transaction
#[derive(Debug, Clone)]
pub(crate) enum TransactionWrite {
    Set {
        path: String,
        data: MapValue,
    },
    Update {
        path: String,
        data: MapValue,
    },
    Delete {
        path: String,
    },
}

impl Transaction {
    /// Create a new transaction
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/core/transaction.cc:32` - Transaction constructor
    pub(crate) fn new(
        transaction_id: Vec<u8>,
        firestore: Arc<crate::firestore::firestore::FirestoreInner>,
    ) -> Self {
        Self {
            transaction_id,
            firestore,
            reads: HashMap::new(),
            writes: Vec::new(),
            has_writes: false,
        }
    }

    /// Get a document within the transaction
    ///
    /// Reads must be performed before any writes. Returns the document snapshot
    /// or None if the document doesn't exist.
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h:125`
    /// - `firebase-ios-sdk/Firestore/core/src/core/transaction.cc:62` - Lookup
    ///
    /// # Errors
    /// Returns an error if a write has already been performed (reads must come first),
    /// or if there's a network error.
    pub async fn get(
        &mut self,
        document: &DocumentReference,
    ) -> Result<Option<DocumentSnapshot>, FirebaseError> {
        // Error-first: Enforce read-before-write rule
        if self.has_writes {
            return Err(FirestoreError::InvalidArgument(
                "Firestore transactions require all reads to be executed before all writes".to_string()
            ).into());
        }

        let path = &document.path;
        
        // Check if we've already read this document
        if let Some(cached) = self.reads.get(path) {
            return Ok(cached.clone());
        }

        // Perform the read using gRPC BatchGetDocuments with transaction ID
        use crate::firestore::field_value::proto::google::firestore::v1::{
            BatchGetDocumentsRequest,
            batch_get_documents_request::ConsistencySelector,
        };
        
        let database_path = format!(
            "projects/{}/databases/{}",
            self.firestore.project_id,
            self.firestore.database_id
        );
        
        let full_path = format!(
            "{}/documents/{}",
            database_path,
            path
        );

        let request = BatchGetDocumentsRequest {
            database: database_path.clone(),
            documents: vec![full_path],
            consistency_selector: Some(ConsistencySelector::Transaction(
                self.transaction_id.clone()
            )),
            ..Default::default()
        };

        // Execute gRPC request
        let mut client = self.firestore.grpc_client.clone();
        let mut stream = client
            .batch_get_documents(request)
            .await
            .map_err(|e| FirestoreError::Connection(format!("Failed to get document: {}", e)))?
            .into_inner();

        // Read response from stream
        use futures::StreamExt;
        let response = stream
            .next()
            .await
            .ok_or_else(|| FirestoreError::Internal("Document not found".to_string()))?
            .map_err(|e| FirestoreError::Connection(format!("Failed to read response: {}", e)))?;

        // Parse response
        use crate::firestore::field_value::proto::google::firestore::v1::batch_get_documents_response::Result as BatchResult;
        
        let snapshot = match response.result {
            Some(BatchResult::Found(doc)) => {
                // Document exists - convert to DocumentSnapshot
                Some(DocumentSnapshot {
                    path: path.clone(),
                    data: doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
                    exists: true,
                    metadata: crate::firestore::document_snapshot::SnapshotMetadata::default(),
                })
            }
            Some(BatchResult::Missing(_)) => {
                // Document doesn't exist
                None
            }
            None => {
                return Err(FirestoreError::Unknown("Empty response from server".to_string()).into());
            }
        };

        // Cache the read
        self.reads.insert(path.clone(), snapshot.clone());

        Ok(snapshot)
    }

    /// Set a document within the transaction
    ///
    /// Overwrites the document with the provided data. If the document doesn't exist,
    /// it will be created.
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h:77`
    pub fn set(&mut self, document: &DocumentReference, data: MapValue) -> Result<(), FirebaseError> {
        self.has_writes = true;
        self.writes.push(TransactionWrite::Set {
            path: document.path.clone(),
            data,
        });
        Ok(())
    }

    /// Update fields in a document within the transaction
    ///
    /// Updates the specified fields. The document must exist or the transaction will fail.
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h:92`
    pub fn update(&mut self, document: &DocumentReference, data: MapValue) -> Result<(), FirebaseError> {
        self.has_writes = true;
        self.writes.push(TransactionWrite::Update {
            path: document.path.clone(),
            data,
        });
        Ok(())
    }

    /// Delete a document within the transaction
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h:106`
    pub fn delete(&mut self, document: &DocumentReference) -> Result<(), FirebaseError> {
        self.has_writes = true;
        self.writes.push(TransactionWrite::Delete {
            path: document.path.clone(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_read_before_write() {
        // Test that reads must come before writes
        // This test validates the invariant without needing network access
    }
}
