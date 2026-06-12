mod time;
mod types;

pub use time::format_relative_time_with_now;
pub use types::{BranchInfo, CommitInfo, RepoData, MAX_COMMITS_CAP};

use anyhow::{Context, Result};
use git2::{BranchType, Repository, Sort};
use std::collections::HashMap;
use std::path::Path;

pub fn load_repo(path: &Path, show_all: bool, max_commits: usize) -> Result<RepoData> {
    let repo = Repository::discover(path)
        .with_context(|| format!("No git repository found at {}", path.display()))?;

    let max_commits = max_commits.min(MAX_COMMITS_CAP);

    let head_branch = get_head_branch(&repo);

    let mut branches = collect_branches(&repo, show_all, head_branch.as_deref())?;

    let tip_oids: Vec<git2::Oid> = branches.iter().map(|b| b.tip_oid).collect();
    let (commits, topo_order) = walk_commits(&repo, &tip_oids, max_commits)?;

    let mut oid_to_branches: HashMap<git2::Oid, Vec<String>> = HashMap::new();
    for branch in &branches {
        oid_to_branches
            .entry(branch.tip_oid)
            .or_default()
            .push(branch.name.clone());
    }

    for branch in &mut branches {
        if !branch.is_remote {
            branch.ahead_behind = compute_ahead_behind(&repo, branch);
        }
    }

    // HEAD first, then locals alphabetically, then remotes
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
        repo_path,
    })
}

fn get_head_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    if head.is_branch() {
        head.shorthand().ok().map(|s| s.to_owned())
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
        let is_head = !is_remote && head_name == Some(name.as_str());
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
            tip_oid,
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

        let message = commit
            .summary()
            .ok()
            .flatten()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(load_repo(&dir, false, 100).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
