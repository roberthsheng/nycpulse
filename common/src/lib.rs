// common/src/lib.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubwayStatus {
    pub line: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub delays: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_subway_status_equality() {
        let timestamp = Utc.timestamp_opt(1640995200, 0).unwrap();
        let status1 = SubwayStatus {
            line: "A".to_string(),
            status: "Good Service".to_string(),
            timestamp,
            delays: false,
        };

        let status2 = SubwayStatus {
            line: "A".to_string(),
            status: "Good Service".to_string(),
            timestamp,
            delays: false,
        };

        assert_eq!(status1, status2);
    }

    #[test]
    fn test_subway_status_inequality() {
        let timestamp = Utc.timestamp_opt(1640995200, 0).unwrap();
        let status1 = SubwayStatus {
            line: "A".to_string(),
            status: "Good Service".to_string(),
            timestamp,
            delays: false,
        };

        let status2 = SubwayStatus {
            line: "B".to_string(), // Different line
            status: "Good Service".to_string(),
            timestamp,
            delays: false,
        };

        assert_ne!(status1, status2);
    }
}
