# MTConnect Server in Rust

A lightweight MTConnect aggregation server implementation using Rust and Hyper. Collects data from multiple CNC machines and stores it in PostgreSQL.

## Features

- MTConnect 1.8 compliant
- HTTP server using Hyper
- Fetches data from multiple remote MTConnect agents in parallel
- PostgreSQL database storage for real-time status and historical data
- Standard MTConnect endpoints:
  - `/probe` - Device probe response (aggregated from all remote agents)
  - `/current` - Current data values (aggregated from all remote agents)
  - `/sample` - Sample data stream (aggregated from all remote agents)

## Connected Machines

| Machine Name | URL | Description |
|--------------|-----|-------------|
| HCN6800 | http://192.168.10.5:5000 | Horizontal Machining Center |
| QT350-1 | http://192.168.10.5:5001 | CNC Lathe |
| QT350-2 | http://192.168.10.5:5002 | CNC Lathe |
| QT350-3 | http://192.168.10.5:5003 | CNC Lathe |
| QT350-4 | http://192.168.10.5:5004 | CNC Lathe |
| MNT600-1 | http://192.168.10.5:5005 | Multi-Tasking Machine |
| MNT600S-1 | http://192.168.10.5:5006 | Multi-Tasking Machine |
| MNT600-2 | http://192.168.10.5:5007 | Multi-Tasking Machine |
| MNT600S-2 | http://192.168.10.5:5008 | Multi-Tasking Machine |

## Quick Start

```bash
# Build and run the server
cargo run --release --bin mtconnect-server

# The server will start on http://127.0.0.1:5000

# Run the TUI monitor to check agent connections
cargo run --bin monitor
```

## Environment Configuration

Create a `.env` file in the project root:

```env
DB_HOST=localhost
DB_PORT=5432
DB_NAME=focas_db
DB_USER=focas_user
DB_PASSWORD=focas_password
```

## Endpoints

- `GET /` - Server info page
- `GET /probe` - MTConnect device probe (XML)
- `GET /current` - Current device data (XML) + saves to database
- `GET /sample` - Sample device data (XML)

## Example Usage

```bash
# Get device probe
curl http://127.0.0.1:5000/probe

# Get current data (also saves to database)
curl http://127.0.0.1:5000/current

# Get sample data
curl http://127.0.0.1:5000/sample
```

## Database Access

Data is stored in PostgreSQL for other services to consume:

```sql
-- Real-time machine status
SELECT machine_id, status, mode, part_count
FROM machine_status
ORDER BY timestamp DESC;

-- Historical data for analysis
SELECT machine_id, timestamp, status, mode, spindle_load
FROM machine_data_history
WHERE machine_id = 'QT350-1'
  AND timestamp > EXTRACT(EPOCH FROM NOW() - INTERVAL '1 hour') * 1000
ORDER BY timestamp DESC;
```

### Available Tables

| Table | Description |
|-------|-------------|
| machine_status | Current status of each machine (updated in real-time) |
| machine_data_history | Historical data for trend analysis |

### Data Fields

| Field | Description |
|-------|-------------|
| machine_id | Machine name (HCN6800, QT350-1, etc.) |
| status | Execution status (ACTIVE, READY, INTERRUPTED, etc.) |
| mode | Controller mode (AUTOMATIC, MANUAL, MDI, etc.) |
| part_count | Parts produced |
| spindle_load | Spindle load percentage |
| spindle_override | Spindle override percentage |
| feed_override | Feed override percentage |

## Monitoring Tool

A TUI (Terminal User Interface) monitor is included to check the status of all remote MTConnect agents:

```bash
# Run the monitor
cargo run --bin monitor

# Or run the release version (faster)
cargo run --bin monitor --release
```

The monitor displays real-time data from each device:

**Device Status:**
- Availability: AVAILABLE/UNAVAILABLE
- Execution: ACTIVE/READY/INTERRUPTED/STOPPED
- Controller Mode: AUTOMATIC/MANUAL/MDI
- Emergency Stop: ARMED/TRIGGERED

**Production Info:**
- Program: Current main program
- Subprogram: Current subprogram number
- Part Count: Parts produced

**Spindle & Feed:**
- Spindle Load: 0-100% (color-coded: green <50%, yellow 50-80%, red >80%)
- Spindle Override: 0-100%
- Feed Override: 0-100%
- Rapid Override: 0-100%
- Feedrate (Actual): Real-time feedrate value

**Axis Loads:**
- X-Axis Load: 0-100% (color-coded)
- Z-Axis Load: 0-100% (color-coded)

**Connection Status:**
- Green = Connected and receiving data
- Red = Connection failed
- Yellow = Checking connection

**Controls:**
- Up/Down arrows - Navigate between agents
- `q` - Quit the monitor

**Auto-refresh:** Every 3 seconds

## Architecture

```
Remote MTConnect Agents          MTConnect Server              Database
(192.168.10.5:5000-5008)        (127.0.0.1:5000)             (PostgreSQL)

  HCN6800   ──┐
  QT350-1   ──┤                 ┌─────────────┐          ┌──────────────┐
  QT350-2   ──┤   HTTP/XML      │             │   SQL    │              │
  QT350-3   ──┼────────────────>│  Aggregator │─────────>│ machine_     │
  QT350-4   ──┤                 │   Server    │          │ status       │
  MNT600-1  ──┤                 │             │          │ machine_     │
  MNT600S-1 ──┤                 └─────────────┘          │ data_history │
  MNT600-2  ──┤                       │                  └──────────────┘
  MNT600S-2 ──┘                       │
                                      ▼
                              Other Services
                              (Web Dashboard,
                               Analytics, etc.)
```

## Dependencies

- `hyper` - HTTP server framework
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `uuid` - UUID generation
- `dotenv` - Environment configuration
- `elfin-postgres-data-access` - PostgreSQL operations
- `ratatui` - TUI framework
- `crossterm` - Terminal manipulation
