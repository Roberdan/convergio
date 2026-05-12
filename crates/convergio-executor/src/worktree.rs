//! Pre-create the agent's git worktree before the runner is spawned.
//!
//! The dispatcher used to set `cwd = current_dir()` and let the
//! prompt instruct the agent to "work in a fresh worktree under
//! `.claude/worktrees/<branch>/`". Two production runs proved that
//! contract impossible to honour from inside a non-interactive
//! vendor CLI:
//!
//! - `gh copilot --yolo` cannot run `cd` chains so the agent ended
//!   up calling `git checkout -b` directly on the operator's main
//!   checkout, throwing away uncommitted changes (real incident
//!   that wiped a `runner.rs` edit mid-session).
//! - `claude:sonnet` could create the worktree but spent ~30% of
//!   its turn budget on shell plumbing instead of the actual task.
//!
//! Pre-creating the worktree from the daemon side gives the runner
//! a `cwd` it should never leave. The prompt is also updated to
//! say "you are already in your dedicated worktree" — no checkout,
//! no branch creation, no main-checkout exposure.
//!
//! Branch naming: `agent/<task-id-prefix>` so multiple agents on
//! the same plan don't collide. If the branch already exists
//! (re-dispatch after a reaped run), we reattach by checking out
//! the existing branch into a fresh worktree.

use crate::error::{ExecutorError, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 7-char task-id prefix used in worktree paths and branch names.
fn task_slug(task_id: &str) -> &str {
    task_id.get(..7).unwrap_or(task_id)
}

/// Branch name the agent's worktree tracks.
fn branch_name(task_id: &str) -> String {
    format!("agent/{}", task_slug(task_id))
}

/// Path the agent's worktree lives at.
pub fn worktree_path(repo_root: &Path, task_id: &str) -> PathBuf {
    repo_root
        .join(".claude")
        .join("worktrees")
        .join(format!("agent-{}", task_slug(task_id)))
}

/// Create (or reattach) the worktree for `task_id` under
/// `repo_root`. Returns the path to use as `cwd` for the runner.
///
/// Idempotent: if the path already exists and is a valid worktree
/// of `repo_root` we return it unchanged. If only the branch
/// exists we re-add a fresh worktree on it. Errors fail the
/// dispatch — better than spawning into the main checkout.
///
/// Before any filesystem mutation we run [`crate::guards::enforce`]
/// so a runaway dispatcher cannot fill the disk with parallel
/// worktrees (incident 2026-05-08, see post-mortem in
/// `docs/incidents/`).
pub fn prepare(repo_root: &Path, task_id: &str) -> Result<PathBuf> {
    crate::guards::enforce(repo_root)?;
    let path = worktree_path(repo_root, task_id);
    if path.exists() {
        // The reaper or a previous tick already prepared this
        // worktree and the agent restarted. Reuse it as-is.
        return Ok(path);
    }
    let parent = path.parent().expect("worktree path has parent");
    std::fs::create_dir_all(parent).map_err(|e| ExecutorError::Worktree(e.to_string()))?;
    let branch = branch_name(task_id);
    if branch_exists(repo_root, &branch)? {
        // Re-attach to an existing branch (re-dispatch after the
        // worktree dir was removed but the branch survived).
        run_git_os(repo_root, &build_worktree_add_reattach_argv(&path, &branch))?;
    } else {
        // Fresh branch off main.
        run_git_os(
            repo_root,
            &build_worktree_add_new_branch_argv(&path, &branch, "main"),
        )?;
    }
    Ok(path)
}

/// Build the argv for `git worktree add <path> <branch>` (re-attach
/// path). The worktree path is passed through as an [`OsString`] so
/// non-UTF-8 components on Unix survive byte-for-byte (audit
/// 2026-05-12 LOW · worktree.rs:76).
fn build_worktree_add_reattach_argv(path: &Path, branch: &str) -> Vec<OsString> {
    vec![
        OsString::from("worktree"),
        OsString::from("add"),
        path.as_os_str().to_os_string(),
        OsString::from(branch),
    ]
}

/// Build the argv for `git worktree add <path> -b <branch> <base>`
/// (new-branch path). Path preserved as [`OsString`].
fn build_worktree_add_new_branch_argv(path: &Path, branch: &str, base: &str) -> Vec<OsString> {
    vec![
        OsString::from("worktree"),
        OsString::from("add"),
        path.as_os_str().to_os_string(),
        OsString::from("-b"),
        OsString::from(branch),
        OsString::from(base),
    ]
}

/// Build the argv for `git worktree remove --force <path>`. Path
/// preserved as [`OsString`].
fn build_worktree_remove_argv(path: &Path) -> Vec<OsString> {
    vec![
        OsString::from("worktree"),
        OsString::from("remove"),
        OsString::from("--force"),
        path.as_os_str().to_os_string(),
    ]
}

/// Best-effort cleanup. Called when a task transitions to a
/// terminal state. Errors are logged but never propagated — a
/// stuck worktree is operator-fixable with `git worktree prune`.
pub fn cleanup(repo_root: &Path, task_id: &str) {
    let path = worktree_path(repo_root, task_id);
    if !path.exists() {
        return;
    }
    let _ = run_git_os(repo_root, &build_worktree_remove_argv(&path));
}

fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("show-ref")
        .arg("--verify")
        .arg(format!("refs/heads/{branch}"))
        .output()
        .map_err(|e| ExecutorError::Worktree(format!("git show-ref: {e}")))?;
    Ok(out.status.success())
}

fn run_git_os(repo_root: &Path, args: &[OsString]) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| ExecutorError::Worktree(format!("git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ExecutorError::Worktree(format!(
            "git failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_7_chars_or_full_id_if_shorter() {
        assert_eq!(task_slug("0cdefc9a-a17d-4099"), "0cdefc9");
        assert_eq!(task_slug("abc"), "abc");
    }

    #[test]
    fn worktree_path_is_under_repo_root() {
        let p = worktree_path(Path::new("/repo"), "0cdefc9a-a17d");
        assert_eq!(p, Path::new("/repo/.claude/worktrees/agent-0cdefc9"));
    }

    #[test]
    fn branch_name_is_namespaced() {
        assert_eq!(branch_name("0cdefc9a-a17d"), "agent/0cdefc9");
    }

    /// Audit 2026-05-12 LOW · worktree.rs:76 — Non-UTF-8 worktree
    /// paths must reach `git worktree add` byte-for-byte instead of
    /// being flattened to an empty string by
    /// `Path::to_str().unwrap_or("")`. Unix-only because constructing
    /// a non-UTF-8 path requires `OsStrExt`.
    #[cfg(unix)]
    #[test]
    fn worktree_add_reattach_argv_preserves_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        let mut bytes = b"/tmp/convergio-".to_vec();
        bytes.push(0xFF); // not valid UTF-8
        bytes.extend_from_slice(b"/agent-abc1234");
        let path = PathBuf::from(OsStr::from_bytes(&bytes));
        let argv = build_worktree_add_reattach_argv(&path, "agent/abc1234");
        assert_eq!(argv[0], OsStr::new("worktree"));
        assert_eq!(argv[1], OsStr::new("add"));
        assert_eq!(
            argv[2].as_os_str(),
            path.as_os_str(),
            "non-UTF-8 path must be preserved byte-for-byte"
        );
        assert_eq!(argv[3], OsStr::new("agent/abc1234"));
    }

    #[cfg(unix)]
    #[test]
    fn worktree_add_new_branch_argv_preserves_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        let mut bytes = b"/tmp/convergio-".to_vec();
        bytes.push(0xFE);
        bytes.extend_from_slice(b"/agent-deadbee");
        let path = PathBuf::from(OsStr::from_bytes(&bytes));
        let argv = build_worktree_add_new_branch_argv(&path, "agent/deadbee", "main");
        assert_eq!(
            argv[2].as_os_str(),
            path.as_os_str(),
            "non-UTF-8 path must be preserved byte-for-byte"
        );
        assert_eq!(argv[3], OsStr::new("-b"));
        assert_eq!(argv[4], OsStr::new("agent/deadbee"));
        assert_eq!(argv[5], OsStr::new("main"));
    }

    #[cfg(unix)]
    #[test]
    fn worktree_remove_argv_preserves_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        let mut bytes = b"/tmp/convergio-".to_vec();
        bytes.push(0x80);
        bytes.extend_from_slice(b"/agent-cafebab");
        let path = PathBuf::from(OsStr::from_bytes(&bytes));
        let argv = build_worktree_remove_argv(&path);
        assert_eq!(argv[0], OsStr::new("worktree"));
        assert_eq!(argv[1], OsStr::new("remove"));
        assert_eq!(argv[2], OsStr::new("--force"));
        assert_eq!(
            argv[3].as_os_str(),
            path.as_os_str(),
            "non-UTF-8 path must be preserved byte-for-byte"
        );
    }
}
