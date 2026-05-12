//! Pure analysis helpers for `cvg pr stack`: combine a PR's
//! `## Files touched` body manifest with the result of fetching its
//! real file diff.
//!
//! Split out of `pr.rs` so that file stays under the 300-line
//! per-file cap (CONSTITUTION § Agent context budget).

use super::pr::{AnalysedPr, ManifestStatus};
use super::pr_diff::{compare_manifest, fetch_pr_files};
use super::pr_parse::parse_manifest;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;

/// Best-effort: pull the real diff for one PR and cross-check.
/// Falls back to manifest-only via [`combine_manifest_and_diff`]
/// when the diff fetch fails. The fallback path used to be silent;
/// after the audit fix it surfaces `ManifestStatus::Unverified` so
/// the operator can see the degraded state.
pub(super) fn analyse_pr_with_diff(value: &Value) -> AnalysedPr {
    let diff_result = fetch_pr_files(pr_number_of(value));
    combine_manifest_and_diff(value, diff_result)
}

fn pr_number_of(value: &Value) -> i64 {
    value.get("number").and_then(Value::as_i64).unwrap_or(0)
}

/// Pure combiner: build an `AnalysedPr` from a PR JSON value and the
/// result of fetching its real file diff. Centralising the logic
/// makes the manifest-status decisions testable without shelling out
/// to `gh`. See audit finding (LOW, pr.rs:87): per-PR diff fetch
/// failures must not silently fall back to manifest-only.
pub(crate) fn combine_manifest_and_diff(
    value: &Value,
    diff_result: Result<BTreeSet<String>>,
) -> AnalysedPr {
    let number = pr_number_of(value);
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let body = value.get("body").and_then(Value::as_str).unwrap_or("");
    let manifest = parse_manifest(body);

    match diff_result {
        Ok(diff_files) => {
            let manifest_status = compare_manifest(&manifest, &diff_files);
            AnalysedPr {
                number,
                title,
                files: diff_files,
                depends_on: manifest.depends_on,
                manifest_status,
            }
        }
        Err(_diff_err) => {
            // Bug fix pr.rs:87 — when the diff fetch fails we no
            // longer silently classify the manifest as Match/Missing.
            // Surfacing `Unverified` lets the renderer warn the
            // operator that the manifest was not cross-checked.
            AnalysedPr {
                number,
                title,
                files: manifest.files,
                depends_on: manifest.depends_on,
                manifest_status: ManifestStatus::Unverified,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Audit finding (LOW, pr.rs:87): per-PR diff fetch failures fell
    // back to manifest-only analysis with no visible warning. The
    // fix surfaces `ManifestStatus::Unverified` when the diff
    // fetch returned an Err, so renders can label the PR as
    // unverified instead of silently trusting the body manifest.
    #[test]
    fn combiner_marks_pr_unverified_when_diff_fetch_fails() {
        let body = "## Files touched\n```\nsrc/foo.rs\n```\n";
        let value = serde_json::json!({
            "number": 99,
            "title": "feat: thing",
            "body": body,
        });
        let analysed = combine_manifest_and_diff(
            &value,
            Err(anyhow::anyhow!("gh pr view --json files: HTTP 500")),
        );
        assert_eq!(
            analysed.manifest_status,
            ManifestStatus::Unverified,
            "diff fetch error must surface as Unverified so the operator \
             knows the manifest was not cross-checked against the real diff"
        );
    }

    #[test]
    fn combiner_keeps_match_when_diff_agrees() {
        let body = "## Files touched\n```\nsrc/foo.rs\n```\n";
        let value = serde_json::json!({
            "number": 1,
            "title": "ok",
            "body": body,
        });
        let mut diff = BTreeSet::new();
        diff.insert("src/foo.rs".to_string());
        let analysed = combine_manifest_and_diff(&value, Ok(diff));
        assert_eq!(analysed.manifest_status, ManifestStatus::Match);
    }
}
