pub fn format_relative_time_with_now(seconds: i64, now: i64) -> String {
    // Saturating: a corrupt or hostile commit can carry an extreme timestamp
    // (e.g. `i64::MIN`); `now - seconds` would otherwise overflow and panic in
    // debug builds. Saturation degrades gracefully to a huge "…y ago" instead.
    let diff = now.saturating_sub(seconds);

    if diff < 0 {
        return "just now".to_string();
    }

    match diff {
        0..=59 => format!("{}s ago", diff),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        86400..=2591999 => format!("{}d ago", diff / 86400),
        2592000..=31535999 => format!("{}mo ago", diff / 2592000),
        _ => format!("{}y ago", diff / 31536000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_boundaries() {
        assert_eq!(format_relative_time_with_now(100, 50), "just now");
        assert_eq!(format_relative_time_with_now(100, 100), "0s ago");
        assert_eq!(format_relative_time_with_now(0, 59), "59s ago");
        assert_eq!(format_relative_time_with_now(0, 60), "1m ago");
        assert_eq!(format_relative_time_with_now(0, 3_599), "59m ago");
        assert_eq!(format_relative_time_with_now(0, 3_600), "1h ago");
        assert_eq!(format_relative_time_with_now(0, 86_399), "23h ago");
        assert_eq!(format_relative_time_with_now(0, 86_400), "1d ago");
        assert_eq!(format_relative_time_with_now(0, 2_591_999), "29d ago");
        assert_eq!(format_relative_time_with_now(0, 2_592_000), "1mo ago");
        assert_eq!(format_relative_time_with_now(0, 31_536_000), "1y ago");
    }

    #[test]
    fn extreme_timestamps_do_not_overflow() {
        // Hostile/corrupt commit times must not panic via i64 over/underflow.
        assert_eq!(
            format_relative_time_with_now(i64::MIN, 0),
            format!("{}y ago", i64::MAX / 31_536_000)
        );
        // A far-future commit time saturates to "just now" rather than panicking.
        assert_eq!(
            format_relative_time_with_now(i64::MAX, i64::MIN),
            "just now"
        );
    }
}
