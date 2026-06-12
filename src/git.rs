use anyhow::{Context, Result};
use git2::{BranchType, Repository, Sort};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    #[allow(dead_code)]
    pub remote_name: Option<String>,
    pub tip_oid: git2::Oid,
    #[allow(dead_code)]
    pub tip_message: String,
    #[allow(dead_code)]
    pub tip_author: String,
    pub tip_time: i64,
    pub ahead_behind: Option<(usize, usize)>,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: git2::Oid,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
    pub parent_oids: Vec<git2::Oid>,
}

#[derive(Debug)]
pub struct RepoData {
    pub branches: Vec<BranchInfo>,
    pub commits: HashMap<git2::Oid, CommitInfo>,
    pub topo_order: Vec<git2::Oid>,
    pub oid_to_branches: HashMap<git2::Oid, Vec<String>>,
    #[allow(dead_code)]
    pub head_branch: Option<String>,
    pub repo_path: String,
}

const MAX_COMMITS_CAP: usize = 100_000;

pub fn load_repo(path: &Path, show_all: bool, max_commits: usize) -> Result<RepoData> {
    let repo = Repository::discover(path)
        .with_context(|| format!("No git repository found at {}", path.display()))?;

    let max_commits = max_commits.min(MAX_COMMITS_CAP);

    let head_branch = get_head_branch(&repo);

    let mut branches = collect_branches(&repo, show_all, head_branch.as_deref())?;

    // Collect all tip OIDs to walk from
    let tip_oids: Vec<git2::Oid> = branches.iter().map(|b| b.tip_oid).collect();

    let (commits, topo_order) = walk_commits(&repo, &tip_oids, max_commits)?;

    // Build reverse map: oid -> branch names
    let mut oid_to_branches: HashMap<git2::Oid, Vec<String>> = HashMap::new();
    for branch in &branches {
        oid_to_branches
            .entry(branch.tip_oid)
            .or_default()
            .push(branch.name.clone());
    }

    // Compute ahead/behind for local branches with upstreams
    for branch in &mut branches {
        if !branch.is_remote {
            branch.ahead_behind = compute_ahead_behind(&repo, branch);
        }
    }

    // Sort branches: HEAD first, then locals alphabetically, then remotes
    branches.sort_by(|a, b| {
        if a.is_head {
            return std::cmp::Ordering::Less;
        }
        if b.is_head {
            return std::cmp::Ordering::Greater;
        }
        a.is_remote.cmp(&b.is_remote).then(a.name.cmp(&b.name))
    });

    let repo_path = repo
        .workdir()
        .or_else(|| repo.path().parent())
        .unwrap_or_else(|| repo.path())
        .to_string_lossy()
        .into_owned();

    Ok(RepoData {
        branches,
        commits,
        topo_order,
        oid_to_branches,
        head_branch,
        repo_path,
    })
}

fn get_head_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    if head.is_branch() {
        head.shorthand().map(|s| s.to_owned())
    } else {
        None
    }
}

fn collect_branches(
    repo: &Repository,
    show_all: bool,
    head_name: Option<&str>,
) -> Result<Vec<BranchInfo>> {
    let filter = if show_all {
        None
    } else {
        Some(BranchType::Local)
    };

    let mut branches = Vec::new();

    for branch_result in repo.branches(filter)? {
        let (branch, branch_type) = branch_result?;
        let name = match branch.name()? {
            Some(n) => n.to_owned(),
            None => continue,
        };

        let reference = match branch.get().resolve() {
            Ok(r) => r,
            Err(_) => continue,
        };

        let tip_oid = match reference.target() {
            Some(oid) => oid,
            None => continue,
        };

        let commit = match repo.find_commit(tip_oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let is_remote = branch_type == BranchType::Remote;
        let remote_name = if is_remote {
            name.split('/').next().map(|s| s.to_owned())
        } else {
            None
        };

        // Compare branch names, not OIDs, to avoid false positives when multiple
        // local branches share a tip commit
        let is_head = !is_remote && head_name == Some(name.as_str());

        let tip_message = commit.summary().unwrap_or("").chars().take(80).collect();

        let tip_author = commit.author().name().unwrap_or("?").to_owned();
        let tip_time = commit.time().seconds();

        let upstream = if !is_remote {
            branch
                .upstream()
                .ok()
                .and_then(|u| u.name().ok().flatten().map(|s| s.to_owned()))
        } else {
            None
        };

        branches.push(BranchInfo {
            name,
            is_head,
            is_remote,
            remote_name,
            tip_oid,
            tip_message,
            tip_author,
            tip_time,
            ahead_behind: None,
            upstream,
        });
    }

    Ok(branches)
}

fn walk_commits(
    repo: &Repository,
    tip_oids: &[git2::Oid],
    max_commits: usize,
) -> Result<(HashMap<git2::Oid, CommitInfo>, Vec<git2::Oid>)> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    for &oid in tip_oids {
        let _ = revwalk.push(oid);
    }

    let mut commits = HashMap::new();
    let mut topo_order = Vec::new();

    for oid_result in revwalk.take(max_commits) {
        let oid = oid_result?;

        let commit = repo.find_commit(oid)?;

        let short_id = repo
            .find_object(oid, None)
            .and_then(|o| o.short_id())
            .map(|b| b.as_str().unwrap_or("???????").to_owned())
            .unwrap_or_else(|_| format!("{:.7}", oid));

        let message = commit.summary().unwrap_or("").chars().take(80).collect();

        let author = commit.author().name().unwrap_or("?").to_owned();
        let time = commit.time().seconds();
        let parent_oids: Vec<git2::Oid> = commit.parent_ids().collect();

        topo_order.push(oid);
        commits.insert(
            oid,
            CommitInfo {
                oid,
                short_id,
                message,
                author,
                time,
                parent_oids,
            },
        );
    }

    Ok((commits, topo_order))
}

fn compute_ahead_behind(repo: &Repository, branch: &BranchInfo) -> Option<(usize, usize)> {
    let upstream_name = branch.upstream.as_ref()?;
    let upstream_ref = repo
        .find_reference(&format!("refs/remotes/{}", upstream_name))
        .ok()?;
    let upstream_oid = upstream_ref.target()?;
    let (ahead, behind) = repo.graph_ahead_behind(branch.tip_oid, upstream_oid).ok()?;
    Some((ahead, behind))
}

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

#[allow(dead_code)]
pub fn format_relative_time(seconds: i64) -> String {
    format_relative_time_with_now(seconds, chrono::Utc::now().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_boundaries() {
        // Commit timestamp in the future (clock skew) must not underflow
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

    #[test]
    fn load_repo_reads_branches_and_commits() {
        let dir = std::env::temp_dir().join(format!(
            "gitopo-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        {
            let repo = Repository::init(&dir).unwrap();
            let sig = git2::Signature::now("Test", "test@example.com").unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            let first = repo
                .commit(Some("HEAD"), &sig, &sig, "first", &tree, &[])
                .unwrap();
            let parent = repo.find_commit(first).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        let data = load_repo(&dir, false, 100).unwrap();
        assert_eq!(data.branches.len(), 1);
        assert!(data.branches[0].is_head);
        assert!(!data.branches[0].is_remote);
        assert_eq!(data.topo_order.len(), 2);
        assert_eq!(data.commits.len(), 2);
        // Newest first in topological order
        assert_eq!(data.commits[&data.topo_order[0]].message, "second");
        assert_eq!(data.commits[&data.topo_order[1]].message, "first");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_repo_errors_on_non_repo() {
        let dir = std::env::temp_dir().join(format!(
            "gitopo-nonrepo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Must return an error, not panic
        assert!(load_repo(&dir, false, 100).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
