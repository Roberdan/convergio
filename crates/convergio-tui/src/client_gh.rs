//! `gh pr list` shell-out helpers.
//!
//! Split out from [`crate::client`] so the HTTP fetcher stays focused
//! and so the 300-line cap is respected. The gh integration is
//! intentionally simple: one subprocess invocation per refresh, no
//! caching beyond what the dashboard already does at tick granularity.

use crate::client::PrSummary;
use anyhow::Result;
use std::process::Command;

/// Run `gh pr list` and parse the JSON. When `slug` is `Some`, the
/// query is scoped to that `owner/repo` (`gh pr list -R <slug>`) so
/// the dashboard works from any cwd. When `None`, gh inherits cwd —
/// original behaviour, kept for shells run inside a repo with no
/// workspace `Cargo.toml`.
pub fn fetch_prs(slug: Option<&str>) -> Result<Vec<PrSummary>> {
    let mut args: Vec<String> = vec![
        "pr".into(),
        "list".into(),
        "--state".into(),
        "all".into(),
        "--limit".into(),
        "100".into(),
        "--json".into(),
        "number,title,headRefName,state,statusCheckRollup,additions,deletions,changedFiles,createdAt,updatedAt,closedAt,mergedAt,body".into(),
    ];
    if let Some(s) = slug {
        args.push("-R".into());
        args.push(s.to_string());
    }
    let out = Command::new("gh").args(&args).output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => return Ok(Vec::new()),
    };
    let raw: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout)?;
    let mut prs = Vec::with_capacity(raw.len());
    for v in raw {
        let ci = ci_rollup(&v);
        let body = v.get("body").and_then(|b| b.as_str()).unwrap_or("");
        let pr: PrSummary = serde_json::from_value(serde_json::json!({
            "number": v.get("number").cloned().unwrap_or(serde_json::json!(0)),
            "title": v.get("title").cloned().unwrap_or_default(),
            "headRefName": v.get("headRefName").cloned().unwrap_or_default(),
            "state": v.get("state").cloned().unwrap_or_default(),
            "ci": ci,
            "additions": v.get("additions").cloned().unwrap_or(serde_json::json!(0)),
            "deletions": v.get("deletions").cloned().unwrap_or(serde_json::json!(0)),
            "changed_files": v.get("changedFiles").cloned().unwrap_or(serde_json::json!(0)),
            "created_at": v.get("createdAt").cloned().unwrap_or_default(),
            "updated_at": v.get("updatedAt").cloned().unwrap_or_default(),
            "closed_at": v.get("closedAt").cloned().unwrap_or_default(),
            "merged_at": v.get("mergedAt").cloned().unwrap_or_default(),
            "tracked_task_ids": parse_tracks_lines(body),
        }))?;
        prs.push(pr);
    }
    Ok(prs)
}

fn parse_tracks_lines(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in body.lines() {
        let Some(rest) = raw.trim().strip_prefix("Tracks:") else {
            continue;
        };
        for token in rest.split(|c: char| c == ',' || c.is_whitespace()) {
            let t = token.trim();
            if is_valid_uuid(t) {
                out.push(t.to_string());
            }
        }
    }
    out
}

fn is_valid_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// Roll a PR's `statusCheckRollup` into one of `success`, `failure`,
/// `pending`, or empty string.
pub fn ci_rollup(v: &serde_json::Value) -> String {
    let checks = match v.get("statusCheckRollup").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return String::new(),
    };
    if checks.is_empty() {
        return String::new();
    }
    let mut any_failure = false;
    let mut any_pending = false;
    for c in checks {
        let conclusion = c
            .get("conclusion")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let status = c
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if conclusion == "failure" || conclusion == "cancelled" {
            any_failure = true;
        }
        if status != "completed" {
            any_pending = true;
        }
    }
    if any_failure {
        "failure".into()
    } else if any_pending {
        "pending".into()
    } else {
        "success".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_rollup_classifies_failure_first() {
        let v = serde_json::json!({"statusCheckRollup": [
            {"status": "completed", "conclusion": "success"},
            {"status": "completed", "conclusion": "failure"},
        ]});
        assert_eq!(ci_rollup(&v), "failure");
    }

    #[test]
    fn ci_rollup_pending_when_any_in_progress() {
        let v = serde_json::json!({"statusCheckRollup": [
            {"status": "completed", "conclusion": "success"},
            {"status": "in_progress", "conclusion": ""},
        ]});
        assert_eq!(ci_rollup(&v), "pending");
    }

    #[test]
    fn ci_rollup_success_when_all_clean() {
        let v = serde_json::json!({"statusCheckRollup": [
            {"status": "completed", "conclusion": "success"},
            {"status": "completed", "conclusion": "success"},
        ]});
        assert_eq!(ci_rollup(&v), "success");
    }

    #[test]
    fn ci_rollup_empty_when_no_checks() {
        let v = serde_json::json!({});
        assert_eq!(ci_rollup(&v), "");
        let v = serde_json::json!({"statusCheckRollup": []});
        assert_eq!(ci_rollup(&v), "");
    }

    #[test]
    fn parse_tracks_lines_extracts_declared_task_ids() {
        let id = "7e33309f-1457-4c8e-9eae-dba599a4a452";
        assert_eq!(parse_tracks_lines(&format!("Tracks: {id}\n")), vec![id]);
        assert!(parse_tracks_lines("Some text Tracks: not-inline").is_empty());
    }
}
