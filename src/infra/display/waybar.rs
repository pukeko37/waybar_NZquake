//! Waybar output formatter for earthquake data.

use crate::app::QuakeFormatter;
use crate::domain::QuakeData;
use crate::infra::display::formatting::{
    day_band_header, find_max_local_mmi, format_quality, format_time, format_timestamp_footer,
    should_highlight,
};
use anyhow::Result;
use serde::Serialize;

/// Waybar JSON output format.
#[derive(Debug, Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
}

/// Formatter for creating Waybar JSON output from earthquake data.
pub struct WaybarFormatter;

impl WaybarFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format earthquake data into Waybar output.
    pub fn format(&self, quake_data: &QuakeData) -> Result<WaybarOutput> {
        let text = self.format_display_text(quake_data);
        let tooltip = self.format_tooltip(quake_data);

        Ok(WaybarOutput { text, tooltip })
    }

    /// Create error output for display when earthquake data is unavailable.
    /// Deliberately infallible — this is the fallback `main` reaches for
    /// when everything else has already failed, per the Waybar custom-module
    /// contract's "must emit valid JSON on stdout every invocation" rule.
    pub fn create_error_output(error: anyhow::Error) -> WaybarOutput {
        let text = "🌍 -- Earthquake data unavailable".to_string();
        let tooltip = format!(
            "Unable to fetch earthquake data\n\
             \n\
             Error: {}\n\
             Service: GeoNet API\n\
             \n\
             Last attempt: {}",
            error,
            time::OffsetDateTime::now_local()
                .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
                .format(&time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]"
                ))
                .unwrap_or_else(|_| "Unknown".to_string())
        );

        WaybarOutput { text, tooltip }
    }

    /// Format the main display text (icon + top earthquake or count).
    fn format_display_text(&self, quake_data: &QuakeData) -> String {
        if quake_data.earthquakes.is_empty() {
            return "🌍 No recent earthquakes".to_string();
        }

        let count = quake_data.earthquakes.len();
        let first_quake = &quake_data.earthquakes[0];
        let distance = first_quake.distance_km(&quake_data.user_location);
        let direction = first_quake.direction_from(&quake_data.user_location);

        format!(
            "🌍 M{:.1} MMI{:.1} {:.0}km {} ({} quakes)",
            first_quake.magnitude.value(),
            first_quake.local_mmi.value(),
            distance,
            direction,
            count
        )
    }

    /// Format the detailed tooltip information.
    fn format_tooltip(&self, quake_data: &QuakeData) -> String {
        if quake_data.earthquakes.is_empty() {
            return "No recent earthquakes recorded".to_string();
        }

        let user_location = quake_data.user_location;
        let mut lines = vec!["Recent NZ Earthquakes (by day band):\n".to_string()];

        let max_local_mmi = find_max_local_mmi(&quake_data.earthquakes);
        let mut current_day_band = None;

        for quake in &quake_data.earthquakes {
            let day_band = quake.day_band();

            if current_day_band != Some(day_band) {
                lines.push(day_band_header(day_band).to_string());
                current_day_band = Some(day_band);
            }

            let line = format_earthquake_line(quake, &user_location);
            if should_highlight(quake.local_mmi.value(), max_local_mmi) {
                lines.push(format!("<span foreground=\"#00FF00\">{}</span>", line));
            } else {
                lines.push(line);
            }
        }

        lines.push(format_timestamp_footer());
        lines.join("\n")
    }
}

impl Default for WaybarFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl QuakeFormatter for WaybarFormatter {
    type Output = WaybarOutput;

    fn format(&self, data: &QuakeData) -> Result<Self::Output, anyhow::Error> {
        self.format(data)
    }
}

/// Format a single earthquake line with all details.
fn format_earthquake_line(
    quake: &crate::domain::Earthquake,
    user_location: &crate::domain::Coordinates,
) -> String {
    let distance = quake.distance_km(user_location);
    let direction = quake.direction_from(user_location);
    let time_display = format_time(&quake.time);
    let quality_display = format_quality(quake.quality);
    let geonet_mmi_aside = quake
        .mmi
        .map(|m| format!(" (GeoNet: MMI{})", m.value()))
        .unwrap_or_default();

    format!(
        "  📅 {} | M{:.1} | MMI {:.1} local{} | 📏 {:.1}km | {}{} {:.0}km",
        time_display,
        quake.magnitude.value(),
        quake.local_mmi.value(),
        geonet_mmi_aside,
        quake.depth.value(),
        quality_display,
        direction,
        distance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Coordinates, Depth, Earthquake, Latitude, LocalMmi, Longitude, Magnitude, Mmi,
        QuakeQuality, QuakeTime,
    };

    fn wellington() -> Coordinates {
        Coordinates::new(
            Latitude::new(-41.2865).unwrap(),
            Longitude::new(174.7762).unwrap(),
        )
    }

    fn sample_quake(mmi: Option<Mmi>) -> Earthquake {
        Earthquake {
            time: QuakeTime::parse("2024-01-13T14:30:00.000Z").unwrap(),
            magnitude: Magnitude::new(5.2).unwrap(),
            depth: Depth::new(12.0).unwrap(),
            quality: QuakeQuality::Best,
            mmi,
            epicenter: wellington(),
            local_mmi: LocalMmi::new(4.2).unwrap(),
        }
    }

    #[test]
    fn test_empty_earthquakes_output() {
        let data = QuakeData {
            earthquakes: vec![],
            user_location: wellington(),
        };
        let output = WaybarFormatter::new().format(&data).unwrap();
        assert!(output.text.contains("No recent earthquakes"));
        assert!(output.tooltip.contains("No recent earthquakes recorded"));
    }

    #[test]
    fn test_display_text_with_earthquake() {
        let data = QuakeData {
            earthquakes: vec![sample_quake(None)],
            user_location: wellington(),
        };
        let output = WaybarFormatter::new().format(&data).unwrap();
        assert!(output.text.contains("M5.2"));
        assert!(output.text.contains("MMI4.2"));
        assert!(output.text.contains("1 quakes"));
    }

    #[test]
    fn test_tooltip_shows_local_mmi_and_omits_geonet_mmi_when_absent() {
        let data = QuakeData {
            earthquakes: vec![sample_quake(None)],
            user_location: wellington(),
        };
        let output = WaybarFormatter::new().format(&data).unwrap();
        assert!(output.tooltip.contains("MMI 4.2 local"));
        assert!(!output.tooltip.contains("GeoNet:"));
    }

    #[test]
    fn test_tooltip_shows_geonet_mmi_as_aside_when_present() {
        let data = QuakeData {
            earthquakes: vec![sample_quake(Mmi::new(6).ok())],
            user_location: wellington(),
        };
        let output = WaybarFormatter::new().format(&data).unwrap();
        assert!(output.tooltip.contains("MMI 4.2 local (GeoNet: MMI6)"));
    }

    #[test]
    fn test_error_output_formatting() {
        let error_output = WaybarFormatter::create_error_output(anyhow::anyhow!("Test error"));
        assert!(error_output.text.contains("unavailable"));
        assert!(error_output.tooltip.contains("Test error"));
    }
}
