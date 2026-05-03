//! Release-notes helpers for `cvg update`.
//!
//! Two surfaces:
//!
//! 1. [`fetch_latest_release_body`] — invoke `gh api
//!    repos/Roberdan/convergio/releases/latest --jq .body` to get the
//!    GitHub Release notes for the current install. Returns `None` on
//!    any failure so the caller can fall back silently.
//! 2. [`extract_changelog_slice`] — given `CHANGELOG.md` text and two
//!    versions (installed / new), return only the section that covers
//!    the upgrade window. Section boundaries are `## [vX.Y.Z]`
//!    headers.

use std::process::Command;

/// Run `gh api` and capture stdout. None on any error: missing `gh`
/// binary, network failure, or non-zero exit. The caller MUST fall
/// back gracefully — printing release notes is a nice-to-have, never
/// a blocker for the update flow.
pub fn fetch_latest_release_body(repo: &str) -> Option<String> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{repo}/releases/latest"),
            "--jq",
            ".body",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let body = String::from_utf8(output.stdout).ok()?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Extract the slice of `changelog` covering versions strictly newer
/// than `from` and up to and including `to` (the just-installed
/// version). Section markers match `^## [vX.Y.Z]` (with or without
/// the leading `v`). Returns the joined section text without the
/// leading top-level title.
///
/// If neither version is found, returns an empty string. If `from`
/// is not found, returns the whole tail starting at `to`.
pub fn extract_changelog_slice(changelog: &str, from: &str, to: &str) -> String {
    let from = strip_v(from);
    let to = strip_v(to);
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in changelog.lines() {
        if let Some(version) = parse_version_header(line) {
            if let Some(prev) = current.take() {
                sections.push(prev);
            }
            current = Some((version, vec![line.to_string()]));
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line.to_string());
        }
    }
    if let Some(last) = current.take() {
        sections.push(last);
    }

    let mut keep: Vec<String> = Vec::new();
    let mut started = false;
    for (version, body) in &sections {
        if !started {
            if *version == to {
                started = true;
            } else {
                continue;
            }
        }
        if *version == from {
            break;
        }
        keep.extend(body.iter().cloned());
    }
    keep.join("\n")
}

fn strip_v(raw: &str) -> String {
    raw.strip_prefix('v').unwrap_or(raw).to_string()
}

/// Parse a `## [vX.Y.Z]...` header into the bare `X.Y.Z` version.
/// Accepts both `## [v1.2.3]` and `## [1.2.3]` and tolerates trailing
/// link text — the closing `]` ends the version token.
fn parse_version_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## [")?;
    let end = rest.find(']')?;
    let token = &rest[..end];
    let bare = token.strip_prefix('v').unwrap_or(token);
    if bare.chars().next()?.is_ascii_digit() {
        Some(bare.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/changelog_sample.md");

    #[test]
    fn slice_between_two_versions_includes_only_new_releases() {
        let slice = extract_changelog_slice(FIXTURE, "0.3.7", "0.3.9");
        assert!(slice.contains("[0.3.9]"));
        assert!(slice.contains("[0.3.8]"));
        assert!(!slice.contains("[0.3.7]"));
        assert!(!slice.contains("[0.3.6]"));
    }

    #[test]
    fn slice_strips_leading_v_prefix() {
        let slice = extract_changelog_slice(FIXTURE, "v0.3.8", "v0.3.9");
        assert!(slice.contains("[0.3.9]"));
        assert!(!slice.contains("[0.3.8]"));
    }

    #[test]
    fn slice_returns_empty_when_target_version_missing() {
        let slice = extract_changelog_slice(FIXTURE, "0.3.8", "9.9.9");
        assert!(slice.is_empty());
    }

    #[test]
    fn slice_returns_tail_when_from_missing() {
        let slice = extract_changelog_slice(FIXTURE, "9.9.9", "0.3.8");
        assert!(slice.contains("[0.3.8]"));
        assert!(slice.contains("[0.3.7]"));
    }

    #[test]
    fn fetch_returns_none_when_gh_missing() {
        // Simulate "gh not on PATH" by putting an empty dir first.
        let tmp = tempfile::tempdir().expect("tempdir");
        let original = std::env::var_os("PATH");
        std::env::set_var("PATH", tmp.path());
        let result = fetch_latest_release_body("Roberdan/convergio");
        if let Some(p) = original {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        // Either gh is absent (None) or it ran but failed without
        // network → also None. Both satisfy the contract.
        assert!(result.is_none());
    }

    #[test]
    fn parse_version_header_accepts_both_forms() {
        assert_eq!(parse_version_header("## [0.3.9]"), Some("0.3.9".into()));
        assert_eq!(parse_version_header("## [v0.3.9]"), Some("0.3.9".into()));
        assert_eq!(
            parse_version_header("## [0.3.9](http://x)"),
            Some("0.3.9".into())
        );
        assert_eq!(parse_version_header("## [Unreleased]"), None);
        assert_eq!(parse_version_header("# heading"), None);
    }
}
