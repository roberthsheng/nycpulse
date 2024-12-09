//! # NYC Subway Data Module
//!
//! This module handles fetching, processing and managing real-time NYC subway data including:
//! - Station locations and metadata
//! - Real-time train positions and movement
//! - Data conversion to GeoJSON format for map display
//!
//! ## Key Components
//!
//! - `SubwayStationResponse`: Raw station data from MTA API
//! - `GeoJsonCollection`/`GeoJsonFeature`: GeoJSON structures for map display
//! - `TrainPosition`/`TrainState`: Real-time train tracking
//!
//! ## Data Flow
//!
//! 1. Raw station/train data is fetched from APIs
//! 2. Data is parsed into internal structures
//! 3. Positions are interpolated for smooth animation
//! 4. Data is converted to GeoJSON for map rendering

use gloo_net::http::Request;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the current state of a train including its position and movement progress
#[derive(Clone)]
struct TrainState {
    position: TrainPosition,
    current_progress: f64,
    last_update: f64,
}

/// Global state for tracking all active trains
static TRAIN_STATES: Lazy<Mutex<HashMap<String, TrainState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Raw subway station data received from the MTA API
#[derive(Debug, Deserialize, Clone)]
pub struct SubwayStationResponse {
    /// Station name/label
    pub stop_name: String,
    /// Comma-separated list of train lines serving this station
    pub daytime_routes: String,
    /// Station latitude coordinate
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub gtfs_latitude: f64,
    /// Station longitude coordinate
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub gtfs_longitude: f64,
    /// MTA division (e.g. IRT, BMT, IND)
    pub division: String,
    /// Primary subway line
    pub line: String,
    /// NYC borough location
    pub borough: String,
    /// ADA accessibility status
    pub ada: Option<String>,
    /// Additional accessibility notes
    pub ada_notes: Option<String>,
    /// Uptown/north direction label
    pub north_direction_label: Option<String>,
    /// Downtown/south direction label
    pub south_direction_label: Option<String>,
}

/// Deserializes string coordinates into f64 values
fn deserialize_string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}

/// A GeoJSON Feature representing a subway station or train
#[derive(Debug, Serialize, Clone)]
pub struct GeoJsonFeature {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub properties: GeoJsonProperties,
    pub geometry: GeoJsonGeometry,
}

/// Properties associated with a GeoJSON Feature
#[derive(Debug, Serialize, Clone)]
pub struct GeoJsonProperties {
    pub name: String,
    pub lines: String,
    pub division: String,
    pub borough: String,
    pub ada: bool,
    pub ada_notes: String,
    pub north_direction: String,
    pub south_direction: String,
    pub color: String,
}

/// Geometry component of a GeoJSON Feature
#[derive(Debug, Serialize, Clone)]
pub struct GeoJsonGeometry {
    #[serde(rename = "type")]
    pub geometry_type: String,
    #[serde(rename = "coordinates")]
    pub coordinates: GeoJsonCoordinates,
}

/// Coordinates for either a Point or LineString geometry
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum GeoJsonCoordinates {
    Point([f64; 2]),
    LineString(Vec<[f64; 2]>),
}

/// Collection of GeoJSON Features
#[derive(Debug, Serialize, Clone)]
pub struct GeoJsonCollection {
    #[serde(rename = "type")]
    pub collection_type: String,
    pub features: Vec<GeoJsonFeature>,
}

impl GeoJsonCollection {
    /// Creates a new GeoJSON collection from subway station data
    pub fn new(stations: Vec<SubwayStationResponse>) -> Self {
        let features = stations
            .into_iter()
            .map(|station| GeoJsonFeature {
                feature_type: "Feature".to_string(),
                properties: GeoJsonProperties {
                    name: station.stop_name,
                    lines: station.daytime_routes,
                    division: station.division,
                    borough: station.borough,
                    ada: station.ada.unwrap_or_default() == "TRUE",
                    ada_notes: station.ada_notes.unwrap_or_default(),
                    north_direction: station.north_direction_label.unwrap_or_default(),
                    south_direction: station.south_direction_label.unwrap_or_default(),
                    color: match station.line.chars().next().unwrap_or('_') {
                        'A' | 'C' | 'E' => "#0039A6",       // Dark blue
                        'B' | 'D' | 'F' | 'M' => "#FF6319", // Orange
                        'G' => "#6CBE45",                   // Green
                        'J' | 'Z' => "#996633",             // Brown
                        'L' => "#A7A9AC",                   // Gray
                        'N' | 'Q' | 'R' | 'W' => "#FCCC0A", // Yellow
                        '1' | '2' | '3' => "#EE352E",       // Red
                        '4' | '5' | '6' => "#00933C",       // Green
                        '7' => "#B933AD",                   // Purple
                        'S' => "#808183",                   // Gray
                        _ => "#808183",                     // Default gray
                    }
                    .to_string(),
                },
                geometry: GeoJsonGeometry {
                    geometry_type: "Point".to_string(),
                    coordinates: GeoJsonCoordinates::Point([
                        station.gtfs_longitude,
                        station.gtfs_latitude,
                    ]),
                },
            })
            .collect();

        GeoJsonCollection {
            collection_type: "FeatureCollection".to_string(),
            features,
        }
    }
}

/// Fetches subway station data from the NY Open Data API
pub async fn fetch_subway_stations() -> Result<GeoJsonCollection, gloo_net::Error> {
    let response = Request::get("https://data.ny.gov/resource/39hk-dx4f.json")
        .send()
        .await?;

    let stations: Vec<SubwayStationResponse> = response.json().await?;
    Ok(GeoJsonCollection::new(stations))
}

/// Returns the Tailwind CSS class for styling a subway line indicator
pub fn get_line_style(line: &str) -> &'static str {
    match line {
        "A" | "C" | "E" => "bg-blue-500",
        "B" | "D" | "F" | "M" => "bg-orange-500",
        "G" => "bg-green-500",
        "J" | "Z" => "bg-brown-500",
        "L" => "bg-gray-500",
        "N" | "Q" | "R" | "W" => "bg-yellow-500 text-black",
        "1" | "2" | "3" => "bg-red-500",
        "4" | "5" | "6" => "bg-green-500",
        "7" => "bg-purple-500",
        "S" => "bg-gray-500",
        _ => "bg-gray-400",
    }
}

/// Real-time train position data from the MTA API
#[derive(Debug, Deserialize, Clone)]
pub struct TrainPosition {
    pub trip_id: String,
    pub route_id: String,
    pub from_stop: StopLocation,
    pub to_stop: StopLocation,
    pub progress: f64,
    pub start_time: i64,
    pub end_time: i64,
}

/// Location data for a subway stop/station
#[derive(Debug, Deserialize, Clone)]
pub struct StopLocation {
    pub stop_id: String,
    pub latitude: f64,
    pub longitude: f64,
}

/// GeoJSON Feature specifically for train positions
#[derive(Debug, Serialize, Clone)]
pub struct TrainFeature {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub properties: TrainProperties,
    pub geometry: GeoJsonGeometry,
}

/// Properties specific to train features
#[derive(Debug, Serialize, Clone)]
pub struct TrainProperties {
    pub trip_id: String,
    pub route_id: String,
    pub progress: f64,
}

/// Fetches and processes real-time train position data
///
/// This function:
/// 1. Fetches latest positions from the API
/// 2. Updates the global train state
/// 3. Interpolates positions for smooth animation
/// 4. Converts to GeoJSON format
pub async fn fetch_train_positions() -> Result<GeoJsonCollection, gloo_net::Error> {
    let response = Request::get("http://localhost:3000/api/trains")
        .send()
        .await?;

    let text = response.text().await?;
    let new_positions: Vec<TrainPosition> = serde_json::from_str(&text)?;
    let current_time = js_sys::Date::now() / 1000.0;

    let mut train_states = TRAIN_STATES.lock();

    // Clear any trains that are at the end of their journey (progress >= 1.0)
    train_states.retain(|_, state| state.current_progress < 1.0);

    // Create a set of trip IDs from the new update
    let updated_trips: std::collections::HashSet<String> = new_positions
        .iter()
        .map(|pos| pos.trip_id.clone())
        .collect();

    // Update existing trains with new data or continue their movement
    for (trip_id, state) in train_states.iter_mut() {
        if !updated_trips.contains(trip_id) {
            // Train wasn't in the update, continue its movement
            let time_delta = current_time - state.last_update;
            let total_journey_time = (state.position.end_time - state.position.start_time) as f64;
            let progress_increment = if total_journey_time > 0.0 {
                time_delta / total_journey_time
            } else {
                0.0
            };
            state.current_progress = (state.current_progress + progress_increment).min(1.0);
            state.last_update = current_time;
        }
    }

    // Process new position updates
    for new_pos in new_positions {
        train_states
            .entry(new_pos.trip_id.clone())
            .and_modify(|state| {
                // Only update if the train has moved to a new segment
                if state.position.from_stop.stop_id != new_pos.from_stop.stop_id
                    || state.position.to_stop.stop_id != new_pos.to_stop.stop_id
                {
                    state.position = new_pos.clone();
                    state.current_progress = new_pos.progress;
                    state.last_update = current_time;
                }
            })
            .or_insert_with(|| TrainState {
                position: new_pos,
                current_progress: 0.0,
                last_update: current_time,
            });
    }

    // Only include trains that are actively moving (progress < 1.0)
    let features: Vec<GeoJsonFeature> = train_states
        .iter()
        .filter(|(_, state)| state.current_progress < 1.0)
        .map(|(_, state)| {
            // Calculate interpolated position
            let current_lat = state.position.from_stop.latitude
                + (state.position.to_stop.latitude - state.position.from_stop.latitude)
                    * state.current_progress;
            let current_lon = state.position.from_stop.longitude
                + (state.position.to_stop.longitude - state.position.from_stop.longitude)
                    * state.current_progress;

            GeoJsonFeature {
                feature_type: "Feature".to_string(),
                properties: GeoJsonProperties {
                    name: format!("Train {}", state.position.route_id), // Changed from trip_id to route_id
                    lines: state.position.route_id.clone(),
                    division: String::new(),
                    borough: String::new(),
                    ada: false,
                    ada_notes: String::new(),
                    north_direction: String::new(),
                    south_direction: String::new(),
                    color: match state.position.route_id.chars().next().unwrap_or('_') {
                        'A' | 'C' | 'E' => "#0039A6",
                        'B' | 'D' | 'F' | 'M' => "#FF6319",
                        'G' => "#6CBE45",
                        'J' | 'Z' => "#996633",
                        'L' => "#A7A9AC",
                        'N' | 'Q' | 'R' | 'W' => "#FCCC0A",
                        '1' | '2' | '3' => "#EE352E",
                        '4' | '5' | '6' => "#00933C",
                        '7' => "#B933AD",
                        'S' => "#808183",
                        _ => "#808183",
                    }
                    .to_string(),
                },
                geometry: GeoJsonGeometry {
                    geometry_type: "Point".to_string(),
                    coordinates: GeoJsonCoordinates::Point([current_lon, current_lat]),
                },
            }
        })
        .collect();

    Ok(GeoJsonCollection {
        collection_type: "FeatureCollection".to_string(),
        features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_style_colors() {
        // Test A/C/E lines (blue)
        assert_eq!(get_line_style("A"), "bg-blue-500");
        assert_eq!(get_line_style("C"), "bg-blue-500");
        assert_eq!(get_line_style("E"), "bg-blue-500");

        // Test B/D/F/M lines (orange)
        assert_eq!(get_line_style("B"), "bg-orange-500");
        assert_eq!(get_line_style("D"), "bg-orange-500");
        assert_eq!(get_line_style("F"), "bg-orange-500");
        assert_eq!(get_line_style("M"), "bg-orange-500");

        // Test numbered lines
        assert_eq!(get_line_style("1"), "bg-red-500");
        assert_eq!(get_line_style("4"), "bg-green-500");
        assert_eq!(get_line_style("7"), "bg-purple-500");

        // Test special cases
        assert_eq!(get_line_style("S"), "bg-gray-500");
        assert_eq!(get_line_style("unknown"), "bg-gray-400");
    }

    #[test]
    fn test_geojson_collection_creation() {
        let stations = vec![
            SubwayStationResponse {
                stop_name: "14th St".to_string(),
                line: "L".to_string(),
                daytime_routes: "L".to_string(),
                gtfs_latitude: 40.7,
                gtfs_longitude: -73.9,
                division: "IRT".to_string(),
                borough: "Manhattan".to_string(),
                ada: Some("Y".to_string()),
                ada_notes: Some(String::new()),
                north_direction_label: Some("Manhattan".to_string()),
                south_direction_label: Some("Brooklyn".to_string()),
            },
            SubwayStationResponse {
                stop_name: "Union Square".to_string(),
                line: "L,N,Q,R,4,5,6".to_string(),
                daytime_routes: "L,N,Q,R,4,5,6".to_string(),
                gtfs_latitude: 40.735,
                gtfs_longitude: -73.99,
                division: "IRT".to_string(),
                borough: "Manhattan".to_string(),
                ada: Some("Y".to_string()),
                ada_notes: Some("Elevators available".to_string()),
                north_direction_label: Some("Uptown".to_string()),
                south_direction_label: Some("Downtown".to_string()),
            },
        ];

        let collection = GeoJsonCollection::new(stations);

        assert_eq!(collection.collection_type, "FeatureCollection");
        assert_eq!(collection.features.len(), 2);

        // Check first feature
        let first = &collection.features[0];
        assert_eq!(first.feature_type, "Feature");
        assert_eq!(first.properties.name, "14th St");
        assert_eq!(first.properties.lines, "L");

        if let GeoJsonCoordinates::Point(coords) = &first.geometry.coordinates {
            assert_eq!(coords[0], -73.9);
            assert_eq!(coords[1], 40.7);
        }

        // Check second feature
        let second = &collection.features[1];
        assert_eq!(second.properties.name, "Union Square");
        assert_eq!(second.properties.lines, "L,N,Q,R,4,5,6");

        if let GeoJsonCoordinates::Point(coords) = &second.geometry.coordinates {
            assert_eq!(coords[0], -73.99);
            assert_eq!(coords[1], 40.735);
        }
    }

    #[test]
    fn test_train_feature_creation() {
        let train = TrainPosition {
            trip_id: "123".to_string(),
            route_id: "L".to_string(),
            from_stop: StopLocation {
                stop_id: "L06".to_string(),
                latitude: 40.7,
                longitude: -73.9,
            },
            to_stop: StopLocation {
                stop_id: "L08".to_string(),
                latitude: 40.71,
                longitude: -73.92,
            },
            progress: 0.5,
            start_time: 1000,
            end_time: 2000,
        };

        let feature = GeoJsonFeature {
            feature_type: "Feature".to_string(),
            properties: GeoJsonProperties {
                name: format!("Train {}", train.route_id),
                lines: train.route_id.clone(),
                division: String::new(),
                borough: String::new(),
                ada: false,
                ada_notes: String::new(),
                north_direction: String::new(),
                south_direction: String::new(),
                color: "#A7A9AC".to_string(),
            },
            geometry: GeoJsonGeometry {
                geometry_type: "Point".to_string(),
                coordinates: GeoJsonCoordinates::Point([-73.91, 40.705]),
            },
        };

        assert_eq!(feature.feature_type, "Feature");
        assert_eq!(feature.properties.name, "Train L");
        assert_eq!(feature.properties.lines, "L");

        if let GeoJsonCoordinates::Point(coords) = &feature.geometry.coordinates {
            assert_eq!(coords[0], -73.91); // Interpolated longitude
            assert_eq!(coords[1], 40.705); // Interpolated latitude
        }
    }
}
