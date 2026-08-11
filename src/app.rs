//! Application layer: orchestrates domain logic through port traits.
//!
//! Defines the port traits (`QuakeFetcher`, `QuakeFormatter`) that
//! infrastructure adapters implement. No `use crate::infra::` imports here
//! outside `#[cfg(test)]`.

use crate::domain::{Coordinates, QuakeData};

/// Port trait for fetching earthquake data.
///
/// Uses `anyhow::Error` because network/HTTP errors are genuinely
/// open-ended infrastructure concerns.
pub trait QuakeFetcher {
    fn fetch_earthquakes(&self, location: Coordinates) -> Result<QuakeData, anyhow::Error>;
}

/// Port trait for formatting earthquake data into some output representation.
///
/// The associated `Output` type lets each adapter choose its own output
/// (e.g., `WaybarOutput` for the Waybar formatter).
pub trait QuakeFormatter {
    type Output;
    fn format(&self, data: &QuakeData) -> Result<Self::Output, anyhow::Error>;
}

/// Fetch earthquake data, score and filter it, and format it for output.
///
/// Generic over both ports, enabling test doubles for either side.
/// `score_and_filter` lives here rather than in `infra` — it's pure,
/// I/O-free business logic with no dependency on the wire format, which is
/// exactly what this orchestration function is for.
pub fn fetch_and_format<F: QuakeFetcher, Fmt: QuakeFormatter>(
    fetcher: &F,
    formatter: &Fmt,
    location: Coordinates,
) -> Result<Fmt::Output, anyhow::Error> {
    let quake_data = fetcher.fetch_earthquakes(location)?.score_and_filter();
    formatter.format(&quake_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Latitude, Longitude};
    use crate::infra::display::WaybarFormatter;

    struct StubQuakeFetcher {
        data: Result<Coordinates, anyhow::Error>,
    }

    impl QuakeFetcher for StubQuakeFetcher {
        fn fetch_earthquakes(&self, _location: Coordinates) -> Result<QuakeData, anyhow::Error> {
            match &self.data {
                Ok(user_location) => Ok(QuakeData {
                    earthquakes: vec![],
                    user_location: *user_location,
                }),
                Err(e) => Err(anyhow::anyhow!("{}", e)),
            }
        }
    }

    fn wellington() -> Coordinates {
        Coordinates::new(
            Latitude::new(-41.2865).unwrap(),
            Longitude::new(174.7762).unwrap(),
        )
    }

    #[test]
    fn test_fetch_and_format_success() {
        let fetcher = StubQuakeFetcher {
            data: Ok(wellington()),
        };
        let formatter = WaybarFormatter::new();

        let output = fetch_and_format(&fetcher, &formatter, wellington()).unwrap();
        assert!(output.text.contains("No recent earthquakes"));
    }

    #[test]
    fn test_fetch_and_format_error() {
        let fetcher = StubQuakeFetcher {
            data: Err(anyhow::anyhow!("connection refused")),
        };
        let formatter = WaybarFormatter::new();

        let result = fetch_and_format(&fetcher, &formatter, wellington());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("connection refused"));
    }
}
