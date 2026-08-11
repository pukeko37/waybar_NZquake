//! Closed error types for the domain layer.

use std::fmt;

/// Domain error enum representing all possible validation failures.
#[derive(Debug)]
pub enum QuakeError {
    /// A numeric value fell outside its valid range.
    OutOfRange {
        value: String,
        min: String,
        max: String,
        unit: &'static str,
    },
    /// A quake quality string didn't match any of GeoNet's documented values.
    UnknownQuality(String),
    /// A timestamp could not be parsed.
    InvalidTimestamp(String),
}

impl fmt::Display for QuakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange {
                value,
                min,
                max,
                unit,
            } => write!(
                f,
                "Value {} {} is outside valid range ({} to {})",
                value, unit, min, max
            ),
            Self::UnknownQuality(quality) => write!(f, "Unknown quake quality: {}", quality),
            Self::InvalidTimestamp(ts) => write!(f, "Invalid timestamp: {}", ts),
        }
    }
}

impl std::error::Error for QuakeError {}
