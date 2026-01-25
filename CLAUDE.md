# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based MTConnect aggregation server that acts as a proxy, fetching data from multiple remote MTConnect agents and aggregating their responses. The server implements MTConnect 1.8 protocol and provides standard MTConnect endpoints. Data is stored in PostgreSQL for other services to consume.

## Architecture

### Core Components

- **main.rs**: HTTP server using Hyper 1.0 with async/await. Each request spawns a new task that handles the HTTP connection and routes to appropriate endpoints (/, /probe, /current, /sample). Also handles database storage on each /current request.

- **mtconnect.rs**: MTConnect agent implementation with two key responsibilities:
  1. Generate local MTConnect XML responses (probe, current, sample)
  2. Fetch and aggregate XML data from remote agents in parallel using tokio::spawn
  3. Parse device information and convert to CncData for database storage

- **monitor.rs**: TUI application for real-time monitoring of remote agent connections.

- **elfin-postgres-data-access**: External library for PostgreSQL database operations.

### Data Flow

1. Client requests arrive at main.rs endpoints
2. `MTConnectAgent` makes parallel HTTP requests to all configured remote agents
3. Remote XML responses are parsed to extract content between key tags (`<Devices>` or `<Streams>`)
4. Extracted content is aggregated into a single XML response
5. Device data is parsed and stored in PostgreSQL (machine_status, machine_data_history)
6. Combined XML is returned to the client

### Remote Agent Configuration

The server aggregates data from 9 remote MTConnect agents on 192.168.10.5 with different ports:

| Device Name | URL | Port |
|-------------|-----|------|
| HCN6800 | http://192.168.10.5:5000 | 5000 |
| QT350-1 | http://192.168.10.5:5001 | 5001 |
| QT350-2 | http://192.168.10.5:5002 | 5002 |
| QT350-3 | http://192.168.10.5:5003 | 5003 |
| QT350-4 | http://192.168.10.5:5004 | 5004 |
| MNT600-1 | http://192.168.10.5:5005 | 5005 |
| MNT600S-1 | http://192.168.10.5:5006 | 5006 |
| MNT600-2 | http://192.168.10.5:5007 | 5007 |
| MNT600S-2 | http://192.168.10.5:5008 | 5008 |

These are defined in `MTConnectAgent::new()` in src/mtconnect.rs.

### Database Storage

Data is stored in PostgreSQL using the `elfin-postgres-data-access` library:

**Tables:**
- `machine_status`: Real-time machine status (updated on each /current request)
- `machine_data_history`: Historical data for analysis

**Configuration (.env):**
```
DB_HOST=localhost
DB_PORT=5432
DB_NAME=focas_db
DB_USER=focas_user
DB_PASSWORD=focas_password
```

### Async Architecture

- Uses Tokio runtime for async operations
- Parallel fetching: `fetch_probe_from_all()` and `fetch_current_from_all()` spawn separate tasks for each remote agent
- HTTP client is shared via `Arc<Client>` across all requests
- 5-second timeout for remote agent requests

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the server (binds to 127.0.0.1:5000)
cargo run --bin mtconnect-server

# Run with release optimizations
cargo run --release --bin mtconnect-server

# Run the TUI monitor to check remote agent status
cargo run --bin monitor
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Quality
```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Lint with clippy
cargo clippy
```

## Key Dependencies

- **hyper 1.0**: HTTP server framework (with hyper-util for server/client utilities)
- **tokio**: Async runtime with full features
- **http-body-util**: HTTP body utilities for request/response handling
- **chrono**: DateTime handling for MTConnect timestamps
- **uuid**: UUID generation for device and instance IDs
- **dotenv**: Environment variable loading for database configuration
- **elfin-postgres-data-access**: PostgreSQL database operations
- **ratatui**: TUI library for the monitoring tool
- **crossterm**: Cross-platform terminal manipulation

## Testing the Server

```bash
# Start the server
cargo run --bin mtconnect-server

# In another terminal, test endpoints:
curl http://127.0.0.1:5000/
curl http://127.0.0.1:5000/probe
curl http://127.0.0.1:5000/current
curl http://127.0.0.1:5000/sample

# Check database storage
psql -h localhost -U focas_user -d focas_db -c "SELECT machine_id, status, mode FROM machine_status;"
```

## Monitoring Tool

### monitor.rs
A standalone TUI application for monitoring remote MTConnect agent connections. Features:
- Real-time connection status for all 9 configured remote agents
- Displays device name and URL for each agent
- Parses MTConnect XML to extract detailed device information:
  - Device name, UUID
  - Availability, Execution state, Controller mode
  - Emergency stop status
  - Program name and subprogram
  - Part count
  - Spindle load and override
  - Feed/rapid overrides and actual feedrate
  - X-axis and Z-axis loads
- Color-coded indicators:
  - Status: green=good, red=bad, yellow=checking
  - Load: green (<50%), yellow (50-80%), red (>80%)
- Auto-refreshes every 3 seconds with 2-second timeout per request
- Interactive navigation with arrow keys

Run with: `cargo run --bin monitor`

## Data Access for Other Services

Other services can access the collected data through:

### 1. HTTP API (MTConnect XML)
```bash
curl http://127.0.0.1:5000/current
curl http://127.0.0.1:5000/probe
```

### 2. PostgreSQL Direct Access
```sql
-- Real-time status
SELECT * FROM machine_status WHERE machine_id = 'HCN6800';

-- Historical data
SELECT * FROM machine_data_history
WHERE machine_id = 'QT350-1'
ORDER BY timestamp DESC LIMIT 100;
```

### 3. elfin-postgres-data-access Library (Rust)
```rust
use elfin_postgres_data_access::DatabaseService;

let db = DatabaseService::from_env().await?;
let status = db.get_machine_status("HCN6800").await?;
```

## Important Implementation Details

- `/sample` endpoint currently returns the same data as `/current`
- XML parsing uses simple string operations (find/substring) rather than XML parser
- Failed remote agent requests are silently skipped
- Database storage happens on each `/current` request
- Device names (HCN6800, QT350-1, etc.) are used as machine_id in database
