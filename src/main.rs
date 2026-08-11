//! Earthquake application with domain-driven design and type safety.
//! Fetches earthquake data from GeoNet API and outputs JSON for Waybar.
//!
//! This file is the composition root: it constructs concrete types and
//! delegates to the application layer.

use anyhow::{Context, Result};
use waybar_nzquake::app;
use waybar_nzquake::domain::{Coordinates, Latitude, Longitude};
use waybar_nzquake::infra::api::QuakeClient;
use waybar_nzquake::infra::display::WaybarFormatter;

fn main() -> Result<()> {
    // Parse command line argument: latitude,longitude (defaults to Wellington if not provided)
    let location = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "-41.2865,174.7762".to_string());

    // A malformed location must never produce a non-zero exit — same
    // always-valid-JSON-on-every-invocation rule every other failure path
    // here follows, per the Waybar custom-module contract.
    let coordinates = match parse_location(&location) {
        Ok(coordinates) => coordinates,
        Err(e) => {
            let error_output = WaybarFormatter::create_error_output(e);
            println!("{}", serde_json::to_string(&error_output)?);
            return Ok(());
        }
    };

    let client = QuakeClient::new();
    let formatter = WaybarFormatter::new();

    match app::fetch_and_format(&client, &formatter, coordinates) {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output)?);
        }
        Err(e) => {
            let error_output = WaybarFormatter::create_error_output(e);
            println!("{}", serde_json::to_string(&error_output)?);
        }
    }

    Ok(())
}

/// Parse location string in format "latitude,longitude" into validated
/// `Coordinates`. Out-of-typical-NZ-range coordinates only warn (still used)
/// — only a malformed string or a globally impossible coordinate is fatal.
fn parse_location(location: &str) -> Result<Coordinates> {
    let parts: Vec<&str> = location.split(',').collect();

    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid location format. Expected 'latitude,longitude', got '{}'",
            location
        );
    }

    let latitude_value = parts[0]
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Invalid latitude: {}", parts[0]))?;

    let longitude_value = parts[1]
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Invalid longitude: {}", parts[1]))?;

    // Basic warning for NZ coordinates — non-fatal, unlike the global range
    // validation Latitude/Longitude enforce below.
    if !(-48.0..=-34.0).contains(&latitude_value) {
        eprintln!(
            "Warning: Latitude {} is outside typical NZ range (-48 to -34)",
            latitude_value
        );
    }

    if !(166.0..=179.0).contains(&longitude_value) {
        eprintln!(
            "Warning: Longitude {} is outside typical NZ range (166 to 179)",
            longitude_value
        );
    }

    let latitude = Latitude::new(latitude_value).context("Latitude out of range")?;
    let longitude = Longitude::new(longitude_value).context("Longitude out of range")?;

    Ok(Coordinates::new(latitude, longitude))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location() {
        let coordinates = parse_location("-41.2865,174.7762").unwrap();
        assert!((coordinates.latitude.value() - (-41.2865)).abs() < 0.0001);
        assert!((coordinates.longitude.value() - 174.7762).abs() < 0.0001);

        // Test with spaces
        let coordinates = parse_location("-41.2865, 174.7762").unwrap();
        assert!((coordinates.latitude.value() - (-41.2865)).abs() < 0.0001);
        assert!((coordinates.longitude.value() - 174.7762).abs() < 0.0001);

        // Test invalid format
        assert!(parse_location("invalid").is_err());
        assert!(parse_location("-41.2865").is_err());
        assert!(parse_location("-41.2865,abc").is_err());
    }

    #[test]
    fn test_parse_location_rejects_globally_impossible_coordinates() {
        assert!(parse_location("-95.0,174.7762").is_err());
        assert!(parse_location("-41.2865,185.0").is_err());
    }

    #[test]
    fn test_error_handling_flow() {
        let error = anyhow::anyhow!("Test error");
        let error_output = WaybarFormatter::create_error_output(error);

        assert!(error_output.text.contains("unavailable"));
        assert!(error_output.tooltip.contains("Test error"));

        // Validate JSON serialization
        let json = serde_json::to_string(&error_output).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("tooltip"));
    }
}
