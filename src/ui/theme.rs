use ratatui::style::Color;

pub const COLOR_HEAD: Color = Color::Rgb(100, 220, 100);
pub const COLOR_LOCAL: Color = Color::Rgb(130, 180, 255);
pub const COLOR_REMOTE: Color = Color::Rgb(255, 180, 80);
pub const COLOR_SELECTED_BG: Color = Color::Rgb(40, 55, 75);
pub const COLOR_DIM: Color = Color::Rgb(120, 120, 130);
pub const COLOR_GRAPH_NODE: Color = Color::Rgb(180, 140, 255);
pub const COLOR_GRAPH_LINE: Color = Color::Rgb(80, 80, 100);
pub const COLOR_TAG: Color = Color::Rgb(255, 220, 80);
pub const COLOR_AHEAD: Color = Color::Rgb(100, 220, 180);
pub const COLOR_BEHIND: Color = Color::Rgb(255, 120, 100);
pub const COLOR_BORDER_FOCUSED: Color = Color::Rgb(100, 160, 255);
pub const COLOR_BORDER_UNFOCUSED: Color = Color::Rgb(60, 60, 80);
pub const COLOR_TITLE: Color = Color::Rgb(200, 200, 220);

/// Truncate `s` to at most `max` Unicode scalar values.
/// If truncated, the last character is replaced with '…'.
/// Uses byte-index arithmetic — no heap allocation until the result string.
pub fn truncate_str(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut iter = s.char_indices();
    match iter.nth(max.saturating_sub(1)) {
        // Fewer than `max` chars — fits as-is
        None => s.to_owned(),
        Some((byte_pos, _)) => {
            if iter.next().is_none() {
                // Exactly `max` chars — fits without truncation
                s.to_owned()
            } else {
                // More than `max` chars — show (max-1) chars + ellipsis
                let mut out = s[..byte_pos].to_owned();
                out.push('…');
                out
            }
        }
    }
}

/// Shorten a file-system path for display. Shows the last component prefixed
/// with "…/" when the full path exceeds `max` characters.
pub fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        return path.to_owned();
    }
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.len() <= 2 {
        return truncate_str(path, max);
    }
    let short = format!("…/{}", parts[parts.len() - 1]);
    if short.len() <= max {
        short
    } else {
        truncate_str(path, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_str_handles_fit_overflow_and_zero() {
        assert_eq!(truncate_str("hello", 5), "hello");
        assert_eq!(truncate_str("hello!", 5), "hell…");
        assert_eq!(truncate_str("anything", 0), "");
        // Counts chars, not bytes — must not split multibyte glyphs
        assert_eq!(truncate_str("héllo", 3), "hé…");
    }

    #[test]
    fn truncate_path_shortens_to_last_component() {
        assert_eq!(truncate_path("short", 10), "short");
        assert_eq!(truncate_path("aaaa/bbbb/cc", 8), "…/cc");
        assert_eq!(truncate_path("C:\\Users\\someone\\repo", 12), "…/repo");
    }
}
