use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::events::{key_to_action, Action, AppEvent};
use crate::git::{self, BranchInfo, RepoData};
use crate::graph::{build_graph, GraphRow};

fn index_graph_rows(rows: &[GraphRow]) -> HashMap<git2::Oid, usize> {
    rows.iter().enumerate().map(|(i, r)| (r.oid, i)).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    BranchList,
    CommitGraph,
    Help,
}

#[derive(Debug)]
pub struct App {
    pub repo_path: PathBuf,
    pub show_all: bool,
    pub max_commits: usize,

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
    pub fn new(repo_path: PathBuf, show_all: bool, max_commits: usize) -> Result<Self> {
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
        if self.repo_data.branches.is_empty() {
            self.active_branch_oids.clear();
            return;
        }

        let branch = &self.repo_data.branches[idx];
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
        let q = self.search_query.to_lowercase();
        self.search_matches = self
            .repo_data
            .branches
            .iter()
            .enumerate()
            .filter(|(_, b)| b.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
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
        if viewport_height == 0 {
            return;
        }
        if self.branch_selected < self.branch_offset {
            self.branch_offset = self.branch_selected;
        } else if self.branch_selected >= self.branch_offset + viewport_height {
            self.branch_offset = self.branch_selected + 1 - viewport_height;
        }
    }

    /// Ensure graph_offset keeps the selection visible
    pub fn scroll_graph(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.graph_selected < self.graph_offset {
            self.graph_offset = self.graph_selected;
        } else if self.graph_selected >= self.graph_offset + viewport_height {
            self.graph_offset = self.graph_selected + 1 - viewport_height;
        }
    }
}
