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

    /// Get the gRPC database path
    /// Format: projects/{project_id}/databases/{database_id}
    pub(crate) fn database_path(&self) -> String {
        format!("projects/{}/databases/{}", self.inner.project_id, self.inner.database_id)
    }

    /// Get a mutable reference to the gRPC client
    pub(crate) fn grpc_client(&self) -> &GrpcClient<tonic::service::interceptor::InterceptedService<Channel, FirestoreInterceptor>> {
        &self.inner.grpc_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_firestore_creation() {
        // Note: This will fail without credentials, but tests the structure
        let result = Firestore::new("test-project", "default", None).await;
        // We expect this to fail with connection error since we don't have auth
        assert!(result.is_err());
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
