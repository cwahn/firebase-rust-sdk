//! Firestore Timestamp type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/timestamp.h:41`

use crate::error::FirestoreError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export Value type for to_value() method
use super::field_value::Value;

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
        use super::field_value::proto::google::firestore::v1::value::ValueType;
        Value {
            value_type: Some(ValueType::TimestampValue(prost_types::Timestamp {
                seconds: self.seconds,
                nanos: self.nanoseconds,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use super::super::field_value::proto::google::firestore::v1::value::ValueType;
        
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
        
        use super::super::field_value::proto::google::firestore::v1::value::ValueType;
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
}
