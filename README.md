# waybar_nzquake - Waybar NZ Earthquake Widget

A Rust-based earthquake data fetcher for Waybar that retrieves earthquake information from the GeoNet API and outputs Waybar-compatible JSON format.

## Features

- Fetches earthquake data from GeoNet API (api.geonet.org.nz)
- Outputs Waybar-compatible JSON format with text and tooltip
- Displays recent New Zealand earthquakes
- Comprehensive earthquake information including:
  - Magnitude and depth
  - Location and distance from major cities
  - Time of occurrence
  - Earthquake intensity
- Robust error handling with informative messages
- No API key required (uses public GeoNet API)

## Prerequisites

No API keys or authentication required. The GeoNet API is a public service provided by GNS Science for New Zealand earthquake data.

## Installation

### For Nix Users

This project provides a Nix flake for reproducible builds and easy integration with NixOS.

#### Quick Start with Nix

```bash
# Run directly from GitHub
nix run github:pukeko37/waybar_nzquake

# Build locally
nix build

# The binary will be available at ./result/bin/waybar_nzquake
./result/bin/waybar_nzquake
```

#### Add to NixOS Configuration

Add this flake as an input in your NixOS configuration:

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    waybar-nzquake.url = "github:pukeko37/waybar_nzquake";
  };

  outputs = { self, nixpkgs, waybar-nzquake, ... }: {
    nixosConfigurations.yourhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        {
          environment.systemPackages = [
            waybar-nzquake.packages.x86_64-linux.default
          ];
        }
      ];
    };
  };
}
```

#### Use in Home Manager

```nix
# home.nix
{ inputs, pkgs, ... }: {
  home.packages = [
    inputs.waybar-nzquake.packages.${pkgs.system}.default
  ];
}
```

#### Development Shell

Enter a development environment with all required tools:

```bash
nix develop

# Now you have cargo, rust-analyzer, and other tools available
cargo build
cargo test
```

### Building with Cargo

```bash
cargo build --release
```

The binary will be available at `target/release/waybar_nzquake`.

## Usage

The program requires a location parameter in latitude,longitude format:

```bash
# Wellington, New Zealand
./target/release/waybar_nzquake "-41.2865,174.7762"

# Auckland, New Zealand
./target/release/waybar_nzquake "-36.8485,174.7633"

# Christchurch, New Zealand
./target/release/waybar_nzquake "-43.5321,172.6362"
```

The location is used to calculate distance and direction from each earthquake, and to prioritize earthquakes that are:
- More recent (exponential decay: 1, 2, 4, 8 days)
- Closer to your location (exponential decay: 20, 40, 80, 160, 320 km using 3D distance including depth)
- Higher magnitude (weighted ×2 for significance, so lower magnitude events score proportionally lower)
- Higher intensity (MMI scale: 9, 8, 7, 6, 5, 4, 3)

Only earthquakes M3.0 and above are shown. The top 8 earthquakes are displayed, grouped by day bands and sorted by relevance score within each band. The highest scoring earthquake is highlighted in green.

## Output Format

The program outputs JSON in the format expected by Waybar:

```json
{
  "text": "🌍 M4.0 369km SW (8 quakes)",
  "tooltip": "Recent NZ Earthquakes (by day band):\n\n⏰ Last 24 hours:\n  📅 2025-11-21 06:50:52 | M4.0 | 📏 24.6km | SW 369km | MMI 4\n..."
}
```

### Text Format
- Earthquake icon 🌍
- Magnitude of highest-scored recent earthquake
- Distance and direction from your location
- Total count of earthquakes shown

### Tooltip Information
Grouped by day bands (Last 24 hours, 1-2 days ago, 2-4 days ago, 4-8 days ago, 8+ days ago), each earthquake shows:
- 📅 Date and time (ISO 8601 format)
- Magnitude (Richter scale)
- 📏 Depth (kilometers)
- 🎯 Quality (only shown if not "best" - preliminary, automatic, etc.)
- Direction and distance from your location (horizontal surface distance)
- MMI (Modified Mercalli Intensity)
- 🕐 Last updated timestamp
- The highest scoring earthquake is highlighted in green

## Error Handling

If the earthquake data cannot be fetched, the program outputs an error message in Waybar format:

```json
{
  "text": "🌍 -- Earthquake data unavailable",
  "tooltip": "Unable to fetch earthquake data\n\nError: [error details]\nService: GeoNet API\n\nLast attempt: [timestamp]"
}
```

Common errors:
- Network connectivity issues
- GeoNet API service unavailable

## Dependencies

- `ureq` - Synchronous HTTP client with JSON support
- `serde` and `serde_json` - JSON serialization/deserialization
- `time` - Date and time handling
- `anyhow` - Error handling with context

## Waybar Configuration

To use waybar_nzquake in your Waybar setup:

1. Build the release binary (or install via Nix)
2. Configure your Waybar to use the binary

Example Waybar config (replace coordinates with your location):

```json
{
    "custom/earthquake": {
        "format": "{}",
        "exec": "/path/to/waybar_nzquake/target/release/waybar_nzquake '-41.2865,174.7762'",
        "interval": 300,
        "return-type": "json"
    }
}
```

Or for Nix users with the package installed:

```json
{
    "custom/earthquake": {
        "format": "{}",
        "exec": "waybar_nzquake '-41.2865,174.7762'",
        "interval": 300,
        "return-type": "json"
    }
}
```

**Note:** Replace `-41.2865,174.7762` with your actual latitude,longitude. Common NZ locations:
- Wellington: `-41.2865,174.7762`
- Auckland: `-36.8485,174.7633`
- Christchurch: `-43.5321,172.6362`
- Dunedin: `-45.8788,170.5028`

## Testing

Run the test suite with:

```bash
cargo test
```

Tests include:
- API response parsing
- Domain model validation
- Mock data processing
- Integration test (requires internet connection)

## GeoNet API

This application uses the GeoNet Quake API:
- Endpoint: https://api.geonet.org.nz/
- Public API (no authentication required)
- Provides real-time earthquake data for New Zealand
- Maintained by GNS Science

## Performance

- Binary size: ~2MB (optimized release build)
- No caching mechanism (fetches fresh data each time)
- 10-second timeout for API requests
- Synchronous HTTP client for simplicity and smaller binary size
- Minimal memory usage and fast execution
- Type-safe domain modeling with zero-cost abstractions

## About GeoNet

GeoNet is New Zealand's official source for geological hazard information. The service is a collaboration between the Earthquake Commission (EQC), GNS Science, and Land Information New Zealand (LINZ).

## License

MIT
