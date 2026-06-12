use ratatui::style::Color;

/// Colour palette for the UI. When `enabled` is false every colour collapses
/// to [`Color::Reset`], so the TUI renders using the terminal's default
/// foreground/background — useful for non-colour terminals or when piping.
///
/// Construct with [`Theme::color`] or [`Theme::no_color`] and read the fields
/// directly. All fields are pre-resolved at construction time, so rendering
/// never branches on the colour mode.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub head: Color,
    pub local: Color,
    pub remote: Color,
    pub selected_bg: Color,
    pub dim: Color,
    pub graph_node: Color,
    pub graph_line: Color,
    pub tag: Color,
    pub ahead: Color,
    pub behind: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub title: Color,
    /// Foreground for emphasised text (commit messages, headings).
    pub text_strong: Color,
    /// Full commit hash colour in the detail pane.
    pub hash: Color,
    /// Background tint for the title bar.
    pub title_bar_bg: Color,
    /// Background tint for the status bar.
    pub status_bar_bg: Color,
    /// Foreground for the search-match marker in the branch list.
    pub search_match: Color,
    /// Background highlight for the active search match.
    pub search_active_bg: Color,
}

impl Theme {
    /// The full-colour palette used by default.
    pub fn color() -> Self {
        Self {
            head: Color::Rgb(100, 220, 100),
            local: Color::Rgb(130, 180, 255),
            remote: Color::Rgb(255, 180, 80),
            selected_bg: Color::Rgb(40, 55, 75),
            dim: Color::Rgb(120, 120, 130),
            graph_node: Color::Rgb(180, 140, 255),
            graph_line: Color::Rgb(80, 80, 100),
            tag: Color::Rgb(255, 220, 80),
            ahead: Color::Rgb(100, 220, 180),
            behind: Color::Rgb(255, 120, 100),
            border_focused: Color::Rgb(100, 160, 255),
            border_unfocused: Color::Rgb(60, 60, 80),
            title: Color::Rgb(200, 200, 220),
            text_strong: Color::White,
            hash: Color::Rgb(255, 200, 100),
            title_bar_bg: Color::Rgb(25, 28, 38),
            status_bar_bg: Color::Rgb(20, 22, 30),
            search_match: Color::Rgb(255, 230, 120),
            search_active_bg: Color::Rgb(80, 70, 30),
        }
    }

    /// A palette where every colour is [`Color::Reset`], i.e. no styling.
    /// Modifiers such as bold/underline still apply, so structure stays legible
    /// on monochrome terminals.
    pub fn no_color() -> Self {
        let r = Color::Reset;
        Self {
            head: r,
            local: r,
            remote: r,
            selected_bg: r,
            dim: r,
            graph_node: r,
            graph_line: r,
            tag: r,
            ahead: r,
            behind: r,
            border_focused: r,
            border_unfocused: r,
            title: r,
            text_strong: r,
            hash: r,
            title_bar_bg: r,
            status_bar_bg: r,
            search_match: r,
            search_active_bg: r,
        }
    }

    /// Select the palette for the given colour mode.
    pub fn new(no_color: bool) -> Self {
        if no_color {
            Self::no_color()
        } else {
            Self::color()
        }
    }
}

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

    #[test]
    fn no_color_theme_is_all_reset() {
        let t = Theme::no_color();
        assert_eq!(t.head, Color::Reset);
        assert_eq!(t.selected_bg, Color::Reset);
        assert_eq!(t.border_focused, Color::Reset);
        assert_eq!(t.search_active_bg, Color::Reset);
    }

    #[test]
    fn color_theme_is_not_reset() {
        let t = Theme::color();
        assert_ne!(t.head, Color::Reset);
        assert_ne!(t.border_focused, Color::Reset);
        assert_ne!(t.search_match, Color::Reset);
    }

    #[test]
    fn new_selects_palette_by_flag() {
        assert_eq!(Theme::new(true).head, Color::Reset);
        assert_ne!(Theme::new(false).head, Color::Reset);
    }
}
