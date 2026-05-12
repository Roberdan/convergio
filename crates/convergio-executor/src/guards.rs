//! Hard guard rails for runner dispatch.
//!
//! Two production incidents (`docs/incidents/2026-05-08-dirty-state-postmortem.md`)
//! showed that the dispatcher will happily fan out one git worktree per
//! pending task, and that with 30+ Rust worktrees each carrying its
//! own `target/` the laptop runs out of disk and grinds to a halt
//! before any human can intervene.
//!
//! These guards refuse to spawn a new runner when any of the following
//! hold. Every check is environment-tunable so operators can relax
//! them per-host without recompiling.
//!
//! | Env var                                    | Default   | Effect when exceeded |
//! |--------------------------------------------|-----------|----------------------|
//! | `CONVERGIO_DISPATCH_DISABLED=1`            | unset     | hard kill switch — refuse all spawns |
//! | `CONVERGIO_GUARD_MAX_WORKTREES`            | `2`       | refuse if `.claude/worktrees/` already has ≥ N child dirs |
//! | `CONVERGIO_GUARD_MAX_WORKTREES_BYTES`      | `5 GiB`   | refuse if total bytes under `.claude/worktrees/` ≥ cap |
//!
//! All checks are best-effort and log on failure rather than panic;
//! a transient I/O error is treated as "assume safe" so an unrelated
//! filesystem hiccup never wedges the executor permanently. The
//! caller is expected to surface the refusal via `ExecutorError` so
//! the audit log captures it.

use crate::error::{ExecutorError, Result};
use std::path::Path;
use tracing::warn;

const DEFAULT_MAX_WORKTREES: usize = 2;
const DEFAULT_MAX_WORKTREES_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const KILL_SWITCH_ENV: &str = "CONVERGIO_DISPATCH_DISABLED";
const COUNT_CAP_ENV: &str = "CONVERGIO_GUARD_MAX_WORKTREES";
const BYTES_CAP_ENV: &str = "CONVERGIO_GUARD_MAX_WORKTREES_BYTES";

/// Refuse new runner spawn when any guard rail trips.
///
/// `repo_root` is the operator's repo root — same value
/// `convergio-server` passes to [`crate::Executor::with_repo_path`].
/// The function inspects `<repo_root>/.claude/worktrees/`; if that
/// directory does not exist yet the guards are vacuously satisfied
/// (first dispatch wins).
pub fn enforce(repo_root: &Path) -> Result<()> {
    if std::env::var(KILL_SWITCH_ENV).as_deref() == Ok("1") {
        return Err(ExecutorError::Worktree(format!(
            "dispatch refused: {KILL_SWITCH_ENV}=1 (kill switch active)"
        )));
    }

    let worktrees = repo_root.join(".claude").join("worktrees");
    if !worktrees.exists() {
        return Ok(());
    }

    let cap_count = env_usize(COUNT_CAP_ENV, DEFAULT_MAX_WORKTREES);
    let cap_bytes = env_u64(BYTES_CAP_ENV, DEFAULT_MAX_WORKTREES_BYTES);

    let count = count_subdirs(&worktrees);
    if count >= cap_count {
        return Err(ExecutorError::Worktree(format!(
            "dispatch refused: {count} worktrees ≥ cap {cap_count} ({COUNT_CAP_ENV}). \
             To recover: \
             (1) `git -C {} worktree list` to inspect, \
             (2) `git -C {} worktree remove --force <path>` for stale ones, \
             or (3) raise the cap with `launchctl setenv {COUNT_CAP_ENV} N` + daemon restart.",
            repo_root.display(),
            repo_root.display(),
        )));
    }

    let bytes = dir_size_bytes(&worktrees);
    if bytes >= cap_bytes {
        return Err(ExecutorError::Worktree(format!(
            "dispatch refused: worktrees occupy {bytes} bytes ≥ cap {cap_bytes} ({BYTES_CAP_ENV})"
        )));
    }

    Ok(())
}

fn count_subdirs(p: &Path) -> usize {
    match std::fs::read_dir(p) {
        Ok(iter) => iter
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .count(),
        Err(e) => {
            warn!(path = %p.display(), error = %e, "guards: count_subdirs read_dir failed; assuming 0");
            0
        }
    }
}

fn dir_size_bytes(p: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(p) else {
        warn!(path = %p.display(), "guards: dir_size_bytes read_dir failed; assuming 0");
        return 0;
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        total = total.saturating_add(if meta.is_dir() {
            dir_size_bytes(&entry.path())
        } else {
            meta.len()
        });
    }
    total
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn fresh_repo() -> TempDir {
        let tmp = TempDir::new().expect("tmp");
        std::fs::create_dir_all(tmp.path().join(".claude").join("worktrees")).expect("mkdir");
        tmp
    }

    #[test]
    fn empty_worktrees_dir_is_safe() {
        let tmp = fresh_repo();
        env::remove_var(KILL_SWITCH_ENV);
        assert!(enforce(tmp.path()).is_ok());
    }

    #[test]
    fn missing_worktrees_dir_is_safe() {
        let tmp = TempDir::new().expect("tmp");
        env::remove_var(KILL_SWITCH_ENV);
        assert!(enforce(tmp.path()).is_ok());
    }

    #[test]
    fn kill_switch_refuses_spawn() {
        let tmp = fresh_repo();
        env::set_var(KILL_SWITCH_ENV, "1");
        let result = enforce(tmp.path());
        env::remove_var(KILL_SWITCH_ENV);
        assert!(result.is_err(), "kill switch must refuse");
    }

    #[test]
    fn count_cap_refuses_when_too_many_worktrees() {
        let tmp = fresh_repo();
        let wt = tmp.path().join(".claude").join("worktrees");
        for i in 0..3 {
            std::fs::create_dir_all(wt.join(format!("agent-{i:03}"))).expect("mkdir");
        }
        env::remove_var(KILL_SWITCH_ENV);
        env::set_var(COUNT_CAP_ENV, "2");
        let result = enforce(tmp.path());
        env::remove_var(COUNT_CAP_ENV);
        assert!(result.is_err(), "3 worktrees with cap 2 must refuse");
    }
}
