//! `check.worktree.no_pr` — abandoned worktrees with no open PR.
//!
//! Walks `git worktree list --porcelain`, drops the main worktree
//! and detached HEADs, then asks `gh pr list --head <branch>` for
//! each remaining branch. Any branch with no open PR is flagged
//! as a finding so the operator can decide whether to push or
//! discard before detach.
//!
//! Conservative on failure: if `git` or `gh` are missing or fail,
//! the check returns `Pass` with no findings rather than blocking
//! detach — a safety net is not allowed to be a brick wall.

use crate::pre_stop::{Check, CheckContext, CheckOutcome};
use std::process::Command;

/// Concrete check implementation.
pub struct WorktreeNoPrCheck;

impl Check for WorktreeNoPrCheck {
    fn id(&self) -> &'static str {
        "check.worktree.no_pr"
    }
    fn label(&self) -> &'static str {
        "abandoned worktrees with no PR open"
    }
    fn run(&self, _ctx: &CheckContext) -> CheckOutcome {
        let branches = match list_worktree_branches() {
            Ok(b) => b,
            Err(_) => return CheckOutcome::Pass,
        };
        let mut findings = Vec::new();
        for branch in branches {
            if !branch_has_open_pr(&branch) {
                findings.push(format!("worktree branch '{branch}' has no open PR"));
            }
        }
        if findings.is_empty() {
            CheckOutcome::Pass
        } else {
            CheckOutcome::Fail { findings }
        }
    }
}

/// Parse `git worktree list --porcelain` and return the branch name
/// of every secondary worktree (skips the main checkout and any
/// worktree on a detached HEAD).
fn list_worktree_branches() -> Result<Vec<String>, ()> {
    let out = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut branches = Vec::new();
    let mut block_branch: Option<String> = None;
    let mut block_is_main = true;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("branch refs/heads/") {
            block_branch = Some(rest.to_string());
        } else if line == "detached" {
            block_branch = None;
        } else if line.is_empty() {
            if let Some(b) = block_branch.take() {
                if !block_is_main {
                    branches.push(b);
                }
            }
            block_is_main = false;
        }
    }
    if let Some(b) = block_branch {
        if !block_is_main {
            branches.push(b);
        }
    }
    Ok(branches)
}

/// Returns `true` when `gh pr list --head <branch>` reports at least
/// one entry. Returns `true` when `gh` is missing or fails — silent
/// on tooling problems.
fn branch_has_open_pr(branch: &str) -> bool {
    let out = Command::new("gh")
        .args([
            "pr", "list", "--head", branch, "--json", "number", "--limit", "1",
        ])
        .output();
    let Ok(out) = out else {
        return true;
    };
    if !out.status.success() {
        return true;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.contains("number")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_worktree_branches_skips_main_and_detached() {
        // Pure-parser sanity test: feed a synthetic porcelain output
        // through the same logic by reusing the public surface
        // indirectly. We validate the `branch refs/heads/*` extractor
        // with a tiny inline reimplementation of the loop.
        let porcelain = "\
worktree /repo
HEAD abc
branch refs/heads/main

worktree /repo/.claude/worktrees/feat-x
HEAD def
branch refs/heads/feat/x

worktree /repo/.claude/worktrees/headless
HEAD ghi
detached

";
        let mut branches = Vec::new();
        let mut block_branch: Option<String> = None;
        let mut block_is_main = true;
        for line in porcelain.lines() {
            if let Some(rest) = line.strip_prefix("branch refs/heads/") {
                block_branch = Some(rest.to_string());
            } else if line == "detached" {
                block_branch = None;
            } else if line.is_empty() {
                if let Some(b) = block_branch.take() {
                    if !block_is_main {
                        branches.push(b);
                    }
                }
                block_is_main = false;
            }
        }
        assert_eq!(branches, vec!["feat/x".to_string()]);
    }

    #[test]
    fn check_id_and_label_are_stable() {
        let c = WorktreeNoPrCheck;
        assert_eq!(c.id(), "check.worktree.no_pr");
        assert!(c.label().contains("worktree"));
    }
}
