# MTConnect Server in Rust

A lightweight MTConnect server implementation using Rust and Hyper.

## Features

- MTConnect 1.8 compliant
- HTTP server using Hyper
- Fetches data from multiple remote MTConnect agents
- Standard MTConnect endpoints:
  - `/probe` - Device probe response (aggregated from all remote agents)
  - `/current` - Current data values (aggregated from all remote agents)
  - `/sample` - Sample data stream (aggregated from all remote agents)

## Remote Agents

The server fetches data from the following IP addresses:
- 192.168.20.10
- 192.168.20.20
- 192.168.20.30
- 192.168.20.40
- 192.168.20.50
- 192.168.20.60
- 192.168.20.70
- 192.168.20.80
- 192.168.20.100

Each endpoint aggregates data from all configured remote agents running MTConnect servers on port 5000.

## Quick Start

```bash
# Build and run the server
cargo run

# The server will start on http://127.0.0.1:5000

# Run the TUI monitor to check agent connections
cargo run --bin monitor
```

## Endpoints

- `GET /` - Server info page
- `GET /probe` - MTConnect device probe (XML)
- `GET /current` - Current device data (XML)
- `GET /sample` - Sample device data (XML)

## Example Usage

```bash
# Get device probe
curl http://127.0.0.1:5000/probe

# Get current data
curl http://127.0.0.1:5000/current

# Get sample data
curl http://127.0.0.1:5000/sample
```

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
- Controller Mode: AUTOMATIC/MANUAL
- Emergency Stop: ARMED/TRIGGERED

**Production Info:**
- Program: Current main program (e.g., QT-MAIN)
- Subprogram: Current subprogram number (e.g., 3600)
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
- ● Green = Connected and receiving data
- ✗ Red = Connection failed
- ○ Yellow = Checking connection

**Controls:**
- `↑/↓` - Navigate between agents
- `q` - Quit the monitor

**Auto-refresh:** Every 3 seconds

## Dependencies

- `hyper` - HTTP server framework
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `uuid` - UUID generation