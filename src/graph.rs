/// Builds an ASCII-art graph column structure for commit history rendering.
/// Each commit gets a column position; merge commits show connecting lines.
use crate::git::CommitInfo;
use git2::Oid;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GraphRow {
    pub oid: Oid,
    /// Which column this commit sits in
    pub col: usize,
    /// Number of columns in this row
    pub num_cols: usize,
    /// Connector lines to draw below this row (col_index -> ConnectorKind)
    pub connectors: Vec<Connector>,
    /// Branch labels to display (branch names pointing here)
    #[allow(dead_code)]
    pub branch_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Connector {
    Vertical,
    VerticalRight,
    VerticalLeft,
    #[allow(dead_code)]
    MergeLeft,
    #[allow(dead_code)]
    MergeRight,
    #[allow(dead_code)]
    Horizontal,
    #[allow(dead_code)]
    BranchDown,
    Empty,
}

pub fn build_graph(
    topo_order: &[Oid],
    commits: &HashMap<Oid, CommitInfo>,
    oid_to_branches: &HashMap<Oid, Vec<String>>,
) -> Vec<GraphRow> {
    if topo_order.is_empty() {
        return Vec::new();
    }

    // Active lanes: each lane holds the OID it's "waiting for" (its child's parent)
    let mut lanes: Vec<Option<Oid>> = Vec::new();
    let mut rows = Vec::with_capacity(topo_order.len());

    for &oid in topo_order {
        let commit = match commits.get(&oid) {
            Some(c) => c,
            None => continue,
        };

        // Find or assign a lane for this commit
        let col = find_or_assign_lane(&mut lanes, oid);

        let branch_labels = oid_to_branches
            .get(&oid)
            .cloned()
            .unwrap_or_default();

        let num_cols = lanes.iter().filter(|l| l.is_some()).count().max(col + 1);

        // Compute connectors (simplified: vertical lines for active lanes)
        let connectors = build_connectors(&lanes, col, commit);

        // Update lanes: replace this commit's slot with its first parent,
        // add extra parents to new lanes
        update_lanes(&mut lanes, col, commit);

        rows.push(GraphRow {
            oid,
            col,
            num_cols,
            connectors,
            branch_labels,
        });
    }

    rows
}

fn find_or_assign_lane(lanes: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    // Look for a lane already waiting for this OID
    for (i, lane) in lanes.iter().enumerate() {
        if *lane == Some(oid) {
            return i;
        }
    }
    // No lane waiting — assign leftmost free lane
    for (i, lane) in lanes.iter_mut().enumerate() {
        if lane.is_none() {
            *lane = Some(oid);
            return i;
        }
    }
    lanes.push(Some(oid));
    lanes.len() - 1
}

fn build_connectors(lanes: &[Option<Oid>], commit_col: usize, commit: &CommitInfo) -> Vec<Connector> {
    let width = lanes.len().max(commit_col + 1);
    let mut connectors = vec![Connector::Empty; width];

    for (i, lane) in lanes.iter().enumerate() {
        if lane.is_some() && i != commit_col {
            connectors[i] = Connector::Vertical;
        }
    }

    // If commit has parents, show continuation
    if !commit.parent_oids.is_empty() {
        connectors[commit_col] = Connector::Vertical;
    }

    connectors
}

fn update_lanes(lanes: &mut Vec<Option<Oid>>, col: usize, commit: &CommitInfo) {
    let parents = &commit.parent_oids;

    if parents.is_empty() {
        // Root commit — free the lane
        if col < lanes.len() {
            lanes[col] = None;
        }
        return;
    }

    // First parent continues in the same lane
    lanes[col] = Some(parents[0]);

    // Additional parents (merge commits) get new lanes
    for &extra_parent in &parents[1..] {
        // Check if any existing lane already tracks this parent
        let already_tracked = lanes.iter().any(|l| *l == Some(extra_parent));
        if !already_tracked {
            // Find a free slot or append
            let placed = lanes.iter_mut().enumerate().find(|(_, l)| l.is_none());
            if let Some((_, slot)) = placed {
                *slot = Some(extra_parent);
            } else {
                lanes.push(Some(extra_parent));
            }
        }
    }

    // Compact trailing None lanes
    while lanes.last() == Some(&None) {
        lanes.pop();
    }
}

/// Render the graph column prefix string for a given row
pub fn render_graph_prefix(row: &GraphRow) -> String {
    let width = row.num_cols.max(row.col + 1);
    let mut s = String::with_capacity(width * 2);

    for i in 0..width {
        if i == row.col {
            s.push('●');
        } else {
            // Check if there's a vertical connector
            let has_connector = row.connectors.get(i).map_or(false, |c| {
                *c == Connector::Vertical
                    || *c == Connector::VerticalRight
                    || *c == Connector::VerticalLeft
            });
            if has_connector {
                s.push('│');
            } else {
                s.push(' ');
            }
        }
        s.push(' ');
    }

    s
}

#[allow(dead_code)]
pub fn render_connector_prefix(row: &GraphRow) -> String {
    let width = row.num_cols.max(row.col + 1);
    let mut s = String::with_capacity(width * 2);

    for i in 0..width {
        let c = row.connectors.get(i).unwrap_or(&Connector::Empty);
        match c {
            Connector::Vertical | Connector::VerticalRight | Connector::VerticalLeft => {
                s.push('│');
            }
            _ => {
                s.push(' ');
            }
        }
        s.push(' ');
    }

    s
}
