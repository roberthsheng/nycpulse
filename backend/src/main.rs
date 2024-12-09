//! NYC Pulse Backend Server
//!
//! This is the main server application for NYC Pulse, providing real-time NYC subway
//! information via a REST API. The server exposes endpoints for:
//!
//! - Current subway line status information
//! - Real-time train positions from GTFS feeds
//!
//! The application uses:
//! - Axum web framework for the REST API
//! - SQLx for PostgreSQL database access
//! - GTFS-realtime feeds for train position data
//!
//! # Architecture
//! The server maintains a connection pool to the PostgreSQL database and a GTFS handler
//! for processing real-time transit feeds. These are shared across request handlers via
//! the application state.
//!
//! # API Endpoints
//! - `GET /api/subway/status` - Returns current status for all subway lines
//! - `GET /api/trains` - Returns real-time positions of all trains

mod gtfs;

use crate::gtfs::GtfsHandler;
use axum::{extract::State, routing::get, Json, Router};
use dotenv::dotenv;
use nyc_pulse_backend as backend;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

/// Shared application state available to all request handlers
#[derive(Clone)]
struct AppState {
    /// PostgreSQL connection pool
    db: PgPool,
    /// Handler for GTFS real-time data
    gtfs: GtfsHandler,
}

/// Handler for fetching current subway line status
///
/// Returns the most recent status for each subway line from the database.
/// Status includes service condition and any delays.
///
/// # Returns
/// - JSON array of [`SubwayStatus`] objects, one per line
async fn get_subway_status(State(state): State<AppState>) -> Json<Vec<backend::SubwayStatus>> {
    let statuses = sqlx::query_as!(
        backend::SubwayStatus,
        r#"
        WITH latest_statuses AS (
            SELECT DISTINCT ON (line) *
            FROM subway_status
            ORDER BY line, timestamp DESC
        )
        SELECT line, status, timestamp, delays
        FROM latest_statuses
        ORDER BY line ASC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(statuses)
}

/// Handler for fetching real-time train positions
///
/// Retrieves current positions of all trains from GTFS feeds via the GTFS handler.
///
/// # Returns
/// - JSON array of [`TrainPosition`] objects representing current train locations
async fn get_train_positions(State(state): State<AppState>) -> Json<Vec<backend::TrainPosition>> {
    let positions = state.gtfs.get_train_positions().await.unwrap_or_default();
    Json(positions)
}

/// Main entry point for the NYC Pulse backend server
///
/// Sets up the database connection, GTFS handler, and web server with API routes.
/// The server runs on port 3000 and accepts connections from any origin via CORS.
///
/// # Errors
/// Returns an error if:
/// - Database connection fails
/// - GTFS handler initialization fails
/// - Server fails to start
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let state = AppState {
        db,
        gtfs: GtfsHandler::new().await?,
    };

    let app = Router::new()
        .route("/api/subway/status", get(get_subway_status))
        .route("/api/trains", get(get_train_positions))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("Server running on http://localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
