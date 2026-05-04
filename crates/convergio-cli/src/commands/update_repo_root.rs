//! Workspace-root discovery for `cvg update`. Three-level cascade:
//! `CONVERGIO_REPO_DIR` env, `repo_path` in `~/.convergio/config.toml`,
//! walk-up from cwd. Each candidate validated as a `[workspace]`
//! Cargo.toml dir; invalid candidates fall through (don't abort).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_VAR: &str = "CONVERGIO_REPO_DIR";
const CONFIG_FIELD: &str = "repo_path";

/// Resolve the workspace root using the env / config / walk-up
/// cascade.
pub fn resolve() -> Result<PathBuf> {
    if let Some(p) = candidate_from_env() {
        if let Some(root) = validate(&p) {
            return Ok(root);
        }
    }
    if let Some(p) = candidate_from_config() {
        if let Some(root) = validate(&p) {
            return Ok(root);
        }
    }
    walk_up_from_cwd()
}

fn candidate_from_env() -> Option<PathBuf> {
    let raw = std::env::var(ENV_VAR).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn candidate_from_config() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = Path::new(&home).join(".convergio").join("config.toml");
    let text = std::fs::read_to_string(&path).ok()?;
    parse_repo_path(&text)
}

/// Parse the `repo_path` field out of the simple `key = "value"`
/// config Convergio writes today. Avoids pulling a TOML parser into
/// this code path; the file shape is stable (one field per line).
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

fn validate(p: &Path) -> Option<PathBuf> {
    if !p.is_dir() {
        return None;
    }
    let toml = p.join("Cargo.toml");
    if !toml.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&toml).ok()?;
    if text.contains("[workspace]") {
        Some(p.to_path_buf())
    } else {
        None
    }
}

/// Derive the GitHub slug (`owner/repo`) from the workspace's
/// `origin` remote. Returns `None` when the remote is unset, when
/// `git` is unavailable, or when the URL does not look like a
/// GitHub https/ssh URL (gracefully — never panics).
///
/// Used by `cvg dash` to scope `gh pr list` to the workspace's
/// repository instead of inheriting the operator's cwd.
pub fn github_slug(repo_path: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = std::str::from_utf8(&out.stdout).ok()?.trim();
    parse_github_slug(url)
}

fn parse_github_slug(url: &str) -> Option<String> {
    // Accepts:
    //   https://github.com/Roberdan/convergio[.git]
    //   http://github.com/Roberdan/convergio[.git]
    //   git@github.com:Roberdan/convergio[.git]
    //   ssh://git@github.com/Roberdan/convergio[.git]
    let trimmed = url
        .trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(url.trim_end_matches('/'));
    let path = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = path.splitn(3, '/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn walk_up_from_cwd() -> Result<PathBuf> {
    let mut here = std::env::current_dir().context("cwd")?;
    loop {
        let candidate = here.join("Cargo.toml");
        if candidate.is_file() {
            if let Ok(text) = std::fs::read_to_string(&candidate) {
                if text.contains("[workspace]") {
                    return Ok(here);
                }
            }
        }
        if !here.pop() {
            anyhow::bail!(
                "could not locate the Convergio workspace. Set {ENV_VAR}=... or run `cvg setup` from inside the repo to record `{CONFIG_FIELD}` in ~/.convergio/config.toml.",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_workspace(dir: &Path) {
        std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    }

    #[test]
    fn parse_repo_path_extracts_quoted_value() {
        let cfg = "version = 1\nurl = \"http://x\"\nrepo_path = \"/tmp/wk\"\n";
        assert_eq!(parse_repo_path(cfg), Some(PathBuf::from("/tmp/wk")));
    }

    #[test]
    fn parse_repo_path_ignores_comments_and_missing() {
        let cfg = "version = 1\n# repo_path = \"/no\"\n";
        assert_eq!(parse_repo_path(cfg), None);
    }

    #[test]
    fn parse_repo_path_expands_home() {
        std::env::set_var("HOME", "/Users/test");
        let cfg = "repo_path = \"$HOME/code/convergio\"\n";
        let got = parse_repo_path(cfg).expect("parsed");
        assert_eq!(got, PathBuf::from("/Users/test/code/convergio"));
        let cfg2 = "repo_path = \"~/code/convergio\"\n";
        let got2 = parse_repo_path(cfg2).expect("parsed");
        assert_eq!(got2, PathBuf::from("/Users/test/code/convergio"));
    }

    #[test]
    fn validate_accepts_workspace_root() {
        let tmp = tempdir().unwrap();
        make_workspace(tmp.path());
        assert_eq!(validate(tmp.path()), Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn validate_rejects_non_workspace_dir() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = 'x'\n").unwrap();
        assert_eq!(validate(tmp.path()), None);
    }

    #[test]
    fn validate_rejects_missing_dir() {
        assert_eq!(validate(Path::new("/this/path/does/not/exist/12345")), None);
    }

    #[test]
    fn env_overrides_when_pointing_at_workspace() {
        let tmp = tempdir().unwrap();
        make_workspace(tmp.path());
        std::env::set_var(ENV_VAR, tmp.path().display().to_string());
        let got = resolve().expect("env hit");
        assert_eq!(got, tmp.path().to_path_buf());
        std::env::remove_var(ENV_VAR);
    }

    #[test]
    fn env_pointing_at_garbage_falls_through_to_walk_up() {
        // Walk-up from this test's cwd should still find the
        // workspace root (we run inside it).
        std::env::set_var(ENV_VAR, "/path/that/does/not/exist/zzz");
        let got = resolve().expect("walk up wins");
        let toml = std::fs::read_to_string(got.join("Cargo.toml")).expect("read root toml");
        assert!(toml.contains("[workspace]"));
        std::env::remove_var(ENV_VAR);
    }

    #[test]
    fn parse_github_slug_table() {
        for (url, want) in [
            (
                "https://github.com/Roberdan/convergio.git",
                Some("Roberdan/convergio"),
            ),
            (
                "https://github.com/Roberdan/convergio",
                Some("Roberdan/convergio"),
            ),
            (
                "git@github.com:Roberdan/convergio.git",
                Some("Roberdan/convergio"),
            ),
            (
                "ssh://git@github.com/Roberdan/convergio",
                Some("Roberdan/convergio"),
            ),
            ("https://gitlab.com/foo/bar.git", None),
            ("", None),
            ("not-a-url", None),
            ("https://github.com/just-owner", None),
        ] {
            assert_eq!(parse_github_slug(url), want.map(String::from), "url={url}");
        }
    }
}
