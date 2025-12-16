//! Cloud Firestore
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore.h:91` - Firestore class

use crate::error::FirebaseError;
use crate::firestore::types::{DocumentReference, DocumentSnapshot, SnapshotMetadata, Settings};
use crate::app::App;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rand::Rng;

/// Global map of (App name, database ID) to Firestore instances
///
/// C++ equivalent: Similar to Auth singleton pattern
static FIRESTORE_INSTANCES: Lazy<RwLock<HashMap<String, Firestore>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Cloud Firestore instance
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore.h:91`
///
/// Entry point for the Firebase Firestore SDK.
/// Use `Firestore::get_instance(app)` to obtain an instance.
#[derive(Clone)]
pub struct Firestore {
    inner: Arc<FirestoreInner>,
}

struct FirestoreInner {
    project_id: String,
    database_id: String,
    http_client: reqwest::Client,
    settings: RwLock<Settings>,
    app: App,
}

impl Firestore {
    /// Get or create Firestore instance for the given App with default database
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:118` - GetInstance(app)
    ///
    /// Returns existing Firestore if one exists for this App, otherwise creates new.
    /// Thread-safe singleton pattern following C++ implementation.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::{App, AppOptions, firestore::Firestore};
    ///
    /// let app = App::create(AppOptions {
    ///     api_key: "YOUR_API_KEY".to_string(),
    ///     project_id: "your-project".to_string(),
    ///     app_name: None,
    /// }).await?;
    /// let firestore = Firestore::get_instance(&app).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_instance(app: &App) -> Result<Self, FirebaseError> {
        Self::get_instance_with_database(app, "default").await
    }

    /// Get or create Firestore instance with specific database ID
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:158` - GetInstance(app, db_name)
    ///
    /// # Arguments
    /// * `app` - The Firebase App instance
    /// * `database_id` - The database ID (default: "default")
    pub async fn get_instance_with_database(
        app: &App,
        database_id: impl Into<String>,
    ) -> Result<Self, FirebaseError> {
        let database_id = database_id.into();
        let project_id = app.options().project_id.clone();

        // Create composite key for instances map
        let key = format!("{}:{}:{}", app.name(), project_id, database_id);

        let mut instances = FIRESTORE_INSTANCES.write().await;

        // Check if instance already exists
        if let Some(firestore) = instances.get(&key) {
            return Ok(firestore.clone());
        }

        // Create new Firestore instance
        let http_client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Err(e) => return Err(FirebaseError::Internal(format!("Failed to create HTTP client: {}", e))),
            Ok(client) => client,
        };

        let firestore = Firestore {
            inner: Arc::new(FirestoreInner {
                project_id,
                database_id,
                http_client,
                settings: RwLock::new(Settings::default()),
                app: app.clone(),
            }),
        };

        instances.insert(key, firestore.clone());

        Ok(firestore)
    }

    /// Get the project ID
    pub fn project_id(&self) -> &str {
        &self.inner.project_id
    }

    /// Get the database ID
    pub fn database_id(&self) -> &str {
        &self.inner.database_id
    }

    /// Get the App instance this Firestore is associated with
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:145` - app()
    pub fn app(&self) -> &crate::app::App {
        &self.inner.app
    }

    /// Internal: Get auth token from App if Auth is initialized
    async fn get_auth_token(&self) -> Option<String> {
        self.inner.app.get_auth_token(false).await
    }

    /// Get a reference to a collection
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:227` - Collection(path)
    ///
    /// # Arguments
    /// * `path` - Slash-separated path to the collection
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let users = firestore.collection("users");
    /// # Ok(())
    /// # }
    /// ```
    pub fn collection(&self, path: impl AsRef<str>) -> CollectionReference {
        CollectionReference::new(self.clone(), path.as_ref().to_string())
    }

    /// Get a reference to a document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:246` - Document(path)
    ///
    /// # Arguments
    /// * `path` - Slash-separated path to the document
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let doc = firestore.document("users/alice");
    /// # Ok(())
    /// # }
    /// ```
    pub fn document(&self, path: impl AsRef<str>) -> DocumentReference {
        DocumentReference::new(path.as_ref())
    }

    /// Internal: Get HTTP client
    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.inner.http_client
    }

    /// Internal: Build an authenticated request
    async fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.http_client().request(method, url);
        
        // Add Authorization header if auth token is set
        if let Some(token) = self.get_auth_token().await {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }
        
        builder
    }

    /// Get current Firestore settings
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:182` - settings()
    ///
    /// Returns a copy of the current Firestore settings.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let settings = firestore.settings().await;
    /// println!("Persistence enabled: {}", settings.persistence_enabled);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn settings(&self) -> Settings {
        self.inner.settings.read().await.clone()
    }

    /// Set Firestore settings
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:197` - set_settings()
    ///
    /// Configure Firestore behavior including persistence and caching.
    /// Must be called before any other Firestore operation.
    ///
    /// # Arguments
    /// * `settings` - The settings to apply
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::{Firestore, types::Settings};
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// 
    /// let mut settings = Settings::new();
    /// settings.persistence_enabled = true;
    /// settings.cache_size_bytes = Settings::CACHE_SIZE_UNLIMITED;
    /// 
    /// firestore.set_settings(settings).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_settings(&self, settings: Settings) -> Result<(), FirebaseError> {
        // TODO: Implement persistence backend initialization when settings.persistence_enabled = true
        // - For native: Initialize REDB/SQLite at settings.cache_directory
        // - For WASM: Initialize IndexedDB connection
        // - Create cache tables/collections if needed
        // - Set up TTL/eviction policies based on cache_size_bytes
        
        *self.inner.settings.write().await = settings;
        todo!("Persistence backend initialization not yet implemented. See PERSISTENCE_DESIGN.md for architecture.")
    }

    /// Enable network access for this Firestore instance
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:285` - EnableNetwork()
    ///
    /// Re-enables network usage for this Firestore instance after a prior call to DisableNetwork().
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// firestore.enable_network().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enable_network(&self) -> Result<(), FirebaseError> {
        // TODO: Implement network enable
        // - Resume snapshot listeners
        // - Flush pending writes queue
        // - Resume automatic sync
        todo!("Network enable not yet implemented")
    }

    /// Disable network access for this Firestore instance
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:270` - DisableNetwork()
    ///
    /// Disables network usage for this Firestore instance. It can be re-enabled via EnableNetwork().
    /// While the network is disabled, any snapshot listeners or get() calls will return results from cache,
    /// and any write operations will be queued until the network is restored.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// firestore.disable_network().await?;
    /// // All subsequent operations will use cache only
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disable_network(&self) -> Result<(), FirebaseError> {
        // TODO: Implement network disable
        // - Pause all snapshot listeners
        // - Queue all writes instead of sending immediately
        // - Set flag to use cache-only reads
        todo!("Network disable not yet implemented")
    }

    /// Clear the persistence cache
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:309` - ClearPersistence()
    ///
    /// Clears the persistent storage. This includes pending writes and cached documents.
    /// Must be called while the Firestore instance is not started (after initialization but before any operations).
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// firestore.clear_persistence().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_persistence(&self) -> Result<(), FirebaseError> {
        // TODO: Implement persistence clearing
        // - Delete all cached documents
        // - Clear pending writes queue
        // - Reset metadata (timestamps, versions)
        // - For REDB: db.clear_all_tables()
        // - For IndexedDB: indexedDB.deleteDatabase()
        todo!("Clear persistence not yet implemented")
    }

    /// Wait for pending writes to complete
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore.h:329` - WaitForPendingWrites()
    ///
    /// Waits until all currently pending writes for the active user have been acknowledged by the backend.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// 
    /// // Queue some writes
    /// firestore.set_document("users/alice", json!({"name": "Alice"})).await?;
    /// firestore.set_document("users/bob", json!({"name": "Bob"})).await?;
    /// 
    /// // Wait for all writes to complete
    /// firestore.wait_for_pending_writes().await?;
    /// println!("All writes completed");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_pending_writes(&self) -> Result<(), FirebaseError> {
        // TODO: Implement pending writes wait
        // - Check pending writes queue
        // - Wait for all items to be synced
        // - Return when queue is empty or after timeout
        todo!("Wait for pending writes not yet implemented")
    }
    
    /// Get document data
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:193` - Get()
    pub async fn get_document(&self, path: impl AsRef<str>) -> Result<DocumentSnapshot, FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let response = self.build_request(reqwest::Method::GET, &url).await
            .send()
            .await?;
        
        // Handle 404 (document not found)
        if response.status() == 404 {
            return Ok(DocumentSnapshot {
                reference: DocumentReference::new(path.as_ref()),
                data: None,
                metadata: SnapshotMetadata::default(),
            });
        }
        
        // Handle other errors
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Get failed: {}", response.status())));
        }
        
        let doc_data: serde_json::Value = response.json().await?;
        let fields = doc_data.get("fields").cloned();
        
        // Convert Firestore fields format back to plain JSON
        let data = fields.map(|f| convert_firestore_fields_to_value(&f));
        
        Ok(DocumentSnapshot {
            reference: DocumentReference::new(path.as_ref()),
            data,
            metadata: SnapshotMetadata {
                has_pending_writes: false,
                is_from_cache: false,
            },
        })
    }
    
    /// Set (write/overwrite) document data
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:206` - Set()
    pub async fn set_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let doc = serde_json::json!({ "fields": convert_value_to_firestore_fields(&data) });
        
        let response = self.build_request(reqwest::Method::PATCH, &url).await
            .json(&doc)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Set failed: {}", response.status())));
        }
        
        Ok(())
    }
    
    /// Update specific fields in document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:219` - Update()
    pub async fn update_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError> {
        let mut update_mask = Vec::new();
        if let Some(obj) = data.as_object() {
            update_mask.extend(obj.keys().map(|k| k.as_str()));
        }
        
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}?updateMask.fieldPaths={}",
            self.project_id(),
            self.database_id(),
            path.as_ref(),
            update_mask.join("&updateMask.fieldPaths=")
        );
        
        let doc = serde_json::json!({ "fields": convert_value_to_firestore_fields(&data) });
        
        let response = self.build_request(reqwest::Method::PATCH, &url).await
            .json(&doc)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(FirebaseError::Internal(format!("Update failed: {}", response.status())));
        }
        
        Ok(())
    }
    
    /// Delete document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_reference.h:243` - Delete()
    pub async fn delete_document(&self, path: impl AsRef<str>) -> Result<(), FirebaseError> {
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id(),
            self.database_id(),
            path.as_ref()
        );
        
        let response = self.build_request(reqwest::Method::DELETE, &url).await
            .send()
            .await?;
        
        // 404 is acceptable for delete
        if !response.status().is_success() && response.status() != 404 {
            return Err(FirebaseError::Internal(format!("Delete failed: {}", response.status())));
        }
        
        Ok(())
    }

    /// Add a snapshot listener to a document
    ///
    /// Returns a stream of document snapshots that emits whenever the document changes.
    /// The listener will continue until the returned ListenerRegistration is dropped
    /// or explicitly removed.
    ///
    /// # C++ Reference
    /// - `firestore/src/common/document_reference.cc:184` - AddSnapshotListener
    /// - `firestore/src/include/firebase/firestore/document_reference.h:264`
    ///
    /// # Arguments
    /// * `path` - Document path (e.g. "users/alice")
    ///
    /// # Returns
    /// Tuple of (ListenerRegistration, Stream<Result<DocumentSnapshot>>)
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # use futures::StreamExt;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::get_instance(&app).await?;
    /// 
    /// let (registration, mut stream) = firestore.add_document_snapshot_listener("users/alice").await?;
    /// 
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(snapshot) => {
    ///             if snapshot.exists() {
    ///                 println!("Document updated: {:?}", snapshot.data);
    ///             } else {
    ///                 println!("Document deleted");
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Listener error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn listen_to_document<F>(
        &self,
        path: impl Into<String>,
        callback: F,
    ) -> Result<crate::firestore::listener::ListenerRegistration, FirebaseError>
    where
        F: FnMut(Result<DocumentSnapshot, FirebaseError>) + Send + 'static,
    {
        use crate::auth::Auth;
        
        let path = path.into();
        
        // Get the current user's ID token for authentication
        let auth = Auth::get_auth(&self.inner.app).await
            .map_err(|e| FirebaseError::internal(format!("Failed to get Auth: {}", e)))?;
        
        let user = auth.current_user().await
            .ok_or_else(|| FirebaseError::Auth(crate::error::AuthError::UserNotFound))?;
        
        let auth_token = user.get_id_token(false).await
            .map_err(|e| FirebaseError::internal(format!("Failed to get ID token: {}", e)))?;
        
        // Call the gRPC listener implementation
        crate::firestore::listener::add_document_listener(
            auth_token,
            self.project_id().to_string(),
            self.database_id().to_string(),
            path,
            crate::firestore::listener::ListenerOptions::default(),
            callback,
        ).await
    }
    
    /// Add document snapshot listener (legacy polling implementation)
    /// 
    /// **Note:** This uses REST API polling. For production use, prefer `listen_to_document()`
    /// which uses gRPC bidirectional streaming for real-time updates.
    pub async fn add_document_snapshot_listener(
        &self,
        path: impl Into<String>,
    ) -> Result<(
        crate::firestore::types::ListenerRegistration,
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<crate::firestore::types::DocumentSnapshot, FirebaseError>> + Send>>,
    ), FirebaseError> {
        use crate::firestore::types::{DocumentSnapshot, ListenerRegistration};
        
        let path = path.into();
        let listener_id = format!("doc_listener_{}", uuid::Uuid::new_v4());
        let registration = ListenerRegistration::new(listener_id.clone());
        let cancelled = registration.cancelled.clone();
        
        let project_id = self.project_id().to_string();
        let database_id = self.database_id().to_string();
        let client = self.http_client().clone();
        
        // Create a stream that polls the document periodically
        let stream = async_stream::stream! {
            let mut last_update_time = None;
            
            while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                // Poll the document
                let url = format!(
                    "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
                    project_id, database_id, path
                );
                
                let response = match client.get(&url).send().await {
                    Err(e) => {
                        yield Err(FirebaseError::Internal(format!("Listener request failed: {}", e)));
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    Ok(r) => r,
                };
                
                if response.status() == 404 {
                    // Document doesn't exist
                    yield Ok(DocumentSnapshot {
                        reference: crate::firestore::types::DocumentReference::new(path.clone()),
                        data: None,
                        metadata: crate::firestore::types::SnapshotMetadata::default(),
                    });
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
                
                if !response.status().is_success() {
                    yield Err(FirebaseError::Internal(format!("Listener request failed: {}", response.status())));
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
                
                let doc: serde_json::Value = match response.json().await {
                    Err(e) => {
                        yield Err(FirebaseError::Internal(format!("Failed to parse document: {}", e)));
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    Ok(d) => d,
                };
                
                // Check if document changed
                let update_time = doc.get("updateTime").and_then(|v| v.as_str()).map(|s| s.to_string());
                if last_update_time.as_ref() != update_time.as_ref() {
                    last_update_time = update_time;
                    
                    let data = if let Some(fields) = doc.get("fields") {
                        Some(convert_firestore_fields_to_value(fields))
                    } else {
                        None
                    };
                    
                    yield Ok(DocumentSnapshot {
                        reference: crate::firestore::types::DocumentReference::new(path.clone()),
                        data,
                        metadata: crate::firestore::types::SnapshotMetadata::default(),
                    });
                }
                
                // Poll every second
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        };
        
        Ok((registration, Box::pin(stream)))
    }

    /// Add a snapshot listener to a query
    ///
    /// Returns a stream of query snapshots that emits whenever the query results change.
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:642` - AddSnapshotListener
    ///
    /// # Arguments
    /// * `query` - Query to listen to
    ///
    /// # Returns
    /// Tuple of (ListenerRegistration, Stream<Result<QuerySnapshot>>)
    pub async fn add_query_snapshot_listener(
        &self,
        query: Query,
    ) -> Result<(
        crate::firestore::types::ListenerRegistration,
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<crate::firestore::types::QuerySnapshot, FirebaseError>> + Send>>,
    ), FirebaseError> {
        use crate::firestore::types::{ListenerRegistration, QuerySnapshot};
        
        let listener_id = format!("query_listener_{}", uuid::Uuid::new_v4());
        let registration = ListenerRegistration::new(listener_id.clone());
        let cancelled = registration.cancelled.clone();
        
        let firestore = self.clone();
        let query_clone = query.clone();
        
        // Create a stream that polls the query periodically
        let stream = async_stream::stream! {
            let mut last_doc_count = 0;
            let _last_update_times: Vec<Option<String>> = Vec::new();
            
            while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                // Execute the query
                let docs = match firestore.execute_query(query_clone.clone()).await {
                    Err(e) => {
                        yield Err(e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    Ok(d) => d,
                };
                
                // Check if results changed (simple comparison)
                let current_count = docs.len();
                let has_changed = current_count != last_doc_count;
                
                if has_changed {
                    last_doc_count = current_count;
                    
                    yield Ok(QuerySnapshot::new(
                        query_clone.collection_path.clone(),
                        docs,
                    ));
                }
                
                // Poll every second
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        };
        
        Ok((registration, Box::pin(stream)))
    }

    /// Commit a write batch atomically
    ///
    /// # C++ Reference
    /// - `firestore/src/main/write_batch_main.cc:70` - WriteBatchInternal::Commit
    /// - `firestore/src/common/write_batch.cc:140` - WriteBatch::Commit
    ///
    /// Executes all write operations in the batch atomically. If any operation fails,
    /// the entire batch is rolled back and no changes are applied.
    ///
    /// Uses the Firestore REST API `:commit` endpoint which accepts a batch of writes.
    ///
    /// # Arguments
    /// * `batch` - WriteBatch containing operations to commit
    ///
    /// # Errors
    /// Returns `FirebaseError` if:
    /// - Batch is empty (nothing to commit)
    /// - Network request fails
    /// - Any write operation fails (entire batch is rolled back)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::{Firestore, types::WriteBatch};
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let mut batch = WriteBatch::new();
    /// batch.set("users/alice", json!({"name": "Alice", "age": 30}))
    ///      .update("users/bob", json!({"age": 31}))
    ///      .delete("users/charlie");
    /// firestore.commit_batch(batch).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commit_batch(&self, batch: crate::firestore::types::WriteBatch) -> Result<(), FirebaseError> {
        use crate::error::FirestoreError;
        use crate::firestore::types::WriteOperation;

        // Error-first: check if batch is empty
        if batch.is_empty() {
            return Err(FirebaseError::Firestore(
                FirestoreError::InvalidArgument("Batch is empty, nothing to commit".to_string()),
            ));
        }

        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents:commit",
            self.project_id(),
            self.database_id()
        );

        // Build writes array for the batch commit request
        let writes: Vec<serde_json::Value> = batch.operations().iter().map(|op| {
            match op {
                WriteOperation::Set { path, data } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    serde_json::json!({
                        "update": {
                            "name": full_path,
                            "fields": convert_value_to_firestore_fields(data)
                        }
                    })
                }
                WriteOperation::Update { path, data } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    let update_mask: Vec<String> = if let Some(obj) = data.as_object() {
                        obj.keys().cloned().collect()
                    } else {
                        vec![]
                    };
                    serde_json::json!({
                        "update": {
                            "name": full_path,
                            "fields": convert_value_to_firestore_fields(data)
                        },
                        "updateMask": {
                            "fieldPaths": update_mask
                        }
                    })
                }
                WriteOperation::Delete { path } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    serde_json::json!({
                        "delete": full_path
                    })
                }
            }
        }).collect();

        let request_body = serde_json::json!({
            "writes": writes
        });

        let response = match self.http_client()
            .post(&url)
            .json(&request_body)
            .send()
            .await
        {
            Err(e) => return Err(FirebaseError::Firestore(
                FirestoreError::Internal(format!("Batch commit failed: {}", e))
            )),
            Ok(resp) => resp,
        };

        // Error-first: check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(FirebaseError::Firestore(
                FirestoreError::Internal(format!("Batch commit failed: {} - {}", status, error_text))
            ));
        }

        Ok(())
    }

    /// Run a transaction for atomic read-modify-write operations
    ///
    /// Transactions allow you to read and write data atomically. All reads must
    /// happen before any writes. The transaction will automatically retry up to
    /// max_attempts times if there are conflicts.
    ///
    /// # C++ Reference
    /// - `firestore/src/common/firestore.cc:359` - RunTransaction
    /// - `firestore/src/include/firebase/firestore/transaction.h:42`
    ///
    /// # Arguments
    /// * `callback` - Async function that receives a Transaction reference
    /// * `max_attempts` - Maximum number of retry attempts (default: 5)
    ///
    /// # Example
    /// ```no_run
    /// # use firebase_rust_sdk::firestore::Firestore;
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::get_instance(&app).await?;
    /// 
    /// // Atomic counter increment
    /// firestore.run_transaction(|mut txn| async move {
    ///     let doc = txn.get("counters/visits").await?;
    ///     let count = doc.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
    ///     txn.set("counters/visits", json!({"value": count + 1}));
    ///     Ok(())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_transaction<F, Fut>(&self, callback: F) -> Result<(), FirebaseError>
    where
        F: Fn(crate::firestore::types::Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<crate::firestore::types::Transaction, FirebaseError>>,
    {
        self.run_transaction_with_options(callback, 5).await
    }

    /// Run a transaction with custom retry options
    ///
    /// # C++ Reference
    /// - `firestore/src/common/firestore.cc:359` - RunTransaction with TransactionOptions
    ///
    /// # Arguments
    /// * `callback` - Async function that receives a Transaction reference
    /// * `max_attempts` - Maximum number of retry attempts
    pub async fn run_transaction_with_options<F, Fut>(
        &self,
        callback: F,
        max_attempts: u32,
    ) -> Result<(), FirebaseError>
    where
        F: Fn(crate::firestore::types::Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<crate::firestore::types::Transaction, FirebaseError>>,
    {
        use crate::error::FirestoreError;
        use crate::firestore::types::Transaction;

        // Error-first: validate max_attempts
        if max_attempts == 0 {
            return Err(FirebaseError::Firestore(
                FirestoreError::InvalidArgument("max_attempts must be at least 1".to_string()),
            ));
        }

        let mut last_error = None;

        for attempt in 0..max_attempts {
            // Create a new transaction for this attempt
            let txn = Transaction::new(
                self.project_id().to_string(),
                self.database_id().to_string(),
                self.inner.app.options().api_key.clone(),
            );

            // Run the callback and get the modified transaction back
            let txn = match callback(txn).await {
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
                Ok(txn) => txn,
            };

            // Try to commit the transaction
            match self.commit_transaction(&txn).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Check if it's a conflict error that should be retried
                    if attempt + 1 < max_attempts {
                        last_error = Some(e);
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // All attempts failed
        Err(last_error.unwrap_or_else(|| {
            FirebaseError::Firestore(FirestoreError::Internal(
                "Transaction failed after all retry attempts".to_string(),
            ))
        }))
    }

    /// Commit a transaction (internal method)
    async fn commit_transaction(&self, txn: &crate::firestore::types::Transaction) -> Result<(), FirebaseError> {
        use crate::error::FirestoreError;
        use crate::firestore::types::WriteOperation;

        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents:commit",
            self.project_id(),
            self.database_id()
        );

        // Build writes array
        let writes: Vec<serde_json::Value> = txn.operations().iter().map(|op| {
            match op {
                WriteOperation::Set { path, data } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    serde_json::json!({
                        "update": {
                            "name": full_path,
                            "fields": convert_value_to_firestore_fields(data)
                        }
                    })
                }
                WriteOperation::Update { path, data } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    let update_mask: Vec<String> = if let Some(obj) = data.as_object() {
                        obj.keys().cloned().collect()
                    } else {
                        vec![]
                    };
                    serde_json::json!({
                        "update": {
                            "name": full_path,
                            "fields": convert_value_to_firestore_fields(data)
                        },
                        "updateMask": {
                            "fieldPaths": update_mask
                        }
                    })
                }
                WriteOperation::Delete { path } => {
                    let full_path = format!(
                        "projects/{}/databases/{}/documents/{}",
                        self.project_id(),
                        self.database_id(),
                        path
                    );
                    serde_json::json!({
                        "delete": full_path
                    })
                }
            }
        }).collect();

        let mut request_body = serde_json::json!({
            "writes": writes
        });

        // Add transaction ID if present
        if let Some(txn_id) = txn.id() {
            request_body["transaction"] = serde_json::json!(txn_id);
        }

        let response = match self.http_client()
            .post(&url)
            .json(&request_body)
            .send()
            .await
        {
            Err(e) => return Err(FirebaseError::Firestore(
                FirestoreError::Internal(format!("Transaction commit failed: {}", e))
            )),
            Ok(resp) => resp,
        };

        // Error-first: check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(FirebaseError::Firestore(
                FirestoreError::Internal(format!("Transaction commit failed: {} - {}", status, error_text))
            ));
        }

        Ok(())
    }

    /// Convert a single filter to Firestore REST API format
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/filter.h:268` - And(filters)
    /// - `firestore/src/include/firebase/firestore/filter.h:308` - Or(filters)
    fn convert_filter_to_json(filter: &crate::firestore::types::FilterCondition) -> serde_json::Value {
        use crate::firestore::types::FilterCondition;

        match filter {
            // Compound filters (recursive)
            FilterCondition::And(filters) => {
                // Error-first: empty And is a no-op
                if filters.is_empty() {
                    return serde_json::json!(null);
                }
                // Single filter acts as that filter
                if filters.len() == 1 {
                    return Self::convert_filter_to_json(&filters[0]);
                }
                serde_json::json!({
                    "compositeFilter": {
                        "op": "AND",
                        "filters": filters.iter().map(|f| Self::convert_filter_to_json(f)).collect::<Vec<_>>()
                    }
                })
            }
            FilterCondition::Or(filters) => {
                // Error-first: empty Or is a no-op
                if filters.is_empty() {
                    return serde_json::json!(null);
                }
                // Single filter acts as that filter
                if filters.len() == 1 {
                    return Self::convert_filter_to_json(&filters[0]);
                }
                serde_json::json!({
                    "compositeFilter": {
                        "op": "OR",
                        "filters": filters.iter().map(|f| Self::convert_filter_to_json(f)).collect::<Vec<_>>()
                    }
                })
            }
            // Field filters
            FilterCondition::Equal(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "EQUAL",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::LessThan(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "LESS_THAN",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::LessThanOrEqual(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "LESS_THAN_OR_EQUAL",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::GreaterThan(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "GREATER_THAN",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::GreaterThanOrEqual(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "GREATER_THAN_OR_EQUAL",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::ArrayContains(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "ARRAY_CONTAINS",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::ArrayContainsAny(field, vals) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "ARRAY_CONTAINS_ANY",
                        "value": {
                            "arrayValue": {
                                "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                            }
                        }
                    }
                })
            }
            FilterCondition::In(field, vals) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "IN",
                        "value": {
                            "arrayValue": {
                                "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                            }
                        }
                    }
                })
            }
            FilterCondition::NotEqual(field, val) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "NOT_EQUAL",
                        "value": convert_value_to_firestore(val.clone())
                    }
                })
            }
            FilterCondition::NotIn(field, vals) => {
                serde_json::json!({
                    "fieldFilter": {
                        "field": {"fieldPath": field},
                        "op": "NOT_IN",
                        "value": {
                            "arrayValue": {
                                "values": vals.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>()
                            }
                        }
                    }
                })
            }
        }
    }

    /// Execute a query (internal method)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:318` - Get()
    async fn execute_query(&self, query: Query) -> Result<Vec<DocumentSnapshot>, FirebaseError> {
        use crate::firestore::types::OrderDirection;

        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents:runQuery",
            self.project_id(),
            self.database_id()
        );

        // Build structured query
        let mut structured_query = serde_json::json!({
            "from": [{
                "collectionId": query.collection_path.rsplit('/').next().unwrap_or(&query.collection_path)
            }]
        });

        // Add filters using convert_filter_to_json helper
        if !query.filters.is_empty() {
            let filters: Vec<serde_json::Value> = query.filters.iter()
                .map(|filter| Self::convert_filter_to_json(filter))
                .collect();

            if filters.len() == 1 {
                // Safe: We just checked filters.len() == 1
                if let Some(filter) = filters.into_iter().next() {
                    structured_query["where"] = filter;
                }
            } else {
                structured_query["where"] = serde_json::json!({
                    "compositeFilter": {
                        "op": "AND",
                        "filters": filters
                    }
                });
            }
        }

        // Add order by
        if !query.order_by.is_empty() {
            structured_query["orderBy"] = serde_json::json!(
                query.order_by.iter().map(|(field, direction)| {
                    serde_json::json!({
                        "field": {"fieldPath": field},
                        "direction": match direction {
                            OrderDirection::Ascending => "ASCENDING",
                            OrderDirection::Descending => "DESCENDING",
                        }
                    })
                }).collect::<Vec<_>>()
            );
        }

        // Add limit
        if let Some(limit) = query.limit_value {
            structured_query["limit"] = serde_json::json!(limit);
        }

        // Add start cursor (inclusive)
        if let Some(start_values) = &query.start_at {
            structured_query["startAt"] = serde_json::json!({
                "values": start_values.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>(),
                "before": false
            });
        }

        // Add start cursor (exclusive - after)
        if let Some(start_values) = &query.start_after {
            structured_query["startAt"] = serde_json::json!({
                "values": start_values.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>(),
                "before": true
            });
        }

        // Add end cursor (inclusive)
        if let Some(end_values) = &query.end_at {
            structured_query["endAt"] = serde_json::json!({
                "values": end_values.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>(),
                "before": false
            });
        }

        // Add end cursor (exclusive - before)
        if let Some(end_values) = &query.end_before {
            structured_query["endAt"] = serde_json::json!({
                "values": end_values.iter().map(|v| convert_value_to_firestore(v.clone())).collect::<Vec<_>>(),
                "before": true
            });
        }

        let request_body = serde_json::json!({
            "structuredQuery": structured_query
        });

        let response = self.http_client()
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read response body".to_string());
            return Err(FirebaseError::Internal(format!("Query failed: {} - {}", status, error_body)));
        }

        let results: Vec<serde_json::Value> = response.json().await?;
        
        let documents: Vec<DocumentSnapshot> = results
            .into_iter()
            .filter_map(|result| {
                result.get("document").and_then(|doc| {
                    let name = doc.get("name")?.as_str()?;
                    let path = name.split("/documents/").nth(1)?;
                    let fields = doc.get("fields").cloned();
                    
                    // Convert Firestore fields format back to plain JSON
                    let data = fields.map(|f| convert_firestore_fields_to_value(&f));
                    
                    Some(DocumentSnapshot {
                        reference: DocumentReference::new(path),
                        data,
                        metadata: SnapshotMetadata {
                            has_pending_writes: false,
                            is_from_cache: false,
                        },
                    })
                })
            })
            .collect();

        Ok(documents)
    }
}

// TODO Should support timestamps in both directions
// ? Need to figure if reference types are needed
/// Convert serde_json::Value to Firestore value format
fn convert_value_to_firestore(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({"nullValue": null}),
        serde_json::Value::Bool(b) => serde_json::json!({"booleanValue": b}),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::json!({"integerValue": i.to_string()})
            } else if let Some(f) = n.as_f64() {
                serde_json::json!({"doubleValue": f})
            } else {
                serde_json::json!({"nullValue": null})
            }
        }
        serde_json::Value::String(s) => serde_json::json!({"stringValue": s}),
        serde_json::Value::Array(arr) => {
            serde_json::json!({
                "arrayValue": {
                    "values": arr.into_iter().map(convert_value_to_firestore).collect::<Vec<_>>()
                }
            })
        }
        serde_json::Value::Object(obj) => {
            serde_json::json!({
                "mapValue": {
                    "fields": obj.into_iter().map(|(k, v)| (k, convert_value_to_firestore(v))).collect::<serde_json::Map<String, serde_json::Value>>()
                }
            })
        }
    }
}

/// Convert a JSON object to Firestore fields format
///
/// Takes a JSON Value and converts it to the fields format expected by Firestore REST API.
/// If the value is an object, returns a map of field names to Firestore values.
/// Otherwise returns an empty map.
fn convert_value_to_firestore_fields(value: &serde_json::Value) -> serde_json::Value {
    if let Some(obj) = value.as_object() {
        let fields: serde_json::Map<String, serde_json::Value> = obj
            .iter()
            .map(|(k, v)| (k.clone(), convert_value_to_firestore(v.clone())))
            .collect();
        serde_json::Value::Object(fields)
    } else {
        serde_json::json!({})
    }
}

/// Convert Firestore fields format to plain JSON (inverse of convert_value_to_firestore_fields)
fn convert_firestore_fields_to_value(fields: &serde_json::Value) -> serde_json::Value {
    use serde_json::{json, Map, Value as JsonValue};
    
    if let Some(obj) = fields.as_object() {
        let mut result = Map::new();
        for (key, value) in obj {
            result.insert(key.clone(), convert_firestore_value_to_json(value));
        }
        JsonValue::Object(result)
    } else {
        json!({})
    }
}

/// Convert a single Firestore value to plain JSON
fn convert_firestore_value_to_json(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;

    // Firestore format: {"integerValue": "123"} or {"stringValue": "hello"}
    if let Some(obj) = value.as_object() {
        if let Some(string_val) = obj.get("stringValue") {
            return string_val.clone();
        } else if let Some(int_val) = obj.get("integerValue") {
            if let Some(s) = int_val.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return json!(n);
                }
            }
            return int_val.clone();
        } else if let Some(double_val) = obj.get("doubleValue") {
            return double_val.clone();
        } else if let Some(bool_val) = obj.get("booleanValue") {
            return bool_val.clone();
        } else if let Some(_null_val) = obj.get("nullValue") {
            return json!(null);
        } else if let Some(array_val) = obj.get("arrayValue") {
            if let Some(values) = array_val.get("values").and_then(|v| v.as_array()) {
                return json!(values.iter().map(|v| convert_firestore_value_to_json(v)).collect::<Vec<_>>());
            }
        } else if let Some(map_val) = obj.get("mapValue") {
            if let Some(fields) = map_val.get("fields") {
                return convert_firestore_fields_to_value(fields);
            }
        }
    }
    
    value.clone()
}

/// Firestore query builder
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:76`
#[derive(Clone)]
pub struct Query {
    firestore: Firestore,
    collection_path: String,
    filters: Vec<crate::firestore::types::FilterCondition>,
    order_by: Vec<(String, crate::firestore::types::OrderDirection)>,
    limit_value: Option<usize>,
    start_at: Option<Vec<serde_json::Value>>,
    start_after: Option<Vec<serde_json::Value>>,
    end_at: Option<Vec<serde_json::Value>>,
    end_before: Option<Vec<serde_json::Value>>,
}

impl Query {
    /// Create a new query for a collection
    pub(crate) fn new(firestore: Firestore, collection_path: String) -> Self {
        Self {
            firestore,
            collection_path,
            filters: Vec::new(),
            order_by: Vec::new(),
            limit_value: None,
            start_at: None,
            start_after: None,
            end_at: None,
            end_before: None,
        }
    }

    /// Add a filter condition to the query
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:142` - Where()
    pub fn where_filter(mut self, condition: crate::firestore::types::FilterCondition) -> Self {
        self.filters.push(condition);
        self
    }

    /// Order results by a field
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:204` - OrderBy()
    pub fn order_by(
        mut self,
        field: impl Into<String>,
        direction: crate::firestore::types::OrderDirection,
    ) -> Self {
        self.order_by.push((field.into(), direction));
        self
    }

    /// Limit the number of results
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:232` - Limit()
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Start query at cursor values (inclusive)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:285` - StartAt()
    ///
    /// Creates a query that starts at the provided fields relative to the order of the query.
    /// The order of the field values must match the order of the order by clauses.
    ///
    /// # Arguments
    /// * `values` - Field values to start at (inclusive)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .order_by("age", firebase_rust_sdk::firestore::types::OrderDirection::Ascending)
    ///     .start_at(vec![json!(25)])  // Start at age 25 (inclusive)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_at(mut self, values: Vec<serde_json::Value>) -> Self {
        self.start_at = Some(values);
        self.start_after = None;  // Clear conflicting cursor
        self
    }

    /// Start query after cursor values (exclusive)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:555` - StartAfter()
    ///
    /// Creates a query that starts after the provided fields relative to the order of the query.
    /// The order of the field values must match the order of the order by clauses.
    ///
    /// # Arguments
    /// * `values` - Field values to start after (exclusive)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .order_by("age", firebase_rust_sdk::firestore::types::OrderDirection::Ascending)
    ///     .start_after(vec![json!(25)])  // Start after age 25 (exclusive)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_after(mut self, values: Vec<serde_json::Value>) -> Self {
        self.start_after = Some(values);
        self.start_at = None;  // Clear conflicting cursor
        self
    }

    /// End query at cursor values (inclusive)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:298` - EndAt()
    ///
    /// Creates a query that ends at the provided fields relative to the order of the query.
    /// The order of the field values must match the order of the order by clauses.
    ///
    /// # Arguments
    /// * `values` - Field values to end at (inclusive)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .order_by("age", firebase_rust_sdk::firestore::types::OrderDirection::Ascending)
    ///     .end_at(vec![json!(65)])  // End at age 65 (inclusive)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn end_at(mut self, values: Vec<serde_json::Value>) -> Self {
        self.end_at = Some(values);
        self.end_before = None;  // Clear conflicting cursor
        self
    }

    /// End query before cursor values (exclusive)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:579` - EndBefore()
    ///
    /// Creates a query that ends before the provided fields relative to the order of the query.
    /// The order of the field values must match the order of the order by clauses.
    ///
    /// # Arguments
    /// * `values` - Field values to end before (exclusive)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .order_by("age", firebase_rust_sdk::firestore::types::OrderDirection::Ascending)
    ///     .end_before(vec![json!(65)])  // End before age 65 (exclusive)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn end_before(mut self, values: Vec<serde_json::Value>) -> Self {
        self.end_before = Some(values);
        self.end_at = None;  // Clear conflicting cursor
        self
    }

    /// Execute the query
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/query.h:318` - Get()
    pub async fn get(self) -> Result<Vec<DocumentSnapshot>, FirebaseError> {
        let firestore = self.firestore.clone();
        firestore.execute_query(self).await
    }
}

impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("collection_path", &self.collection_path)
            .field("filters", &self.filters.len())
            .field("order_by", &self.order_by)
            .field("limit", &self.limit_value)
            .finish()
    }
}

/// Collection reference
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/collection_reference.h`
pub struct CollectionReference {
    firestore: Firestore,
    path: String,
}

impl CollectionReference {
    fn new(firestore: Firestore, path: String) -> Self {
        Self { firestore, path }
    }

    /// Get the collection path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the collection ID (last segment of path)
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get a document reference within this collection
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/collection_reference.h:96` - Document(path)
    pub fn document(&self, document_id: impl AsRef<str>) -> DocumentReference {
        let full_path = format!("{}/{}", self.path, document_id.as_ref());
        DocumentReference::new(full_path)
    }

    /// Add a new document with auto-generated ID
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/collection_reference.h:124` - Add(data)
    /// - `firestore/src/main/collection_reference_main.cc:78` - AddDocument implementation
    ///
    /// Generates a 20-character alphanumeric ID and creates the document.
    /// This follows Firestore's auto-ID generation pattern.
    ///
    /// # Arguments
    /// * `data` - Document data as JSON value
    ///
    /// # Errors
    /// Returns `FirebaseError::FirestoreError` if:
    /// - Data is not a JSON object
    /// - Network request fails
    /// - API returns an error
    ///
    /// # Example
    /// ```no_run
    /// use firebase_rust_sdk::firestore::Firestore;
    /// use serde_json::json;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let doc_ref = firestore.collection("users")
    ///     .add(json!({"name": "Alice", "age": 30}))
    ///     .await?;
    /// println!("Created document: {}", doc_ref.path);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add(&self, data: serde_json::Value) -> Result<DocumentReference, FirebaseError> {
        use crate::error::FirestoreError;
        
        // Error-first: validate data is an object
        if !data.is_object() {
            return Err(FirebaseError::Firestore(
                FirestoreError::InvalidData("Data must be a JSON object".to_string()),
            ));
        }

        // Generate auto-ID: 20 alphanumeric characters
        // Firestore uses [a-zA-Z0-9] for auto-generated IDs
        let doc_id = generate_auto_id();
        let doc_ref = self.document(&doc_id);

        // Use set_document to create the document
        self.firestore.set_document(&doc_ref.path, data).await?;

        Ok(doc_ref)
    }

    /// Create a query for this collection
    ///
    /// # C++ Reference
    /// - Query inherits from CollectionReference in C++
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::{Firestore, types::{FilterCondition, OrderDirection}};
    /// use serde_json::json;
    ///
    /// let firestore = Firestore::get_instance(&app).await?;
    /// let docs = firestore.collection("users")
    ///     .query()
    ///     .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
    ///     .order_by("age", OrderDirection::Ascending)
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(&self) -> Query {
        Query::new(self.firestore.clone(), self.path.clone())
    }
}

/// Generate a Firestore auto-ID
///
/// Creates a 20-character random alphanumeric string matching Firestore's
/// auto-ID generation pattern: [a-zA-Z0-9]{20}
///
/// # C++ Reference
/// - Firestore auto-ID generation (called by AddDocument)
fn generate_auto_id() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const ID_LENGTH: usize = 20;
    
    let mut rng = rand::thread_rng();
    (0..ID_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

impl std::fmt::Debug for Firestore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Firestore")
            .field("project_id", &self.inner.project_id)
            .field("database_id", &self.inner.database_id)
            .finish()
    }
}

impl std::fmt::Debug for CollectionReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectionReference")
            .field("path", &self.path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, AppOptions};

    // Helper to create test App instances
    async fn test_app(project_id: &str) -> App {
        App::create(AppOptions {
            api_key: "test-api-key".to_string(),
            project_id: project_id.to_string(),
            app_name: Some(format!("test-{}-{}", project_id, rand::random::<u32>())),
        }).await.unwrap()
    }

    #[tokio::test]
    async fn test_get_firestore_creates_instance() {
        let app = test_app("test-project-1").await;
        let fs = Firestore::get_instance(&app).await.unwrap();
        assert_eq!(fs.project_id(), "test-project-1");
        assert_eq!(fs.database_id(), "default");
    }

    #[tokio::test]
    async fn test_get_firestore_returns_same_instance() {
        let app = test_app("test-project-2").await;
        let fs1 = Firestore::get_instance(&app).await.unwrap();
        let fs2 = Firestore::get_instance(&app).await.unwrap();

        // Should return same instance (same Arc pointer)
        assert!(Arc::ptr_eq(&fs1.inner, &fs2.inner));
    }

    #[tokio::test]
    async fn test_get_firestore_empty_project_error() {
        // App creation should fail with empty project_id
        let result = App::create(AppOptions {
            api_key: "test-api-key".to_string(),
            project_id: "".to_string(),
            app_name: None,
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_projects_different_instances() {
        let fs1 = Firestore::get_instance(&test_app("project-a").await).await.unwrap();
        let fs2 = Firestore::get_instance(&test_app("project-b").await).await.unwrap();

        // Should be different instances
        assert!(!Arc::ptr_eq(&fs1.inner, &fs2.inner));
        assert_eq!(fs1.project_id(), "project-a");
        assert_eq!(fs2.project_id(), "project-b");
    }

    #[tokio::test]
    async fn test_different_databases_different_instances() {
        let app = test_app("project-c").await;
        let fs1 = Firestore::get_instance_with_database(&app, "default")
            .await
            .unwrap();
        let fs2 = Firestore::get_instance_with_database(&app, "custom-db")
            .await
            .unwrap();

        // Should be different instances even with same project
        assert!(!Arc::ptr_eq(&fs1.inner, &fs2.inner));
        assert_eq!(fs1.database_id(), "default");
        assert_eq!(fs2.database_id(), "custom-db");
    }

    #[tokio::test]
    async fn test_collection_reference() {
        let fs = Firestore::get_instance(&test_app("test-project-3").await).await.unwrap();
        let users = fs.collection("users");

        assert_eq!(users.path(), "users");
        assert_eq!(users.id(), "users");
    }

    #[tokio::test]
    async fn test_collection_document() {
        let fs = Firestore::get_instance(&test_app("test-project-4").await).await.unwrap();
        let users = fs.collection("users");
        let alice = users.document("alice");

        assert_eq!(alice.path, "users/alice");
        assert_eq!(alice.id(), "alice");
    }

    #[tokio::test]
    async fn test_document_reference() {
        let fs = Firestore::get_instance(&test_app("test-project-5").await).await.unwrap();
        let doc = fs.document("users/bob");

        assert_eq!(doc.path, "users/bob");
        assert_eq!(doc.id(), "bob");
    }

    #[tokio::test]
    async fn test_nested_collection_reference() {
        let fs = Firestore::get_instance(&test_app("test-project-6").await).await.unwrap();
        let posts = fs.collection("users/alice/posts");

        assert_eq!(posts.path(), "users/alice/posts");
        assert_eq!(posts.id(), "posts");
    }

    #[tokio::test]
    async fn test_query_builder() {
        use crate::firestore::types::{FilterCondition, OrderDirection};
        use serde_json::json;

        let fs = Firestore::get_instance(&test_app("test-project-7").await).await.unwrap();
        let query = fs.collection("users")
            .query()
            .where_filter(FilterCondition::Equal("name".to_string(), json!("Alice")))
            .order_by("age", OrderDirection::Ascending)
            .limit(10);

        // Verify query structure (can't execute without real Firebase)
        assert_eq!(query.collection_path, "users");
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.order_by.len(), 1);
        assert_eq!(query.limit_value, Some(10));
    }

    #[tokio::test]
    async fn test_query_multiple_filters() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        let fs = Firestore::get_instance(&test_app("test-project-8").await).await.unwrap();
        let query = fs.collection("users")
            .query()
            .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
            .where_filter(FilterCondition::LessThan("age".to_string(), json!(65)))
            .where_filter(FilterCondition::Equal("active".to_string(), json!(true)));

        assert_eq!(query.filters.len(), 3);
    }

    #[tokio::test]
    async fn test_query_with_cursors() {
        use serde_json::json;

        let fs = Firestore::get_instance(&test_app("test-project-9").await).await.unwrap();
        let query = fs.collection("posts")
            .query()
            .start_at(vec![json!("2024-01-01")])
            .end_at(vec![json!("2024-12-31")]);

        assert!(query.start_at.is_some());
        assert!(query.end_at.is_some());
        assert!(query.start_after.is_none());
        assert!(query.end_before.is_none());
    }

    #[tokio::test]
    async fn test_query_with_start_after() {
        use serde_json::json;
        use crate::firestore::types::OrderDirection;

        let fs = Firestore::get_instance(&test_app("test-project-pagination-1").await).await.unwrap();
        let query = fs.collection("users")
            .query()
            .order_by("age", OrderDirection::Ascending)
            .start_after(vec![json!(25)]);

        assert!(query.start_after.is_some());
        assert!(query.start_at.is_none());  // start_after should clear start_at
        assert_eq!(query.start_after.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_end_before() {
        use serde_json::json;
        use crate::firestore::types::OrderDirection;

        let fs = Firestore::get_instance(&test_app("test-project-pagination-2").await).await.unwrap();
        let query = fs.collection("users")
            .query()
            .order_by("age", OrderDirection::Ascending)
            .end_before(vec![json!(65)]);

        assert!(query.end_before.is_some());
        assert!(query.end_at.is_none());  // end_before should clear end_at
        assert_eq!(query.end_before.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_query_pagination_range() {
        use serde_json::json;
        use crate::firestore::types::OrderDirection;

        let fs = Firestore::get_instance(&test_app("test-project-pagination-3").await).await.unwrap();
        let query = fs.collection("products")
            .query()
            .order_by("price", OrderDirection::Ascending)
            .start_after(vec![json!(10.00)])
            .end_before(vec![json!(100.00)])
            .limit(20);

        // Verify pagination configuration
        assert!(query.start_after.is_some());
        assert!(query.end_before.is_some());
        assert_eq!(query.limit_value, Some(20));
        assert_eq!(query.start_after.as_ref().unwrap()[0], json!(10.00));
        assert_eq!(query.end_before.as_ref().unwrap()[0], json!(100.00));
    }

    #[tokio::test]
    async fn test_query_cursor_conflicts() {
        use serde_json::json;

        let fs = Firestore::get_instance(&test_app("test-project-pagination-4").await).await.unwrap();
        
        // start_after should clear start_at
        let query = fs.collection("data")
            .query()
            .start_at(vec![json!(1)])
            .start_after(vec![json!(2)]);
        
        assert!(query.start_after.is_some());
        assert!(query.start_at.is_none());

        // end_before should clear end_at
        let query2 = fs.collection("data")
            .query()
            .end_at(vec![json!(10)])
            .end_before(vec![json!(9)]);
        
        assert!(query2.end_before.is_some());
        assert!(query2.end_at.is_none());
    }

    #[tokio::test]
    async fn test_filter_condition_field_path() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        let filter = FilterCondition::Equal("user.name".to_string(), json!("Bob"));
        assert_eq!(filter.field_path(), "user.name");

        let filter2 = FilterCondition::GreaterThan("score".to_string(), json!(100));
        assert_eq!(filter2.field_path(), "score");
    }

    #[tokio::test]
    async fn test_filter_condition_operators() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;

        assert_eq!(FilterCondition::Equal("f".to_string(), json!(1)).operator(), "EQUAL");
        assert_eq!(FilterCondition::LessThan("f".to_string(), json!(1)).operator(), "LESS_THAN");
        assert_eq!(FilterCondition::GreaterThanOrEqual("f".to_string(), json!(1)).operator(), "GREATER_THAN_OR_EQUAL");
        assert_eq!(FilterCondition::ArrayContains("f".to_string(), json!(1)).operator(), "ARRAY_CONTAINS");
        assert_eq!(FilterCondition::In("f".to_string(), vec![json!(1)]).operator(), "IN");
    }

    #[tokio::test]
    async fn test_convert_value_to_firestore() {
        use serde_json::json;

        // Test null
        let result = convert_value_to_firestore(json!(null));
        assert_eq!(result, json!({"nullValue": null}));

        // Test boolean
        let result = convert_value_to_firestore(json!(true));
        assert_eq!(result, json!({"booleanValue": true}));

        // Test integer
        let result = convert_value_to_firestore(json!(42));
        assert_eq!(result, json!({"integerValue": "42"}));

        // Test string
        let result = convert_value_to_firestore(json!("hello"));
        assert_eq!(result, json!({"stringValue": "hello"}));

        // Test array
        let result = convert_value_to_firestore(json!([1, 2, 3]));
        assert!(result.get("arrayValue").is_some());
    }

    #[tokio::test]
    async fn test_collection_reference_add_generates_id() {
        use serde_json::json;
        
        let fs = Firestore::get_instance(&test_app("test-project-10").await).await.unwrap();
        let collection = fs.collection("users");
        
        // Create document with auto-generated ID (would fail without real Firebase)
        // Test that the ID generation works
        let doc_id_1 = generate_auto_id();
        let doc_id_2 = generate_auto_id();
        
        // IDs should be 20 characters
        assert_eq!(doc_id_1.len(), 20);
        assert_eq!(doc_id_2.len(), 20);
        
        // IDs should be different (statistically)
        assert_ne!(doc_id_1, doc_id_2);
        
        // IDs should only contain alphanumeric characters
        assert!(doc_id_1.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(doc_id_2.chars().all(|c| c.is_ascii_alphanumeric()));
    }
    
    #[tokio::test]
    async fn test_collection_reference_add_validates_data() {
        use serde_json::json;
        use crate::error::{FirebaseError, FirestoreError};
        
        let fs = Firestore::get_instance(&test_app("test-project-11").await).await.unwrap();
        let collection = fs.collection("users");
        
        // Test that non-object data is rejected
        let result = collection.add(json!("not an object")).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            FirebaseError::Firestore(FirestoreError::InvalidData(msg)) => {
                assert!(msg.contains("JSON object"));
            }
            _ => panic!("Expected InvalidData error"),
        }
    }

    #[tokio::test]
    async fn test_write_batch_builder() {
        use serde_json::json;
        use crate::firestore::types::WriteBatch;
        
        let mut batch = WriteBatch::new();
        batch.set("users/alice", json!({"name": "Alice", "age": 30}))
             .update("users/bob", json!({"age": 31}))
             .delete("users/charlie");
        
        assert_eq!(batch.len(), 3);
        assert!(!batch.is_empty());
        
        // Verify operations are stored
        let ops = batch.operations();
        assert_eq!(ops.len(), 3);
    }

    #[tokio::test]
    async fn test_write_batch_empty() {
        use crate::firestore::types::WriteBatch;
        use crate::error::{FirebaseError, FirestoreError};
        
        let fs = Firestore::get_instance(&test_app("test-project-12").await).await.unwrap();
        let batch = WriteBatch::new();
        
        // Empty batch should be rejected
        let result = fs.commit_batch(batch).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            FirebaseError::Firestore(FirestoreError::InvalidArgument(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[tokio::test]
    async fn test_write_batch_operations_chaining() {
        use serde_json::json;
        use crate::firestore::types::{WriteBatch, WriteOperation};
        
        let mut batch = WriteBatch::new();
        
        // Test method chaining
        batch
            .set("path1", json!({"field": "value1"}))
            .update("path2", json!({"field": "value2"}))
            .delete("path3")
            .set("path4", json!({"field": "value3"}));
        
        assert_eq!(batch.len(), 4);
        
        let ops = batch.operations();
        
        // Verify order is preserved
        match &ops[0] {
            WriteOperation::Set { path, .. } => assert_eq!(path, "path1"),
            _ => panic!("Expected Set operation"),
        }
        
        match &ops[1] {
            WriteOperation::Update { path, .. } => assert_eq!(path, "path2"),
            _ => panic!("Expected Update operation"),
        }
        
        match &ops[2] {
            WriteOperation::Delete { path } => assert_eq!(path, "path3"),
            _ => panic!("Expected Delete operation"),
        }
        
        match &ops[3] {
            WriteOperation::Set { path, .. } => assert_eq!(path, "path4"),
            _ => panic!("Expected Set operation"),
        }
    }

    #[tokio::test]
    async fn test_transaction_operations() {
        use crate::firestore::types::{Transaction, WriteOperation};
        use serde_json::json;
        
        let mut txn = Transaction::new(
            "test-project".to_string(),
            "default".to_string(),
            "test-key".to_string(),
        );

        // Test set operation
        txn.set("users/alice", json!({"name": "Alice", "age": 30}));
        
        // Test update operation
        txn.update("users/bob", json!({"age": 31}));
        
        // Test delete operation
        txn.delete("users/charlie");

        assert_eq!(txn.operations().len(), 3);
        
        // Verify operations
        match &txn.operations()[0] {
            WriteOperation::Set { path, data } => {
                assert_eq!(path, "users/alice");
                assert_eq!(data["name"], "Alice");
            }
            _ => panic!("Expected Set operation"),
        }
        
        match &txn.operations()[1] {
            WriteOperation::Update { path, .. } => assert_eq!(path, "users/bob"),
            _ => panic!("Expected Update operation"),
        }
        
        match &txn.operations()[2] {
            WriteOperation::Delete { path } => assert_eq!(path, "users/charlie"),
            _ => panic!("Expected Delete operation"),
        }
    }

    #[tokio::test]
    async fn test_transaction_chaining() {
        use crate::firestore::types::Transaction;
        use serde_json::json;
        
        let mut txn = Transaction::new(
            "test-project".to_string(),
            "default".to_string(),
            "test-key".to_string(),
        );

        // Test chaining
        txn.set("doc1", json!({"a": 1}))
           .update("doc2", json!({"b": 2}))
           .delete("doc3");

        assert_eq!(txn.operations().len(), 3);
    }

    #[tokio::test]
    async fn test_run_transaction_max_attempts_validation() {
        use crate::error::{FirebaseError, FirestoreError};
        
        let app = test_app("test-transaction-validation").await;
        let firestore = Firestore::get_instance(&app).await.unwrap();

        let result = firestore.run_transaction_with_options(
            |txn| async move { Ok(txn) },
            0 // invalid: must be at least 1
        ).await;

        assert!(result.is_err());
        match result {
            Err(FirebaseError::Firestore(FirestoreError::InvalidArgument(msg))) => {
                assert!(msg.contains("max_attempts must be at least 1"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[tokio::test]
    async fn test_listener_registration_creation() {
        use crate::firestore::types::ListenerRegistration;
        
        let registration = ListenerRegistration::new("test_listener_123".to_string());
        assert_eq!(registration.id, "test_listener_123");
        assert!(!registration.is_cancelled());
    }

    #[tokio::test]
    async fn test_listener_registration_remove() {
        use crate::firestore::types::ListenerRegistration;
        
        let registration = ListenerRegistration::new("test_listener".to_string());
        assert!(!registration.is_cancelled());
        
        registration.remove();
        assert!(registration.is_cancelled());
    }

    #[tokio::test]
    async fn test_listener_registration_drop() {
        use crate::firestore::types::ListenerRegistration;
        
        let cancelled = {
            let registration = ListenerRegistration::new("test_listener".to_string());
            let cancelled = registration.cancelled.clone();
            assert!(!cancelled.load(std::sync::atomic::Ordering::SeqCst));
            cancelled
        }; // registration dropped here
        
        // After drop, listener should be cancelled
        assert!(cancelled.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_query_snapshot_creation() {
        use crate::firestore::types::{DocumentReference, DocumentSnapshot, QuerySnapshot, SnapshotMetadata};
        
        let doc1 = DocumentSnapshot {
            reference: DocumentReference::new("users/alice"),
            data: Some(serde_json::json!({"name": "Alice"})),
            metadata: SnapshotMetadata::default(),
        };
        
        let doc2 = DocumentSnapshot {
            reference: DocumentReference::new("users/bob"),
            data: Some(serde_json::json!({"name": "Bob"})),
            metadata: SnapshotMetadata::default(),
        };
        
        let snapshot = QuerySnapshot::new("users".to_string(), vec![doc1, doc2]);
        assert_eq!(snapshot.len(), 2);
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.query_path, "users");
    }

    #[tokio::test]
    async fn test_convert_firestore_fields_roundtrip() {
        use serde_json::json;
        
        let original = json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "score": 95.5
        });
        
        // Convert to Firestore format
        let firestore_fields = convert_value_to_firestore_fields(&original);
        
        // Convert back to plain JSON
        let converted_back = convert_firestore_fields_to_value(&firestore_fields);
        
        assert_eq!(converted_back["name"], "Alice");
        assert_eq!(converted_back["age"], 30);
        assert_eq!(converted_back["active"], true);
        assert_eq!(converted_back["score"], 95.5);
    }

    #[tokio::test]
    async fn test_compound_filter_and() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;
        
        let firestore = Firestore::get_instance(&test_app("test-compound-and").await).await.unwrap();
        
        // Create compound And filter
        let and_filter = FilterCondition::And(vec![
            FilterCondition::GreaterThan("age".to_string(), json!(18)),
            FilterCondition::LessThan("age".to_string(), json!(65)),
            FilterCondition::Equal("active".to_string(), json!(true)),
        ]);
        
        let query = firestore.collection("users")
            .query()
            .where_filter(and_filter);
        
        assert_eq!(query.filters.len(), 1);
        
        // Verify it's an And filter
        match &query.filters[0] {
            FilterCondition::And(filters) => {
                assert_eq!(filters.len(), 3);
            }
            _ => panic!("Expected And filter"),
        }
    }

    #[tokio::test]
    async fn test_compound_filter_or() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;
        
        let firestore = Firestore::get_instance(&test_app("test-compound-or").await).await.unwrap();
        
        // Create compound Or filter
        let or_filter = FilterCondition::Or(vec![
            FilterCondition::Equal("status".to_string(), json!("active")),
            FilterCondition::Equal("status".to_string(), json!("pending")),
        ]);
        
        let query = firestore.collection("orders")
            .query()
            .where_filter(or_filter);
        
        assert_eq!(query.filters.len(), 1);
        
        // Verify it's an Or filter
        match &query.filters[0] {
            FilterCondition::Or(filters) => {
                assert_eq!(filters.len(), 2);
            }
            _ => panic!("Expected Or filter"),
        }
    }

    #[tokio::test]
    async fn test_compound_filter_nested() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;
        
        let firestore = Firestore::get_instance(&test_app("test-compound-nested").await).await.unwrap();
        
        // Create nested compound filter: (age > 18 AND age < 65) OR (status = "vip")
        let nested_filter = FilterCondition::Or(vec![
            FilterCondition::And(vec![
                FilterCondition::GreaterThan("age".to_string(), json!(18)),
                FilterCondition::LessThan("age".to_string(), json!(65)),
            ]),
            FilterCondition::Equal("status".to_string(), json!("vip")),
        ]);
        
        let query = firestore.collection("users")
            .query()
            .where_filter(nested_filter);
        
        assert_eq!(query.filters.len(), 1);
        
        // Verify nested structure
        match &query.filters[0] {
            FilterCondition::Or(or_filters) => {
                assert_eq!(or_filters.len(), 2);
                match &or_filters[0] {
                    FilterCondition::And(and_filters) => {
                        assert_eq!(and_filters.len(), 2);
                    }
                    _ => panic!("Expected And filter in Or"),
                }
            }
            _ => panic!("Expected Or filter"),
        }
    }

    #[tokio::test]
    async fn test_compound_filter_empty_is_noop() {
        use crate::firestore::types::FilterCondition;
        
        // Empty And filter
        let empty_and = FilterCondition::And(vec![]);
        let json = Firestore::convert_filter_to_json(&empty_and);
        assert!(json.is_null());
        
        // Empty Or filter
        let empty_or = FilterCondition::Or(vec![]);
        let json = Firestore::convert_filter_to_json(&empty_or);
        assert!(json.is_null());
    }

    #[tokio::test]
    async fn test_compound_filter_single_unwraps() {
        use crate::firestore::types::FilterCondition;
        use serde_json::json;
        
        // Single filter in And should behave as that filter
        let single_and = FilterCondition::And(vec![
            FilterCondition::Equal("name".to_string(), json!("Alice")),
        ]);
        let json_and = Firestore::convert_filter_to_json(&single_and);
        assert!(json_and["fieldFilter"].is_object());
        assert!(!json_and["compositeFilter"].is_object());
        
        // Single filter in Or should behave as that filter
        let single_or = FilterCondition::Or(vec![
            FilterCondition::Equal("name".to_string(), json!("Bob")),
        ]);
        let json_or = Firestore::convert_filter_to_json(&single_or);
        assert!(json_or["fieldFilter"].is_object());
        assert!(!json_or["compositeFilter"].is_object());
    }

    // ========================================================================
    // Persistence API Tests
    // ========================================================================

    #[tokio::test]
    async fn test_settings_default() {
        use crate::firestore::types::Settings;
        
        let settings = Settings::default();
        
        assert_eq!(settings.host, "firestore.googleapis.com");
        assert!(settings.ssl_enabled);
        assert!(settings.persistence_enabled);
        assert_eq!(settings.cache_size_bytes, 100 * 1024 * 1024); // 100 MB
        assert!(settings.cache_directory.is_none());
    }

    #[tokio::test]
    async fn test_settings_unlimited_cache() {
        use crate::firestore::types::Settings;
        
        let mut settings = Settings::new();
        settings.cache_size_bytes = Settings::CACHE_SIZE_UNLIMITED;
        
        assert_eq!(settings.cache_size_bytes, -1);
    }

    #[tokio::test]
    async fn test_settings_get() {
        let firestore = Firestore::get_instance(&test_app("test-settings-get").await).await.unwrap();
        
        let settings = firestore.settings().await;
        assert!(settings.persistence_enabled);
        assert_eq!(settings.host, "firestore.googleapis.com");
    }

    #[tokio::test]
    #[should_panic(expected = "not yet implemented")]
    async fn test_settings_set_persistence() {
        use crate::firestore::types::Settings;
        
        let firestore = Firestore::get_instance(&test_app("test-settings-set").await).await.unwrap();
        
        let mut settings = Settings::new();
        settings.persistence_enabled = true;
        settings.cache_size_bytes = 50 * 1024 * 1024; // 50 MB
        
        // TODO: This will panic with todo!() until persistence is implemented
        firestore.set_settings(settings).await.unwrap();
    }

    #[tokio::test]
    async fn test_source_enum() {
        use crate::firestore::types::Source;
        
        assert_eq!(Source::default(), Source::Default);
        
        let sources = vec![Source::Default, Source::Server, Source::Cache];
        assert_eq!(sources.len(), 3);
    }

    #[tokio::test]
    #[should_panic(expected = "not yet implemented")]
    async fn test_enable_network() {
        let firestore = Firestore::get_instance(&test_app("test-enable-network").await).await.unwrap();
        
        // TODO: This will panic with todo!() until network control is implemented
        firestore.enable_network().await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "not yet implemented")]
    async fn test_disable_network() {
        let firestore = Firestore::get_instance(&test_app("test-disable-network").await).await.unwrap();
        
        // TODO: This will panic with todo!() until network control is implemented
        firestore.disable_network().await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "not yet implemented")]
    async fn test_clear_persistence() {
        let firestore = Firestore::get_instance(&test_app("test-clear-persistence").await).await.unwrap();
        
        // TODO: This will panic with todo!() until persistence clearing is implemented
        firestore.clear_persistence().await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "not yet implemented")]
    async fn test_wait_for_pending_writes() {
        let firestore = Firestore::get_instance(&test_app("test-pending-writes").await).await.unwrap();
        
        // TODO: This will panic with todo!() until pending writes queue is implemented
        firestore.wait_for_pending_writes().await.unwrap();
    }

    #[tokio::test]
    async fn test_settings_custom_cache_directory() {
        use crate::firestore::types::Settings;
        use std::path::PathBuf;
        
        let mut settings = Settings::new();
        settings.cache_directory = Some(PathBuf::from("/tmp/firebase_cache"));
        settings.persistence_enabled = true;
        
        assert_eq!(settings.cache_directory, Some(PathBuf::from("/tmp/firebase_cache")));
    }

    #[tokio::test]
    async fn test_settings_disable_persistence() {
        use crate::firestore::types::Settings;
        
        let mut settings = Settings::new();
        settings.persistence_enabled = false;
        
        assert!(!settings.persistence_enabled);
    }
}
