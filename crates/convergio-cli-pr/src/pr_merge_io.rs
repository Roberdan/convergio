//! `cvg pr merge` — gh / git I/O helpers. Pure shelling-out; no
//! daemon HTTP. AUTO-block auto-resolve lives in `pr_merge_resolve`.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// Subset of `gh pr view --json …` we consume.
#[derive(Debug, Clone, Default)]
pub(super) struct PrView {
    pub head_ref: String,
    pub body: String,
    pub mergeable: String,
    pub merge_state_status: String,
    pub review_decision: String,
    pub status_check_rollup_pass: bool,
}

pub(super) fn fetch_pr_view(pr: u64) -> Result<PrView> {
    let out = Command::new("gh")
        .args([
            "pr",
            "view",
            &pr.to_string(),
            "--json",
            "headRefName,body,mergeable,mergeStateStatus,reviewDecision,statusCheckRollup",
        ])
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr view {} failed: {}",
            pr,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let v: Value = serde_json::from_slice(&out.stdout).context("parse gh pr view json")?;
    let rollup_pass = v
        .get("statusCheckRollup")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter().all(|c| {
                let conclusion = c.get("conclusion").and_then(Value::as_str).unwrap_or("");
                let state = c.get("state").and_then(Value::as_str).unwrap_or("");
                matches!(conclusion, "SUCCESS" | "NEUTRAL" | "SKIPPED")
                    || matches!(state, "SUCCESS")
            })
        })
        .unwrap_or(false);
    Ok(PrView {
        head_ref: string_of(&v, "headRefName"),
        body: string_of(&v, "body"),
        mergeable: string_of(&v, "mergeable"),
        merge_state_status: string_of(&v, "mergeStateStatus"),
        review_decision: string_of(&v, "reviewDecision"),
        status_check_rollup_pass: rollup_pass,
    })
}

fn string_of(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn gh_merge(pr: u64) -> Result<()> {
    let out = Command::new("gh")
        .args(["pr", "merge", &pr.to_string(), "--merge", "--delete-branch"])
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr merge {} failed: {}",
            pr,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub(super) fn is_auto_block_conflict(err: &anyhow::Error) -> bool {
    let msg = format!("{err:#}").to_lowercase();
    msg.contains("not mergeable") || msg.contains("conflict") || msg.contains("merge_conflict")
}

pub(super) fn list_worktrees() -> Result<Vec<(PathBuf, String)>> {
    let out = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("spawn git worktree list")?;
    if !out.status.success() {
        anyhow::bail!("git worktree list failed");
    }
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    Ok(parse_worktree_porcelain(&s))
}

pub(super) fn parse_worktree_porcelain(s: &str) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    let mut cur_path: Option<PathBuf> = None;
    let mut cur_branch: Option<String> = None;
    for line in s.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            if let (Some(path), Some(branch)) = (cur_path.take(), cur_branch.take()) {
                result.push((path, branch));
            }
            cur_path = Some(PathBuf::from(p));
            cur_branch = None;
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            cur_branch = Some(b.to_string());
        }
    }
    if let (Some(path), Some(branch)) = (cur_path, cur_branch) {
        result.push((path, branch));
    }
    result
}

pub(super) fn remove_worktree(head_ref: &str) -> Result<Option<PathBuf>> {
    let trees = list_worktrees().unwrap_or_default();
    let Some((path, _)) = trees.into_iter().find(|(_, b)| b == head_ref) else {
        return Ok(None);
    };
    let path_str = path.to_string_lossy().to_string();
    let force = Command::new("git")
        .args(["worktree", "remove", "--force", &path_str])
        .output()?;
    if !force.status.success() {
        anyhow::bail!(
            "could not remove worktree {}: {}",
            path.display(),
            String::from_utf8_lossy(&force.stderr)
        );
    }
    Ok(Some(path))
}

pub(super) fn delete_local_branch(head_ref: &str) -> Result<bool> {
    let out = Command::new("git")
        .args(["branch", "-D", head_ref])
        .output()
        .context("spawn git branch -D")?;
    Ok(out.status.success())
}

// Remote branch deletion is handled by `gh pr merge --delete-branch`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_conflict_detection_matches_common_phrasings() {
        let err = anyhow::anyhow!("not mergeable: conflict in AGENTS.md");
        assert!(is_auto_block_conflict(&err));
        let err2 = anyhow::anyhow!("merge_conflict in pull request");
        assert!(is_auto_block_conflict(&err2));
        let err3 = anyhow::anyhow!("HTTP 401: bad credentials");
        assert!(!is_auto_block_conflict(&err3));
    }

    #[test]
    fn parse_worktree_porcelain_extracts_branch_and_path() {
        let porcelain = "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\n\
                         worktree /repo/.claude/worktrees/feat/x\nHEAD def\nbranch refs/heads/feat/x\n";
        let parsed = parse_worktree_porcelain(porcelain);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].1, "main");
        assert_eq!(parsed[1].1, "feat/x");
    }
}
