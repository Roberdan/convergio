//! `cvg fleet cleanup` — sweep operator-side residue.
//!
//! Operator-only counterpart to `scripts/post-merge-fleet-cleanup.sh`.
//! Removes orphan agent worktrees and dead local agent branches that
//! autonomous PR sessions tend to leave behind. The DB side
//! (`agent_processes` rows whose PID is dead) is left to the daemon's
//! reaper — this CLI cannot write SQLite directly per
//! `crates/convergio-cli/AGENTS.md`.
//!
//! Driven by the 2026-05 insights audit:
//!
//! > "the F2 session also closed with 13 stale branches and
//! > 6 zombie processes."
//!
//! Now a single verb instead of seven hand-rolled `git worktree
//! remove` / `git branch -D` calls per session.

use super::OutputMode;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One cleanup pass. Returns a [`Report`] suitable for human or JSON rendering.
pub fn run(output: OutputMode, dry_run: bool) -> Result<()> {
    let repo_root = locate_repo_root().context("not inside a git repo")?;
    let report = sweep(&repo_root, dry_run)?;
    render(&report, output, dry_run);
    Ok(())
}

/// What the sweep found / did. `dry_run=true` populates the same
/// fields but skips the destructive calls.
#[derive(Debug, Default)]
struct Report {
    /// Worktrees under `.claude/worktrees/agent-*` whose tracked
    /// branch no longer exists on `origin` → safe to remove.
    orphan_worktrees: Vec<PathBuf>,
    /// Local `agent/*` branches whose remote ref is gone → safe to delete.
    stale_branches: Vec<String>,
    /// `git worktree prune` output (admin-dir cleanup count from stderr).
    prune_ran: bool,
}

fn sweep(repo_root: &Path, dry_run: bool) -> Result<Report> {
    let mut report = Report::default();

    if !dry_run {
        let _ = run_git(repo_root, &["worktree", "prune"]);
        report.prune_ran = true;
    }

    // 1. Orphan agent worktrees.
    let worktrees_dir = repo_root.join(".claude").join("worktrees");
    if worktrees_dir.exists() {
        for entry in std::fs::read_dir(&worktrees_dir)
            .with_context(|| format!("read_dir {}", worktrees_dir.display()))?
            .flatten()
        {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) if n.starts_with("agent-") => n.to_owned(),
                _ => continue,
            };
            let branch = format!("agent/{}", name.trim_start_matches("agent-"));
            if remote_branch_exists(repo_root, &branch) {
                continue;
            }
            if !dry_run {
                let _ = run_git(
                    repo_root,
                    &["worktree", "remove", "--force", path.to_str().unwrap_or("")],
                );
                let _ = run_git(repo_root, &["branch", "-D", &branch]);
            }
            report.orphan_worktrees.push(path);
        }
    }

    // 2. Local agent/* branches whose origin ref is gone.
    let local_branches = run_git(
        repo_root,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/agent/",
        ],
    )?;
    for branch in local_branches.lines() {
        let branch = branch.trim();
        if branch.is_empty() || remote_branch_exists(repo_root, branch) {
            continue;
        }
        if !dry_run {
            let _ = run_git(repo_root, &["branch", "-D", branch]);
        }
        report.stale_branches.push(branch.to_owned());
    }

    Ok(report)
}

fn render(report: &Report, output: OutputMode, dry_run: bool) {
    match output {
        OutputMode::Json => {
            let v = json!({
                "dry_run": dry_run,
                "orphan_worktrees": report.orphan_worktrees.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "stale_branches":   report.stale_branches,
                "prune_ran":        report.prune_ran,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
            );
        }
        OutputMode::Plain => {
            for p in &report.orphan_worktrees {
                println!("worktree {}", p.display());
            }
            for b in &report.stale_branches {
                println!("branch {b}");
            }
        }
        OutputMode::Human => {
            let prefix = if dry_run { "would remove" } else { "removed" };
            println!(
                "cvg fleet cleanup — {} {} orphan worktree(s), {} stale branch(es).",
                prefix,
                report.orphan_worktrees.len(),
                report.stale_branches.len(),
            );
            for p in &report.orphan_worktrees {
                println!("  worktree  {}", p.display());
            }
            for b in &report.stale_branches {
                println!("  branch    {b}");
            }
            if !dry_run && report.prune_ran {
                println!("  (git worktree prune ran)");
            }
            println!(
                "  note: agent_processes rows in state.db are reconciled by the daemon reaper."
            );
        }
    }
}

/// Walk up from `cwd` until a `.git` is found.
fn locate_repo_root() -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn remote_branch_exists(repo_root: &Path, branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("ls-remote")
        .arg("--exit-code")
        .arg("--heads")
        .arg("origin")
        .arg(branch)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .with_context(|| format!("git {} failed", args.join(" ")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("tmp");
        let path = tmp.path().to_path_buf();
        // git init so locate_repo_root succeeds + git commands work.
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["init", "-q", "-b", "main"])
            .status()
            .expect("git init");
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("config email");
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["config", "user.name", "test"])
            .status()
            .expect("config name");
        // need at least one commit before branches make sense
        fs::write(path.join("seed"), "seed").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["add", "."])
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(["commit", "-q", "-m", "seed"])
            .status()
            .unwrap();
        (tmp, path)
    }

    #[test]
    fn sweep_on_clean_repo_reports_nothing() {
        let (_tmp, repo) = init_repo();
        let report = sweep(&repo, true).expect("sweep");
        assert!(report.orphan_worktrees.is_empty());
        assert!(report.stale_branches.is_empty());
    }

    #[test]
    fn sweep_finds_local_agent_branch_with_no_remote() {
        let (_tmp, repo) = init_repo();
        // create a local agent/foo branch with no origin
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["branch", "agent/foo"])
            .status()
            .unwrap();
        let report = sweep(&repo, true).expect("sweep");
        assert!(report.stale_branches.iter().any(|b| b == "agent/foo"));
    }
}
