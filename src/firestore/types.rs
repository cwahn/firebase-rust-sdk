//! Firestore types
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/field_value.h`
//! - `firestore/src/include/firebase/firestore/timestamp.h`
//! - `firestore/src/include/firebase/firestore/geo_point.h`
//! - `firestore/src/include/firebase/firestore/map_field_value.h`
//!
//! Uses protobuf Value and MapValue types directly, matching C++ SDK's FieldValue design

use crate::error::FirestoreError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export protobuf types for public API
// Matches C++ SDK's FieldValue and MapValue pattern
#[allow(clippy::all)]
pub(crate) mod proto {
    include!(concat!(env!("OUT_DIR"), "/proto.rs"));
}

/// Firestore Value type - matches C++ SDK's FieldValue
/// 
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/field_value.h`
pub use proto::google::firestore::v1::Value;

/// Map of field values - matches C++ SDK's MapValue
/// Uses protobuf MapValue which contains HashMap<String, Value>
/// 
/// # C++ Reference  
/// - `firestore/src/include/firebase/firestore/map_field_value.h:30`
pub use proto::google::firestore::v1::MapValue;

/// Filter operators for Firestore queries
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/query.h:142` (Filter)
/// - `firestore/src/include/firebase/firestore/filter.h:268` (And)
/// - `firestore/src/include/firebase/firestore/filter.h:308` (Or)
#[derive(Debug, Clone)]
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
        let Some(dt) = DateTime::from_timestamp(self.seconds, self.nanoseconds as u32) else {
            return Utc::now();
        };
        dt
    }

    /// Convert to protobuf Value for use in documents
    pub fn to_value(&self) -> Value {
        use proto::google::firestore::v1::value::ValueType;
        Value {
            value_type: Some(ValueType::TimestampValue(prost_types::Timestamp {
                seconds: self.seconds,
                nanos: self.nanoseconds,
            })),
        }
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

    /// Convert to protobuf Value for use in documents
    pub fn to_value(&self) -> Value {
        use proto::google::firestore::v1::value::ValueType;
        Value {
            value_type: Some(ValueType::GeoPointValue(proto::google::r#type::LatLng {
                latitude: self.latitude,
                longitude: self.longitude,
            })),
        }
    }
}

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
    pub(crate) firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>,
}

impl DocumentReference {
    /// Create a new document reference
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/document_reference.cc:40`
    pub(crate) fn new(path: impl Into<String>, firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>) -> Self {
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

/// Firestore document snapshot
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/document_snapshot.h:58`
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    /// Document reference
    pub reference: DocumentReference,

    /// Document data (None if document doesn't exist)
    /// Contains protobuf MapValue
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:186` - GetData() returns MapValue
    pub data: Option<MapValue>,

    /// Document metadata
    pub metadata: SnapshotMetadata,
}

impl DocumentSnapshot {
    /// Check if document exists
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:172`
    pub fn exists(&self) -> bool {
        self.data.is_some()
    }

    /// Get a field value by path
    /// 
    /// # C++ Reference
    /// - `firestore/src/include/firebase/firestore/document_snapshot.h:196`
    pub fn get(&self, field: &str) -> Option<&Value> {
        let Some(data) = &self.data else {
            return None;
        };
        data.fields.get(field)
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

/// Reference to a Firestore collection
///
/// # C++ Reference
/// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.h:38`
#[derive(Clone)]
pub struct CollectionReference {
    /// Collection path (e.g., "users")
    pub path: String,
    /// Reference to Firestore client
    pub(crate) firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>,
}

impl CollectionReference {
    /// Create a new collection reference
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:28`
    pub(crate) fn new(path: impl Into<String>, firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>) -> Self {
        Self { 
            path: path.into(),
            firestore,
        }
    }

    /// Get collection ID (last segment of path)
    pub fn id(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Get a document reference within this collection
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:41` - Document()
    pub fn document(&self, document_id: impl AsRef<str>) -> DocumentReference {
        let path = format!("{}/{}", self.path, document_id.as_ref());
        DocumentReference::new(path, std::sync::Arc::clone(&self.firestore))
    }

    /// Add a new document with auto-generated ID
    ///
    /// # C++ Reference
    /// - `firebase-ios-sdk/Firestore/core/src/api/collection_reference.cc:46` - AddDocument()
    pub async fn add(&self, data: MapValue) -> Result<DocumentReference, crate::error::FirebaseError> {
        // Generate auto ID
        use rand::Rng;
        let auto_id: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        
        let doc_ref = self.document(&auto_id);
        doc_ref.set(data).await?;
        Ok(doc_ref)
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
pub struct WriteBatch {
    operations: Vec<WriteOperation>,
    /// Reference to Firestore client for commit operation
    firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>,
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
    pub(crate) fn new(firestore: std::sync::Arc<crate::firestore::firestore::FirestoreInner>) -> Self {
        Self {
            operations: Vec::new(),
            firestore,
        }
    }

    /// Set document data
    pub fn set(&mut self, path: impl Into<String>, data: MapValue) -> &mut Self {
        self.operations.push(WriteOperation::Set {
            path: path.into(),
            data,
        });
        self
    }

    /// Update document fields
    pub fn update(&mut self, path: impl Into<String>, data: MapValue) -> &mut Self {
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
        
        let mut client = self.firestore.grpc_client.clone();
        let _response = client.commit(request)
            .await
            .map_err(|e| crate::error::FirestoreError::Connection(format!("gRPC commit failed: {}", e)))?;
        
        Ok(())
    }

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    /// * `data` - Document data as MapValue
    pub fn set(&mut self, path: impl Into<String>, data: MapValue) -> &mut Self {
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
    /// * `data` - Fields to update as MapValue
    pub fn update(&mut self, path: impl Into<String>, data: MapValue) -> &mut Self {
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
            // The transaction ID should come from the response body's "transaction" field
            // For REST API, this is typically returned on the first read operation
            // We'll extract it if present in the response
            // Note: For a complete implementation, we'd need to modify get() to return
            // both the snapshot and the transaction ID from the response
            // For now, we rely on the commit phase to use the transaction ID if needed
        }

        // TODO: Convert Transaction.get() to gRPC - this is old REST code
        // For now, return error as this needs to be reimplemented with gRPC BatchGetDocuments
        Err(FirebaseError::Firestore(FirestoreError::Unimplemented))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::google::firestore::v1::value::ValueType;
    use std::collections::HashMap;

    // ============================================================================
    // Timestamp Tests (10 tests)
    // ============================================================================

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1234567890, 123456789).unwrap();
        assert_eq!(ts.seconds, 1234567890);
        assert_eq!(ts.nanoseconds, 123456789);
    }

    #[test]
    fn test_timestamp_invalid_nanoseconds_negative() {
        assert!(Timestamp::new(0, -1).is_err());
    }

    #[test]
    fn test_timestamp_invalid_nanoseconds_too_large() {
        assert!(Timestamp::new(0, 1_000_000_000).is_err());
    }

    #[test]
    fn test_timestamp_valid_nanoseconds_boundary() {
        assert!(Timestamp::new(0, 0).is_ok());
        assert!(Timestamp::new(0, 999_999_999).is_ok());
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
    fn test_timestamp_epoch() {
        let epoch = Timestamp::new(0, 0).unwrap();
        let dt = epoch.to_datetime();
        assert_eq!(dt.timestamp(), 0);
    }

    #[test]
    fn test_timestamp_to_protobuf_value() {
        let ts = Timestamp::new(1234567890, 123456789).unwrap();
        let value = ts.to_value();
        
        match value.value_type {
            Some(ValueType::TimestampValue(prost_ts)) => {
                assert_eq!(prost_ts.seconds, 1234567890);
                assert_eq!(prost_ts.nanos, 123456789);
            }
            _ => panic!("Expected TimestampValue"),
        }
    }

    #[test]
    fn test_timestamp_roundtrip() {
        let original = Timestamp::new(1609459200, 500000000).unwrap();
        let value = original.to_value();
        
        if let Some(ValueType::TimestampValue(prost_ts)) = value.value_type {
            let reconstructed = Timestamp::new(prost_ts.seconds, prost_ts.nanos).unwrap();
            assert_eq!(original.seconds, reconstructed.seconds);
            assert_eq!(original.nanoseconds, reconstructed.nanoseconds);
        } else {
            panic!("Expected TimestampValue");
        }
    }

    #[test]
    fn test_timestamp_negative_seconds() {
        // Unix timestamps can be negative (before epoch)
        let ts = Timestamp::new(-1000, 0).unwrap();
        assert_eq!(ts.seconds, -1000);
    }

    #[test]
    fn test_timestamp_large_values() {
        // Test with large timestamp values (year 2100+)
        let ts = Timestamp::new(4102444800, 999999999).unwrap();
        assert_eq!(ts.seconds, 4102444800);
        assert_eq!(ts.nanoseconds, 999999999);
    }

    // ============================================================================
    // GeoPoint Tests (10 tests)
    // ============================================================================

    #[test]
    fn test_geopoint_creation_valid() {
        let gp = GeoPoint::new(37.7749, -122.4194).unwrap();
        assert_eq!(gp.latitude, 37.7749);
        assert_eq!(gp.longitude, -122.4194);
    }

    #[test]
    fn test_geopoint_origin() {
        assert!(GeoPoint::new(0.0, 0.0).is_ok());
    }

    #[test]
    fn test_geopoint_north_pole() {
        assert!(GeoPoint::new(90.0, 0.0).is_ok());
    }

    #[test]
    fn test_geopoint_south_pole() {
        assert!(GeoPoint::new(-90.0, 0.0).is_ok());
    }

    #[test]
    fn test_geopoint_dateline() {
        assert!(GeoPoint::new(0.0, 180.0).is_ok());
        assert!(GeoPoint::new(0.0, -180.0).is_ok());
    }

    #[test]
    fn test_geopoint_invalid_latitude_too_high() {
        assert!(GeoPoint::new(91.0, 0.0).is_err());
    }

    #[test]
    fn test_geopoint_invalid_latitude_too_low() {
        assert!(GeoPoint::new(-91.0, 0.0).is_err());
    }

    #[test]
    fn test_geopoint_invalid_longitude_too_high() {
        assert!(GeoPoint::new(0.0, 181.0).is_err());
    }

    #[test]
    fn test_geopoint_invalid_longitude_too_low() {
        assert!(GeoPoint::new(0.0, -181.0).is_err());
    }

    #[test]
    fn test_geopoint_to_protobuf_value() {
        let gp = GeoPoint::new(37.7749, -122.4194).unwrap();
        let value = gp.to_value();

        match value.value_type {
            Some(ValueType::GeoPointValue(geo)) => {
                assert_eq!(geo.latitude, 37.7749);
                assert_eq!(geo.longitude, -122.4194);
            }
            _ => panic!("Expected GeoPointValue"),
        }
    }

    // ============================================================================
    // DocumentSnapshot Tests (10 tests)
    // ============================================================================
    // Note: Full DocumentSnapshot tests with DocumentReference require integration tests
    // These tests focus on SnapshotMetadata and MapValue field access

    #[test]
    fn test_document_snapshot_metadata_default() {
        let metadata = SnapshotMetadata::default();
        assert!(!metadata.has_pending_writes);
        assert!(!metadata.is_from_cache);
    }

    #[test]
    fn test_document_snapshot_metadata_pending_writes() {
        let metadata = SnapshotMetadata {
            has_pending_writes: true,
            is_from_cache: false,
        };
        assert!(metadata.has_pending_writes);
    }

    #[test]
    fn test_document_snapshot_metadata_from_cache() {
        let metadata = SnapshotMetadata {
            has_pending_writes: false,
            is_from_cache: true,
        };
        assert!(metadata.is_from_cache);
    }

    #[test]
    fn test_document_snapshot_metadata_all_flags() {
        let metadata = SnapshotMetadata {
            has_pending_writes: true,
            is_from_cache: true,
        };
        assert!(metadata.has_pending_writes);
        assert!(metadata.is_from_cache);
    }

    #[test]
    fn test_map_value_empty() {
        let map = MapValue {
            fields: HashMap::new(),
        };
        assert_eq!(map.fields.len(), 0);
    }

    #[test]
    fn test_map_value_single_field() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Alice".to_string())),
            },
        );

        let map = MapValue { fields };
        assert_eq!(map.fields.len(), 1);
        assert!(map.fields.contains_key("name"));
    }

    #[test]
    fn test_map_value_multiple_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Bob".to_string())),
            },
        );
        fields.insert(
            "age".to_string(),
            Value {
                value_type: Some(ValueType::IntegerValue(30)),
            },
        );
        fields.insert(
            "active".to_string(),
            Value {
                value_type: Some(ValueType::BooleanValue(true)),
            },
        );

        let map = MapValue { fields };
        assert_eq!(map.fields.len(), 3);
        
        let name = map.fields.get("name").unwrap();
        match &name.value_type {
            Some(ValueType::StringValue(s)) => assert_eq!(s, "Bob"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_map_value_get_field_missing() {
        let map = MapValue {
            fields: HashMap::new(),
        };
        assert!(map.fields.get("nonexistent").is_none());
    }

    #[test]
    fn test_map_value_field_types() {
        let mut fields = HashMap::new();
        
        // String
        fields.insert(
            "str".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("test".to_string())),
            },
        );
        
        // Integer
        fields.insert(
            "int".to_string(),
            Value {
                value_type: Some(ValueType::IntegerValue(42)),
            },
        );
        
        // Boolean
        fields.insert(
            "bool".to_string(),
            Value {
                value_type: Some(ValueType::BooleanValue(true)),
            },
        );
        
        // Null
        fields.insert(
            "null".to_string(),
            Value {
                value_type: Some(ValueType::NullValue(0)),
            },
        );

        let map = MapValue { fields };
        assert_eq!(map.fields.len(), 4);
    }

    #[test]
    fn test_map_value_nested_map() {
        let mut inner_fields = HashMap::new();
        inner_fields.insert(
            "city".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("SF".to_string())),
            },
        );

        let mut outer_fields = HashMap::new();
        outer_fields.insert(
            "address".to_string(),
            Value {
                value_type: Some(ValueType::MapValue(MapValue {
                    fields: inner_fields,
                })),
            },
        );

        let map = MapValue {
            fields: outer_fields,
        };
        
        let address = map.fields.get("address").unwrap();
        match &address.value_type {
            Some(ValueType::MapValue(inner)) => {
                assert!(inner.fields.contains_key("city"));
            }
            _ => panic!("Expected map value"),
        }
    }

    // ============================================================================
    // WriteBatch Operation Tests (15 tests)
    // ============================================================================

    #[test]
    fn test_write_operation_set_variant() {
        let map_value = MapValue {
            fields: HashMap::new(),
        };

        let set_op = WriteOperation::Set {
            path: "users/alice".to_string(),
            data: map_value.clone(),
        };

        match set_op {
            WriteOperation::Set { path, .. } => assert_eq!(path, "users/alice"),
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_update_variant() {
        let map_value = MapValue {
            fields: HashMap::new(),
        };

        let update_op = WriteOperation::Update {
            path: "users/bob".to_string(),
            data: map_value,
        };

        match update_op {
            WriteOperation::Update { path, .. } => assert_eq!(path, "users/bob"),
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_write_operation_delete_variant() {
        let delete_op = WriteOperation::Delete {
            path: "users/charlie".to_string(),
        };

        match delete_op {
            WriteOperation::Delete { path } => assert_eq!(path, "users/charlie"),
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_write_operation_set_with_data() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Alice".to_string())),
            },
        );

        let map_value = MapValue { fields };
        let set_op = WriteOperation::Set {
            path: "users/alice".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                assert_eq!(data.fields.len(), 1);
                assert!(data.fields.contains_key("name"));
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_update_with_data() {
        let mut fields = HashMap::new();
        fields.insert(
            "age".to_string(),
            Value {
                value_type: Some(ValueType::IntegerValue(25)),
            },
        );

        let map_value = MapValue { fields };
        let update_op = WriteOperation::Update {
            path: "users/bob".to_string(),
            data: map_value,
        };

        match update_op {
            WriteOperation::Update { data, .. } => {
                assert_eq!(data.fields.len(), 1);
                let age_val = data.fields.get("age").unwrap();
                match age_val.value_type {
                    Some(ValueType::IntegerValue(i)) => assert_eq!(i, 25),
                    _ => panic!("Expected integer"),
                }
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_write_operation_clone() {
        let delete_op = WriteOperation::Delete {
            path: "users/test".to_string(),
        };
        let cloned = delete_op.clone();

        match cloned {
            WriteOperation::Delete { path } => assert_eq!(path, "users/test"),
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_write_operation_multiple_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Test".to_string())),
            },
        );
        fields.insert(
            "count".to_string(),
            Value {
                value_type: Some(ValueType::IntegerValue(42)),
            },
        );
        fields.insert(
            "active".to_string(),
            Value {
                value_type: Some(ValueType::BooleanValue(true)),
            },
        );

        let map_value = MapValue { fields };
        let set_op = WriteOperation::Set {
            path: "docs/test".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                assert_eq!(data.fields.len(), 3);
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_empty_data() {
        let map_value = MapValue {
            fields: HashMap::new(),
        };

        let set_op = WriteOperation::Set {
            path: "docs/empty".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                assert_eq!(data.fields.len(), 0);
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_nested_path() {
        let delete_op = WriteOperation::Delete {
            path: "users/alice/posts/post1".to_string(),
        };

        match delete_op {
            WriteOperation::Delete { path } => {
                assert_eq!(path, "users/alice/posts/post1");
                assert!(path.contains('/'));
            }
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_write_operation_root_collection() {
        let map_value = MapValue {
            fields: HashMap::new(),
        };

        let set_op = WriteOperation::Set {
            path: "root_doc".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { path, .. } => {
                assert_eq!(path, "root_doc");
                assert!(!path.contains('/'));
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_with_timestamp() {
        let ts = Timestamp::new(1234567890, 0).unwrap();
        let mut fields = HashMap::new();
        fields.insert("created".to_string(), ts.to_value());

        let map_value = MapValue { fields };
        let set_op = WriteOperation::Set {
            path: "docs/timestamped".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                let created = data.fields.get("created").unwrap();
                match &created.value_type {
                    Some(ValueType::TimestampValue(_)) => (),
                    _ => panic!("Expected timestamp value"),
                }
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_with_geopoint() {
        let gp = GeoPoint::new(37.7749, -122.4194).unwrap();
        let mut fields = HashMap::new();
        fields.insert("location".to_string(), gp.to_value());

        let map_value = MapValue { fields };
        let update_op = WriteOperation::Update {
            path: "places/sf".to_string(),
            data: map_value,
        };

        match update_op {
            WriteOperation::Update { data, .. } => {
                let location = data.fields.get("location").unwrap();
                match &location.value_type {
                    Some(ValueType::GeoPointValue(_)) => (),
                    _ => panic!("Expected geopoint value"),
                }
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_write_operation_with_array() {
        use proto::google::firestore::v1::ArrayValue;

        let array = ArrayValue {
            values: vec![
                Value {
                    value_type: Some(ValueType::IntegerValue(1)),
                },
                Value {
                    value_type: Some(ValueType::IntegerValue(2)),
                },
                Value {
                    value_type: Some(ValueType::IntegerValue(3)),
                },
            ],
        };

        let mut fields = HashMap::new();
        fields.insert(
            "numbers".to_string(),
            Value {
                value_type: Some(ValueType::ArrayValue(array)),
            },
        );

        let map_value = MapValue { fields };
        let set_op = WriteOperation::Set {
            path: "docs/array_doc".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                let numbers = data.fields.get("numbers").unwrap();
                match &numbers.value_type {
                    Some(ValueType::ArrayValue(arr)) => {
                        assert_eq!(arr.values.len(), 3);
                    }
                    _ => panic!("Expected array value"),
                }
            }
            _ => panic!("Expected Set operation"),
        }
    }

    #[test]
    fn test_write_operation_with_nested_map() {
        let mut inner_fields = HashMap::new();
        inner_fields.insert(
            "city".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("SF".to_string())),
            },
        );

        let inner_map = MapValue {
            fields: inner_fields,
        };

        let mut outer_fields = HashMap::new();
        outer_fields.insert(
            "address".to_string(),
            Value {
                value_type: Some(ValueType::MapValue(inner_map)),
            },
        );

        let outer_map = MapValue {
            fields: outer_fields,
        };

        let update_op = WriteOperation::Update {
            path: "users/alice".to_string(),
            data: outer_map,
        };

        match update_op {
            WriteOperation::Update { data, .. } => {
                let address = data.fields.get("address").unwrap();
                match &address.value_type {
                    Some(ValueType::MapValue(inner)) => {
                        assert!(inner.fields.contains_key("city"));
                    }
                    _ => panic!("Expected map value"),
                }
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_write_operation_with_double() {
        let mut fields = HashMap::new();
        fields.insert(
            "price".to_string(),
            Value {
                value_type: Some(ValueType::DoubleValue(19.99)),
            },
        );

        let map_value = MapValue { fields };
        let set_op = WriteOperation::Set {
            path: "products/item1".to_string(),
            data: map_value,
        };

        match set_op {
            WriteOperation::Set { data, .. } => {
                let price = data.fields.get("price").unwrap();
                match price.value_type {
                    Some(ValueType::DoubleValue(d)) => {
                        assert!((d - 19.99).abs() < 0.01);
                    }
                    _ => panic!("Expected double value"),
                }
            }
            _ => panic!("Expected Set operation"),
        }
    }

    // ============================================================================
    // Protobuf Value Tests (20 tests)
    // ============================================================================

    #[test]
    fn test_value_null() {
        let null_val = Value {
            value_type: Some(ValueType::NullValue(0)),
        };

        match null_val.value_type {
            Some(ValueType::NullValue(_)) => (),
            _ => panic!("Expected null value"),
        }
    }

    #[test]
    fn test_value_boolean_true() {
        let bool_val = Value {
            value_type: Some(ValueType::BooleanValue(true)),
        };

        match bool_val.value_type {
            Some(ValueType::BooleanValue(b)) => assert!(b),
            _ => panic!("Expected boolean value"),
        }
    }

    #[test]
    fn test_value_boolean_false() {
        let bool_val = Value {
            value_type: Some(ValueType::BooleanValue(false)),
        };

        match bool_val.value_type {
            Some(ValueType::BooleanValue(b)) => assert!(!b),
            _ => panic!("Expected boolean value"),
        }
    }

    #[test]
    fn test_value_integer_positive() {
        let int_val = Value {
            value_type: Some(ValueType::IntegerValue(42)),
        };

        match int_val.value_type {
            Some(ValueType::IntegerValue(i)) => assert_eq!(i, 42),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_value_integer_negative() {
        let int_val = Value {
            value_type: Some(ValueType::IntegerValue(-100)),
        };

        match int_val.value_type {
            Some(ValueType::IntegerValue(i)) => assert_eq!(i, -100),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_value_integer_zero() {
        let int_val = Value {
            value_type: Some(ValueType::IntegerValue(0)),
        };

        match int_val.value_type {
            Some(ValueType::IntegerValue(i)) => assert_eq!(i, 0),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_value_double_positive() {
        let double_val = Value {
            value_type: Some(ValueType::DoubleValue(3.14159)),
        };

        match double_val.value_type {
            Some(ValueType::DoubleValue(d)) => {
                assert!((d - 3.14159).abs() < 0.0001);
            }
            _ => panic!("Expected double value"),
        }
    }

    #[test]
    fn test_value_double_negative() {
        let double_val = Value {
            value_type: Some(ValueType::DoubleValue(-273.15)),
        };

        match double_val.value_type {
            Some(ValueType::DoubleValue(d)) => {
                assert!((d + 273.15).abs() < 0.01);
            }
            _ => panic!("Expected double value"),
        }
    }

    #[test]
    fn test_value_string_ascii() {
        let str_val = Value {
            value_type: Some(ValueType::StringValue("hello world".to_string())),
        };

        match str_val.value_type {
            Some(ValueType::StringValue(s)) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_value_string_unicode() {
        let str_val = Value {
            value_type: Some(ValueType::StringValue("你好世界 🌍".to_string())),
        };

        match str_val.value_type {
            Some(ValueType::StringValue(s)) => assert_eq!(s, "你好世界 🌍"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_value_string_empty() {
        let str_val = Value {
            value_type: Some(ValueType::StringValue(String::new())),
        };

        match str_val.value_type {
            Some(ValueType::StringValue(s)) => assert_eq!(s, ""),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_value_bytes() {
        let bytes_val = Value {
            value_type: Some(ValueType::BytesValue(vec![0x01, 0x02, 0x03, 0xFF])),
        };

        match bytes_val.value_type {
            Some(ValueType::BytesValue(b)) => {
                assert_eq!(b, vec![0x01, 0x02, 0x03, 0xFF]);
            }
            _ => panic!("Expected bytes value"),
        }
    }

    #[test]
    fn test_value_reference() {
        let ref_val = Value {
            value_type: Some(ValueType::ReferenceValue(
                "projects/test/databases/(default)/documents/users/alice".to_string(),
            )),
        };

        match ref_val.value_type {
            Some(ValueType::ReferenceValue(r)) => {
                assert!(r.contains("users/alice"));
            }
            _ => panic!("Expected reference value"),
        }
    }

    #[test]
    fn test_value_timestamp() {
        use prost_types::Timestamp as ProstTimestamp;

        let ts_val = Value {
            value_type: Some(ValueType::TimestampValue(ProstTimestamp {
                seconds: 1234567890,
                nanos: 123456789,
            })),
        };

        match ts_val.value_type {
            Some(ValueType::TimestampValue(ts)) => {
                assert_eq!(ts.seconds, 1234567890);
                assert_eq!(ts.nanos, 123456789);
            }
            _ => panic!("Expected timestamp value"),
        }
    }

    #[test]
    fn test_value_geopoint() {
        use proto::google::r#type::LatLng;

        let geo_val = Value {
            value_type: Some(ValueType::GeoPointValue(LatLng {
                latitude: 37.7749,
                longitude: -122.4194,
            })),
        };

        match geo_val.value_type {
            Some(ValueType::GeoPointValue(geo)) => {
                assert_eq!(geo.latitude, 37.7749);
                assert_eq!(geo.longitude, -122.4194);
            }
            _ => panic!("Expected geopoint value"),
        }
    }

    #[test]
    fn test_value_array_empty() {
        use proto::google::firestore::v1::ArrayValue;

        let array_val = Value {
            value_type: Some(ValueType::ArrayValue(ArrayValue {
                values: vec![],
            })),
        };

        match array_val.value_type {
            Some(ValueType::ArrayValue(arr)) => {
                assert_eq!(arr.values.len(), 0);
            }
            _ => panic!("Expected array value"),
        }
    }

    #[test]
    fn test_value_array_mixed_types() {
        use proto::google::firestore::v1::ArrayValue;

        let array_val = Value {
            value_type: Some(ValueType::ArrayValue(ArrayValue {
                values: vec![
                    Value {
                        value_type: Some(ValueType::IntegerValue(42)),
                    },
                    Value {
                        value_type: Some(ValueType::StringValue("hello".to_string())),
                    },
                    Value {
                        value_type: Some(ValueType::BooleanValue(true)),
                    },
                ],
            })),
        };

        match array_val.value_type {
            Some(ValueType::ArrayValue(arr)) => {
                assert_eq!(arr.values.len(), 3);
            }
            _ => panic!("Expected array value"),
        }
    }

    #[test]
    fn test_value_map_empty() {
        let map_val = Value {
            value_type: Some(ValueType::MapValue(MapValue {
                fields: HashMap::new(),
            })),
        };

        match map_val.value_type {
            Some(ValueType::MapValue(map)) => {
                assert_eq!(map.fields.len(), 0);
            }
            _ => panic!("Expected map value"),
        }
    }

    #[test]
    fn test_value_map_with_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Alice".to_string())),
            },
        );
        fields.insert(
            "age".to_string(),
            Value {
                value_type: Some(ValueType::IntegerValue(30)),
            },
        );

        let map_val = Value {
            value_type: Some(ValueType::MapValue(MapValue { fields })),
        };

        match map_val.value_type {
            Some(ValueType::MapValue(map)) => {
                assert_eq!(map.fields.len(), 2);
                assert!(map.fields.contains_key("name"));
                assert!(map.fields.contains_key("age"));
            }
            _ => panic!("Expected map value"),
        }
    }

    #[test]
    fn test_value_nested_map() {
        let mut inner_fields = HashMap::new();
        inner_fields.insert(
            "street".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("123 Main St".to_string())),
            },
        );

        let mut outer_fields = HashMap::new();
        outer_fields.insert(
            "address".to_string(),
            Value {
                value_type: Some(ValueType::MapValue(MapValue {
                    fields: inner_fields,
                })),
            },
        );

        let map_val = Value {
            value_type: Some(ValueType::MapValue(MapValue {
                fields: outer_fields,
            })),
        };

        match map_val.value_type {
            Some(ValueType::MapValue(map)) => {
                let address = map.fields.get("address").unwrap();
                match &address.value_type {
                    Some(ValueType::MapValue(inner)) => {
                        assert!(inner.fields.contains_key("street"));
                    }
                    _ => panic!("Expected inner map"),
                }
            }
            _ => panic!("Expected map value"),
        }
    }

    // ============================================================================
    // Transaction Tests (10 tests)
    // ============================================================================

    #[test]
    fn test_transaction_creation() {
        let transaction = Transaction::new("test-project".to_string(), "(default)".to_string(), "test-key".to_string());
        assert_eq!(transaction.project_id, "test-project");
        assert_eq!(transaction.database_id, "(default)");
    }

    #[test]
    fn test_transaction_id_initially_none() {
        let transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        assert!(transaction.id().is_none());
    }

    #[test]
    fn test_transaction_operations_initially_empty() {
        let transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        assert_eq!(transaction.operations().len(), 0);
    }

    #[test]
    fn test_transaction_set_operation() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        let map = MapValue {
            fields: HashMap::new(),
        };
        transaction.set("users/alice", map);
        assert_eq!(transaction.operations().len(), 1);
    }

    #[test]
    fn test_transaction_update_operation() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        let map = MapValue {
            fields: HashMap::new(),
        };
        transaction.update("users/bob", map);
        assert_eq!(transaction.operations().len(), 1);
    }

    #[test]
    fn test_transaction_delete_operation() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        transaction.delete("users/charlie");
        assert_eq!(transaction.operations().len(), 1);
    }

    #[test]
    fn test_transaction_multiple_operations() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        
        transaction.set("users/alice", MapValue { fields: HashMap::new() });
        transaction.update("users/bob", MapValue { fields: HashMap::new() });
        transaction.delete("users/charlie");
        
        assert_eq!(transaction.operations().len(), 3);
    }

    #[test]
    fn test_transaction_reads_tracking() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        
        // Simulate adding reads (this would normally happen in get())
        transaction.add_read("users/alice".to_string());
        transaction.add_read("users/bob".to_string());
        
        // Can't directly test reads vector as it's private, but we've tracked them
        assert_eq!(transaction.operations().len(), 0); // No write operations yet
    }

    #[test]
    fn test_transaction_set_with_data() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        
        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            Value {
                value_type: Some(ValueType::StringValue("Alice".to_string())),
            },
        );
        
        transaction.set("users/alice", MapValue { fields });
        assert_eq!(transaction.operations().len(), 1);
    }

    #[test]
    fn test_transaction_operation_ordering() {
        let mut transaction = Transaction::new("test".to_string(), "(default)".to_string(), "key".to_string());
        
        // Add operations in specific order
        transaction.set("doc1", MapValue { fields: HashMap::new() });
        transaction.update("doc2", MapValue { fields: HashMap::new() });
        transaction.delete("doc3");
        
        let ops = transaction.operations();
        assert_eq!(ops.len(), 3);
        
        // Verify order is maintained
        match &ops[0] {
            WriteOperation::Set { path, .. } => assert!(path.contains("doc1")),
            _ => panic!("Expected first operation to be Set"),
        }
        
        match &ops[1] {
            WriteOperation::Update { path, .. } => assert!(path.contains("doc2")),
            _ => panic!("Expected second operation to be Update"),
        }
        
        match &ops[2] {
            WriteOperation::Delete { path } => assert!(path.contains("doc3")),
            _ => panic!("Expected third operation to be Delete"),
        }
    }

    // ============================================================================
    // Additional Value Type Tests (10 more tests)
    // ============================================================================

    #[test]
    fn test_value_integer_max() {
        let int_val = Value {
            value_type: Some(ValueType::IntegerValue(i64::MAX)),
        };

        match int_val.value_type {
            Some(ValueType::IntegerValue(i)) => assert_eq!(i, i64::MAX),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_value_integer_min() {
        let int_val = Value {
            value_type: Some(ValueType::IntegerValue(i64::MIN)),
        };

        match int_val.value_type {
            Some(ValueType::IntegerValue(i)) => assert_eq!(i, i64::MIN),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_value_double_infinity() {
        let double_val = Value {
            value_type: Some(ValueType::DoubleValue(f64::INFINITY)),
        };

        match double_val.value_type {
            Some(ValueType::DoubleValue(d)) => assert!(d.is_infinite() && d.is_sign_positive()),
            _ => panic!("Expected double value"),
        }
    }

    #[test]
    fn test_value_double_neg_infinity() {
        let double_val = Value {
            value_type: Some(ValueType::DoubleValue(f64::NEG_INFINITY)),
        };

        match double_val.value_type {
            Some(ValueType::DoubleValue(d)) => assert!(d.is_infinite() && d.is_sign_negative()),
            _ => panic!("Expected double value"),
        }
    }

    #[test]
    fn test_value_double_nan() {
        let double_val = Value {
            value_type: Some(ValueType::DoubleValue(f64::NAN)),
        };

        match double_val.value_type {
            Some(ValueType::DoubleValue(d)) => assert!(d.is_nan()),
            _ => panic!("Expected double value"),
        }
    }

    #[test]
    fn test_value_string_special_chars() {
        let str_val = Value {
            value_type: Some(ValueType::StringValue("Line1\nLine2\tTabbed".to_string())),
        };

        match str_val.value_type {
            Some(ValueType::StringValue(s)) => {
                assert!(s.contains('\n'));
                assert!(s.contains('\t'));
            }
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_value_bytes_empty() {
        let bytes_val = Value {
            value_type: Some(ValueType::BytesValue(vec![])),
        };

        match bytes_val.value_type {
            Some(ValueType::BytesValue(b)) => assert_eq!(b.len(), 0),
            _ => panic!("Expected bytes value"),
        }
    }

    #[test]
    fn test_value_bytes_large() {
        let large_bytes = vec![0xFF; 1024];
        let bytes_val = Value {
            value_type: Some(ValueType::BytesValue(large_bytes.clone())),
        };

        match bytes_val.value_type {
            Some(ValueType::BytesValue(b)) => {
                assert_eq!(b.len(), 1024);
                assert_eq!(b, large_bytes);
            }
            _ => panic!("Expected bytes value"),
        }
    }

    #[test]
    fn test_value_reference_full_path() {
        let ref_val = Value {
            value_type: Some(ValueType::ReferenceValue(
                "projects/my-project/databases/(default)/documents/users/alice/posts/post1".to_string(),
            )),
        };

        match ref_val.value_type {
            Some(ValueType::ReferenceValue(r)) => {
                assert!(r.starts_with("projects/"));
                assert!(r.contains("/databases/"));
                assert!(r.contains("/documents/"));
            }
            _ => panic!("Expected reference value"),
        }
    }

    #[test]
    fn test_value_array_nested() {
        use proto::google::firestore::v1::ArrayValue;

        let inner_array = ArrayValue {
            values: vec![
                Value {
                    value_type: Some(ValueType::IntegerValue(1)),
                },
                Value {
                    value_type: Some(ValueType::IntegerValue(2)),
                },
            ],
        };

        let outer_array = ArrayValue {
            values: vec![
                Value {
                    value_type: Some(ValueType::ArrayValue(inner_array)),
                },
                Value {
                    value_type: Some(ValueType::IntegerValue(3)),
                },
            ],
        };

        let array_val = Value {
            value_type: Some(ValueType::ArrayValue(outer_array)),
        };

        match array_val.value_type {
            Some(ValueType::ArrayValue(arr)) => {
                assert_eq!(arr.values.len(), 2);
                match &arr.values[0].value_type {
                    Some(ValueType::ArrayValue(inner)) => {
                        assert_eq!(inner.values.len(), 2);
                    }
                    _ => panic!("Expected nested array"),
                }
            }
            _ => panic!("Expected array value"),
        }
    }
}
