-- project-roberthsheng/backend/schema.sql
-- backend/schema.sql
CREATE TABLE IF NOT EXISTS subway_status (
    id SERIAL PRIMARY KEY,
    line VARCHAR(10) NOT NULL,
    status VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    delays BOOLEAN NOT NULL
);

CREATE TABLE bike_stations (
    id SERIAL PRIMARY KEY,
    station_id VARCHAR(50) NOT NULL,
    name VARCHAR(100) NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    bikes_available INTEGER NOT NULL,
    docks_available INTEGER NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE TABLE air_quality (
    id SERIAL PRIMARY KEY,
    station_id VARCHAR(50) NOT NULL,
    pm25 DOUBLE PRECISION NOT NULL,
    ozone DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE TABLE service_requests (
    id SERIAL PRIMARY KEY,
    request_id VARCHAR(50) NOT NULL,
    request_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION
);

-- Create indexes for time-based queries
CREATE INDEX idx_subway_status_timestamp ON subway_status(timestamp);
CREATE INDEX idx_bike_stations_timestamp ON bike_stations(timestamp);
CREATE INDEX idx_air_quality_timestamp ON air_quality(timestamp);
CREATE INDEX idx_service_requests_created_at ON service_requests(created_at);
CREATE INDEX IF NOT EXISTS idx_subway_status_timestamp ON subway_status(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_subway_status_line ON subway_status(line);