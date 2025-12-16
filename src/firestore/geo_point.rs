//! Firestore GeoPoint type
//!
//! # C++ Reference
//! - `firestore/src/include/firebase/firestore/geo_point.h:37`

use crate::error::FirestoreError;
use serde::{Deserialize, Serialize};

// Re-export Value type for to_value() method
use super::field_value::{Value, proto};

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

#[cfg(test)]
mod tests {
    use super::*;

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
        use super::super::field_value::proto::google::firestore::v1::value::ValueType;
        
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
}
