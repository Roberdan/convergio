//! Diff-validation helpers for `cvg pr stack`. Split out of
//! [`super::pr`] to keep both files under the 300-line cap.
//!
//! `gh pr view --json files` is ground truth; the manifest body
//! is human-authored and can drift. These functions cross-check
//! the two and surface a [`ManifestStatus`] for the renderer.

use super::pr::ManifestStatus;
use super::pr_parse::ParsedManifest;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::process::Command;

/// Pull the diffed files for one PR via `gh pr view`. Empty result
/// when gh is silent (rare — usually a permissions issue).
pub(crate) fn fetch_pr_files(number: i64) -> Result<BTreeSet<String>> {
    let out = Command::new("gh")
        .args(["pr", "view", &number.to_string(), "--json", "files"])
        .output()
        .context("spawn gh pr view")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr view {} failed: {}",
            number,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    let mut files = BTreeSet::new();
    if let Some(arr) = v.get("files").and_then(Value::as_array) {
        for f in arr {
            if let Some(p) = f.get("path").and_then(Value::as_str) {
                files.insert(p.to_string());
            }
        }
    }
    Ok(files)
}

/// Cross-check a parsed manifest against the real diff.
///
/// - missing manifest -> `Missing`
/// - manifest matches the diff exactly -> `Match`
/// - any disagreement -> `Mismatch`
///
/// We do not try to be clever: the manifest is meant to be the
/// human declaration; if it lies, we surface that loud.
pub(crate) fn compare_manifest(
    manifest: &ParsedManifest,
    diff: &BTreeSet<String>,
) -> ManifestStatus {
    if manifest.files.is_empty() {
        return ManifestStatus::Missing;
    }
    if &manifest.files == diff {
        ManifestStatus::Match
    } else {
        ManifestStatus::Mismatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pr_parse::ParsedManifest;

    fn pm(items: &[&str]) -> ParsedManifest {
        ParsedManifest {
            files: items.iter().map(|s| s.to_string()).collect(),
            depends_on: BTreeSet::new(),
        }
    }
    fn diff_set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn match_when_manifest_equals_diff() {
        assert_eq!(
            compare_manifest(&pm(&["a", "b"]), &diff_set(&["a", "b"])),
            ManifestStatus::Match
        );
    }

    #[test]
    fn missing_when_manifest_empty() {
        assert_eq!(
            compare_manifest(&pm(&[]), &diff_set(&["a"])),
            ManifestStatus::Missing
        );
    }

    #[test]
    fn mismatch_when_diff_has_extras() {
        // Caller wrote `a` in the manifest but actually changed `a` and `b`.
        assert_eq!(
            compare_manifest(&pm(&["a"]), &diff_set(&["a", "b"])),
            ManifestStatus::Mismatch
        );
    }

    #[test]
    fn mismatch_when_manifest_has_extras() {
        // Caller wrote `a, b` but only `a` changed (typo or stale list).
        assert_eq!(
            compare_manifest(&pm(&["a", "b"]), &diff_set(&["a"])),
            ManifestStatus::Mismatch
        );
    }
}
