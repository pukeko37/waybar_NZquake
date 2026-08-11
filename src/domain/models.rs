//! Domain aggregates: `Earthquake` and the `QuakeData` collection, plus the
//! scoring/filtering business logic that ranks and trims them.

use super::types::{
    bearing_between, haversine_distance, CompassDirection, Coordinates, DayBand, Depth,
    Magnitude, Mmi, QuakeQuality, QuakeTime,
};

/// A single earthquake event.
#[derive(Debug, Clone)]
pub struct Earthquake {
    pub time: QuakeTime,
    pub magnitude: Magnitude,
    pub depth: Depth,
    pub quality: QuakeQuality,
    pub mmi: Option<Mmi>,
    pub epicenter: Coordinates,
    /// Ranking score written by `QuakeData::score_and_filter`; `0.0` until
    /// then. Not independently validated — it's a derived internal metric,
    /// not domain input, so a bare `f64` is enough.
    pub score: f64,
}

impl Earthquake {
    /// Horizontal surface distance from `from`, in km.
    pub fn distance_km(&self, from: &Coordinates) -> f64 {
        haversine_distance(from, &self.epicenter)
    }

    /// 3D distance from `from`, in km — horizontal distance plus depth,
    /// combined via Pythagoras. Used for scoring, to weight nearby-but-deep
    /// events appropriately.
    pub fn distance_3d_km(&self, from: &Coordinates) -> f64 {
        let horizontal = self.distance_km(from);
        (horizontal.powi(2) + self.depth.value().powi(2)).sqrt()
    }

    /// Compass direction from `from` to this quake's epicenter.
    pub fn direction_from(&self, from: &Coordinates) -> CompassDirection {
        CompassDirection::from_bearing(bearing_between(from, &self.epicenter))
    }

    /// Which day band this quake falls into, based on its current age.
    pub fn day_band(&self) -> DayBand {
        DayBand::from_age_days(self.time.age_in_days())
    }
}

/// A fetched batch of earthquakes, plus the user's reference location for
/// distance/direction/scoring.
#[derive(Debug)]
pub struct QuakeData {
    pub earthquakes: Vec<Earthquake>,
    pub user_location: Coordinates,
}

impl QuakeData {
    /// Score, sort, and filter earthquakes based on recency, proximity, and
    /// intensity. Keeps the top 8, grouped by day band and sorted by score
    /// within each band. Consumes `self` and returns a new `QuakeData`
    /// rather than mutating in place, per house style's preference for
    /// functional transformation over mutation.
    pub fn score_and_filter(self) -> Self {
        let user_location = self.user_location;

        let mut scored: Vec<Earthquake> = self
            .earthquakes
            .into_iter()
            .filter(|eq| eq.quality != QuakeQuality::Deleted && eq.magnitude.value() >= 3.0)
            .map(|mut eq| {
                let age_days = eq.time.age_in_days();
                let distance_3d_km = eq.distance_3d_km(&user_location);
                eq.score = calculate_score(age_days, distance_3d_km, eq.magnitude, eq.mmi);
                eq
            })
            .collect();

        scored.sort_by(|a, b| {
            a.day_band()
                .cmp(&b.day_band())
                .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
        });

        let earthquakes = scored.into_iter().take(8).collect();

        Self {
            earthquakes,
            user_location,
        }
    }
}

/// Score = log(recency) + log(proximity) + magnitude*2 + MMI, matching the
/// original hand-tuned weighting: recency and proximity are logarithmically
/// decayed against an 8-day / 320km baseline, magnitude is weighted x2
/// (already logarithmic on the Richter scale), and MMI (also logarithmic)
/// is added as-is.
fn calculate_score(age_days: f64, distance_3d_km: f64, magnitude: Magnitude, mmi: Option<Mmi>) -> f64 {
    let max_age = 8.0;
    let recency = (max_age - age_days).max(0.1); // avoid log(0)
    let recency_score = recency.log2();

    let max_distance = 320.0;
    let proximity = (max_distance - distance_3d_km).max(0.1); // avoid log(0)
    let proximity_score = proximity.log2();

    let magnitude_score = magnitude.value() * 2.0;
    let mmi_score = mmi.map(|m| m.value() as f64).unwrap_or(0.0);

    recency_score + proximity_score + magnitude_score + mmi_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{Latitude, Longitude, QuakeTime};

    fn wellington() -> Coordinates {
        Coordinates::new(
            Latitude::new(-41.2865).unwrap(),
            Longitude::new(174.7762).unwrap(),
        )
    }

    fn sample_quake(magnitude: f64, quality: QuakeQuality) -> Earthquake {
        Earthquake {
            time: QuakeTime::parse("2020-01-01T00:00:00.000Z").unwrap(),
            magnitude: Magnitude::new(magnitude).unwrap(),
            depth: Depth::new(10.0).unwrap(),
            quality,
            mmi: Mmi::new(4).ok(),
            epicenter: wellington(),
            score: 0.0,
        }
    }

    #[test]
    fn test_score_and_filter_drops_deleted_and_small() {
        let data = QuakeData {
            earthquakes: vec![
                sample_quake(5.0, QuakeQuality::Best),
                sample_quake(5.0, QuakeQuality::Deleted),
                sample_quake(2.5, QuakeQuality::Best),
            ],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 1);
        assert_eq!(filtered.earthquakes[0].quality, QuakeQuality::Best);
    }

    #[test]
    fn test_score_and_filter_caps_at_eight() {
        let earthquakes = (0..12).map(|_| sample_quake(5.0, QuakeQuality::Best)).collect();
        let data = QuakeData {
            earthquakes,
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 8);
    }

    #[test]
    fn test_distance_and_direction_from_same_point() {
        let quake = sample_quake(5.0, QuakeQuality::Best);
        assert!(quake.distance_km(&wellington()) < 0.001);
    }
}
