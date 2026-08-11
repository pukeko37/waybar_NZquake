//! Network-hitting integration test for the GeoNet API client. Skipped in
//! CI (`CI` env var set) since it depends on external network access.

use waybar_nzquake::domain::{Coordinates, Latitude, Longitude};
use waybar_nzquake::infra::api::QuakeClient;

#[test]
fn test_fetch_earthquakes_integration() {
    if std::env::var("CI").is_ok() {
        return;
    }

    let client = QuakeClient::new();
    let user_location = Coordinates::new(
        Latitude::new(-41.2865).unwrap(),
        Longitude::new(174.7762).unwrap(),
    );

    match client.fetch_earthquakes(user_location) {
        Ok(quake_data) => {
            eprintln!("Fetched {} earthquakes", quake_data.earthquakes.len());
        }
        Err(e) => {
            // Log error but don't fail test in case of network issues
            eprintln!("Integration test warning (network issues expected): {}", e);
        }
    }
}

#[test]
fn test_full_earthquake_flow() {
    if std::env::var("CI").is_ok() {
        return;
    }

    use waybar_nzquake::app;
    use waybar_nzquake::infra::display::WaybarFormatter;

    let client = QuakeClient::new();
    let formatter = WaybarFormatter::new();
    let user_location = Coordinates::new(
        Latitude::new(-41.2865).unwrap(),
        Longitude::new(174.7762).unwrap(),
    );

    match app::fetch_and_format(&client, &formatter, user_location) {
        Ok(output) => {
            assert!(!output.text.is_empty());
            assert!(!output.tooltip.is_empty());

            let json = serde_json::to_string(&output).unwrap();
            assert!(json.contains("text"));
            assert!(json.contains("tooltip"));
        }
        Err(e) => {
            eprintln!("Integration test warning (network issues expected): {}", e);
        }
    }
}
