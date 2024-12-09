//! NYC Pulse Data Collector
//!
//! This binary is responsible for collecting real-time NYC subway status data and storing it in a PostgreSQL database.
//! It periodically polls the MTA's GTFS real-time feeds to gather information about subway line statuses.
//!
//! # Architecture
//! The collector runs as a background process that:
//! - Connects to a PostgreSQL database using connection details from environment variables
//! - Creates necessary database tables and indices if they don't exist
//! - Polls subway status data at regular intervals (currently every 30 seconds)
//! - Stores status updates in the database
//!
//! # Environment Variables
//! - `DATABASE_URL`: PostgreSQL connection string (required)
//!
//! # Database Schema
//! The collector manages the `subway_status` table with the following structure:
//! - `id`: Serial primary key
//! - `line`: Subway line identifier (e.g. "A", "1")
//! - `status`: Current service status
//! - `timestamp`: When the status was recorded
//! - `delays`: Boolean indicating if there are delays
//!
//! Appropriate indices are created for efficient querying by timestamp and line.

use dotenv::dotenv;
use nyc_pulse_backend as backend;
use rand::Rng;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time;

/// Mapping of MTA GTFS feed URLs to the subway lines they contain
///
/// Each tuple contains:
/// - The GTFS feed URL for a group of subway lines
/// - Array of line identifiers included in that feed
const FEED_URLS: [(&str, &[&str]); 8] = [
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-ace",
        &["A", "C", "E", "S"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-bdfm",
        &["B", "D", "F", "M"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-g",
        &["G"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-jz",
        &["J", "Z"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-nqrw",
        &["N", "Q", "R", "W"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-l",
        &["L"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs",
        &["1", "2", "3", "4", "5", "6", "7"],
    ),
    (
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-si",
        &["SI"],
    ),
];

/// Main collector struct that handles database connections and data collection
#[derive(Clone)]
struct Collector {
    /// PostgreSQL connection pool
    db: PgPool,
}

impl Collector {
    /// Creates a new Collector instance
    ///
    /// Initializes database connection and creates required tables/indices
    ///
    /// # Returns
    /// - `Result<Self>` - New collector instance or error if initialization fails
    ///
    /// # Errors
    /// - If DATABASE_URL environment variable is not set
    /// - If database connection fails
    /// - If table/index creation fails
    async fn new() -> backend::Result<Self> {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| backend::Error::Environment("DATABASE_URL not set".into()))?;

        let db = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        // Initialize table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS subway_status (
                id SERIAL PRIMARY KEY,
                line VARCHAR(10) NOT NULL,
                status VARCHAR(100) NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                delays BOOLEAN NOT NULL
            )
            "#,
        )
        .execute(&db)
        .await?;

        // Create indices
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_subway_status_timestamp ON subway_status(timestamp DESC)"
        )
        .execute(&db)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_subway_status_line ON subway_status(line)")
            .execute(&db)
            .await?;

        Ok(Self { db })
    }

    /// Collects current subway status for all lines
    ///
    /// Currently generates sample data for development. In production, this would
    /// fetch real status data from the MTA's GTFS feeds.
    ///
    /// # Returns
    /// - `Result<()>` - Success or database error
    ///
    /// # Errors
    /// - If database insert fails
    async fn collect_subway_status(&self) -> backend::Result<()> {
        println!("Collecting subway status...");
        let mut rng = rand::thread_rng();

        // Generate some sample statuses for development
        for (_, lines) in FEED_URLS.iter() {
            for &line in *lines {
                // Randomly decide if there are delays (20% chance)
                let has_delays = rng.gen_bool(0.2);

                let status = if has_delays { "Delays" } else { "Good Service" };

                let data = backend::SubwayStatus {
                    line: line.to_string(),
                    status: status.to_string(),
                    timestamp: chrono::Utc::now(),
                    delays: has_delays,
                };

                sqlx::query!(
                    r#"
                    INSERT INTO subway_status (line, status, timestamp, delays)
                    VALUES ($1, $2, $3, $4)
                    "#,
                    data.line,
                    data.status,
                    data.timestamp,
                    data.delays
                )
                .execute(&self.db)
                .await?;
            }
        }

        println!("Updated subway status");
        Ok(())
    }
}

/// Main entry point for the collector binary
///
/// Creates a collector instance and runs an infinite loop collecting
/// subway status data every 30 seconds.
#[tokio::main]
async fn main() -> backend::Result<()> {
    let collector = Collector::new().await?;

    // collect data every 5 seconds
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        if let Err(e) = collector.collect_subway_status().await {
            eprintln!("Error collecting subway status: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_urls_validity() {
        for (url, lines) in FEED_URLS.iter() {
            // Check URL format
            assert!(url.starts_with("https://"));
            assert!(url.contains("api-endpoint.mta.info"));
            assert!(url.contains("gtfs"));

            // Check line IDs
            for line in *lines {
                assert!(!line.is_empty());
                assert!(line.len() <= 2); // NYC subway lines are 1-2 characters
            }
        }
    }

    #[test]
    fn test_feed_urls_completeness() {
        // Get all unique lines from FEED_URLS
        let mut all_lines: Vec<&str> = FEED_URLS
            .iter()
            .flat_map(|(_, lines)| lines.iter().copied())
            .collect();

        all_lines.sort();
        all_lines.dedup();

        // Check for major subway lines
        let required_lines = [
            "A", "B", "C", "D", "E", "F", "G", "L", "M", "N", "Q", "R", "W", "1", "2", "3", "4",
            "5", "6", "7",
        ];
        for line in required_lines.iter() {
            assert!(all_lines.contains(line), "Missing line: {}", line);
        }
    }

    #[test]
    fn test_feed_urls_no_duplicates() {
        // Check that no line appears in multiple feeds
        let mut seen_lines = std::collections::HashSet::new();

        for (_, lines) in FEED_URLS.iter() {
            for &line in *lines {
                assert!(
                    seen_lines.insert(line),
                    "Line {} appears in multiple feeds",
                    line
                );
            }
        }
    }
}
