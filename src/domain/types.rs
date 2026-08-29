//! Core domain types for earthquake data with compile-time safety and validation.

use super::error::QuakeError;
use std::fmt;
use std::marker::PhantomData;
use time::OffsetDateTime;

// === Range Validation Trait ===

/// Trait for types that validate values within a compile-time range.
pub trait RangeValidated<T>
where
    T: PartialOrd + Copy + fmt::Display,
{
    const MIN: T;
    const MAX: T;
    const UNIT: &'static str;

    /// Validate that a value is within the range [MIN, MAX].
    fn validate(value: T) -> Result<(), QuakeError> {
        if value < Self::MIN || value > Self::MAX {
            return Err(QuakeError::OutOfRange {
                value: value.to_string(),
                min: Self::MIN.to_string(),
                max: Self::MAX.to_string(),
                unit: Self::UNIT,
            });
        }
        Ok(())
    }
}

// === Generic Range-Validated Type ===

/// Generic validated type, phantom-tagged with a range marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeValidatedValue<T, R>
where
    T: PartialOrd + Copy + fmt::Display,
    R: RangeValidated<T>,
{
    value: T,
    _range: PhantomData<R>,
}

impl<T, R> RangeValidatedValue<T, R>
where
    T: PartialOrd + Copy + fmt::Display,
    R: RangeValidated<T>,
{
    /// Create a new validated value - the only way to construct this type.
    pub fn new(value: T) -> Result<Self, QuakeError> {
        R::validate(value)?;
        Ok(Self {
            value,
            _range: PhantomData,
        })
    }

    /// Get the validated value.
    pub fn value(&self) -> T {
        self.value
    }
}

// === Range Definitions ===

/// Earthquake magnitude range, generous enough to cover the Richter scale's
/// practical extremes (small negative-magnitude events are real, recorded
/// GeoNet data; nothing on Earth has exceeded M10).
#[derive(Debug, Clone, Copy)]
pub struct MagnitudeRange;
impl RangeValidated<f64> for MagnitudeRange {
    const MIN: f64 = -3.0;
    const MAX: f64 = 10.0;
    const UNIT: &'static str = "magnitude";
}

/// Earthquake depth range in km (deepest recorded earthquakes are ~700km).
#[derive(Debug, Clone, Copy)]
pub struct DepthRange;
impl RangeValidated<f64> for DepthRange {
    const MIN: f64 = 0.0;
    const MAX: f64 = 700.0;
    const UNIT: &'static str = "km";
}

/// Modified Mercalli Intensity range (0 to 12), as GeoNet reports it.
#[derive(Debug, Clone, Copy)]
pub struct MmiRange;
impl RangeValidated<i32> for MmiRange {
    const MIN: i32 = 0;
    const MAX: i32 = 12;
    const UNIT: &'static str = "MMI";
}

/// Locally-derived MMI range, same 0..=12 scale as `MmiRange` but continuous
/// (f64) rather than integer — the Dowrick & Rhoades attenuation formula
/// this backs produces fractional intensities, and the tooltip displays one
/// decimal place, so `MmiRange`'s integer `Mmi` isn't the right fit despite
/// the shared scale. See [[quake-significance-metric]] in the project wiki.
#[derive(Debug, Clone, Copy)]
pub struct LocalMmiRange;
impl RangeValidated<f64> for LocalMmiRange {
    const MIN: f64 = 0.0;
    const MAX: f64 = 12.0;
    const UNIT: &'static str = "MMI (local)";
}

/// Latitude range, globally valid (not NZ-specific — see main.rs for the
/// separate, non-fatal NZ-typical-range warning).
#[derive(Debug, Clone, Copy)]
pub struct LatitudeRange;
impl RangeValidated<f64> for LatitudeRange {
    const MIN: f64 = -90.0;
    const MAX: f64 = 90.0;
    const UNIT: &'static str = "°";
}

/// Longitude range, globally valid.
#[derive(Debug, Clone, Copy)]
pub struct LongitudeRange;
impl RangeValidated<f64> for LongitudeRange {
    const MIN: f64 = -180.0;
    const MAX: f64 = 180.0;
    const UNIT: &'static str = "°";
}

/// Earthquake magnitude with validation.
pub type Magnitude = RangeValidatedValue<f64, MagnitudeRange>;

/// Earthquake depth in km with validation.
pub type Depth = RangeValidatedValue<f64, DepthRange>;

/// Modified Mercalli Intensity with validation. Not every quake carries one
/// — callers use `Option<Mmi>`.
pub type Mmi = RangeValidatedValue<i32, MmiRange>;

/// Locally-derived MMI with validation — always present (unlike `Mmi`,
/// which mirrors GeoNet's own sporadically-supplied reading).
pub type LocalMmi = RangeValidatedValue<f64, LocalMmiRange>;

/// Latitude with validation.
pub type Latitude = RangeValidatedValue<f64, LatitudeRange>;

/// Longitude with validation.
pub type Longitude = RangeValidatedValue<f64, LongitudeRange>;

// === Coordinates ===

/// A geographic point — either the user's reference location or a quake's
/// epicenter.
#[derive(Debug, Clone, Copy)]
pub struct Coordinates {
    pub latitude: Latitude,
    pub longitude: Longitude,
}

impl Coordinates {
    pub fn new(latitude: Latitude, longitude: Longitude) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

/// Calculate great-circle surface distance between two points in km
/// (Haversine formula).
pub fn haversine_distance(a: &Coordinates, b: &Coordinates) -> f64 {
    let r = 6371.0; // Earth's radius in km
    let lat1_rad = a.latitude.value().to_radians();
    let lat2_rad = b.latitude.value().to_radians();
    let delta_lat = (b.latitude.value() - a.latitude.value()).to_radians();
    let delta_lon = (b.longitude.value() - a.longitude.value()).to_radians();

    let h = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * h.sqrt().atan2((1.0 - h).sqrt());

    r * c
}

/// Calculate the bearing from `from` to `to`, in degrees (0 = north).
pub fn bearing_between(from: &Coordinates, to: &Coordinates) -> f64 {
    let lat1_rad = from.latitude.value().to_radians();
    let lat2_rad = to.latitude.value().to_radians();
    let delta_lon = (to.longitude.value() - from.longitude.value()).to_radians();

    let y = delta_lon.sin() * lat2_rad.cos();
    let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();
    let bearing = y.atan2(x).to_degrees();

    (bearing + 360.0) % 360.0
}

// === Compass Direction ===

/// The 8-point compass, banded from a bearing in degrees. Illegal states
/// (an out-of-range or nonsensical direction string) are unrepresentable —
/// same pattern as `WindDirection` in `waybar_weather`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompassDirection {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

impl CompassDirection {
    /// Band a bearing in degrees into one of the 8 compass points.
    pub fn from_bearing(bearing: f64) -> Self {
        const DIRECTIONS: [CompassDirection; 8] = [
            CompassDirection::N,
            CompassDirection::Ne,
            CompassDirection::E,
            CompassDirection::Se,
            CompassDirection::S,
            CompassDirection::Sw,
            CompassDirection::W,
            CompassDirection::Nw,
        ];
        let index = ((bearing + 22.5) / 45.0) as usize % 8;
        DIRECTIONS[index]
    }
}

impl fmt::Display for CompassDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::N => "N",
            Self::Ne => "NE",
            Self::E => "E",
            Self::Se => "SE",
            Self::S => "S",
            Self::Sw => "SW",
            Self::W => "W",
            Self::Nw => "NW",
        };
        write!(f, "{}", s)
    }
}

// === Day Band ===

/// Groups earthquakes by age for tooltip display, extended to six bands to
/// match the 32-day retention window (see [[quake-significance-metric]]).
/// `from_age_days` is total over `0..=32` days — the retention step in
/// `QuakeData::score_and_filter` is what guarantees every quake reaching
/// display is within that window; a caller invoking this on an older
/// timestamp will get `SixteenToThirtyTwoDays` rather than a dedicated
/// "older" band, since there no longer is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DayBand {
    Today,
    OneToTwoDays,
    TwoToFourDays,
    FourToEightDays,
    EightToSixteenDays,
    SixteenToThirtyTwoDays,
}

impl DayBand {
    pub fn from_age_days(age_days: f64) -> Self {
        if age_days < 1.0 {
            Self::Today
        } else if age_days < 2.0 {
            Self::OneToTwoDays
        } else if age_days < 4.0 {
            Self::TwoToFourDays
        } else if age_days < 8.0 {
            Self::FourToEightDays
        } else if age_days < 16.0 {
            Self::EightToSixteenDays
        } else {
            Self::SixteenToThirtyTwoDays
        }
    }
}

// === Quake Quality ===

/// GeoNet's documented quake data-quality values. Closed, no catch-all — an
/// unrecognised value is a hard parse failure rather than a silently
/// swallowed unknown, per the domain's illegal-states-unrepresentable stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuakeQuality {
    Best,
    Preliminary,
    Automatic,
    Deleted,
}

impl QuakeQuality {
    pub fn parse(value: &str) -> Result<Self, QuakeError> {
        match value {
            "best" => Ok(Self::Best),
            "preliminary" => Ok(Self::Preliminary),
            "automatic" => Ok(Self::Automatic),
            "deleted" => Ok(Self::Deleted),
            other => Err(QuakeError::UnknownQuality(other.to_string())),
        }
    }
}

impl fmt::Display for QuakeQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Best => "best",
            Self::Preliminary => "preliminary",
            Self::Automatic => "automatic",
            Self::Deleted => "deleted",
        };
        write!(f, "{}", s)
    }
}

// === Quake Time ===

/// A parsed earthquake timestamp.
#[derive(Debug, Clone, Copy)]
pub struct QuakeTime(OffsetDateTime);

impl QuakeTime {
    /// Parse an ISO-8601 timestamp as GeoNet emits it.
    pub fn parse(value: &str) -> Result<Self, QuakeError> {
        OffsetDateTime::parse(
            value,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .map(Self)
        .map_err(|_| QuakeError::InvalidTimestamp(value.to_string()))
    }

    /// Age of this timestamp in days, relative to now. Non-deterministic by
    /// design (reads the wall clock) — this app has no need for injected
    /// clocks beyond what its existing tests already tolerate.
    pub fn age_in_days(&self) -> f64 {
        let now = OffsetDateTime::now_utc();
        (now - self.0).whole_seconds() as f64 / 86400.0
    }

    /// The underlying timestamp, for display-layer local-time conversion.
    pub fn as_offset_date_time(&self) -> OffsetDateTime {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_cost_abstractions() {
        assert_eq!(std::mem::size_of::<Magnitude>(), std::mem::size_of::<f64>());
        assert_eq!(std::mem::size_of::<Depth>(), std::mem::size_of::<f64>());
        assert_eq!(std::mem::size_of::<Mmi>(), std::mem::size_of::<i32>());
    }

    #[test]
    fn test_magnitude_range() {
        assert!(Magnitude::new(5.2).is_ok());
        assert!(Magnitude::new(-3.0).is_ok());
        assert!(Magnitude::new(10.0).is_ok());
        assert!(Magnitude::new(10.1).is_err());
        assert!(Magnitude::new(-3.1).is_err());
    }

    #[test]
    fn test_depth_range() {
        assert!(Depth::new(12.0).is_ok());
        assert!(Depth::new(-1.0).is_err());
        assert!(Depth::new(701.0).is_err());
    }

    #[test]
    fn test_mmi_range() {
        assert!(Mmi::new(6).is_ok());
        assert!(Mmi::new(-1).is_err());
        assert!(Mmi::new(13).is_err());
    }

    #[test]
    fn test_latitude_longitude_range() {
        assert!(Latitude::new(-41.2865).is_ok());
        assert!(Latitude::new(-91.0).is_err());
        assert!(Longitude::new(174.7762).is_ok());
        assert!(Longitude::new(181.0).is_err());
    }

    #[test]
    fn test_quake_quality_parse() {
        assert_eq!(QuakeQuality::parse("best").unwrap(), QuakeQuality::Best);
        assert_eq!(
            QuakeQuality::parse("deleted").unwrap(),
            QuakeQuality::Deleted
        );
        assert!(QuakeQuality::parse("unknown").is_err());
    }

    #[test]
    fn test_compass_direction_from_bearing() {
        assert_eq!(CompassDirection::from_bearing(0.0), CompassDirection::N);
        assert_eq!(CompassDirection::from_bearing(90.0), CompassDirection::E);
        assert_eq!(CompassDirection::from_bearing(180.0), CompassDirection::S);
        assert_eq!(CompassDirection::from_bearing(270.0), CompassDirection::W);
    }

    #[test]
    fn test_day_band_from_age_days() {
        assert_eq!(DayBand::from_age_days(0.5), DayBand::Today);
        assert_eq!(DayBand::from_age_days(1.5), DayBand::OneToTwoDays);
        assert_eq!(DayBand::from_age_days(3.0), DayBand::TwoToFourDays);
        assert_eq!(DayBand::from_age_days(6.0), DayBand::FourToEightDays);
        assert_eq!(DayBand::from_age_days(12.0), DayBand::EightToSixteenDays);
        assert_eq!(
            DayBand::from_age_days(30.0),
            DayBand::SixteenToThirtyTwoDays
        );
    }

    #[test]
    fn test_day_band_ordering() {
        assert!(DayBand::Today < DayBand::OneToTwoDays);
        assert!(DayBand::EightToSixteenDays < DayBand::SixteenToThirtyTwoDays);
    }

    #[test]
    fn test_haversine_distance_zero_for_same_point() {
        let wellington = Coordinates::new(
            Latitude::new(-41.2865).unwrap(),
            Longitude::new(174.7762).unwrap(),
        );
        assert!(haversine_distance(&wellington, &wellington) < 0.001);
    }

    #[test]
    fn test_quake_time_parse_and_age() {
        let time = QuakeTime::parse("2020-01-01T00:00:00.000Z").unwrap();
        assert!(time.age_in_days() > 0.0);
        assert!(QuakeTime::parse("not-a-time").is_err());
    }
}
