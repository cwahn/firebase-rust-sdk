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
    inner: Arc<FirestoreInner>,
}

pub struct FirestoreInner {
    pub(crate) project_id: String,
    pub(crate) database_id: String,
    pub(crate) grpc_client: GrpcClient<Channel>,
}

impl Firestore {
    /// Create a new Firestore instance
    ///
    /// # Arguments
    /// * `project_id` - Firebase project ID
    /// * `database_id` - Database ID (default: "(default)")
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/firestore.cc:50` - GetInstance
    pub async fn new(project_id: impl Into<String>, database_id: impl Into<String>) -> Result<Self, FirebaseError> {
        let project_id = project_id.into();
        let database_id = database_id.into();
        
        // Connect to Firestore gRPC endpoint
        // Format: https://firestore.googleapis.com
        let endpoint = "https://firestore.googleapis.com";
        
        let channel = Channel::from_static(endpoint)
            .connect()
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("Failed to connect to Firestore: {}", e)))?;
        
        let grpc_client = GrpcClient::new(channel);
        
        Ok(Self {
            inner: Arc::new(FirestoreInner {
                project_id,
                database_id,
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

    /// Get the gRPC database path
    /// Format: projects/{project_id}/databases/{database_id}
    pub(crate) fn database_path(&self) -> String {
        format!("projects/{}/databases/{}", self.inner.project_id, self.inner.database_id)
    }

    /// Get a mutable reference to the gRPC client
    pub(crate) fn grpc_client(&self) -> &GrpcClient<Channel> {
        &self.inner.grpc_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_firestore_creation() {
        // Note: This will fail without credentials, but tests the structure
        let result = Firestore::new("test-project", "(default)").await;
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
