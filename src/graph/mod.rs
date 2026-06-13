mod render;
mod types;

pub use render::{classify_glyph, render_graph_prefix, GlyphKind};
pub use types::{Connector, GraphRow};

use crate::git::CommitInfo;
use git2::Oid;
use std::collections::HashMap;

pub fn build_graph(
    topo_order: &[Oid],
    commits: &HashMap<Oid, CommitInfo>,
    oid_to_branches: &HashMap<Oid, Vec<String>>,
) -> Vec<GraphRow> {
    if topo_order.is_empty() {
        return Vec::new();
    }

    let mut lanes: Vec<Option<Oid>> = Vec::new();
    let mut rows = Vec::with_capacity(topo_order.len());

    for &oid in topo_order {
        let commit = match commits.get(&oid) {
            Some(c) => c,
            None => continue,
        };

        let col = find_or_assign_lane(&mut lanes, oid);
        let num_cols = lanes.iter().filter(|l| l.is_some()).count().max(col + 1);
        let connectors = build_connectors(&lanes, col, commit);

        // Suppress unused warning: oid_to_branches is intentionally not stored on
        // GraphRow (the UI reads labels directly from RepoData.oid_to_branches).
        let _ = oid_to_branches;

        update_lanes(&mut lanes, col, commit);

        rows.push(GraphRow {
            oid,
            col,
            num_cols,
            connectors,
        });
    }

    rows
}

fn find_or_assign_lane(lanes: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    // Collapse every lane waiting for this OID into the leftmost one.
    // Without clearing duplicates, converging branches draw phantom │ lines.
    let mut found: Option<usize> = None;
    for (i, lane) in lanes.iter_mut().enumerate() {
        if *lane == Some(oid) {
            if found.is_none() {
                found = Some(i);
            } else {
                *lane = None;
            }
        }
    }
    if let Some(i) = found {
        return i;
    }
    for (i, lane) in lanes.iter_mut().enumerate() {
        if lane.is_none() {
            *lane = Some(oid);
            return i;
        }
    }
    lanes.push(Some(oid));
    lanes.len() - 1
}

fn build_connectors(
    lanes: &[Option<Oid>],
    commit_col: usize,
    commit: &CommitInfo,
) -> Vec<Connector> {
    let width = lanes.len().max(commit_col + 1);
    let mut connectors = vec![Connector::Empty; width];

    for (i, lane) in lanes.iter().enumerate() {
        if lane.is_some() && i != commit_col {
            connectors[i] = Connector::Vertical;
        }
    }

    if !commit.parent_oids.is_empty() {
        connectors[commit_col] = Connector::Vertical;
    }

    connectors
}

fn update_lanes(lanes: &mut Vec<Option<Oid>>, col: usize, commit: &CommitInfo) {
    let parents = &commit.parent_oids;

    if parents.is_empty() {
        if col < lanes.len() {
            lanes[col] = None;
        }
        return;
    }

    lanes[col] = Some(parents[0]);

    for &extra_parent in &parents[1..] {
        if !lanes.contains(&Some(extra_parent)) {
            let placed = lanes.iter_mut().find(|l| l.is_none());
            if let Some(slot) = placed {
                *slot = Some(extra_parent);
            } else {
                lanes.push(Some(extra_parent));
            }
        }
    }

    while lanes.last() == Some(&None) {
        lanes.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> Oid {
        Oid::from_str(&format!("{:040x}", n)).unwrap()
    }

    fn commit(o: Oid, parents: Vec<Oid>) -> CommitInfo {
        CommitInfo {
            oid: o,
            short_id: String::new(),
            message: String::new(),
            author: String::new(),
            time: 0,
            parent_oids: parents,
        }
    }

    fn graph_for(chain: Vec<CommitInfo>) -> Vec<GraphRow> {
        let topo: Vec<Oid> = chain.iter().map(|c| c.oid).collect();
        let commits: HashMap<Oid, CommitInfo> = chain.into_iter().map(|c| (c.oid, c)).collect();
        build_graph(&topo, &commits, &HashMap::new())
    }

    #[test]
    fn empty_history_yields_no_rows() {
        assert!(graph_for(Vec::new()).is_empty());
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let rows = graph_for(vec![
            commit(oid(3), vec![oid(2)]),
            commit(oid(2), vec![oid(1)]),
            commit(oid(1), vec![]),
        ]);
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.col, 0);
            assert_eq!(row.num_cols, 1);
        }
    }

    #[test]
    fn merge_opens_a_second_lane() {
        let rows = graph_for(vec![
            commit(oid(4), vec![oid(3), oid(2)]),
            commit(oid(3), vec![oid(1)]),
            commit(oid(2), vec![oid(1)]),
            commit(oid(1), vec![]),
        ]);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].col, 0);
        assert_eq!(rows[1].col, 0);
        assert_eq!(rows[2].col, 1);
        assert_eq!(rows[3].col, 0);
    }

    #[test]
    fn converging_lanes_collapse_at_shared_ancestor() {
        let rows = graph_for(vec![
            commit(oid(4), vec![oid(3), oid(2)]),
            commit(oid(3), vec![oid(1)]),
            commit(oid(2), vec![oid(1)]),
            commit(oid(1), vec![]),
        ]);
        let root_row = &rows[3];
        assert_eq!(root_row.num_cols, 1, "stale duplicate lane survived");
        assert!(root_row
            .connectors
            .iter()
            .skip(1)
            .all(|c| *c == Connector::Empty));
    }

    #[test]
    fn render_prefix_marks_commit_and_connectors() {
        use super::render::render_graph_prefix;
        let row = GraphRow {
            oid: oid(1),
            col: 0,
            num_cols: 2,
            connectors: vec![Connector::Vertical, Connector::Vertical],
        };
        assert_eq!(render_graph_prefix(&row), "● │ ");
    }
}
