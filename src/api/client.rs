//! HTTP client for fetching earthquake data from GeoNet API.

use crate::api::models::{Earthquake, QuakeApiResponse, QuakeData};
use anyhow::{Context, Result};
use std::time::Duration;

/// Earthquake API client for GeoNet service
pub struct QuakeClient {
    agent: ureq::Agent,
    base_url: String,
}

impl QuakeClient {
    /// Create a new earthquake client
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();

        Self {
            agent,
            base_url: "https://api.geonet.org.nz".to_string(),
        }
    }

    /// Fetch recent earthquake data for a given user location
    ///
    /// user_lat: User's latitude
    /// user_lon: User's longitude
    pub fn fetch_earthquakes(&self, user_lat: f64, user_lon: f64) -> Result<QuakeData> {
        // GeoNet API endpoint for quakes with MMI >= 3
        let url = format!("{}/quake?MMI=3", self.base_url);

        let response = self
            .agent
            .get(&url)
            .call()
            .with_context(|| format!("Failed to send request to: {}", url))?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.into_string().unwrap_or_default();
            anyhow::bail!(
                "GeoNet API returned error status {}: {}. Response: {}",
                status,
                url,
                error_text
            );
        }

        let api_response: QuakeApiResponse = response
            .into_json()
            .context("Failed to parse JSON response from GeoNet API")?;

        // Convert API response to domain model with user location
        let earthquakes = api_response
            .features
            .into_iter()
            .map(|feature| Earthquake {
                time: feature.properties.time,
                magnitude: feature.properties.magnitude,
                depth: feature.properties.depth,
                quality: feature.properties.quality,
                mmi: feature.properties.mmi,
                longitude: feature.geometry.coordinates.first().copied().unwrap_or(0.0),
                latitude: feature.geometry.coordinates.get(1).copied().unwrap_or(0.0),
                score: 0.0,
            })
            .collect();

        Ok(QuakeData {
            earthquakes,
            user_location: (user_lat, user_lon),
        })
    }
}

impl Default for QuakeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = QuakeClient::new();
        assert_eq!(client.base_url, "https://api.geonet.org.nz");
    }

    #[test]
    fn test_fetch_earthquakes_integration() {
        // Integration test - only runs when not in CI
        if std::env::var("CI").is_ok() {
            return;
        }

        let client = QuakeClient::new();

        // Wellington coordinates
        match client.fetch_earthquakes(-41.2865, 174.7762) {
            Ok(quake_data) => {
                // Basic validation that we got earthquake data
                eprintln!("Fetched {} earthquakes", quake_data.earthquakes.len());
                assert_eq!(quake_data.user_location, (-41.2865, 174.7762));
            }
            Err(e) => {
                // Log error but don't fail test in case of network issues
                eprintln!("Integration test warning (network issues expected): {}", e);
            }
        }
    }
}
