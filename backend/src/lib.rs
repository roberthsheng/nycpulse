//! NYC Pulse Backend Library
//!
//! This library provides core data structures and error handling for the NYC Pulse application,
//! which tracks real-time NYC subway information. The library defines various data types for
//! subway status, train positions, and stop locations, along with error handling utilities.
//!
//! # Features
//!
//! * Real-time subway status tracking
//! * Train position monitoring
//! * Stop location management
//!
//! # Future Features
//!
//!   These features are not currently implemented but provide extension points for future development.
//!
//!   * Bike sharing station status
//!   * Air quality measurements
//!   * 311 service request tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents the current status of a subway line
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SubwayStatus {
    /// The subway line identifier (e.g., "A", "1", "L")
    pub line: String,
    /// Current service status (e.g., "Good Service", "Delays")
    pub status: String,
    /// Timestamp when this status was recorded
    pub timestamp: DateTime<Utc>,
    /// Boolean indicating if there are currently delays
    pub delays: bool,
}

/// Represents a bike sharing station (future feature)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BikeStation {
    /// Unique identifier for the station
    pub station_id: String,
    /// Human-readable station name
    pub name: String,
    /// Station latitude coordinate
    pub latitude: f64,
    /// Station longitude coordinate
    pub longitude: f64,
    /// Number of bikes currently available
    pub bikes_available: i32,
    /// Number of docks currently available
    pub docks_available: i32,
    /// Timestamp of last update
    pub timestamp: DateTime<Utc>,
}

/// Represents air quality measurements from a monitoring station (future feature)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AirQuality {
    /// Unique identifier for the monitoring station
    pub station_id: String,
    /// PM2.5 particulate matter measurement
    pub pm25: f64,
    /// Ozone level measurement
    pub ozone: f64,
    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
}

/// Represents a 311 service request (future feature)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceRequest {
    /// Unique identifier for the request
    pub request_id: String,
    /// Category of the service request
    pub request_type: String,
    /// Current status of the request
    pub status: String,
    /// When the request was created
    pub created_at: DateTime<Utc>,
    /// Optional latitude of the request location
    pub latitude: Option<f64>,
    /// Optional longitude of the request location
    pub longitude: Option<f64>,
}

/// Represents the current position of a subway train
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainPosition {
    /// GTFS trip identifier
    pub trip_id: String,
    /// Subway route identifier (e.g., "A", "1")
    pub route_id: String,
    /// The previous stop location
    pub from_stop: StopLocation,
    /// The next stop location
    pub to_stop: StopLocation,
    /// Progress between stops (0.0 to 1.0)
    pub progress: f64,
    /// Unix timestamp when train departed from_stop
    pub start_time: i64,
    /// Estimated Unix timestamp when train will arrive at to_stop
    pub end_time: i64,
}

/// Represents a subway stop location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLocation {
    /// GTFS stop identifier
    pub stop_id: String,
    /// Stop latitude coordinate
    pub latitude: f64,
    /// Stop longitude coordinate
    pub longitude: f64,
}

/// Custom error types for the application
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// External API errors
    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),
    /// File system I/O errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Environment/configuration errors
    #[error("Environment error: {0}")]
    Environment(String),
}

/// Convenience type alias for Results using our custom Error type
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_subway_status_creation() {
        let timestamp = Utc.timestamp_opt(1640995200, 0).unwrap(); // 2022-01-01 00:00:00 UTC

        let status = SubwayStatus {
            line: "A".to_string(),
            status: "Good Service".to_string(),
            timestamp,
            delays: false,
        };

        assert_eq!(status.line, "A");
        assert_eq!(status.status, "Good Service");
        assert_eq!(status.timestamp.timestamp(), 1640995200);
        assert!(!status.delays);
    }

    #[test]
    fn test_subway_status_with_delays() {
        let timestamp = Utc::now();
        let status = SubwayStatus {
            line: "7".to_string(),
            status: "Delays".to_string(),
            timestamp,
            delays: true,
        };

        assert_eq!(status.line, "7");
        assert_eq!(status.status, "Delays");
        assert!(status.delays);
    }

    #[test]
    fn test_train_position_creation() {
        let position = TrainPosition {
            trip_id: "123".to_string(),
            route_id: "A".to_string(),
            from_stop: StopLocation {
                stop_id: "A01".to_string(),
                latitude: 40.7,
                longitude: -73.9,
            },
            to_stop: StopLocation {
                stop_id: "A02".to_string(),
                latitude: 40.8,
                longitude: -73.8,
            },
            progress: 0.5,
            start_time: 1000,
            end_time: 2000,
        };

        assert_eq!(position.trip_id, "123");
        assert_eq!(position.route_id, "A");
        assert_eq!(position.from_stop.stop_id, "A01");
        assert_eq!(position.to_stop.stop_id, "A02");
        assert_eq!(position.progress, 0.5);
        assert_eq!(position.start_time, 1000);
        assert_eq!(position.end_time, 2000);
    }

    #[test]
    fn test_stop_location_creation() {
        let stop = StopLocation {
            stop_id: "L06".to_string(),
            latitude: 40.7,
            longitude: -73.9,
        };

        assert_eq!(stop.stop_id, "L06");
        assert_eq!(stop.latitude, 40.7);
        assert_eq!(stop.longitude, -73.9);
    }
}
