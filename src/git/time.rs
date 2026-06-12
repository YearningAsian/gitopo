pub fn format_relative_time_with_now(seconds: i64, now: i64) -> String {
    let diff = now - seconds;

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
}
