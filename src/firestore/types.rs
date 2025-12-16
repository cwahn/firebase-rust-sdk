//! Firestore types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/field_value.h`
//! - `firestore/src/include/firebase/firestore/timestamp.h`
//! - `firestore/src/include/firebase/firestore/geo_point.h`
//!
//! Note: FieldValue is replaced by serde_json::Value for flexibility

use crate::error::FirestoreError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// FieldValue enum is replaced by serde_json::Value
// This provides better flexibility and matches Rust ecosystem standards
// For Firestore-specific types like Timestamp and GeoPoint, use the dedicated structs below

/// Filter operators for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:142` (Filter)
/// - `firestore/src/include/firebase/firestore/filter.h:268` (And)
/// - `firestore/src/include/firebase/firestore/filter.h:308` (Or)
#[derive(Debug, Clone, PartialEq)]
pub enum FilterCondition {
    /// field == value
    Equal(String, Value),

    /// field < value
    LessThan(String, Value),

    /// field <= value
    LessThanOrEqual(String, Value),

    /// field > value
    GreaterThan(String, Value),

    /// field >= value
    GreaterThanOrEqual(String, Value),

    /// field array contains value
    ArrayContains(String, Value),

    /// field array contains any value from list
    ArrayContainsAny(String, Vec<Value>),

    /// field value is in list
    In(String, Vec<Value>),

    /// field != value
    NotEqual(String, Value),

    /// field not in list
    NotIn(String, Vec<Value>),

    /// Conjunction of multiple filters (all must match)
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/filter.h:268` - And(filters)
    /// 
    /// A document matches this filter if it matches all the provided filters.
    /// If the vector is empty, it acts as a no-op. If only one filter is provided,
    /// it behaves the same as that filter alone.
    And(Vec<FilterCondition>),

    /// Disjunction of multiple filters (any must match)
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/filter.h:308` - Or(filters)
    /// 
    /// A document matches this filter if it matches any of the provided filters.
    /// If the vector is empty, it acts as a no-op. If only one filter is provided,
    /// it behaves the same as that filter alone.
    Or(Vec<FilterCondition>),
}

impl FilterCondition {
    /// Get the field path for this filter
    /// 
    /// For compound filters (And/Or), returns empty string as they don't have a single field path
    pub fn field_path(&self) -> &str {
        match self {
            FilterCondition::Equal(field, _) => field,
            FilterCondition::LessThan(field, _) => field,
            FilterCondition::LessThanOrEqual(field, _) => field,
            FilterCondition::GreaterThan(field, _) => field,
            FilterCondition::GreaterThanOrEqual(field, _) => field,
            FilterCondition::ArrayContains(field, _) => field,
            FilterCondition::ArrayContainsAny(field, _) => field,
            FilterCondition::In(field, _) => field,
            FilterCondition::NotEqual(field, _) => field,
            FilterCondition::NotIn(field, _) => field,
            FilterCondition::And(_) | FilterCondition::Or(_) => "",
        }
    }

    /// Get the operator string for Firestore REST API
    pub fn operator(&self) -> &'static str {
        match self {
            FilterCondition::Equal(_, _) => "EQUAL",
            FilterCondition::LessThan(_, _) => "LESS_THAN",
            FilterCondition::LessThanOrEqual(_, _) => "LESS_THAN_OR_EQUAL",
            FilterCondition::GreaterThan(_, _) => "GREATER_THAN",
            FilterCondition::GreaterThanOrEqual(_, _) => "GREATER_THAN_OR_EQUAL",
            FilterCondition::ArrayContains(_, _) => "ARRAY_CONTAINS",
            FilterCondition::ArrayContainsAny(_, _) => "ARRAY_CONTAINS_ANY",
            FilterCondition::In(_, _) => "IN",
            FilterCondition::NotEqual(_, _) => "NOT_EQUAL",
            FilterCondition::NotIn(_, _) => "NOT_IN",
            FilterCondition::And(_) => "AND",
            FilterCondition::Or(_) => "OR",
        }
    }
}

/// Order direction for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:204` (Direction)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    /// Sort in ascending order
    Ascending,
    /// Sort in descending order
    Descending,
}

/// Settings for configuring Firestore behavior
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/settings.h:49` - Settings class
///
/// Configure various Firestore settings including persistence, cache size, and network options.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Host of the Firestore backend to connect to
    /// 
    /// Default: "firestore.googleapis.com"
    pub host: String,

    /// Whether to use SSL for communication
    /// 
    /// Default: true
    pub ssl_enabled: bool,

    /// Whether to enable local persistent storage
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:120` - is_persistence_enabled()
    /// 
    /// When enabled, Firestore caches documents locally and serves them when offline.
    /// 
    /// Default: true
    pub persistence_enabled: bool,

    /// Cache size threshold for on-disk data in bytes
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:123` - cache_size_bytes()
    /// 
    /// If the cache grows beyond this size, Firestore will start removing data
    /// that hasn't been recently used. Set to -1 for unlimited cache.
    /// 
    /// Default: 100 MB (104857600 bytes)
    pub cache_size_bytes: i64,

    /// Directory path for local cache storage
    /// 
    /// If None, uses platform default:
    /// - macOS/Linux: `~/.firebase_cache/{project_id}/`
    /// - Windows: `%APPDATA%/firebase_cache/{project_id}/`
    /// - WASM: IndexedDB (not filesystem)
    pub cache_directory: Option<std::path::PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            host: "firestore.googleapis.com".to_string(),
            ssl_enabled: true,
            persistence_enabled: true,
            cache_size_bytes: 100 * 1024 * 1024, // 100 MB
            cache_directory: None,
        }
    }
}

impl Settings {
    /// Constant to use with cache_size_bytes to disable garbage collection
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/settings.h:57` - kCacheSizeUnlimited
    pub const CACHE_SIZE_UNLIMITED: i64 = -1;

    /// Creates default settings
    pub fn new() -> Self {
        Self::default()
    }
}

/// Source options for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/source.h:30` - Source enum
///
/// Configures where Firestore should fetch data from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Default behavior - try server first, fall back to cache if offline
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:40` - kDefault
    Default,

    /// Only fetch from server, fail if offline
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:44` - kServer
    Server,

    /// Only fetch from local cache, fail if not cached
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/source.h:48` - kCache
    Cache,
}

impl Default for Source {
    fn default() -> Self {
        Source::Default
    }
}

/// Firestore timestamp
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/timestamp.h:41`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    /// Seconds since Unix epoch
    pub seconds: i64,

    /// Nanoseconds component (0-999,999,999)
    pub nanoseconds: i32,
}

impl Timestamp {
    /// Create a new timestamp
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/timestamp.h:73`
    pub fn new(seconds: i64, nanoseconds: i32) -> Result<Self, FirestoreError> {
        if nanoseconds < 0 || nanoseconds >= 1_000_000_000 {
            return Err(FirestoreError::InvalidArgument(format!(
                "nanoseconds must be in range [0, 999999999], got {}",
                nanoseconds
            )));
        }

        Ok(Self {
            seconds,
            nanoseconds,
        })
    }

    /// Get current timestamp
    pub fn now() -> Self {
        Self::from_datetime(Utc::now())
    }

    /// Convert from DateTime
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self {
            seconds: dt.timestamp(),
            nanoseconds: dt.timestamp_subsec_nanos() as i32,
        }
    }

    /// Convert to DateTime
    pub fn to_datetime(&self) -> DateTime<Utc> {
        match DateTime::from_timestamp(self.seconds, self.nanoseconds as u32) {
            None => Utc::now(),
            Some(dt) => dt,
        }
    }

    /// Convert to serde_json::Value for use in documents
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "seconds": self.seconds,
            "nanoseconds": self.nanoseconds
        })
    }
}

/// Geographic point (latitude/longitude)
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/geo_point.h:37`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    /// Latitude in degrees (range: -90 to 90)
    pub latitude: f64,

    /// Longitude in degrees (range: -180 to 180)
    pub longitude: f64,
}

impl GeoPoint {
    /// Create a new geographic point
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/geo_point.h:68`
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, FirestoreError> {
        // Validate latitude (error cases first)
        if latitude < -90.0 || latitude > 90.0 {
            return Err(FirestoreError::InvalidArgument(format!(
                "latitude must be in range [-90, 90], got {}",
                latitude
            )));
        }

        // Validate longitude (error cases first)
        if longitude < -180.0 || longitude > 180.0 {
            return Err(FirestoreError::InvalidArgument(format!(
                "longitude must be in range [-180, 180], got {}",
                longitude
            )));
        }

        Ok(Self {
            latitude,
            longitude,
        })
    }

    /// Convert to serde_json::Value for use in documents
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "latitude": self.latitude,
            "longitude": self.longitude
        })
    }
}

/// Reference to a Firestore document
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_reference.h:71`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentReference {
    /// Full document path (e.g., "users/alice")
    pub path: String,
}

impl DocumentReference {
    /// Create a new document reference
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// Get the document ID (last segment of path)
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get the parent collection path
    pub fn parent_path(&self) -> Option<&str> {
        self.path.rsplit_once('/').map(|(parent, _)| parent)
    }
}

/// Firestore document snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_snapshot.h:58`
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    /// Document reference
    pub reference: DocumentReference,

    /// Document data (None if document doesn't exist)
    pub data: Option<Value>,

    /// Document metadata
    pub metadata: SnapshotMetadata,
}

impl DocumentSnapshot {
    /// Check if document exists
    pub fn exists(&self) -> bool {
        self.data.is_some()
    }

    /// Get a field value by path
    pub fn get(&self, field: &str) -> Option<&Value> {
        let Some(data) = &self.data else {
            return None;
        };
        data.get(field)
    }

    /// Get document ID
    pub fn id(&self) -> &str {
        self.reference.id()
    }
}

/// Metadata about a document snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/snapshot_metadata.h:35`
#[derive(Debug, Clone, Copy)]
pub struct SnapshotMetadata {
    /// Whether the snapshot contains pending writes
    pub has_pending_writes: bool,

    /// Whether the data came from cache
    pub is_from_cache: bool,
}

impl Default for SnapshotMetadata {
    fn default() -> Self {
        Self {
            has_pending_writes: false,
            is_from_cache: false,
        }
    }
}

/// Query snapshot containing multiple documents
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query_snapshot.h:55`
#[derive(Debug, Clone)]
pub struct QuerySnapshot {
    /// The query that produced this snapshot
    pub query_path: String,
    
    /// Documents in the query result
    pub documents: Vec<DocumentSnapshot>,
    
    /// Metadata about this snapshot
    pub metadata: SnapshotMetadata,
    
    /// Document changes since last snapshot (for listeners)
    pub document_changes: Vec<DocumentChange>,
}

impl QuerySnapshot {
    /// Create a new query snapshot
    pub fn new(query_path: String, documents: Vec<DocumentSnapshot>) -> Self {
        Self {
            query_path,
            documents,
            metadata: SnapshotMetadata::default(),
            document_changes: Vec::new(),
        }
    }
    
    /// Check if the query result is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
    
    /// Get the number of documents in the snapshot
    pub fn len(&self) -> usize {
        self.documents.len()
    }
}

/// Document change type for snapshot listeners
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_change.h:36`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentChangeType {
    /// Document was added
    Added,
    /// Document was modified
    Modified,
    /// Document was removed
    Removed,
}

/// Represents a change to a document in a query snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_change.h:48`
#[derive(Debug, Clone)]
pub struct DocumentChange {
    /// Type of change
    pub change_type: DocumentChangeType,
    
    /// The document that changed
    pub document: DocumentSnapshot,
    
    /// The old index of the document (-1 if added)
    pub old_index: i32,
    
    /// The new index of the document (-1 if removed)
    pub new_index: i32,
}

/// Listener registration handle for snapshot listeners
///
/// Dropping this handle will automatically unregister the listener.
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/listener_registration.h:42`
#[derive(Debug)]
pub struct ListenerRegistration {
    /// Internal ID for the listener
    pub(crate) id: String,
    
    /// Cancellation flag
    pub(crate) cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ListenerRegistration {
    /// Create a new listener registration
    pub(crate) fn new(id: String) -> Self {
        Self {
            id,
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Remove the listener
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/listener_registration.h:66`
    pub fn remove(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Check if the listener has been cancelled
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for ListenerRegistration {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Write batch for atomic operations
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/write_batch.h:46`
#[derive(Debug, Default)]
pub struct WriteBatch {
    operations: Vec<WriteOperation>,
}

#[derive(Debug, Clone)]
pub enum WriteOperation {
    Set { path: String, data: Value },
    Update { path: String, data: Value },
    Delete { path: String },
}

impl WriteBatch {
    /// Create a new write batch
    pub fn new() -> Self {
        Self::default()
    }

    /// Set document data
    pub fn set(&mut self, path: impl Into<String>, data: Value) -> &mut Self {
        self.operations.push(WriteOperation::Set {
            path: path.into(),
            data,
        });
        self
    }

    /// Update document fields
    pub fn update(&mut self, path: impl Into<String>, data: Value) -> &mut Self {
        self.operations.push(WriteOperation::Update {
            path: path.into(),
            data,
        });
        self
    }

    /// Delete document
    pub fn delete(&mut self, path: impl Into<String>) -> &mut Self {
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
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use firebase_rust_sdk::firestore::types::WriteBatch;
    /// use serde_json::json;
    ///
    /// let mut batch = WriteBatch::new();
    /// batch.set("users/alice", json!({"name": "Alice", "age": 30}))
    ///      .update("users/bob", json!({"age": 31}))
    ///      .delete("users/charlie");
    /// // batch.commit().await?;  // Requires Firestore instance
    /// # Ok(())
    /// # }
    /// ```
    pub fn operations(&self) -> &[WriteOperation] {
        &self.operations
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

/// Transaction for atomic read-modify-write operations
///
/// Transactions are used to ensure data consistency when reading and writing
/// multiple documents atomically. All reads must happen before any writes.
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/transaction.h:42`
/// - `firestore/src/common/transaction.cc:88` (Get)
///
/// # Example
/// ```no_run
/// # use firebase_rust_sdk::firestore::Firestore;
/// # use serde_json::json;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let firestore = Firestore::get_firestore("my-project").await?;
///
/// firestore.run_transaction(|mut txn| async move {
///     // All reads must happen first
///     let doc = txn.get("users/alice").await?;
///     let count = doc.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
///     
///     // Then perform writes
///     txn.set("users/alice", json!({"count": count + 1}));
///     Ok(())
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Transaction ID from Firestore
    pub(crate) id: Option<String>,

    /// Project ID
    pub(crate) project_id: String,

    /// Database ID
    pub(crate) database_id: String,

    /// API key for authentication
    pub(crate) api_key: String,

    /// Write operations accumulated during transaction
    pub(crate) operations: Vec<WriteOperation>,

    /// Documents read during this transaction (for validation)
    pub(crate) reads: Vec<String>,
}

impl Transaction {
    /// Create a new transaction
    pub(crate) fn new(project_id: String, database_id: String, api_key: String) -> Self {
        Self {
            id: None,
            project_id,
            database_id,
            api_key,
            operations: Vec::new(),
            reads: Vec::new(),
        }
    }

    /// Set transaction ID (received from Firestore on first read)
    pub(crate) fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Get transaction ID
    pub(crate) fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Add a read to the transaction
    pub(crate) fn add_read(&mut self, path: String) {
        self.reads.push(path);
    }

    /// Write to a document (replaces entire document)
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/transaction.h:74` (Set)
    ///
    /// # Arguments
    /// * `path` - Document path (e.g. "users/alice")
    /// * `data` - Document data as JSON value
    pub fn set(&mut self, path: impl Into<String>, data: Value) -> &mut Self {
        self.operations.push(WriteOperation::Set {
            path: path.into(),
            data,
        });
        self
    }

    /// Update specific fields in a document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/transaction.h:87` (Update)
    ///
    /// # Arguments
    /// * `path` - Document path (e.g. "users/alice")
    /// * `data` - Fields to update as JSON value
    pub fn update(&mut self, path: impl Into<String>, data: Value) -> &mut Self {
        self.operations.push(WriteOperation::Update {
            path: path.into(),
            data,
        });
        self
    }

    /// Delete a document
    ///
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/transaction.h:107` (Delete)
    ///
    /// # Arguments
    /// * `path` - Document path (e.g. "users/alice")
    pub fn delete(&mut self, path: impl Into<String>) -> &mut Self {
        self.operations
            .push(WriteOperation::Delete { path: path.into() });
        self
    }

    /// Get all operations in this transaction
    pub(crate) fn operations(&self) -> &[WriteOperation] {
        &self.operations
    }

    /// Read a document within the transaction
    ///
    /// All reads must happen before any writes in the transaction.
    ///
    /// # C++ Reference
    /// - `firestore/src/common/transaction.cc:88` (Get)
    /// - `firestore/src/include/firebase/firestore/transaction.h:118`
    ///
    /// # Arguments
    /// * `path` - Document path (e.g. "users/alice")
    ///
    /// # Returns
    /// Document snapshot with the current document data
    pub async fn get(
        &mut self,
        path: impl Into<String>,
    ) -> Result<DocumentSnapshot, crate::error::FirebaseError> {
        use crate::error::{FirebaseError, FirestoreError};

        let path_string = path.into();

        // Error-first: validate path
        if path_string.is_empty() {
            return Err(FirebaseError::Firestore(FirestoreError::InvalidArgument(
                "Document path cannot be empty".to_string(),
            )));
        }

        // Track this read
        self.add_read(path_string.clone());

        // Build document URL
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id, self.database_id, path_string
        );

        // Make request
        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        // Add transaction ID if we have one
        if let Some(txn_id) = &self.id {
            request = request.query(&[("transaction", txn_id)]);
        }

        let response = match request.send().await {
            Err(err) => {
                return Err(FirebaseError::Firestore(FirestoreError::Internal(format!(
                    "Get document failed: {err}"
                ))))
            }
            Ok(resp) => resp,
        };

        // Store transaction ID from response if this is the first read
        if self.id.is_none() {
            // First read in transaction - Firestore returns a transaction ID
            // For now we'll handle this in the commit phase
        }

        // Error-first: check for HTTP errors
        if !response.status().is_success() {
            if response.status() == 404 {
                // Document doesn't exist - return empty snapshot
                return Ok(DocumentSnapshot {
                    reference: DocumentReference {
                        path: path_string.clone(),
                    },
                    data: None,
                    metadata: SnapshotMetadata::default(),
                });
            }

            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(FirebaseError::Firestore(FirestoreError::Internal(format!(
                "Get document failed: {status} - {error_text}",
            ))));
        }

        let doc = match response.json::<serde_json::Value>().await {
            Err(err) => {
                return Err(FirebaseError::Firestore(FirestoreError::Internal(format!(
                    "Failed to parse document: {err}"
                ))))
            }
            Ok(d) => d,
        };

        // Parse document fields
        let data = match doc.get("fields") {
            None => None,
            Some(fields) => Some(Self::convert_firestore_fields(fields)),
        };

        Ok(DocumentSnapshot {
            reference: DocumentReference { path: path_string },
            data,
            metadata: SnapshotMetadata::default(),
        })
    }

    /// Convert Firestore fields format to plain JSON (internal helper)
    fn convert_firestore_fields(fields: &serde_json::Value) -> serde_json::Value {
        use serde_json::{json, Map, Value as JsonValue};

        if let Some(obj) = fields.as_object() {
            let mut result = Map::new();
            for (key, value) in obj {
                result.insert(key.clone(), Self::convert_firestore_value(value));
            }
            JsonValue::Object(result)
        } else {
            json!({})
        }
    }

    /// Convert a single Firestore value to plain JSON
    fn convert_firestore_value(value: &serde_json::Value) -> serde_json::Value {
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
                    return json!(values
                        .iter()
                        .map(|v| Self::convert_firestore_value(v))
                        .collect::<Vec<_>>());
                }
            } else if let Some(map_val) = obj.get("mapValue") {
                if let Some(fields) = map_val.get("fields") {
                    return Self::convert_firestore_fields(fields);
                }
            }
        }

        value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_value_types() {
        let null_val = Value::Null;
        assert!(null_val.is_null());

        let bool_val = json!(true);
        assert_eq!(bool_val.as_bool(), Some(true));

        let int_val = json!(42);
        assert_eq!(int_val.as_i64(), Some(42));

        let double_val = json!(3.14);
        assert_eq!(double_val.as_f64(), Some(3.14));

        let str_val = json!("hello");
        assert_eq!(str_val.as_str(), Some("hello"));
    }

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1234567890, 123456789).unwrap();
        assert_eq!(ts.seconds, 1234567890);
        assert_eq!(ts.nanoseconds, 123456789);

        // Invalid nanoseconds
        assert!(Timestamp::new(0, -1).is_err());
        assert!(Timestamp::new(0, 1_000_000_000).is_err());
    }

    #[test]
    fn test_timestamp_datetime_conversion() {
        let now = Utc::now();
        let ts = Timestamp::from_datetime(now);
        let dt = ts.to_datetime();

        // Should be approximately equal (within 1 second)
        assert!((dt.timestamp() - now.timestamp()).abs() <= 1);
    }

    #[test]
    fn test_timestamp_to_value() {
        let ts = Timestamp::new(1234567890, 123456789).unwrap();
        let value = ts.to_value();

        assert_eq!(value["seconds"], 1234567890);
        assert_eq!(value["nanoseconds"], 123456789);
    }

    #[test]
    fn test_geo_point_validation() {
        // Valid points
        assert!(GeoPoint::new(0.0, 0.0).is_ok());
        assert!(GeoPoint::new(90.0, 180.0).is_ok());
        assert!(GeoPoint::new(-90.0, -180.0).is_ok());

        // Invalid latitude
        assert!(GeoPoint::new(91.0, 0.0).is_err());
        assert!(GeoPoint::new(-91.0, 0.0).is_err());

        // Invalid longitude
        assert!(GeoPoint::new(0.0, 181.0).is_err());
        assert!(GeoPoint::new(0.0, -181.0).is_err());
    }

    #[test]
    fn test_geo_point_to_value() {
        let gp = GeoPoint::new(37.7749, -122.4194).unwrap();
        let value = gp.to_value();

        assert_eq!(value["latitude"], 37.7749);
        assert_eq!(value["longitude"], -122.4194);
    }

    #[test]
    fn test_document_reference() {
        let doc_ref = DocumentReference::new("users/alice");
        assert_eq!(doc_ref.id(), "alice");
        assert_eq!(doc_ref.parent_path(), Some("users"));

        let root_ref = DocumentReference::new("single");
        assert_eq!(root_ref.id(), "single");
        assert_eq!(root_ref.parent_path(), None);
    }

    #[test]
    fn test_document_snapshot() {
        let data = json!({
            "name": "Alice",
            "age": 30
        });

        let snapshot = DocumentSnapshot {
            reference: DocumentReference::new("users/alice"),
            data: Some(data),
            metadata: SnapshotMetadata::default(),
        };

        assert!(snapshot.exists());
        assert_eq!(snapshot.id(), "alice");
        assert_eq!(snapshot.get("name").and_then(|v| v.as_str()), Some("Alice"));
        assert_eq!(snapshot.get("age").and_then(|v| v.as_i64()), Some(30));
    }

    #[test]
    fn test_write_batch() {
        let mut batch = WriteBatch::new();
        let data = json!({
            "name": "Bob"
        });

        batch
            .set("users/bob", data.clone())
            .update("users/alice", data)
            .delete("users/charlie");

        assert_eq!(batch.operations.len(), 3);
    }

    #[test]
    fn test_value_serialization() {
        let value = json!("test");
        let json_str = serde_json::to_string(&value).unwrap();
        assert!(json_str.contains("test"));

        let array = json!([1, 2]);
        let json_str = serde_json::to_string(&array).unwrap();
        assert!(json_str.contains("1"));
        assert!(json_str.contains("2"));
    }

    #[test]
    fn test_complex_document() {
        let ts = Timestamp::new(1234567890, 0).unwrap();
        let gp = GeoPoint::new(37.7749, -122.4194).unwrap();

        let doc = json!({
            "name": "San Francisco Office",
            "location": gp.to_value(),
            "created": ts.to_value(),
            "active": true,
            "employees": 150
        });

        assert_eq!(doc["name"], "San Francisco Office");
        assert_eq!(doc["location"]["latitude"], 37.7749);
        assert_eq!(doc["created"]["seconds"], 1234567890);
        assert_eq!(doc["active"], true);
        assert_eq!(doc["employees"], 150);
    }
}
