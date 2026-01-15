# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based MTConnect aggregation server that acts as a proxy, fetching data from multiple remote MTConnect agents and aggregating their responses. The server implements MTConnect 1.8 protocol and provides standard MTConnect endpoints.

## Architecture

### Core Components

- **main.rs**: HTTP server using Hyper 1.0 with async/await. Each request spawns a new task that handles the HTTP connection and routes to appropriate endpoints (/, /probe, /current, /sample).

- **mtconnect.rs**: MTConnect agent implementation with two key responsibilities:
  1. Generate local MTConnect XML responses (probe, current, sample)
  2. Fetch and aggregate XML data from remote agents in parallel using tokio::spawn

### Data Flow

1. Client requests arrive at main.rs endpoints
2. `MTConnectAgent` makes parallel HTTP requests to all configured remote agents (192.168.20.x IPs)
3. Remote XML responses are parsed to extract content between key tags (`<Devices>` or `<Streams>`)
4. Extracted content is aggregated into a single XML response
5. Combined XML is returned to the client

### Remote Agent Configuration

The server aggregates data from 9 hardcoded remote MTConnect agents on port 5000:
- 192.168.20.10, 192.168.20.20, 192.168.20.30, 192.168.20.40, 192.168.20.50
- 192.168.20.60, 192.168.20.70, 192.168.20.80, 192.168.20.100

These IPs are defined in `MTConnectAgent::new()` in src/mtconnect.rs:44-54.

### Async Architecture

- Uses Tokio runtime for async operations
- Parallel fetching: `fetch_probe_from_all()` and `fetch_current_from_all()` spawn separate tasks for each remote agent
- HTTP client is shared via `Arc<Client>` across all requests
- No response timeout configured (requests wait indefinitely)

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the server (binds to 127.0.0.1:5000)
cargo run

# Run with release optimizations
cargo run --release

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
- **serde + serde-xml-rs**: XML serialization (currently not actively used in main code)
- **ratatui**: TUI library for the monitoring tool
- **crossterm**: Cross-platform terminal manipulation

## Testing the Server

```bash
# Start the server
cargo run

# In another terminal, test endpoints:
curl http://127.0.0.1:5000/
curl http://127.0.0.1:5000/probe
curl http://127.0.0.1:5000/current
curl http://127.0.0.1:5000/sample
```

## Monitoring Tool

### monitor.rs
A standalone TUI application for monitoring remote MTConnect agent connections. Features:
- Real-time connection status for all 9 configured remote agents
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

XML Parsing:
- Uses custom string parsing functions (`extract_between`, `extract_attribute`)
- Extracts data from `<DeviceStream>` elements and nested data items
- Searches for specific MTConnect data item names (Sload, Fovr, etc.)
- Handles multiple devices per agent

Run with: `cargo run --bin monitor`

## Important Implementation Details

- `/sample` endpoint currently returns the same data as `/current` (see main.rs:43-50)
- XML parsing uses simple string operations (find/substring) rather than XML parser
- No error recovery for failed remote agent requests (failed fetches are silently skipped)
- The server maintains a `cached_data` HashMap but it's currently unused
- Clone is cheap for `MTConnectAgent` because it uses Arc for the HTTP client
- Monitor tool spawns separate async tasks for each remote agent to fetch data in parallel
