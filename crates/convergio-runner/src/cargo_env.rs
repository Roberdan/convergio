//! Shared Cargo environment for spawned agent worktrees.
//!
//! Agent work happens in one git worktree per task. Without a shared
//! target directory, every worktree gets its own `target/` and Rust
//! recompiles the world per agent.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const TARGET_OVERRIDE_ENV: &str = "CONVERGIO_AGENT_CARGO_TARGET_DIR";

/// Cargo-related environment variables for an agent subprocess.
pub fn env_for(cwd: &Path) -> Vec<(String, String)> {
    if let Some(target) = std::env::var_os(TARGET_OVERRIDE_ENV).filter(|v| !v.is_empty()) {
        return vec![(
            "CARGO_TARGET_DIR".into(),
            target.to_string_lossy().into_owned(),
        )];
    }

    if std::env::var_os("CARGO_TARGET_DIR").is_some() {
        return Vec::new();
    }

    vec![(
        "CARGO_TARGET_DIR".into(),
        shared_target_dir(cwd).display().to_string(),
    )]
}

fn shared_target_dir(cwd: &Path) -> PathBuf {
    repo_root_from_worktree(cwd)
        .unwrap_or_else(|| cwd.to_path_buf())
        .join(".claude")
        .join("cargo-target")
}

fn repo_root_from_worktree(cwd: &Path) -> Option<PathBuf> {
    for ancestor in cwd.ancestors() {
        if ancestor.file_name() == Some(OsStr::new("worktrees")) {
            return ancestor.parent()?.parent().map(Path::to_path_buf);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_target_lives_outside_agent_worktree() {
        let cwd = Path::new("/repo/.claude/worktrees/agent-1234567");
        assert_eq!(
            shared_target_dir(cwd),
            Path::new("/repo/.claude/cargo-target")
        );
    }

    #[test]
    fn non_worktree_falls_back_under_cwd() {
        let cwd = Path::new("/tmp/project");
        assert_eq!(
            shared_target_dir(cwd),
            Path::new("/tmp/project/.claude/cargo-target")
        );
    }
}
