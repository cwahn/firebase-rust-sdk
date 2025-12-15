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
}

impl FilterCondition {
    /// Get the field path for this filter
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
            return Err(FirestoreError::InvalidArgument(
                format!("nanoseconds must be in range [0, 999999999], got {}", nanoseconds)
            ));
        }
        
        Ok(Self { seconds, nanoseconds })
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
        DateTime::from_timestamp(self.seconds, self.nanoseconds as u32)
            .unwrap_or_else(|| Utc::now())
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
            return Err(FirestoreError::InvalidArgument(
                format!("latitude must be in range [-90, 90], got {}", latitude)
            ));
        }
        
        // Validate longitude (error cases first)
        if longitude < -180.0 || longitude > 180.0 {
            return Err(FirestoreError::InvalidArgument(
                format!("longitude must be in range [-180, 180], got {}", longitude)
            ));
        }
        
        Ok(Self { latitude, longitude })
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
        Self {
            path: path.into(),
        }
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

/// Write batch for atomic operations
///
/// # C++ Reference
/// - `firestore/src/include/firebase/firestore/write_batch.h:46`
#[derive(Debug, Default)]
pub struct WriteBatch {
    operations: Vec<WriteOperation>,
}

#[derive(Debug)]
enum WriteOperation {
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
        self.operations.push(WriteOperation::Delete {
            path: path.into(),
        });
        self
    }

    /// Commit the batch
    pub async fn commit(self) -> Result<(), FirestoreError> {
        todo!("Implement batch commit via REST API")
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
        
        batch.set("users/bob", data.clone())
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
