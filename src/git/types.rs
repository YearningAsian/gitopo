use git2::Oid;
use std::collections::HashMap;

pub const MAX_COMMITS_CAP: usize = 100_000;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub tip_oid: Oid,
    pub tip_time: i64,
    pub ahead_behind: Option<(usize, usize)>,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: Oid,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
    pub parent_oids: Vec<Oid>,
}

#[derive(Debug)]
pub struct RepoData {
    pub branches: Vec<BranchInfo>,
    pub commits: HashMap<Oid, CommitInfo>,
    pub topo_order: Vec<Oid>,
    pub oid_to_branches: HashMap<Oid, Vec<String>>,
    pub repo_path: String,
}
