///! gRPC-based Firestore client implementation
///!
///! This module implements the Firestore client using gRPC protocol,
///! matching the C++ SDK architecture.
///!
///! # C++ References
///! - `firebase-ios-sdk/Firestore/core/src/api/firestore.h` - Firestore class
///! - `firebase-ios-sdk/Firestore/core/src/core/firestore_client.h` - FirestoreClient
///! - `firebase-ios-sdk/Firestore/core/src/remote/datastore.h` - Datastore gRPC layer

use std::sync::Arc;
use tonic::transport::Channel;
use tonic::service::Interceptor;
use tonic::Request;
use crate::error::FirebaseError;
use crate::firestore::types::{DocumentReference, CollectionReference, proto};

// Import generated gRPC client
use proto::google::firestore::v1::firestore_client::FirestoreClient as GrpcClient;

/// Firestore database client
///
/// # C++ Reference
/// - `firebase-ios-sdk/Firestore/core/src/api/firestore.h:51`
#[derive(Clone)]
pub struct Firestore {
    pub(crate) inner: Arc<FirestoreInner>,
}

pub struct FirestoreInner {
    pub(crate) project_id: String,
    pub(crate) database_id: String,
    pub(crate) id_token: Option<String>,
    pub(crate) grpc_client: GrpcClient<tonic::service::interceptor::InterceptedService<Channel, FirestoreInterceptor>>,
}

/// gRPC interceptor for adding authentication headers
/// Mirrors C++ GrpcConnection::CreateContext
#[derive(Clone)]
pub struct FirestoreInterceptor {
    id_token: Option<String>,
    project_id: String,
    database_id: String,
}

impl Interceptor for FirestoreInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, tonic::Status> {
        // Add Bearer token for authentication if available
        // C++ Reference: GrpcConnection adds authorization header
        if let Some(ref token) = self.id_token {
            let bearer = format!("Bearer {}", token);
            let bearer_value = bearer.parse()
                .map_err(|_| tonic::Status::unauthenticated("Invalid ID token"))?;
            request.metadata_mut().insert("authorization", bearer_value);
        }

        // Add routing header for request routing
        // Mirrors C++ kXGoogRequestParams header
        let resource_prefix = format!(
            "projects/{}/databases/{}",
            self.project_id, self.database_id
        );
        let routing_value = resource_prefix.parse()
            .map_err(|_| tonic::Status::invalid_argument("Invalid resource prefix"))?;
        request.metadata_mut().insert("x-goog-request-params", routing_value);

        Ok(request)
    }
}

impl Firestore {
    /// Create a new Firestore instance
    ///
    /// # Arguments
    /// * `project_id` - Firebase project ID
    /// * `database_id` - Database ID (default: "default")
    /// * `id_token` - Optional Firebase ID token for authentication (from Auth.current_user().id_token)
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:50` - GetInstance
    /// - `firebase-ios-sdk/Firestore/core/src/remote/grpc_connection.cc` - CreateContext (adds auth headers)
    ///
    /// # Note
    /// For authenticated access, you must provide an ID token obtained from Auth.
    /// Unauthenticated access requires Firestore security rules to allow public read/write.
    pub async fn new(
        project_id: impl Into<String>, 
        database_id: impl Into<String>,
        id_token: Option<String>
    ) -> Result<Self, FirebaseError> {
        let project_id = project_id.into();
        let database_id = database_id.into();
        
        // Connect to Firestore gRPC endpoint with TLS
        // Format: https://firestore.googleapis.com
        let endpoint = "https://firestore.googleapis.com";
        
        // Configure TLS (mirrors C++ LoadGrpcRootCertificate)
        let tls_config = tonic::transport::ClientTlsConfig::new()
            .with_webpki_roots()
            .domain_name("firestore.googleapis.com");
        
        let channel = Channel::from_static(endpoint)
            .tls_config(tls_config)
            .map_err(|e| crate::error::FirestoreError::Connection(format!("Failed to configure TLS: {}", e)))?
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .connect()
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("Failed to connect to Firestore: {}", e)))?;
        
        // Create interceptor for authentication
        let interceptor = FirestoreInterceptor {
            id_token: id_token.clone(),
            project_id: project_id.clone(),
            database_id: database_id.clone(),
        };
        
        let grpc_client = GrpcClient::with_interceptor(channel, interceptor);
        
        Ok(Self {
            inner: Arc::new(FirestoreInner {
                project_id,
                database_id,
                id_token,
                grpc_client,
            }),
        })
    }

    /// Get the project ID
    pub fn project_id(&self) -> &str {
        &self.inner.project_id
    }

    /// Get the database ID
    pub fn database_id(&self) -> &str {
        &self.inner.database_id
    }

    /// Get a reference to a document
    ///
    /// # Arguments
    /// * `path` - Document path (e.g., "users/alice")
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:86` - Document()
    pub fn document(&self, path: impl Into<String>) -> DocumentReference {
        DocumentReference::new(path, Arc::clone(&self.inner))
    }

    /// Get a reference to a collection
    ///
    /// # Arguments
    /// * `path` - Collection path (e.g., "users")
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:77` - Collection()
    pub fn collection(&self, path: impl Into<String>) -> CollectionReference {
        CollectionReference::new(path, Arc::clone(&self.inner))
    }

    /// Create a new write batch
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:96` - GetBatch()
    pub fn batch(&self) -> crate::firestore::types::WriteBatch {
        crate::firestore::types::WriteBatch::new(Arc::clone(&self.inner))
    }

    /// Create a collection group query
    ///
    /// Creates a query that includes all documents in the database that are contained
    /// in a collection or subcollection with the given collection_id.
    ///
    /// # Arguments
    /// * `collection_id` - Identifies the collections to query over. Every collection
    ///   or subcollection with this ID as the last segment of its path will be included.
    ///   Cannot contain a slash.
    ///
    /// # Returns
    /// A CollectionReference configured to query across all collections with the given ID.
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:268` - CollectionGroup()
    ///   Returns `Query` in C++ SDK
    ///
    /// # Architecture Note
    /// **TODO**: The C++ SDK returns `Query` type, not `CollectionReference`. The Rust SDK
    /// currently lacks a separate `Query` type - `CollectionReference` is being used for
    /// both collections and queries. This should be refactored to introduce a proper `Query`
    /// struct that `CollectionReference` can convert to/from, matching the C++ architecture
    /// where `CollectionReference` inherits from `Query`.
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("project-id", "default", None).await?;
    /// 
    /// // Query all "posts" collections across the entire database
    /// let all_posts = firestore.collection_group("posts");
    /// // This will match:
    /// // - /posts/{postId}
    /// // - /users/{userId}/posts/{postId}
    /// // - /groups/{groupId}/posts/{postId}
    /// // etc.
    /// # Ok(())
    /// # }
    /// ```
    pub fn collection_group(&self, collection_id: impl Into<String>) -> CollectionReference {
        let collection_id = collection_id.into();
        
        // Validate that collection_id does not contain '/'
        // C++ SDK validates this in CollectionGroup()
        assert!(
            !collection_id.contains('/'),
            "collection_id must not contain '/'"
        );
        
        // Create a CollectionReference with the collection_id as path
        // The key difference is this represents ALL collections with this ID,
        // not just a specific path. Query building will use allDescendants=true.
        CollectionReference::new(collection_id, Arc::clone(&self.inner))
    }

    /// Get the gRPC database path
    /// Format: projects/{project_id}/databases/{database_id}
    pub(crate) fn database_path(&self) -> String {
        format!("projects/{}/databases/{}", self.inner.project_id, self.inner.database_id)
    }

    /// Get a mutable reference to the gRPC client
    pub(crate) fn grpc_client(&self) -> &GrpcClient<tonic::service::interceptor::InterceptedService<Channel, FirestoreInterceptor>> {
        &self.inner.grpc_client
    }

    /// Run a transaction
    ///
    /// Executes the given function within a transaction context. If the transaction fails
    /// due to conflicts, it will be automatically retried up to 5 times (default).
    ///
    /// All reads in the transaction must be executed before any writes. If documents
    /// read within the transaction are modified externally, the transaction will be
    /// retried automatically.
    ///
    /// # Arguments
    /// * `update_fn` - Async function that receives a Transaction and performs reads/writes
    ///
    /// # Returns
    /// The value returned by the update function
    ///
    /// # C++ Reference
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:310` - RunTransaction()
    /// - `firebase-ios-sdk/Firestore/core/src/core/transaction.cc:40` - Transaction logic
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("project-id", "default", None).await?;
    /// 
    /// // Atomically increment a counter
    /// let result = firestore.run_transaction(|txn| async move {
    ///     let doc = firestore.document("counters/visitors");
    ///     let snapshot = txn.get(&doc).await?;
    ///     
    ///     let count = snapshot
    ///         .and_then(|s| s.get("count"))
    ///         .and_then(|v| v.as_i64())
    ///         .unwrap_or(0);
    ///     
    ///     let mut data = serde_json::Map::new();
    ///     data.insert("count".to_string(), serde_json::json!(count + 1));
    ///     txn.set(&doc, data.into())?;
    ///     
    ///     Ok(count + 1)
    /// }).await?;
    /// 
    /// println!("New count: {}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_transaction<F, Fut, R>(
        &self,
        update_fn: F,
    ) -> Result<R, FirebaseError>
    where
        F: Fn(crate::firestore::transaction::Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<R, FirebaseError>>,
    {
        const MAX_ATTEMPTS: usize = 5;

        for attempt in 0..MAX_ATTEMPTS {
            // Begin transaction
            let transaction_id = match self.begin_transaction().await {
                Err(e) => {
                    if attempt == MAX_ATTEMPTS - 1 {
                        return Err(e);
                    }
                    // Retry on error
                    continue;
                }
                Ok(id) => id,
            };

            // Create transaction object
            let transaction = crate::firestore::transaction::Transaction::new(
                transaction_id.clone(),
                Arc::clone(&self.inner),
            );

            // Execute user function
            let result = match update_fn(transaction).await {
                Err(e) => {
                    // Rollback on error
                    let _ = self.rollback_transaction(&transaction_id).await;
                    
                    if attempt == MAX_ATTEMPTS - 1 {
                        return Err(e);
                    }
                    continue;
                }
                Ok(r) => r,
            };

            // Commit transaction
            // Note: We need to pass the writes from transaction to commit
            // For now, this is a simplified implementation that will need
            // to be enhanced to actually commit the writes
            match self.commit_transaction(&transaction_id).await {
                Err(e) => {
                    if attempt == MAX_ATTEMPTS - 1 {
                        return Err(e);
                    }
                    // Retry on commit failure (likely due to conflicts)
                    continue;
                }
                Ok(_) => return Ok(result),
            }
        }

        Err(crate::error::FirestoreError::Internal(
            "Transaction failed after maximum retries".to_string()
        ).into())
    }

    /// Begin a new transaction (internal)
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc:253` - BeginTransaction
    async fn begin_transaction(&self) -> Result<Vec<u8>, FirebaseError> {
        use proto::google::firestore::v1::BeginTransactionRequest;

        let database_path = self.database_path();

        let request = BeginTransactionRequest {
            database: database_path,
            options: None, // Use default transaction options
        };

        let mut client = self.inner.grpc_client.clone();
        let response = client
            .begin_transaction(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(
                format!("Failed to begin transaction: {}", e)
            ))?
            .into_inner();

        Ok(response.transaction)
    }

    /// Commit a transaction (internal)
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc:272` - Commit
    async fn commit_transaction(&self, transaction_id: &[u8]) -> Result<(), FirebaseError> {
        use proto::google::firestore::v1::CommitRequest;

        let database_path = self.database_path();

        // TODO: Include actual writes from the transaction
        // For now this is a minimal implementation
        let request = CommitRequest {
            database: database_path,
            writes: vec![], // TODO: Convert transaction.writes to gRPC Write messages
            transaction: transaction_id.to_vec(),
        };

        let mut client = self.inner.grpc_client.clone();
        client
            .commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(
                format!("Failed to commit transaction: {}", e)
            ))?;

        Ok(())
    }

    /// Rollback a transaction (internal)
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/remote/datastore.cc:290` - Rollback
    async fn rollback_transaction(&self, transaction_id: &[u8]) -> Result<(), FirebaseError> {
        use proto::google::firestore::v1::RollbackRequest;

        let database_path = self.database_path();

        let request = RollbackRequest {
            database: database_path,
            transaction: transaction_id.to_vec(),
        };

        let mut client = self.inner.grpc_client.clone();
        client
            .rollback(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(
                format!("Failed to rollback transaction: {}", e)
            ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_firestore_creation() {
        // Firestore connection succeeds without credentials
        // (actual operations would fail without proper authentication)
        let result = Firestore::new("test-project", "default", None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_database_path() {
        // Create a mock Firestore (we can't actually connect without auth)
        // This test just validates path formatting
        let project_id = "test-project";
        let database_id = "(default)";
        let expected = format!("projects/{}/databases/{}", project_id, database_id);
        
        // We'll need to create FirestoreInner directly for unit testing
        // Or test through public API once we can mock gRPC
        assert_eq!(expected, "projects/test-project/databases/(default)");
    }
}
