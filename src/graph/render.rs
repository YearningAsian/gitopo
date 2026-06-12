use super::types::{Connector, GraphRow};

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
