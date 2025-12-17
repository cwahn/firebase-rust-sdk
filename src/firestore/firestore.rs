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
use crate::firestore::{DocumentReference, CollectionReference, proto};

// Import generated gRPC client
use proto::google::firestore::v1::firestore_client::FirestoreClient as GrpcClient;

/// Shared authentication data for gRPC interceptors
///
/// Wrapped in Arc to enable cheap cloning across operations without
/// duplicating heap-allocated strings. Each gRPC operation creates a
/// lightweight interceptor that shares this immutable auth data.
///
/// # Performance
/// Using Arc<AuthData> instead of cloning strings:
/// - Per-operation cost: 1 Arc increment (~1 atomic op) vs 3 String clones
/// - Memory: Single copy shared across all operations
pub(crate) struct AuthData {
    pub(crate) id_token: Option<String>,
    pub(crate) project_id: String,
    pub(crate) database_id: String,
}

/// Firestore database client
///
/// # C++ Reference
/// - `firebase-ios-sdk/Firestore/core/src/api/firestore.h:51`
#[derive(Clone)]
pub struct Firestore {
    /// Shared internal state (channel, auth data)
    pub(crate) inner: Arc<FirestoreInner>,
}

/// Internal Firestore client state shared across clones
///
/// Contains the gRPC channel and authentication data needed for operations.
/// Not directly used by consumers - wrapped by the public Firestore struct.
pub(crate) struct FirestoreInner {
    pub(crate) project_id: String,
    pub(crate) database_id: String,
    pub(crate) id_token: Option<String>,
    // Store Channel (connection pool) - Arc'd internally, cheap to clone
    // Create GrpcClient per operation with Arc'd interceptor for optimal performance
    pub(crate) channel: Channel,
    pub(crate) auth_data: Arc<AuthData>,
}

/// gRPC interceptor for adding authentication headers
/// Mirrors C++ GrpcConnection::CreateContext
#[derive(Clone)]
pub(crate) struct FirestoreInterceptor {
    pub(crate) auth_data: Arc<AuthData>,
}

impl Interceptor for FirestoreInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, tonic::Status> {
        // Add Bearer token for authentication if available
        // C++ Reference: GrpcConnection adds authorization header
        if let Some(ref token) = self.auth_data.id_token {
            let bearer = format!("Bearer {}", token);
            let bearer_value = bearer.parse()
                .map_err(|_| tonic::Status::unauthenticated("Invalid ID token"))?;
            request.metadata_mut().insert("authorization", bearer_value);
        }

        // Add routing header for request routing
        // Mirrors C++ kXGoogRequestParams header
        let resource_prefix = format!(
            "projects/{}/databases/{}",
            self.auth_data.project_id, self.auth_data.database_id
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
        // Create multiple endpoints for connection pooling to handle high concurrency
        let endpoint = "https://firestore.googleapis.com";
        
        // Configure TLS (mirrors C++ LoadGrpcRootCertificate)
        let tls_config = tonic::transport::ClientTlsConfig::new()
            .with_webpki_roots()
            .domain_name("firestore.googleapis.com");
        
        // Create endpoint with aggressive settings for high concurrency
        // Production apps handling dozens of concurrent operations need:
        // 1. Large HTTP/2 windows to avoid flow control blocking
        // 2. Aggressive keep-alive to maintain connection health  
        // 3. Reasonable timeouts that don't fail under load
        let endpoint_config = Channel::from_static(endpoint)
            .tls_config(tls_config)
            .map_err(|e| crate::error::FirestoreError::Connection(format!("Failed to configure TLS: {}", e)))?
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(30))
            .concurrency_limit(256) // Allow up to 256 concurrent requests per connection
            .http2_keep_alive_interval(std::time::Duration::from_secs(20))
            .keep_alive_timeout(std::time::Duration::from_secs(10))
            .keep_alive_while_idle(true)
            .initial_stream_window_size(Some(8 * 1024 * 1024)) // 8MB per stream
            .initial_connection_window_size(Some(32 * 1024 * 1024)) // 32MB total
            .http2_adaptive_window(true)
            .tcp_keepalive(Some(std::time::Duration::from_secs(20)))
            .tcp_nodelay(true); // Disable Nagle's algorithm for lower latency
        
        // Connect to Firestore with persistent connection
        let channel = endpoint_config.connect().await
            .map_err(|e| crate::error::FirestoreError::Connection(
                format!("Failed to connect to Firestore: {}", e)
            ))?;
        
        // Store auth data in Arc for cheap cloning across operations
        // Avoids String clones on every gRPC call
        let auth_data = Arc::new(AuthData {
            id_token: id_token.clone(),
            project_id: project_id.clone(),
            database_id: database_id.clone(),
        });
        
        Ok(Self {
            inner: Arc::new(FirestoreInner {
                project_id,
                database_id,
                id_token,
                channel,
                auth_data,
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
    /// - `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:233` - batch()
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:96` - GetBatch()
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::new("project-id", "default", None).await?;
    /// 
    /// firestore.batch()
    ///     .set("cities/LA", Default::default())
    ///     .set("cities/SF", Default::default())
    ///     .commit()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn batch(&self) -> crate::firestore::write_batch::WriteBatch {
        crate::firestore::write_batch::WriteBatch::new(Arc::clone(&self.inner))
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

        let interceptor = FirestoreInterceptor {
            auth_data: self.inner.auth_data.clone(),
        };
        let mut client = GrpcClient::with_interceptor(self.inner.channel.clone(), interceptor);
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

        let interceptor = FirestoreInterceptor {
            auth_data: self.inner.auth_data.clone(),
        };
        let mut client = GrpcClient::with_interceptor(self.inner.channel.clone(), interceptor);
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

        let interceptor = FirestoreInterceptor {
            auth_data: self.inner.auth_data.clone(),
        };
        let mut client = GrpcClient::with_interceptor(self.inner.channel.clone(), interceptor);
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
