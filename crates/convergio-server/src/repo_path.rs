//! Best-effort repo-root resolution for runner-based dispatch.
//!
//! The daemon needs a repo root to create dedicated agent worktrees under
//! `<repo>/.claude/worktrees/…` (see `convergio-executor`). When running as a
//! user service, the process may start outside the repo, so we resolve from:
//!
//! 1) `CONVERGIO_REPO_DIR` (preferred)
//! 2) `CONVERGIO_REPO_PATH` (legacy alias)
//! 3) `repo_path` in `~/.convergio/config.toml` (written by `cvg setup`)
//! 4) `git rev-parse --show-toplevel` from the current working directory
//!
//! Returns `None` when no valid directory can be resolved.

use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_VARS: [&str; 2] = ["CONVERGIO_REPO_DIR", "CONVERGIO_REPO_PATH"];
const CONFIG_FIELD: &str = "repo_path";

/// Resolve the repo root for worktree creation.
pub fn resolve_repo_path() -> Option<PathBuf> {
    candidate_from_env()
        .or_else(candidate_from_config)
        .or_else(candidate_from_git_cwd)
        .and_then(|p| p.is_dir().then_some(p))
}

fn candidate_from_env() -> Option<PathBuf> {
    for k in ENV_VARS {
        let raw = std::env::var(k).ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        return Some(PathBuf::from(trimmed));
    }
    None
}

fn candidate_from_config() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = Path::new(&home).join(".convergio").join("config.toml");
    let text = std::fs::read_to_string(&path).ok()?;
    parse_repo_path(&text)
}

fn candidate_from_git_cwd() -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn parse_repo_path(text: &str) -> Option<PathBuf> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some(rest) = line.strip_prefix(CONFIG_FIELD) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() {
            return None;
        }
        return Some(PathBuf::from(expand_home(value)));
    }
    None
}

fn expand_home(s: &str) -> String {
    let Some(home) = std::env::var_os("HOME") else {
        return s.to_owned();
    };
    let mut out = PathBuf::from(home);
    if let Some(rest) = s.strip_prefix("$HOME") {
        let trimmed = rest.trim_start_matches('/');
        if !trimmed.is_empty() {
            out.push(trimmed);
        }
        return out.to_string_lossy().into_owned();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        out.push(rest);
        return out.to_string_lossy().into_owned();
    }
    s.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_repo_path_skips_comments_and_extracts_value() {
        let text = "# comment\nurl = \"http://x\"\nrepo_path = \"/tmp/wk\"\n";
        let got = parse_repo_path(text).unwrap();
        assert_eq!(got, PathBuf::from("/tmp/wk"));
    }

    #[test]
    fn parse_repo_path_returns_none_on_empty_value() {
        assert!(parse_repo_path("repo_path = \"\"\n").is_none());
    }
}
