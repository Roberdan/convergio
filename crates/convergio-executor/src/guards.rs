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
use convergio_durability::WorktreeHolder;
use std::path::Path;
use tracing::warn;

const DEFAULT_MAX_WORKTREES: usize = 2;
const DEFAULT_MAX_WORKTREES_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const KILL_SWITCH_ENV: &str = "CONVERGIO_DISPATCH_DISABLED";
const COUNT_CAP_ENV: &str = "CONVERGIO_GUARD_MAX_WORKTREES";
const BYTES_CAP_ENV: &str = "CONVERGIO_GUARD_MAX_WORKTREES_BYTES";

/// List the worktree directory slugs (basename without the
/// `agent-` prefix) currently present under
/// `<repo_root>/.claude/worktrees/`.
///
/// Returns an empty vec when the parent directory does not exist
/// yet. Entries that are not directories or whose name does not
/// start with `agent-` are ignored — those are not produced by the
/// executor and we should not blame anyone for them.
pub fn list_worktree_slugs(repo_root: &Path) -> Vec<String> {
    let worktrees = repo_root.join(".claude").join("worktrees");
    let iter = match std::fs::read_dir(&worktrees) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in iter.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if let Some(slug) = name.strip_prefix("agent-") {
            out.push(slug.to_string());
        }
    }
    out.sort();
    out
}

/// Refuse new runner spawn when any guard rail trips.
///
/// `repo_root` is the operator's repo root — same value
/// `convergio-server` passes to [`crate::Executor::with_repo_path`].
/// The function inspects `<repo_root>/.claude/worktrees/`; if that
/// directory does not exist yet the guards are vacuously satisfied
/// (first dispatch wins).
///
/// Convenience wrapper that passes no enrichment — equivalent to
/// [`enforce_with_holders(repo_root, &[])`]. The error message will
/// still list the worktree count and the recovery steps, but it
/// won't be able to name the blocking task/plan because no caller
/// looked them up.
pub fn enforce(repo_root: &Path) -> Result<()> {
    enforce_with_holders(repo_root, &[])
}

/// Same as [`enforce`] but enriches the refusal message with the
/// task/plan currently holding each on-disk worktree.
///
/// `holders` is expected to be the output of
/// `Durability::worktrees().holders_for_slugs(&list_worktree_slugs(repo_root))`.
/// Callers that cannot reach the database safely pass `&[]` — the
/// guard still trips on the same counts, just without the
/// per-worktree blame line.
pub fn enforce_with_holders(repo_root: &Path, holders: &[WorktreeHolder]) -> Result<()> {
    enforce_with_pressure(repo_root, holders, 0)
}

/// Enforce using both physical worktree directories and active workspace leases.
pub fn enforce_with_pressure(
    repo_root: &Path,
    holders: &[WorktreeHolder],
    active_workspace_leases: usize,
) -> Result<()> {
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

    let worktree_count = count_subdirs(&worktrees);
    let count = worktree_count.max(active_workspace_leases);
    if count >= cap_count {
        let holders_render = crate::guards_format::render_holders(holders);
        let suffix = if holders_render.is_empty() {
            String::new()
        } else {
            format!(" In use: {holders_render}.")
        };
        return Err(ExecutorError::Worktree(format!(
            "dispatch refused: {count}/{cap_count} dispatch slots in use ({worktree_count} worktrees, {active_workspace_leases} active workspace leases; {COUNT_CAP_ENV}).{suffix} \
             To recover: \
             (1) `git -C {repo} worktree list` to inspect, \
             (2) `git -C {repo} worktree remove --force <path>` for stale ones, \
             or (3) raise the cap with `launchctl setenv {COUNT_CAP_ENV} N` + daemon restart.",
            repo = repo_root.display(),
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

    #[test]
    fn list_worktree_slugs_strips_prefix_and_sorts() {
        let tmp = fresh_repo();
        let wt = tmp.path().join(".claude").join("worktrees");
        std::fs::create_dir_all(wt.join("agent-zzz9999")).expect("mkdir");
        std::fs::create_dir_all(wt.join("agent-aaa1111")).expect("mkdir");
        std::fs::create_dir_all(wt.join("not-an-agent-dir")).expect("mkdir");
        let slugs = list_worktree_slugs(tmp.path());
        assert_eq!(slugs, vec!["aaa1111".to_string(), "zzz9999".to_string()]);
    }

    #[test]
    fn lease_pressure_counts_against_cap() {
        let tmp = fresh_repo();
        env::remove_var(KILL_SWITCH_ENV);
        env::set_var(COUNT_CAP_ENV, "2");
        let result = enforce_with_pressure(tmp.path(), &[], 2);
        env::remove_var(COUNT_CAP_ENV);
        let msg = result.expect_err("lease pressure must refuse").to_string();
        assert!(
            msg.contains("0 worktrees, 2 active workspace leases"),
            "got: {msg}"
        );
    }

    #[test]
    fn count_cap_error_enumerates_holders() {
        let tmp = fresh_repo();
        let wt = tmp.path().join(".claude").join("worktrees");
        std::fs::create_dir_all(wt.join("agent-abc1234")).expect("mkdir");
        std::fs::create_dir_all(wt.join("agent-def5678")).expect("mkdir");
        env::remove_var(KILL_SWITCH_ENV);
        env::set_var(COUNT_CAP_ENV, "2");
        let holders = vec![
            WorktreeHolder {
                slug: "abc1234".into(),
                task_id: Some("abc12340-0000-0000-0000-000000000000".into()),
                task_status: Some("in_progress".into()),
                plan_id: Some("plan-id".into()),
                plan_number: Some(7),
                started_at: Some(chrono::Utc::now() - chrono::Duration::minutes(45)),
                agent_id: None,
            },
            WorktreeHolder {
                slug: "def5678".into(),
                task_id: None,
                task_status: None,
                plan_id: None,
                plan_number: None,
                started_at: None,
                agent_id: None,
            },
        ];
        let result = enforce_with_holders(tmp.path(), &holders);
        env::remove_var(COUNT_CAP_ENV);
        let err = result.expect_err("must refuse");
        let msg = err.to_string();
        assert!(msg.contains("2/2 dispatch slots in use"), "got: {msg}");
        assert!(msg.contains("agent-abc1234"), "got: {msg}");
        assert!(msg.contains("plan #7"), "got: {msg}");
        assert!(msg.contains("agent-def5678"), "got: {msg}");
        assert!(msg.contains("orphan"), "got: {msg}");
    }
}
