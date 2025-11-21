//! API models for deserializing GeoNet API JSON responses and converting to domain types.
//!
//! GeoNet provides earthquake data through their public API in GeoJSON format.
//! This module handles the JSON response structure and converts it to our domain models.

use serde::Deserialize;

/// GeoNet API response in GeoJSON FeatureCollection format
#[derive(Debug, Deserialize)]
pub struct QuakeApiResponse {
    pub features: Vec<QuakeFeature>,
}

#[derive(Debug, Deserialize)]
pub struct QuakeFeature {
    pub properties: QuakeProperties,
    pub geometry: QuakeGeometry,
}

#[derive(Debug, Deserialize)]
pub struct QuakeProperties {
    pub time: String,
    pub magnitude: f64,
    pub depth: f64,
    pub quality: String,
    pub mmi: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct QuakeGeometry {
    pub coordinates: Vec<f64>,
}

/// Domain model for earthquake data with user location
#[derive(Debug)]
pub struct QuakeData {
    pub earthquakes: Vec<Earthquake>,
    pub user_location: (f64, f64), // (latitude, longitude)
}

impl QuakeData {
    /// Score, sort, and filter earthquakes based on recency, proximity, and intensity
    /// Returns top 8 earthquakes grouped by day bands, sorted by score within each band
    pub fn score_and_filter(&mut self) {
        let (user_lat, user_lon) = self.user_location;

        // Filter out deleted events and events below M3.0, then calculate scores
        let mut scored: Vec<(Earthquake, f64, f64)> = self
            .earthquakes
            .iter()
            .filter(|eq| eq.quality != "deleted" && eq.magnitude >= 3.0)
            .map(|eq| {
                let age_days = eq.age_in_days();
                let distance_3d_km = eq.distance_3d_from(user_lat, user_lon);
                let score = calculate_score(age_days, distance_3d_km, eq.magnitude, eq.mmi.unwrap_or(0));
                (eq.clone(), score, age_days)
            })
            .collect();

        // Sort by age first (for day bands), then by score within bands
        scored.sort_by(|a, b| {
            let day_band_a = get_day_band(a.2);
            let day_band_b = get_day_band(b.2);

            day_band_a
                .cmp(&day_band_b)
                .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Take top 8 and preserve scores
        self.earthquakes = scored
            .into_iter()
            .take(8)
            .map(|(mut eq, score, _)| {
                eq.score = score;
                eq
            })
            .collect();
    }
}

/// Calculate score based on logarithmic decay of time, distance, magnitude, and MMI
/// Score = log(recency) + log(proximity) + magnitude*2 + MMI
fn calculate_score(age_days: f64, distance_km: f64, magnitude: f64, mmi: i32) -> f64 {
    // Recency score: higher for more recent (invert age)
    // Use 1-day baseline, so 1 day = 0 decay, 2 days = -1, 4 days = -2, etc.
    let max_age = 8.0; // 8 day band
    let recency = (max_age - age_days).max(0.1); // Avoid log(0)
    let recency_score = recency.log2();

    // Proximity score: higher for closer (invert distance)
    // Use 20km baseline
    let max_distance = 320.0; // 320km band
    let proximity = (max_distance - distance_km).max(0.1); // Avoid log(0)
    let proximity_score = proximity.log2();

    // Magnitude is already logarithmic (Richter scale), weight it x2 for significance
    // Lower magnitude events will score proportionally lower
    let magnitude_score = magnitude * 2.0;

    // MMI is also logarithmic, use as-is
    let mmi_score = mmi as f64;

    recency_score + proximity_score + magnitude_score + mmi_score
}

/// Get day band for grouping (0=0-1 days, 1=1-2 days, 2=2-4 days, 3=4-8 days, 4=8+ days)
fn get_day_band(age_days: f64) -> u32 {
    if age_days < 1.0 {
        0
    } else if age_days < 2.0 {
        1
    } else if age_days < 4.0 {
        2
    } else if age_days < 8.0 {
        3
    } else {
        4
    }
}

/// Domain model for a single earthquake event
#[derive(Debug, Clone)]
pub struct Earthquake {
    pub time: String,
    pub magnitude: f64,
    pub depth: f64,
    pub quality: String,
    pub mmi: Option<i32>,
    pub latitude: f64,
    pub longitude: f64,
    pub score: f64,
}

impl Earthquake {
    /// Parse time string to extract age in days
    pub fn age_in_days(&self) -> f64 {
        use time::OffsetDateTime;

        if let Ok(quake_time) = OffsetDateTime::parse(
            &self.time,
            &time::format_description::well_known::Iso8601::DEFAULT,
        ) {
            let now = OffsetDateTime::now_utc();
            let duration = now - quake_time;
            duration.whole_seconds() as f64 / 86400.0
        } else {
            // If parsing fails, assume very old
            365.0
        }
    }

    /// Calculate horizontal surface distance from user location using Haversine formula
    pub fn horizontal_distance_from(&self, user_lat: f64, user_lon: f64) -> f64 {
        haversine_distance(user_lat, user_lon, self.latitude, self.longitude)
    }

    /// Calculate 3D distance from user location using Haversine formula + depth (Pythagorean)
    /// Used for scoring to account for depth
    pub fn distance_3d_from(&self, user_lat: f64, user_lon: f64) -> f64 {
        let horizontal_distance = haversine_distance(user_lat, user_lon, self.latitude, self.longitude);
        // Calculate 3D distance: sqrt(horizontal^2 + depth^2)
        (horizontal_distance.powi(2) + self.depth.powi(2)).sqrt()
    }

    /// Get direction from user location (N, NE, E, SE, S, SW, W, NW)
    pub fn direction_from(&self, user_lat: f64, user_lon: f64) -> String {
        let bearing = calculate_bearing(user_lat, user_lon, self.latitude, self.longitude);
        bearing_to_direction(bearing)
    }
}

/// Calculate Haversine distance between two points in kilometers
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth's radius in km
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    r * c
}

/// Calculate bearing from point 1 to point 2 in degrees
fn calculate_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let y = delta_lon.sin() * lat2_rad.cos();
    let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();
    let bearing = y.atan2(x).to_degrees();

    (bearing + 360.0) % 360.0
}

/// Convert bearing to compass direction
fn bearing_to_direction(bearing: f64) -> String {
    let directions = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((bearing + 22.5) / 45.0) as usize % 8;
    directions[index].to_string()
}
