//! API models for deserializing GeoNet API JSON responses and converting to
//! domain types.
//!
//! GeoNet provides earthquake data through their public API in GeoJSON
//! format. This module handles the JSON response structure and converts it
//! to our domain models.

use crate::domain::{
    Coordinates, Depth, Earthquake, Latitude, LocalMmi, Longitude, Magnitude, Mmi, QuakeQuality,
    QuakeTime,
};
use anyhow::{Context, Result};
use serde::Deserialize;

/// GeoNet API response in GeoJSON FeatureCollection format.
#[derive(Debug, Deserialize)]
pub struct QuakeApiResponse {
    pub features: Vec<QuakeFeatureApi>,
}

#[derive(Debug, Deserialize)]
pub struct QuakeFeatureApi {
    pub properties: QuakePropertiesApi,
    pub geometry: QuakeGeometryApi,
}

#[derive(Debug, Deserialize)]
pub struct QuakePropertiesApi {
    pub time: String,
    pub magnitude: f64,
    pub depth: f64,
    pub quality: String,
    pub mmi: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct QuakeGeometryApi {
    pub coordinates: Vec<f64>,
}

impl TryFrom<QuakeFeatureApi> for Earthquake {
    type Error = anyhow::Error;

    fn try_from(value: QuakeFeatureApi) -> Result<Self> {
        let time = QuakeTime::parse(&value.properties.time)
            .with_context(|| format!("Failed to parse quake time: {}", value.properties.time))?;

        let magnitude = Magnitude::new(value.properties.magnitude)
            .with_context(|| format!("Magnitude out of range: {}", value.properties.magnitude))?;

        let depth = Depth::new(value.properties.depth)
            .with_context(|| format!("Depth out of range: {}", value.properties.depth))?;

        let quality = QuakeQuality::parse(&value.properties.quality)
            .with_context(|| format!("Unrecognised quality: {}", value.properties.quality))?;

        // GeoNet's wire-format sentinel for "no felt reports" is `-1`, not
        // an absent field — treat it as `None` rather than a domain-range
        // failure. See [[mmi-not-felt-sentinel]] in the project wiki.
        let mmi = value
            .properties
            .mmi
            .filter(|&m| m != -1)
            .map(Mmi::new)
            .transpose()
            .with_context(|| format!("MMI out of range: {:?}", value.properties.mmi))?;

        let longitude = value
            .geometry
            .coordinates
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Missing longitude in geometry.coordinates"))
            .and_then(|lon| Longitude::new(lon).context("Longitude out of range"))?;

        let latitude = value
            .geometry
            .coordinates
            .get(1)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Missing latitude in geometry.coordinates"))
            .and_then(|lat| Latitude::new(lat).context("Latitude out of range"))?;

        Ok(Earthquake {
            time,
            magnitude,
            depth,
            quality,
            mmi,
            epicenter: Coordinates::new(latitude, longitude),
            // Written by `QuakeData::score_and_filter`; not yet computed at
            // parse time, so 0.0 is a harmless placeholder (never in range
            // to survive filtering as-is, since retention only happens
            // after score_and_filter recomputes this properly).
            local_mmi: LocalMmi::new(0.0).expect("0.0 is within LocalMmiRange"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quake_feature_parsing() {
        let json_data = r#"
        {
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 5.2,
                "depth": 12.0,
                "quality": "best",
                "mmi": 4
            },
            "geometry": {
                "coordinates": [174.7762, -41.2865]
            }
        }
        "#;

        let feature: QuakeFeatureApi = serde_json::from_str(json_data).expect("Valid JSON");
        let earthquake: Earthquake = feature.try_into().expect("Valid domain conversion");

        assert_eq!(earthquake.magnitude.value(), 5.2);
        assert_eq!(earthquake.depth.value(), 12.0);
        assert_eq!(earthquake.quality, QuakeQuality::Best);
        assert_eq!(earthquake.mmi.unwrap().value(), 4);
        assert_eq!(earthquake.epicenter.latitude.value(), -41.2865);
        assert_eq!(earthquake.epicenter.longitude.value(), 174.7762);
    }

    #[test]
    fn test_quake_feature_mmi_not_felt_sentinel_becomes_none() {
        // GeoNet's wire-format sentinel for "no felt reports" is `-1`, not
        // absence of the field — see [[mmi-not-felt-sentinel]]. A `best`
        // quality record carrying it must not fail domain validation.
        let json_data = r#"
        {
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 4.8,
                "depth": 80.0,
                "quality": "best",
                "mmi": -1
            },
            "geometry": {
                "coordinates": [174.24, -40.33]
            }
        }
        "#;

        let feature: QuakeFeatureApi = serde_json::from_str(json_data).expect("Valid JSON");
        let earthquake: Earthquake = feature.try_into().expect("Valid domain conversion");

        assert!(earthquake.mmi.is_none());
    }

    #[test]
    fn test_quake_feature_without_mmi() {
        let json_data = r#"
        {
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 3.1,
                "depth": 5.0,
                "quality": "automatic",
                "mmi": null
            },
            "geometry": {
                "coordinates": [174.7762, -41.2865]
            }
        }
        "#;

        let feature: QuakeFeatureApi = serde_json::from_str(json_data).expect("Valid JSON");
        let earthquake: Earthquake = feature.try_into().expect("Valid domain conversion");

        assert!(earthquake.mmi.is_none());
    }

    #[test]
    fn test_quake_feature_unknown_quality_rejected() {
        let json_data = r#"
        {
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 5.2,
                "depth": 12.0,
                "quality": "caution",
                "mmi": null
            },
            "geometry": {
                "coordinates": [174.7762, -41.2865]
            }
        }
        "#;

        let feature: QuakeFeatureApi = serde_json::from_str(json_data).expect("Valid JSON");
        let result: Result<Earthquake, _> = feature.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_quake_feature_invalid_magnitude_rejected() {
        let json_data = r#"
        {
            "properties": {
                "time": "2024-01-13T14:30:00.000Z",
                "magnitude": 25.0,
                "depth": 12.0,
                "quality": "best",
                "mmi": null
            },
            "geometry": {
                "coordinates": [174.7762, -41.2865]
            }
        }
        "#;

        let feature: QuakeFeatureApi = serde_json::from_str(json_data).expect("Valid JSON");
        let result: Result<Earthquake, _> = feature.try_into();
        assert!(result.is_err());
    }
}
