use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::events::{key_to_action, Action, AppEvent};
use crate::git::{self, BranchInfo, RepoData};
use crate::graph::{build_graph, GraphRow};
use crate::ui::theme::Theme;

fn index_graph_rows(rows: &[GraphRow]) -> HashMap<git2::Oid, usize> {
    rows.iter().enumerate().map(|(i, r)| (r.oid, i)).collect()
}

/// Pure helper: decide how branch `idx` should be highlighted given the search
/// query, the list of matching branch indices, and the active match cursor.
/// Returns `None` when there is no active query / no matches, or when `idx`
/// is not among the matches.
fn compute_search_highlight(
    idx: usize,
    query: &str,
    matches: &[usize],
    active_idx: usize,
) -> Option<SearchHighlight> {
    if query.is_empty() || matches.is_empty() {
        return None;
    }
    if matches.get(active_idx).copied() == Some(idx) {
        Some(SearchHighlight::Active)
    } else if matches.contains(&idx) {
        Some(SearchHighlight::Match)
    } else {
        None
    }
}

/// Pure helper: given the current scroll `offset`, the `selected` row and the
/// visible `height`, return an offset that keeps `selected` within the viewport
/// while moving as little as possible. Shared by both scrollable panes so the
/// "keep selection visible" rule lives in exactly one place.
fn keep_selection_visible(offset: usize, selected: usize, height: usize) -> usize {
    if height == 0 {
        return offset;
    }
    if selected < offset {
        selected
    } else if selected >= offset + height {
        // selected + 1 - height; selected >= height here so no underflow.
        selected + 1 - height
    } else {
        offset
    }
}

/// Pure helper: branch indices whose (lowercased) name contains the query.
/// Mirrors the filtering used by interactive search so it can be unit-tested
/// without constructing a full [`App`].
fn matching_branch_indices(names: &[String], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    BranchList,
    CommitGraph,
    Help,
}

/// How a branch row should be highlighted with respect to the active search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchHighlight {
    /// A branch that matches the query but is not the current focus.
    Match,
    /// The branch the `n`/`N` cursor is currently sitting on.
    Active,
}

#[derive(Debug)]
pub struct App {
    pub repo_path: PathBuf,
    pub show_all: bool,
    pub max_commits: usize,
    /// Resolved colour palette (honours the `--no-color` flag).
    pub theme: Theme,

    pub repo_data: RepoData,
    pub graph_rows: Vec<GraphRow>,
    /// Lookup from commit OID to its index in `graph_rows`, rebuilt on reload
    /// so rendering doesn't have to reconstruct it every frame.
    pub graph_index: HashMap<git2::Oid, usize>,

    pub focus: Focus,

    // Branch list state
    pub branch_offset: usize,
    pub branch_selected: usize,

    // Commit graph state
    pub graph_offset: usize,
    pub graph_selected: usize,
    /// OIDs belonging to the currently-selected branch
    pub active_branch_oids: Vec<git2::Oid>,

    // Search
    pub search_mode: bool,
    pub search_query: String,
    pub search_matches: Vec<usize>, // indices into repo_data.branches
    pub search_match_idx: usize,

    pub status_msg: Option<String>,
}

impl App {
    pub fn new(
        repo_path: PathBuf,
        show_all: bool,
        max_commits: usize,
        no_color: bool,
    ) -> Result<Self> {
        let repo_data = git::load_repo(&repo_path, show_all, max_commits)?;
        let graph_rows = build_graph(
            &repo_data.topo_order,
            &repo_data.commits,
            &repo_data.oid_to_branches,
        );
        let graph_index = index_graph_rows(&graph_rows);

        let mut app = Self {
            repo_path,
            show_all,
            max_commits,
            theme: Theme::new(no_color),
            repo_data,
            graph_rows,
            graph_index,
            focus: Focus::BranchList,
            branch_offset: 0,
            branch_selected: 0,
            graph_offset: 0,
            graph_selected: 0,
            active_branch_oids: Vec::new(),
            search_mode: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: 0,
            status_msg: None,
        };

        app.select_branch(0);
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<()> {
        self.repo_data = git::load_repo(&self.repo_path, self.show_all, self.max_commits)?;
        self.graph_rows = build_graph(
            &self.repo_data.topo_order,
            &self.repo_data.commits,
            &self.repo_data.oid_to_branches,
        );
        self.graph_index = index_graph_rows(&self.graph_rows);
        let prev = self
            .branch_selected
            .min(self.repo_data.branches.len().saturating_sub(1));
        self.branch_selected = prev;
        self.branch_offset = self.branch_offset.min(self.branch_selected);
        self.select_branch(self.branch_selected);
        self.search_query.clear();
        self.search_matches.clear();
        self.status_msg = Some("Refreshed".into());
        Ok(())
    }

    /// Compute which commits belong to the selected branch (ancestry)
    fn select_branch(&mut self, idx: usize) {
        // Defensive `.get()` rather than `branches[idx]`: a stale index after a
        // reload that shrinks the branch list must clear the selection, not panic.
        let Some(branch) = self.repo_data.branches.get(idx) else {
            self.active_branch_oids.clear();
            return;
        };
        let tip = branch.tip_oid;

        // Walk topo_order keeping only ancestors of tip
        // We do a simple ancestry check: a commit is "on this branch" if it's
        // reachable from tip and (in the topo order) appears before any commit
        // that is ONLY reachable from other branches.
        // Simplified: include all commits reachable from tip (up to max_commits).
        let mut ancestors = std::collections::HashSet::new();
        let mut stack = vec![tip];
        while let Some(oid) = stack.pop() {
            if ancestors.contains(&oid) {
                continue;
            }
            if let Some(commit) = self.repo_data.commits.get(&oid) {
                ancestors.insert(oid);
                for &parent in &commit.parent_oids {
                    stack.push(parent);
                }
            }
        }

        self.active_branch_oids = self
            .repo_data
            .topo_order
            .iter()
            .filter(|oid| ancestors.contains(*oid))
            .copied()
            .collect();

        self.graph_selected = 0;
        self.graph_offset = 0;
    }

    pub fn selected_branch(&self) -> Option<&BranchInfo> {
        self.repo_data.branches.get(self.branch_selected)
    }

    /// Returns the highlight kind for a branch at `idx` given the current
    /// search state. `None` means the branch is not a search match (or there
    /// is no active query).
    pub fn search_highlight(&self, idx: usize) -> Option<SearchHighlight> {
        compute_search_highlight(
            idx,
            &self.search_query,
            &self.search_matches,
            self.search_match_idx,
        )
    }

    pub fn selected_commit_oid(&self) -> Option<git2::Oid> {
        self.active_branch_oids.get(self.graph_selected).copied()
    }

    pub fn handle_event(&mut self, event: AppEvent) -> Result<bool> {
        match event {
            AppEvent::Tick => {
                // Clear transient status after a few ticks — we use a simple approach:
                // status is cleared on next meaningful action
            }
            AppEvent::Key(key) => {
                if self.search_mode {
                    return Ok(self.handle_search_key(key));
                }

                if let Some(action) = key_to_action(key) {
                    if self.focus == Focus::Help {
                        if action == Action::Escape
                            || action == Action::Help
                            || action == Action::Quit
                        {
                            self.focus = Focus::BranchList;
                        }
                        return Ok(false);
                    }
                    return self.handle_action(action);
                }
            }
        }
        Ok(false)
    }

    fn handle_action(&mut self, action: Action) -> Result<bool> {
        self.status_msg = None;
        match action {
            Action::Quit => return Ok(true),
            Action::Help => self.focus = Focus::Help,
            Action::Refresh => self.reload()?,
            Action::ToggleAll => {
                self.show_all = !self.show_all;
                self.reload()?;
                self.status_msg = Some(if self.show_all {
                    "Showing all branches (local + remote)".into()
                } else {
                    "Showing local branches only".into()
                });
            }
            Action::Search => {
                self.search_mode = true;
                self.search_query.clear();
                self.search_matches.clear();
            }
            Action::SearchNext => self.advance_search(1),
            Action::SearchPrev => self.advance_search(-1),
            Action::ClearSearch => {
                self.search_query.clear();
                self.search_matches.clear();
            }
            Action::Escape => {
                if self.focus == Focus::CommitGraph {
                    self.focus = Focus::BranchList;
                } else {
                    self.search_query.clear();
                    self.search_matches.clear();
                }
            }
            Action::SelectBranch | Action::Enter => {
                if self.focus == Focus::BranchList {
                    self.focus = Focus::CommitGraph;
                }
            }

            // Navigation
            Action::MoveUp => self.move_selection(-1),
            Action::MoveDown => self.move_selection(1),
            Action::PageUp => self.move_selection(-10),
            Action::PageDown => self.move_selection(10),
            Action::Top => self.move_to(0),
            Action::Bottom => {
                let len = self.list_len();
                if len > 0 {
                    self.move_to(len - 1);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn list_len(&self) -> usize {
        match self.focus {
            Focus::BranchList | Focus::Help => self.repo_data.branches.len(),
            Focus::CommitGraph => self.active_branch_oids.len(),
        }
    }

    fn move_selection(&mut self, delta: i64) {
        match self.focus {
            Focus::BranchList => {
                let len = self.repo_data.branches.len();
                if len == 0 {
                    return;
                }
                let new = (self.branch_selected as i64 + delta).clamp(0, len as i64 - 1) as usize;
                if new != self.branch_selected {
                    self.branch_selected = new;
                    self.select_branch(new);
                }
            }
            Focus::CommitGraph => {
                let len = self.active_branch_oids.len();
                if len == 0 {
                    return;
                }
                self.graph_selected =
                    (self.graph_selected as i64 + delta).clamp(0, len as i64 - 1) as usize;
            }
            Focus::Help => {}
        }
    }

    fn move_to(&mut self, idx: usize) {
        match self.focus {
            Focus::BranchList => {
                let len = self.repo_data.branches.len();
                if len == 0 {
                    return;
                }
                self.branch_selected = idx.min(len - 1);
                self.select_branch(self.branch_selected);
            }
            Focus::CommitGraph => {
                let len = self.active_branch_oids.len();
                if len == 0 {
                    return;
                }
                self.graph_selected = idx.min(len - 1);
            }
            Focus::Help => {}
        }
    }

    fn handle_search_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.search_matches.clear();
            }
            KeyCode::Enter => {
                self.search_mode = false;
                if let Some(&first) = self.search_matches.first() {
                    self.branch_selected = first;
                    self.select_branch(first);
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search();
            }
            _ => {}
        }
        false
    }

    fn update_search(&mut self) {
        let names: Vec<String> = self
            .repo_data
            .branches
            .iter()
            .map(|b| b.name.clone())
            .collect();
        self.search_matches = matching_branch_indices(&names, &self.search_query);
        self.search_match_idx = 0;
        if let Some(&first) = self.search_matches.first() {
            self.branch_selected = first;
            self.select_branch(first);
        }
    }

    fn advance_search(&mut self, delta: i64) {
        if self.search_matches.is_empty() {
            return;
        }
        let len = self.search_matches.len();
        self.search_match_idx =
            (self.search_match_idx as i64 + delta).rem_euclid(len as i64) as usize;
        let idx = self.search_matches[self.search_match_idx];
        self.branch_selected = idx;
        self.select_branch(idx);
    }

    /// Ensure branch_offset keeps the selection visible given a viewport height
    pub fn scroll_branch_list(&mut self, viewport_height: usize) {
        self.branch_offset =
            keep_selection_visible(self.branch_offset, self.branch_selected, viewport_height);
    }

    /// Ensure graph_offset keeps the selection visible
    pub fn scroll_graph(&mut self, viewport_height: usize) {
        self.graph_offset =
            keep_selection_visible(self.graph_offset, self.graph_selected, viewport_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn matching_is_case_insensitive_substring() {
        let n = names(&["main", "feature/Auth", "release/payments"]);
        assert_eq!(matching_branch_indices(&n, "auth"), vec![1]);
        assert_eq!(matching_branch_indices(&n, "MAIN"), vec![0]);
        // Substring, not prefix
        assert_eq!(matching_branch_indices(&n, "ment"), vec![2]);
    }

    #[test]
    fn empty_query_matches_everything() {
        let n = names(&["a", "b", "c"]);
        assert_eq!(matching_branch_indices(&n, ""), vec![0, 1, 2]);
    }

    #[test]
    fn no_match_yields_empty() {
        let n = names(&["main", "dev"]);
        assert!(matching_branch_indices(&n, "zzz").is_empty());
    }

    #[test]
    fn highlight_none_without_query_or_matches() {
        assert_eq!(compute_search_highlight(0, "", &[0, 1], 0), None);
        assert_eq!(compute_search_highlight(0, "x", &[], 0), None);
    }

    #[test]
    fn highlight_marks_active_and_other_matches() {
        let matches = vec![2, 5, 7];
        // active cursor on the second match (index 5)
        assert_eq!(
            compute_search_highlight(5, "x", &matches, 1),
            Some(SearchHighlight::Active)
        );
        // other matched branches are plain matches
        assert_eq!(
            compute_search_highlight(2, "x", &matches, 1),
            Some(SearchHighlight::Match)
        );
        assert_eq!(
            compute_search_highlight(7, "x", &matches, 1),
            Some(SearchHighlight::Match)
        );
        // a non-matching branch gets nothing
        assert_eq!(compute_search_highlight(3, "x", &matches, 1), None);
    }

    #[test]
    fn keep_visible_scrolls_to_follow_selection() {
        // Already in view: offset unchanged.
        assert_eq!(keep_selection_visible(0, 3, 10), 0);
        // Selection above the window: snap offset up to it.
        assert_eq!(keep_selection_visible(5, 2, 10), 2);
        // Selection below the window: scroll just enough to reveal it.
        assert_eq!(keep_selection_visible(0, 12, 10), 3);
        // Exactly at the bottom edge stays put.
        assert_eq!(keep_selection_visible(0, 9, 10), 0);
        // First row past the bottom edge scrolls by one.
        assert_eq!(keep_selection_visible(0, 10, 10), 1);
    }

    #[test]
    fn keep_visible_zero_height_is_noop() {
        // A collapsed pane must not panic or move the offset.
        assert_eq!(keep_selection_visible(7, 3, 0), 7);
    }

    #[test]
    fn highlight_handles_out_of_range_active_idx() {
        // active_idx past the end must not panic and must not mark Active
        let matches = vec![1, 4];
        assert_eq!(
            compute_search_highlight(1, "x", &matches, 9),
            Some(SearchHighlight::Match)
        );
    }
}
