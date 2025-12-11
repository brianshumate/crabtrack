# CLAUDE.md

**Note**: This project uses [bd (beads)](https://github.com/steveyegge/beads)
for issue tracking. Use `bd` commands instead of markdown TODOs.
See AGENTS.md for workflow details.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Run Commands

```bash
# Build release binary
cargo build --release

# Run the application
cargo run --release

# Check for errors without building
cargo check

# Run clippy for lints
cargo clippy

# Format code
cargo fmt
```

## Project Overview

CrabTrack is a TUI-based satellite tracker written in Rust. It uses Two Line Element (TLE) data to track satellites, predict passes, and calculate communication windows for amateur radio operators.

## Architecture

### Core Modules

- **main.rs** - Application entry point, TLE parsing, pass prediction loop, terminal setup, and event handling
- **satellite.rs** - `Satellite` struct with SGP4 propagation, position calculation (ECI to geodetic conversion)
- **observer.rs** - `Observer` struct representing ground station location with ECEF coordinate conversion
- **pass_prediction.rs** - Pass prediction algorithms, look angle calculations (azimuth/elevation/range), GMST calculation
- **radio.rs** - Doppler shift calculation, communication window evaluation, signal strength estimation
- **config.rs** - TOML configuration parsing with serde
- **ui.rs** - Ratatui-based TUI rendering (header, alerts, radio info, position tables, sky map, satellite details)

### Key Dependencies

- **sgp4** - SGP4/SDP4 orbital propagation
- **ratatui/crossterm** - Terminal UI framework
- **chrono/hifitime** - Time handling
- **nalgebra** - Vector math for coordinate transforms
- **serde/toml** - Configuration parsing

### Data Flow

1. Load `config.toml` for observer location and tracking preferences
2. Parse TLE file (`satellites.tle`) into `Elements` structs
3. Predict passes for each satellite using SGP4 propagation
4. Main loop: update positions every refresh cycle, render TUI, handle keyboard input

### Configuration

Configuration is TOML-based (`config.toml`). Key sections:
- `[observer]` - Ground station coordinates
- `[satellites]` - TLE file path, tracked satellite names, max count
- `[prediction]` - Pass prediction parameters (min elevation, search days)
- `[display]` - UI options (refresh rate, sky map toggle)
- `[radio]` - Doppler/communication window settings
- `[alerts]` - Pass alert thresholds

### Coordinate Systems

The code transforms between multiple coordinate systems:
- **TLE epoch** - Time reference from TLE data
- **ECI (Earth-Centered Inertial)** - SGP4 output coordinates
- **ECEF (Earth-Centered Earth-Fixed)** - Rotating with Earth
- **Geodetic** - Latitude/longitude/altitude
- **Topocentric (SEZ)** - South-East-Zenith from observer for look angles
