# NYC Pulse

## Description

NYC Pulse is a real-time visualization of New York City's subway system. Built with Rust and WebAssembly, it provides a live, interactive map showing train positions and service status updates. The application features:

- Live train tracking with color-coded lines
- Station locations with glowing markers
- Service status updates for all subway lines
- Interactive map with zoom and pan controls

Key technologies and crates used:
- Yew for the frontend framework
- wasm-bindgen for WebAssembly integration
- Mapbox GL JS for map rendering
- gloo for web APIs
- serde for serialization
- Tailwind CSS for styling
- PostgreSQL for data storage
- SQLx for database operations
- Axum for the backend server

## Installation

1. Install Rust and cargo:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install WebAssembly target:
```bash
rustup target add wasm32-unknown-unknown
```

3. Install Trunk:
```bash
cargo install trunk
```

4. Install PostgreSQL:
```bash
# On macOS with Homebrew
brew install postgresql@14
brew services start postgresql@14

# On Ubuntu/Debian
sudo apt update
sudo apt install postgresql postgresql-contrib
```

5. Set up the database:
```bash
# Get your current directory
pwd  # Note this path

# Connect to postgres as superuser
psql postgres

# In psql, run:
DROP DATABASE IF EXISTS nyc_pulse;
CREATE DATABASE nyc_pulse;
\c nyc_pulse
\i /full/path/to/your/project/backend/schema.sql
\q
```

6. Build each component:
```bash
# Build backend
cd backend
cargo build

# Build data collector
cd ../data-collector
cargo build

# Build frontend
cd ../frontend
cargo build
```

7. Set up environment variables:
- Create a `.env` file in the project root
  ```env
  DATABASE_URL=postgres://localhost/nyc_pulse
  ```

## How to use

1. Start the backend server:
```bash
cd backend
cargo run
```

2. In a separate terminal, start the data collector:
```bash
cd data-collector
cargo run
```

3. In a separate terminal, start the frontend development server:
```bash
cd frontend
npm install -D tailwindcss@latest postcss@latest autoprefixer@latest
npx tailwindcss init -p
```

4. Configure Tailwind CSS:
   - Update tailwind.config.js:
   ```javascript
   module.exports = {
     content: [
       "./src/**/*.{html,js,rs}",
       "./index.html"
     ],
     theme: {
       extend: {},
     },
     plugins: [],
   }
   ```
   - Ensure index.scss contains:
   ```css
   @tailwind base;
   @tailwind components;
   @tailwind utilities;
   ```

5. Start the development server:
```bash
# Clean npm cache if you encounter any issues
npm cache clean --force
npm install

# Start the development server
trunk serve
```

6. Open your browser and navigate to `http://localhost:8080`

Features:
- Zoom in/out on the map to see more detail
- Watch trains move in real-time along their routes
- View constantly-refereshed service status updates 

The map will automatically update with new train positions and service statuses. The status panel on the left shows current service conditions for all subway lines.

## Database Schema

The application uses PostgreSQL with the following main tables:

```sql
-- subway_status table
CREATE TABLE subway_status (
    id SERIAL PRIMARY KEY,
    line TEXT NOT NULL,
    status TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delays BOOLEAN NOT NULL DEFAULT FALSE
);

-- Additional indexes
CREATE INDEX idx_subway_status_line_timestamp ON subway_status(line, timestamp DESC);
```
