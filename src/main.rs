//! Earthquake application with domain-driven design and type safety.
//! Fetches earthquake data from GeoNet API and outputs JSON for Waybar.

mod api;
mod display;

use anyhow::{Context, Result};
use api::QuakeClient;
use display::WaybarFormatter;

fn main() -> Result<()> {
    // Parse command line argument: latitude,longitude (defaults to Wellington if not provided)
    let location = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "-41.2865,174.7762".to_string());

    let (latitude, longitude) = parse_location(&location)?;

    let client = QuakeClient::new();
    let formatter = WaybarFormatter::new();

    match client.fetch_earthquakes(latitude, longitude) {
        Ok(mut quake_data) => {
            // Score and filter earthquakes based on proximity, recency, and intensity
            quake_data.score_and_filter();

            let output = formatter.format(&quake_data)?;
            println!("{}", serde_json::to_string(&output)?);
        }
        Err(e) => {
            let error_output = WaybarFormatter::create_error_output(e);
            println!("{}", serde_json::to_string(&error_output)?);
        }
    }

    Ok(())
}

/// Parse location string in format "latitude,longitude"
fn parse_location(location: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = location.split(',').collect();

    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid location format. Expected 'latitude,longitude', got '{}'",
            location
        );
    }

    let latitude = parts[0]
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Invalid latitude: {}", parts[0]))?;

    let longitude = parts[1]
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Invalid longitude: {}", parts[1]))?;

    // Basic validation for NZ coordinates
    if !(-48.0..=-34.0).contains(&latitude) {
        eprintln!(
            "Warning: Latitude {} is outside typical NZ range (-48 to -34)",
            latitude
        );
    }

    if !(166.0..=179.0).contains(&longitude) {
        eprintln!(
            "Warning: Longitude {} is outside typical NZ range (166 to 179)",
            longitude
        );
    }

    Ok((latitude, longitude))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_parse_location() {
        let (lat, lon) = parse_location("-41.2865,174.7762").unwrap();
        assert!((lat - (-41.2865)).abs() < 0.0001);
        assert!((lon - 174.7762).abs() < 0.0001);

        // Test with spaces
        let (lat, lon) = parse_location("-41.2865, 174.7762").unwrap();
        assert!((lat - (-41.2865)).abs() < 0.0001);
        assert!((lon - 174.7762).abs() < 0.0001);

        // Test invalid format
        assert!(parse_location("invalid").is_err());
        assert!(parse_location("-41.2865").is_err());
        assert!(parse_location("-41.2865,abc").is_err());
    }

    #[test]
    fn test_full_earthquake_flow() {
        // Skip in CI environments
        if std::env::var("CI").is_ok() {
            return;
        }

        let client = QuakeClient::new();
        let formatter = WaybarFormatter::new();

        // Wellington coordinates
        match client.fetch_earthquakes(-41.2865, 174.7762) {
            Ok(mut quake_data) => {
                quake_data.score_and_filter();

                // Test formatting
                let output = formatter.format(&quake_data).unwrap();
                assert!(!output.text.is_empty());
                assert!(!output.tooltip.is_empty());

                // Validate JSON serialization
                let json = serde_json::to_string(&output).unwrap();
                assert!(json.contains("text"));
                assert!(json.contains("tooltip"));
            }
            Err(e) => {
                eprintln!("Integration test warning (network issues expected): {}", e);
            }
        }
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
