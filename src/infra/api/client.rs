//! HTTP client for fetching earthquake data from the GeoNet API.

use crate::app::QuakeFetcher;
use crate::domain::{Coordinates, Earthquake, QuakeData};
use crate::infra::api::models::QuakeApiResponse;
use anyhow::{Context, Result};
use std::time::Duration;

/// Earthquake API client for the GeoNet service.
pub struct QuakeClient {
    agent: ureq::Agent,
    base_url: String,
}

impl QuakeClient {
    /// Create a new earthquake client. No API key required — GeoNet is a
    /// public service.
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();

        Self {
            agent,
            base_url: "https://api.geonet.org.nz".to_string(),
        }
    }

    /// Fetch recent earthquake data (MMI >= 3) for a given user location.
    pub fn fetch_earthquakes(&self, user_location: Coordinates) -> Result<QuakeData> {
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

        // Any single malformed feature fails the whole fetch, rather than
        // being silently dropped or given a fallback value — see
        // QuakeError/closed-domain-error-with-boundary-anyhow.
        let earthquakes = api_response
            .features
            .into_iter()
            .map(Earthquake::try_from)
            .collect::<Result<Vec<_>>>()
            .context("Failed to parse earthquake feature")?;

        Ok(QuakeData {
            earthquakes,
            user_location,
        })
    }
}

impl Default for QuakeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl QuakeFetcher for QuakeClient {
    fn fetch_earthquakes(&self, location: Coordinates) -> Result<QuakeData, anyhow::Error> {
        self.fetch_earthquakes(location)
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
}
