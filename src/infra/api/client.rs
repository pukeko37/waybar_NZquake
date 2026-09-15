//! HTTP client for fetching earthquake data from the GeoNet API.

use crate::app::QuakeFetcher;
use crate::domain::{Coordinates, Earthquake, QuakeData};
use crate::infra::api::models::{QuakeApiResponse, QuakeFeatureApi};
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

        let earthquakes = parse_features(api_response.features)?;

        Ok(QuakeData {
            earthquakes,
            user_location,
        })
    }
}

/// Convert a batch of GeoNet features into `Earthquake`s.
///
/// Any single malformed feature fails the whole batch, rather than being
/// silently dropped or given a fallback value — see
/// QuakeError/closed-domain-error-with-boundary-anyhow. `deleted` features
/// are the one exception: their attached values are leftovers from a
/// since-invalidated detection, not validated observations, so they're
/// dropped before domain construction rather than being required to
/// satisfy it — see deleted-quake-validation-bypass.
fn parse_features(features: Vec<QuakeFeatureApi>) -> Result<Vec<Earthquake>> {
    features
        .into_iter()
        .filter(|feature| feature.properties.quality != "deleted")
        .map(Earthquake::try_from)
        .collect::<Result<Vec<_>>>()
        .context("Failed to parse earthquake feature")
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

    fn feature(quality: &str, depth: f64) -> QuakeFeatureApi {
        serde_json::from_value(serde_json::json!({
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 5.2,
                "depth": depth,
                "quality": quality,
                "mmi": null
            },
            "geometry": {
                "coordinates": [174.7762, -41.2865]
            }
        }))
        .expect("valid feature fixture")
    }

    #[test]
    fn test_deleted_feature_with_invalid_depth_is_dropped_not_fatal() {
        let features = vec![feature("best", 12.0), feature("deleted", 750.0)];

        let earthquakes = parse_features(features).expect("deleted feature must not fail batch");

        assert_eq!(earthquakes.len(), 1);
    }

    #[test]
    fn test_non_deleted_feature_with_invalid_depth_still_fails_batch() {
        let features = vec![feature("best", 12.0), feature("best", 750.0)];

        assert!(parse_features(features).is_err());
    }
}
