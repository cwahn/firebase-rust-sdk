//! Firestore WriteBatch type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/write_batch.h:46`

use super::field_value::{MapValue, proto};
use crate::firestore::firestore::{FirestoreInner, FirestoreInterceptor};
use proto::google::firestore::v1::firestore_client::FirestoreClient as GrpcClient;

/// Write batch for atomic operations
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/write_batch.h:46`
pub struct WriteBatch {
    operations: Vec<WriteOperation>,
    /// Reference to Firestore client for commit operation
    firestore: std::sync::Arc<FirestoreInner>,
}

/// Write operations for batch writes and transactions
///
/// Represents different types of Firestore write operations that can be performed.
#[derive(Debug, Clone)]
pub enum WriteOperation {
    /// Set (overwrite) a document
    Set {
        /// Document path
        path: String,
        /// Document data
        data: MapValue,
    },
    /// Update specific fields in a document
    Update {
        /// Document path
        path: String,
        /// Fields to update
        data: MapValue,
    },
    /// Delete a document
    Delete {
        /// Document path to delete
        path: String,
    },
}

impl WriteBatch {
    /// Create a new write batch
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/write_batch.cc:26` - WriteBatch constructor
    pub(crate) fn new(firestore: std::sync::Arc<FirestoreInner>) -> Self {
        Self {
            operations: Vec::new(),
            firestore,
        }
    }

    /// Set document data (overwrites existing document)
    ///
    /// # C++ Reference
    /// - `write_batch.h:117` - Set() method
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let firestore = Firestore::new("project-id", "default", None).await?;
    /// let batch = firestore.batch()
    ///     .set("cities/LA", Default::default())
    ///     .commit()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set(mut self, path: impl Into<String>, data: MapValue) -> Self {
        self.operations.push(WriteOperation::Set {
            path: path.into(),
            data,
        });
        self
    }

    /// Update document fields (document must exist)
    ///
    /// # C++ Reference
    /// - `write_batch.h:131` - Update() method
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let firestore = Firestore::new("project-id", "default", None).await?;
    /// let batch = firestore.batch()
    ///     .update("cities/LA", Default::default())
    ///     .commit()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update(mut self, path: impl Into<String>, data: MapValue) -> Self {
        self.operations.push(WriteOperation::Update {
            path: path.into(),
            data,
        });
        self
    }

    /// Delete document
    ///
    /// # C++ Reference
    /// - `write_batch.h:151` - Delete() method
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let firestore = Firestore::new("project-id", "default", None).await?;
    /// let batch = firestore.batch()
    ///     .delete("cities/LA")
    ///     .commit()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(mut self, path: impl Into<String>) -> Self {
        self.operations
            .push(WriteOperation::Delete { path: path.into() });
        self
    }

    /// Commit the batch
    ///
    /// # C++ Reference
    /// - `firestore/src/main/write_batch_main.cc:70` - WriteBatchInternal::Commit
    /// - `firestore/src/common/write_batch.cc:140` - WriteBatch::Commit
    ///
    /// Commits all batched write operations atomically. If any operation fails,
    /// none of the operations are applied.
    ///
    /// # Errors
    /// Returns `FirestoreError` if:
    /// - Batch is empty (nothing to commit)
    /// - Network request fails
    /// - Any write operation fails (entire batch is rolled back)
    pub async fn commit(self) -> Result<(), crate::error::FirebaseError> {
        use proto::google::firestore::v1::{CommitRequest, Write, write::Operation};
        
        if self.operations.is_empty() {
            return Err(crate::error::FirestoreError::InvalidArgument(
                "Cannot commit empty batch".to_string()
            ).into());
        }
        
        let database_path = format!("projects/{}/databases/{}", 
            self.firestore.project_id, self.firestore.database_id);
        
        // Convert WriteOperations to gRPC Write messages
        let mut writes = Vec::new();
        
        for op in self.operations {
            let write = match op {
                WriteOperation::Set { path, data } => {
                    let full_path = format!("{}/documents/{}", database_path, path);
                    Write {
                        operation: Some(Operation::Update(proto::google::firestore::v1::Document {
                            name: full_path,
                            fields: data.fields,
                            create_time: None,
                            update_time: None,
                        })),
                        update_mask: None,  // None = replace entire document
                        update_transforms: vec![],
                        current_document: None,
                    }
                },
                WriteOperation::Update { path, data } => {
                    let full_path = format!("{}/documents/{}", database_path, path);
                    let field_paths: Vec<String> = data.fields.keys().cloned().collect();
                    
                    Write {
                        operation: Some(Operation::Update(proto::google::firestore::v1::Document {
                            name: full_path,
                            fields: data.fields,
                            create_time: None,
                            update_time: None,
                        })),
                        update_mask: Some(proto::google::firestore::v1::DocumentMask { field_paths }),
                        update_transforms: vec![],
                        current_document: Some(proto::google::firestore::v1::Precondition {
                            condition_type: Some(proto::google::firestore::v1::precondition::ConditionType::Exists(true)),
                        }),
                    }
                },
                WriteOperation::Delete { path } => {
                    let full_path = format!("{}/documents/{}", database_path, path);
                    Write {
                        operation: Some(Operation::Delete(full_path)),
                        update_mask: None,
                        update_transforms: vec![],
                        current_document: None,
                    }
                },
            };
            writes.push(write);
        }
        
        let request = CommitRequest {
            database: database_path,
            writes,
            transaction: vec![],
        };
        
        let interceptor = FirestoreInterceptor {
            auth_data: self.firestore.auth_data.clone(),
        };
        let mut client = GrpcClient::with_interceptor(self.firestore.channel.clone(), interceptor);
        let _response = client.commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC commit failed: {}", e)))?;
        
        Ok(())
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Get number of operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_write_batch_operations() {
        // Test WriteBatch without needing FirestoreInner
        // Just verify the WriteOperation enum works correctly
        let data = MapValue {
            fields: HashMap::new(),
        };
        
        let set_op = WriteOperation::Set {
            path: "cities/LA".to_string(),
            data: data.clone(),
        };
        
        match set_op {
            WriteOperation::Set { ref path, .. } => {
                assert_eq!(path, "cities/LA");
            }
            _ => panic!("Expected Set operation"),
        }
        
        let update_op = WriteOperation::Update {
            path: "cities/SF".to_string(),
            data: data.clone(),
        };
        
        match update_op {
            WriteOperation::Update { ref path, .. } => {
                assert_eq!(path, "cities/SF");
            }
            _ => panic!("Expected Update operation"),
        }
        
        let delete_op = WriteOperation::Delete {
            path: "cities/NYC".to_string(),
        };
        
        match delete_op {
            WriteOperation::Delete { ref path } => {
                assert_eq!(path, "cities/NYC");
            }
            _ => panic!("Expected Delete operation"),
        }
    }
}

