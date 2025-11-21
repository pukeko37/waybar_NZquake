//! Waybar output formatter for earthquake data with functional composition.

use crate::api::models::QuakeData;
use anyhow::Result;
use serde::Serialize;

/// Waybar JSON output format
#[derive(Debug, Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
}

/// Formatter for creating Waybar JSON output from earthquake data
pub struct WaybarFormatter;

impl WaybarFormatter {
    /// Create a new Waybar formatter
    pub fn new() -> Self {
        Self
    }

    /// Format earthquake data into Waybar output
    pub fn format(&self, quake_data: &QuakeData) -> Result<WaybarOutput> {
        let text = self.format_display_text(quake_data);
        let tooltip = self.format_tooltip(quake_data);

        Ok(WaybarOutput { text, tooltip })
    }

    /// Create error output for display when earthquake data is unavailable
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
            time::OffsetDateTime::now_utc()
                .format(&time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]Z"
                ))
                .unwrap_or_else(|_| "Unknown".to_string())
        );

        WaybarOutput { text, tooltip }
    }

    /// Format the main display text (icon + count or top earthquake)
    fn format_display_text(&self, quake_data: &QuakeData) -> String {
        if quake_data.earthquakes.is_empty() {
            return "🌍 No recent earthquakes".to_string();
        }

        let count = quake_data.earthquakes.len();
        if let Some(first_quake) = quake_data.earthquakes.first() {
            let (user_lat, user_lon) = quake_data.user_location;
            let distance = first_quake.horizontal_distance_from(user_lat, user_lon);
            let direction = first_quake.direction_from(user_lat, user_lon);

            format!(
                "🌍 M{:.1} {:.0}km {} ({} quakes)",
                first_quake.magnitude, distance, direction, count
            )
        } else {
            format!("🌍 {} recent earthquakes", count)
        }
    }

    /// Format the detailed tooltip information
    fn format_tooltip(&self, quake_data: &QuakeData) -> String {
        if quake_data.earthquakes.is_empty() {
            return "No recent earthquakes recorded".to_string();
        }

        let (user_lat, user_lon) = quake_data.user_location;
        let mut lines = vec!["Recent NZ Earthquakes (by day band):\n".to_string()];

        // Find the highest score
        let max_score = quake_data
            .earthquakes
            .iter()
            .map(|eq| eq.score)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let mut current_day_band: Option<u32> = None;

        for quake in &quake_data.earthquakes {
            let age_days = quake.age_in_days();
            let day_band = get_day_band(age_days);

            // Add day band header if changed
            if current_day_band != Some(day_band) {
                let band_label = match day_band {
                    0 => "⏰ Last 24 hours:",
                    1 => "\n⏰ 1-2 days ago:",
                    2 => "\n⏰ 2-4 days ago:",
                    3 => "\n⏰ 4-8 days ago:",
                    _ => "\n⏰ 8+ days ago:",
                };
                lines.push(band_label.to_string());
                current_day_band = Some(day_band);
            }

            let distance = quake.horizontal_distance_from(user_lat, user_lon);
            let direction = quake.direction_from(user_lat, user_lon);

            // Format time (remove the .milliseconds and Z)
            let time_display = if let Some(t_pos) = quake.time.find('T') {
                let time_part = &quake.time[t_pos + 1..];
                let time_clean = time_part.split('.').next().unwrap_or(time_part);
                format!("{} {}", &quake.time[..t_pos], time_clean)
            } else {
                quake.time.clone()
            };

            // Only show quality if it's not "best" (to reduce noise)
            let quality_display = if quake.quality == "best" {
                String::new()
            } else {
                format!("🎯 {} | ", quake.quality)
            };

            let line = format!(
                "  📅 {} | M{:.1} | 📏 {:.1}km | {}{} {:.0}km | MMI {}",
                time_display,
                quake.magnitude,
                quake.depth,
                quality_display,
                direction,
                distance,
                quake.mmi.map(|m| m.to_string()).unwrap_or("-".to_string())
            );

            // Highlight the highest scoring event in green
            if (quake.score - max_score).abs() < 0.001 {
                lines.push(format!("<span foreground=\"#00FF00\">{}</span>", line));
            } else {
                lines.push(line);
            }
        }

        lines.push(format!(
            "\n\n🕐 Updated: {}",
            time::OffsetDateTime::now_utc()
                .format(&time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]Z"
                ))
                .unwrap_or_else(|_| "Unknown".to_string())
        ));

        lines.join("\n")
    }
}

impl Default for WaybarFormatter {
    fn default() -> Self {
        Self::new()
    }
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
