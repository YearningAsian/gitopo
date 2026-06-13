use super::types::{Connector, GraphRow};

/// Semantic class of a single glyph in a rendered graph prefix. Lets the UI
/// layer pick a colour without hard-coding the glyph set in two places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind {
    /// The commit marker `●`.
    Node,
    /// A connector line (`│`, box-drawing, curves).
    Line,
    /// Anything else (spaces, padding).
    Other,
}

/// Classify a prefix glyph so callers don't duplicate the glyph→meaning map.
pub fn classify_glyph(ch: char) -> GlyphKind {
    match ch {
        '●' => GlyphKind::Node,
        '│' | '├' | '┤' | '┬' | '┴' | '┼' | '╮' | '╭' | '╯' | '╰' | '─' => {
            GlyphKind::Line
        }
        _ => GlyphKind::Other,
    }
}

/// Returns the ASCII-art graph prefix string for a single commit row,
/// e.g. `"● │ │ "` for a commit in lane 0 with two other active lanes.
pub fn render_graph_prefix(row: &GraphRow) -> String {
    let width = row.num_cols.max(row.col + 1);
    let mut s = String::with_capacity(width * 2);

    for i in 0..width {
        if i == row.col {
            s.push('●');
        } else if row
            .connectors
            .get(i)
            .is_some_and(|c| *c == Connector::Vertical)
        {
            s.push('│');
        } else {
            s.push(' ');
        }
        s.push(' ');
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_glyph_buckets_node_line_and_other() {
        assert_eq!(classify_glyph('●'), GlyphKind::Node);
        assert_eq!(classify_glyph('│'), GlyphKind::Line);
        assert_eq!(classify_glyph('╭'), GlyphKind::Line);
        assert_eq!(classify_glyph('─'), GlyphKind::Line);
        assert_eq!(classify_glyph(' '), GlyphKind::Other);
        assert_eq!(classify_glyph('x'), GlyphKind::Other);
    }
}
