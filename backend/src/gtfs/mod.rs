//! GTFS Real-Time Feed Handler Module
//!
//! This module provides functionality for fetching and parsing GTFS (General Transit Feed Specification)
//! real-time data from the MTA (Metropolitan Transportation Authority) API. It handles:
//!
//! - Fetching station location data from NY Open Data
//! - Processing real-time train position updates from GTFS feeds
//! - Calculating train positions between stops based on timing data
//!
//! The module uses the GTFS Realtime protobuf format for parsing feed data and maintains
//! an in-memory cache of subway station locations for position calculations.

use chrono::Utc;
use gtfs_rt::FeedMessage;
use log::{debug, info};
use nyc_pulse_backend::{Error, Result, StopLocation, TrainPosition};
use prost::Message;
use serde::Deserialize;
use std::collections::HashMap;

/// Response structure for station location data from the NY Open Data API
#[derive(Deserialize)]
struct StationResponse {
    /// GTFS stop ID for the station
    gtfs_stop_id: String,
    /// Latitude coordinate as string
    gtfs_latitude: String,
    /// Longitude coordinate as string
    gtfs_longitude: String,
}

/// Main handler for GTFS real-time data processing
///
/// Maintains station location data and provides methods for fetching
/// real-time train positions from the MTA's GTFS feeds.
#[derive(Clone)]
pub struct GtfsHandler {
    /// HTTP client for making API requests
    client: reqwest::Client,
    /// Cache of station locations indexed by stop ID
    stop_locations: HashMap<String, (f64, f64)>,
}

impl GtfsHandler {
    /// Creates a new GtfsHandler instance
    ///
    /// Initializes by fetching station location data from NY Open Data API
    /// and building an in-memory lookup table of stop coordinates.
    ///
    /// # Returns
    /// - `Result<GtfsHandler>` - New handler instance or error if initialization fails
    ///
    /// # Errors
    /// - If station data API request fails
    /// - If station coordinate parsing fails
    pub async fn new() -> Result<Self> {
        let client = reqwest::Client::new();

        // Fetch all station locations
        let response = client
            .get("https://data.ny.gov/resource/39hk-dx4f.json")
            .send()
            .await?;

        let stations: Vec<StationResponse> = response.json().await?;

        // Create stop locations map with both N and S directions
        let mut stop_locations = HashMap::new();
        for station in stations {
            let lat: f64 = station
                .gtfs_latitude
                .parse()
                .map_err(|e| Error::Environment(format!("Invalid latitude: {}", e)))?;
            let lon: f64 = station
                .gtfs_longitude
                .parse()
                .map_err(|e| Error::Environment(format!("Invalid longitude: {}", e)))?;

            // Add both northbound and southbound stops
            stop_locations.insert(format!("{}N", station.gtfs_stop_id), (lat, lon));
            stop_locations.insert(format!("{}S", station.gtfs_stop_id), (lat, lon));
        }

        println!("Loaded {} stop locations", stop_locations.len() / 2);

        Ok(Self {
            client,
            stop_locations,
        })
    }

    /// Fetches current train positions from all GTFS feeds
    ///
    /// Queries each MTA GTFS feed URL, processes the protobuf responses,
    /// and calculates current train positions based on timing data.
    ///
    /// # Returns
    /// - `Result<Vec<TrainPosition>>` - List of current train positions or error
    ///
    /// # Errors
    /// - If any feed request fails
    /// - If protobuf decoding fails
    pub async fn get_train_positions(&self) -> Result<Vec<TrainPosition>> {
        let mut positions = Vec::new();
        let feeds = vec![
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs", // 1234567
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-ace", // ACE
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-bdfm", // BDFM
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-g", // G
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-jz", // JZ
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-nqrw", // NQRW
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-l", // L
            "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs-si", // Staten Island Railway
        ];

        for url in feeds {
            // Print the raw response
            let response = self.client.get(url).send().await?;
            // println!("\n=== API RESPONSE for {} ===", url);
            // println!("Status: {:?}", response.status());

            let bytes = response.bytes().await?;
            // println!("Got {} bytes", bytes.len());

            // Print stop locations we're looking for
            // println!("\n=== STOP LOCATIONS WE HAVE ===");
            // for (stop_id, (lat, lon)) in &self.stop_locations {
            //     println!("Stop {}: ({}, {})", stop_id, lat, lon);
            // }

            let current_time = Utc::now().timestamp();
            // println!("\nCurrent time: {}", current_time);

            let feed = FeedMessage::decode(bytes.as_ref())
                .map_err(|e| Error::Environment(format!("Failed to decode GTFS feed: {}", e)))?;
            debug!("Decoded Feed: {:?}", feed);

            // println!("\n=== STOPS IN FEED ===");
            // for entity in &feed.entity {
            //     if let Some(trip_update) = &entity.trip_update {
            //         for stop_time in &trip_update.stop_time_update {
            //             if let Some(stop_id) = &stop_time.stop_id {
            //                 println!(
            //                     "Stop {} - exists in our map: {}",
            //                     stop_id,
            //                     self.stop_locations.contains_key(stop_id)
            //                 );
            //             }
            //         }
            //     }
            // }

            for entity in feed.entity {
                if let Some(trip_update) = entity.trip_update {
                    let trip = &trip_update.trip;
                    let route_id = trip.route_id.clone().unwrap_or_default();
                    info!(
                        "Processing Trip: {} on Route: {}",
                        trip.trip_id.clone().unwrap_or_default(),
                        route_id
                    );

                    for window in trip_update.stop_time_update.windows(2) {
                        let from_stop = &window[0];
                        let to_stop = &window[1];

                        let from_time = from_stop
                            .departure
                            .as_ref()
                            .or(from_stop.arrival.as_ref())
                            .and_then(|t| t.time);
                        let to_time = to_stop
                            .arrival
                            .as_ref()
                            .or(to_stop.departure.as_ref())
                            .and_then(|t| t.time);

                        if let (
                            Some(from_time),
                            Some(to_time),
                            Some(from_stop_id),
                            Some(to_stop_id),
                        ) = (
                            from_time,
                            to_time,
                            from_stop.stop_id.as_ref(),
                            to_stop.stop_id.as_ref(),
                        ) {
                            debug!(
                                "From Stop: {}, To Stop: {}, From Time: {}, To Time: {}",
                                from_stop_id, to_stop_id, from_time, to_time
                            );

                            if current_time >= from_time && current_time <= to_time {
                                if let (Some(from_loc), Some(to_loc)) = (
                                    self.stop_locations.get(from_stop_id),
                                    self.stop_locations.get(to_stop_id),
                                ) {
                                    let progress = (current_time - from_time) as f64
                                        / (to_time - from_time) as f64;

                                    positions.push(TrainPosition {
                                        trip_id: trip.trip_id.clone().unwrap_or_default(),
                                        route_id: route_id.clone(),
                                        from_stop: StopLocation {
                                            stop_id: from_stop_id.clone(),
                                            latitude: from_loc.0,
                                            longitude: from_loc.1,
                                        },
                                        to_stop: StopLocation {
                                            stop_id: to_stop_id.clone(),
                                            latitude: to_loc.0,
                                            longitude: to_loc.1,
                                        },
                                        progress,
                                        start_time: from_time,
                                        end_time: to_time,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // println!("\n=== FOUND POSITIONS ===");
            // for pos in &positions {
            //     println!(
            //         "Train {} on route {} from {} to {}, progress: {}",
            //         pos.trip_id,
            //         pos.route_id,
            //         pos.from_stop.stop_id,
            //         pos.to_stop.stop_id,
            //         pos.progress
            //     );
            // }

            info!("Found {} trains in transit", positions.len());
        }
        Ok(positions)
    }
}
