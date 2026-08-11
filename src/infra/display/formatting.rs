//! Presentation helpers producing Pango markup and plain text — concerns
//! that belong in the display layer rather than the domain.

use crate::domain::{DayBand, Earthquake, QuakeQuality, QuakeTime};

/// Day-band header shown above each group of quakes in the tooltip.
pub fn day_band_header(day_band: DayBand) -> &'static str {
    match day_band {
        DayBand::Today => "⏰ Last 24 hours:",
        DayBand::OneToTwoDays => "\n⏰ 1-2 days ago:",
        DayBand::TwoToFourDays => "\n⏰ 2-4 days ago:",
        DayBand::FourToEightDays => "\n⏰ 4-8 days ago:",
        DayBand::Older => "\n⏰ 8+ days ago:",
    }
}

/// Quality display, hiding `Best` to reduce noise (it's the common case).
pub fn format_quality(quality: QuakeQuality) -> String {
    match quality {
        QuakeQuality::Best => String::new(),
        other => format!("🎯 {} | ", other),
    }
}

/// Format a quake timestamp by converting from UTC to system local time.
pub fn format_time(time: &QuakeTime) -> String {
    use time::UtcOffset;

    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let local_time = time.as_offset_date_time().to_offset(local_offset);

    local_time
        .format(&time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .unwrap_or_else(|_| "Unknown".to_string())
}

/// Whether a quake's score is (approximately) the highest in the batch —
/// used to highlight the top entry in the tooltip.
pub fn should_highlight(score: f64, max_score: f64) -> bool {
    (score - max_score).abs() < 0.001
}

/// Find the maximum score among all earthquakes.
pub fn find_max_score(earthquakes: &[Earthquake]) -> f64 {
    earthquakes
        .iter()
        .map(|eq| eq.score)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}

/// Format the "Updated: …" trailer shown at the end of the tooltip.
pub fn format_timestamp_footer() -> String {
    format!(
        "\n\n🕐 Updated: {}",
        time::OffsetDateTime::now_local()
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
            .format(&time::macros::format_description!(
                "[year]-[month]-[day] [hour]:[minute]"
            ))
            .unwrap_or_else(|_| "Unknown".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_band_header() {
        assert_eq!(day_band_header(DayBand::Today), "⏰ Last 24 hours:");
        assert!(day_band_header(DayBand::Older).contains("8+ days ago"));
    }

    #[test]
    fn test_format_quality_hides_best() {
        assert_eq!(format_quality(QuakeQuality::Best), "");
        assert!(format_quality(QuakeQuality::Automatic).contains("automatic"));
    }

    #[test]
    fn test_should_highlight() {
        assert!(should_highlight(5.0, 5.0));
        assert!(!should_highlight(4.0, 5.0));
    }
}
