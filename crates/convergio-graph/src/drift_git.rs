//! Git-side helpers for [`super::drift`].
//!
//! Kept in a sibling module so `drift.rs` stays under the 300-line
//! per-file cap (CONSTITUTION § 13) as the drift report grows.

use crate::error::{GraphError, Result};
use std::path::Path;
use std::process::Command;

/// Run `git -C <repo_root> diff --name-only <since>...HEAD` and
/// return the list of changed files. Empty lines and trailing
/// whitespace are stripped; a non-zero exit status is bubbled up as
/// [`GraphError::Other`].
pub(super) fn git_changed_files(repo_root: &Path, since: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("diff")
        .arg("--name-only")
        .arg(format!("{since}...HEAD"))
        .output()
        .map_err(GraphError::Io)?;
    if !out.status.success() {
        return Err(GraphError::Other(format!(
            "git diff --name-only {since}...HEAD failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect())
}
