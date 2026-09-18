//! Domain aggregates: `Earthquake` and the `QuakeData` collection, plus the
//! scoring/filtering business logic that ranks and trims them.

use super::types::{
    bearing_between, haversine_distance, CompassDirection, Coordinates, DayBand, Depth, LocalMmi,
    Magnitude, Mmi, QuakeQuality, QuakeTime,
};

/// A single earthquake event.
#[derive(Debug, Clone)]
pub struct Earthquake {
    pub time: QuakeTime,
    pub magnitude: Magnitude,
    pub depth: Depth,
    pub quality: QuakeQuality,
    /// GeoNet's own reported MMI, when it supplies one — the intensity at
    /// this quake's own strongest point, not specifically at the user's
    /// location. Distinct from `local_mmi`; see [[quake-significance-metric]]
    /// in the project wiki for why both are kept.
    pub mmi: Option<Mmi>,
    pub epicenter: Coordinates,
    /// Estimated MMI at the user's reference location, written by
    /// `QuakeData::score_and_filter`; `0.0` until then. Always present
    /// (unlike `mmi`) — this is what retains quakes (via decayed
    /// significance) and what the tooltip highlights the top entry by; bar
    /// text leads with the latest quake by time instead, see
    /// [[quake-tooltip-display-order]].
    pub local_mmi: LocalMmi,
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

/// Hard outer edge of the retention window, in days. See
/// [[tooltip-retention-mmi-floor]] (superseding
/// [[quake-significance-metric]]'s 32-day/decay scheme) — extended to match
/// `DayBand`'s seventh bracket, `ThirtyTwoToSixtyFourDays`.
const MAX_RETAINED_AGE_DAYS: f64 = 64.0;

/// A quake's undecayed `local_mmi` must be at or above this to be retained.
/// No decay, no count floor/ceiling — see [[tooltip-retention-mmi-floor]].
const MIN_RETAINED_LOCAL_MMI: f64 = 3.0;

impl QuakeData {
    /// Compute each quake's local MMI, retain those within
    /// [`MAX_RETAINED_AGE_DAYS`] whose `local_mmi` clears
    /// [`MIN_RETAINED_LOCAL_MMI`] (no decay, no count floor/ceiling — see
    /// [[tooltip-retention-mmi-floor]]), and sort the retained set by day
    /// band ascending (most recent band first), then quake time ascending
    /// within a band — see [[quake-tooltip-display-order]]. Consumes `self`
    /// and returns a new `QuakeData` rather than mutating in place, per
    /// house style's preference for functional transformation over
    /// mutation.
    pub fn score_and_filter(self) -> Self {
        let user_location = self.user_location;

        let mut earthquakes: Vec<Earthquake> = self
            .earthquakes
            .into_iter()
            .filter(|eq| eq.quality != QuakeQuality::Deleted)
            .filter_map(|mut eq| {
                let age_days = eq.time.age_in_days();
                if age_days > MAX_RETAINED_AGE_DAYS {
                    return None;
                }

                let distance_km = eq.distance_km(&user_location);
                eq.local_mmi =
                    calculate_local_mmi(eq.depth.value(), distance_km, eq.magnitude.value());

                if eq.local_mmi.value() < MIN_RETAINED_LOCAL_MMI {
                    return None;
                }

                Some(eq)
            })
            .collect();

        // Day band ascending (`Today` first — most recent band first), then
        // quake time ascending within a band (oldest first) — see
        // [[quake-tooltip-display-order]]. `local_mmi` no longer determines
        // storage order or membership beyond the flat floor above.
        earthquakes.sort_by(|a, b| {
            a.day_band().cmp(&b.day_band()).then_with(|| {
                b.time
                    .age_in_days()
                    .partial_cmp(&a.time.age_in_days())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        Self {
            earthquakes,
            user_location,
        }
    }
}

/// Estimate local MMI at a distance of `distance_km` from a quake of
/// `magnitude` and `depth_km`, per Dowrick & Rhoades (2005) — Model 2 (Main
/// Seismic Region) for `depth_km < 70`, Model 3 (Deep region) otherwise.
/// GeoNet's reported magnitude is used as `Mw` directly, with no correction.
/// Model 2's crustal-indicator term is approximated from depth alone (GeoNet
/// exposes no tectonic-type classification): `depth_km < 40` is treated as
/// Crustal, `40..70` as not. This is a deliberately simple approximation,
/// not a reproduction of GeoNet's own (much heavier) tectonic-weighting
/// method — see [[quake-significance-metric]] and [[tectonic-type-approximation]]
/// in the project wiki for the sourced reasoning and the alternatives ruled
/// out. Result is clamped into `LocalMmiRange` — the raw formula can exceed
/// 0..=12 at extreme distance or magnitude, which the papers' own models
/// don't otherwise bound.
fn calculate_local_mmi(depth_km: f64, distance_km: f64, magnitude: f64) -> LocalMmi {
    const DEEP_THRESHOLD_KM: f64 = 70.0;
    const CRUSTAL_THRESHOLD_KM: f64 = 40.0;
    const MODEL2_SMOOTHING_KM: f64 = 11.78;
    const CRUSTAL_BONUS: f64 = 0.409;
    // Avoids log10(0) = -inf for a quake directly beneath the reference
    // point; has no other effect (D and 0.1km apart are indistinguishable
    // for these formulas at NZ-realistic magnitudes).
    let safe_distance_km = distance_km.max(0.1);

    let raw = if depth_km >= DEEP_THRESHOLD_KM {
        3.76 + 1.48 * magnitude - 3.50 * safe_distance_km.log10() + 0.0031 * depth_km
    } else {
        let smoothed_distance = (safe_distance_km.powi(3) + MODEL2_SMOOTHING_KM.powi(3)).cbrt();
        let crustal_bonus = if depth_km < CRUSTAL_THRESHOLD_KM {
            CRUSTAL_BONUS
        } else {
            0.0
        };
        4.40 + 1.26 * magnitude - 3.67 * smoothed_distance.log10()
            + 0.012 * depth_km
            + crustal_bonus
    };

    // Safety: clamped into LocalMmiRange (0.0..=12.0) on the line above, so
    // construction cannot fail.
    LocalMmi::new(raw.clamp(0.0, 12.0)).expect("clamped value is always within LocalMmiRange")
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

    /// A recent (not literally now, to avoid float-seconds flakiness)
    /// timestamp `age_days` old, formatted the way GeoNet's API emits one.
    fn timestamp_days_ago(age_days: f64) -> QuakeTime {
        let when =
            time::OffsetDateTime::now_utc() - time::Duration::seconds((age_days * 86400.0) as i64);
        let formatted = when
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .expect("OffsetDateTime always formats as ISO-8601");
        QuakeTime::parse(&formatted).expect("just-formatted timestamp always reparses")
    }

    fn sample_quake(magnitude: f64, quality: QuakeQuality, age_days: f64) -> Earthquake {
        Earthquake {
            time: timestamp_days_ago(age_days),
            magnitude: Magnitude::new(magnitude).unwrap(),
            depth: Depth::new(10.0).unwrap(),
            quality,
            mmi: Mmi::new(4).ok(),
            epicenter: wellington(),
            local_mmi: LocalMmi::new(0.0).unwrap(),
        }
    }

    #[test]
    fn test_score_and_filter_drops_deleted() {
        let data = QuakeData {
            earthquakes: vec![
                sample_quake(6.0, QuakeQuality::Best, 0.0),
                sample_quake(6.0, QuakeQuality::Deleted, 0.0),
            ],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 1);
        assert_eq!(filtered.earthquakes[0].quality, QuakeQuality::Best);
    }

    #[test]
    fn test_score_and_filter_no_longer_filters_on_magnitude_alone() {
        // A small, very close quake can still have a locally-significant
        // MMI — the old flat magnitude>=3.0 floor is gone (see
        // quake-significance-metric decision).
        let data = QuakeData {
            earthquakes: vec![sample_quake(2.5, QuakeQuality::Best, 0.0)],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 1);
    }

    #[test]
    fn test_score_and_filter_has_no_count_cap() {
        // Per tooltip-retention-mmi-floor: no ceiling — every quake clearing
        // the age + local_mmi filters is retained, however many that is.
        let earthquakes = (0..20)
            .map(|_| sample_quake(7.0, QuakeQuality::Best, 0.0))
            .collect();
        let data = QuakeData {
            earthquakes,
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 20);
    }

    #[test]
    fn test_score_and_filter_has_no_backfill_floor() {
        // Per tooltip-retention-mmi-floor: no minimum count — quakes below
        // MIN_RETAINED_LOCAL_MMI are dropped outright, not backfilled.
        let earthquakes = (0..12)
            .map(|_| sample_quake(0.0, QuakeQuality::Best, 0.0))
            .collect();
        let data = QuakeData {
            earthquakes,
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert!(filtered.earthquakes.is_empty());
    }

    #[test]
    fn test_score_and_filter_drops_quakes_below_local_mmi_floor() {
        // A distant, low-magnitude quake with local_mmi well under 3.0 is
        // dropped even though it's recent — the flat floor, not decay,
        // governs membership now.
        let data = QuakeData {
            earthquakes: vec![sample_quake(1.0, QuakeQuality::Best, 0.0)],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert!(filtered.earthquakes.is_empty());
    }

    #[test]
    fn test_score_and_filter_drops_quakes_older_than_64_days() {
        let data = QuakeData {
            earthquakes: vec![sample_quake(8.0, QuakeQuality::Best, 70.0)],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert!(filtered.earthquakes.is_empty());
    }

    #[test]
    fn test_score_and_filter_retains_quakes_up_to_64_days_old_if_significant() {
        let data = QuakeData {
            earthquakes: vec![sample_quake(8.0, QuakeQuality::Best, 50.0)],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert_eq!(filtered.earthquakes.len(), 1);
    }

    #[test]
    fn test_score_and_filter_sorts_by_day_band_ascending_most_recent_first() {
        // Per quake-tooltip-display-order: day band ascending (Today
        // first), not local_mmi. High-MMI old quake vs. low-MMI new quake —
        // the new one must still lead, which the old MMI-descending sort
        // would have gotten backwards.
        let data = QuakeData {
            earthquakes: vec![
                sample_quake(8.0, QuakeQuality::Best, 10.0), // high MMI, old
                sample_quake(3.0, QuakeQuality::Best, 0.5),  // low MMI, today
            ],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        let bands: Vec<_> = filtered
            .earthquakes
            .iter()
            .map(|eq| eq.day_band())
            .collect();
        let mut sorted_asc = bands.clone();
        sorted_asc.sort();
        assert_eq!(bands, sorted_asc);
    }

    #[test]
    fn test_score_and_filter_sorts_ascending_by_time_within_a_band() {
        // Same day band (Today), different ages — must come out oldest
        // first (ascending date order within a band).
        let data = QuakeData {
            earthquakes: vec![
                sample_quake(5.0, QuakeQuality::Best, 0.1), // newer
                sample_quake(5.0, QuakeQuality::Best, 0.8), // older
            ],
            user_location: wellington(),
        };

        let filtered = data.score_and_filter();
        assert!(
            filtered.earthquakes[0].time.age_in_days() > filtered.earthquakes[1].time.age_in_days()
        );
    }

    #[test]
    fn test_calculate_local_mmi_deeper_reads_higher_at_same_distance_magnitude() {
        let shallow = calculate_local_mmi(10.0, 50.0, 5.5).value();
        let deeper = calculate_local_mmi(65.0, 50.0, 5.5).value();
        assert!(deeper > shallow);
    }

    #[test]
    fn test_calculate_local_mmi_clamped_to_range() {
        // Far enough away and small enough that the raw formula goes
        // negative — must clamp to LocalMmiRange's floor, not error.
        let mmi = calculate_local_mmi(10.0, 2000.0, -2.0);
        assert!((0.0..=12.0).contains(&mmi.value()));
    }

    #[test]
    fn test_distance_and_direction_from_same_point() {
        let quake = sample_quake(5.0, QuakeQuality::Best, 0.0);
        assert!(quake.distance_km(&wellington()) < 0.001);
    }
}
