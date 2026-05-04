//! `check.plan_pr_drift` — plan-vs-merged-PR drift.
//!
//! For each task referenced from a recently merged PR, query the
//! daemon for the task's current state. Flag any task that the
//! merged PR claims to track but whose state is still `pending` or
//! `submitted` — that means the merge happened without the
//! corresponding plan transition, so the plan no longer reflects
//! shipped reality.
//!
//! ## How references are extracted
//!
//! PR bodies and titles are scanned for two patterns:
//!
//! 1. `Tracks: <uuid>` (canonical, what the agent docs ask for)
//! 2. Bare 36-char UUIDs and 8-char UUID prefixes adjacent to the
//!    word `task` (case-insensitive)
//!
//! Both are conservative — false negatives (missed reference) are
//! preferred over false positives (flagging an unrelated task).
//!
//! ## Conservative on failure
//!
//! Missing `gh` / `git` / `curl`, daemon unreachable, JSON parse
//! errors — every failure path collapses to `Pass`. A safety net is
//! not allowed to be a brick wall.

use crate::pre_stop::{Check, CheckContext, CheckOutcome};
use std::collections::BTreeSet;
use std::process::Command;

/// Concrete check implementation.
pub struct PlanPrDriftCheck;

impl Check for PlanPrDriftCheck {
    fn id(&self) -> &'static str {
        "check.plan_pr_drift"
    }
    fn label(&self) -> &'static str {
        "plan-vs-merged-PR drift"
    }
    fn run(&self, ctx: &CheckContext) -> CheckOutcome {
        let merged = match recent_merged_prs() {
            Ok(v) => v,
            Err(_) => return CheckOutcome::Pass,
        };
        let mut task_ids: BTreeSet<String> = BTreeSet::new();
        for pr in &merged {
            for tid in extract_task_ids(&format!("{}\n{}", pr.title, pr.body)) {
                task_ids.insert(tid);
            }
        }
        if task_ids.is_empty() {
            return CheckOutcome::Pass;
        }
        let mut findings = Vec::new();
        for tid in &task_ids {
            match fetch_task_status(&ctx.daemon_url, tid) {
                Some(status) if status == "pending" || status == "submitted" => {
                    let pr = first_pr_referencing(&merged, tid);
                    findings.push(format!(
                        "task {tid} is {status} but PR #{pr_num} ({title}) is merged",
                        pr_num = pr.map(|p| p.number).unwrap_or(0),
                        title = pr.map(|p| p.title.as_str()).unwrap_or("?")
                    ));
                }
                _ => {}
            }
        }
        if findings.is_empty() {
            CheckOutcome::Pass
        } else {
            CheckOutcome::Fail { findings }
        }
    }
}

/// One row from `gh pr list --state merged --json number,title,body`.
#[derive(Debug, Clone)]
pub(crate) struct MergedPr {
    pub(crate) number: i64,
    pub(crate) title: String,
    pub(crate) body: String,
}

/// Default lookback window for merged-PR drift, in days. We deliberately
/// do not look further back: the check is meant to catch drift introduced
/// during the live session, not archaeological cleanup.
const LOOKBACK_DAYS: u32 = 7;

fn recent_merged_prs() -> Result<Vec<MergedPr>, ()> {
    let search = format!("is:merged merged:>={}", iso_days_ago(LOOKBACK_DAYS));
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "merged",
            "--search",
            &search,
            "--json",
            "number,title,body",
            "--limit",
            "50",
        ])
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    parse_gh_output(&String::from_utf8_lossy(&out.stdout))
}

pub(crate) fn parse_gh_output(text: &str) -> Result<Vec<MergedPr>, ()> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|_| ())?;
    let arr = v.as_array().ok_or(())?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let number = item.get("number").and_then(|n| n.as_i64()).ok_or(())?;
        let title = item
            .get("title")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let body = item
            .get("body")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        out.push(MergedPr {
            number,
            title,
            body,
        });
    }
    Ok(out)
}

fn iso_days_ago(days: u32) -> String {
    // gh search supports YYYY-MM-DD; produce one in UTC.
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::days(days as i64);
    cutoff.format("%Y-%m-%d").to_string()
}

/// Extract task ids from a free-text blob.
///
/// Recognised forms:
/// - `Tracks: <uuid>` and `Tracks: <8-hex>`
/// - bare UUIDs adjacent to the word "task"
pub(crate) fn extract_task_ids(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(rest) = lower.strip_prefix("tracks:") {
            for tok in rest.split([' ', ',', ';', '\t']) {
                let tok = tok.trim().trim_start_matches('t');
                if is_task_id(tok) {
                    out.insert(tok.to_string());
                }
            }
        }
        if lower.contains("task") {
            for tok in line.split([' ', ',', ';', '\t', '(', ')', '[', ']']) {
                let tok = tok.trim();
                if is_task_id(tok) {
                    out.insert(tok.to_lowercase());
                }
            }
        }
    }
    out
}

fn is_task_id(s: &str) -> bool {
    // Full uuid or 8-hex prefix.
    let bytes = s.as_bytes();
    match bytes.len() {
        36 => bytes.iter().enumerate().all(|(i, b)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        }),
        8 => bytes.iter().all(|b| b.is_ascii_hexdigit()),
        _ => false,
    }
}

/// Hit `GET /v1/tasks/<id>` via `curl`. Sync-friendly (no async runtime
/// required) — the daemon URL comes from [`CheckContext`].
fn fetch_task_status(daemon_url: &str, task_id: &str) -> Option<String> {
    let url = format!("{daemon_url}/v1/tasks/{task_id}");
    let out = Command::new("curl")
        .args(["-sf", "--max-time", "5", &url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    v.get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

fn first_pr_referencing<'a>(prs: &'a [MergedPr], task_id: &str) -> Option<&'a MergedPr> {
    prs.iter().find(|p| {
        let blob = format!("{}\n{}", p.title, p.body);
        extract_task_ids(&blob)
            .iter()
            .any(|t| t == task_id || task_id.starts_with(t.as_str()) || t.starts_with(task_id))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_task_ids_recognises_tracks_line() {
        let text = "Some preamble.\nTracks: 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea\nMore.";
        let ids = extract_task_ids(text);
        assert!(ids.contains("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
    }

    #[test]
    fn extract_task_ids_recognises_short_prefix_tracks() {
        let text = "Tracks: T5298055b\nMore.";
        let ids = extract_task_ids(text);
        assert!(ids.contains("5298055b"));
    }

    #[test]
    fn extract_task_ids_ignores_non_hex_garbage() {
        let text = "Tracks: notauuid here-is-junk-not-hex 12345678";
        let ids = extract_task_ids(text);
        assert!(ids.contains("12345678"));
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn extract_task_ids_finds_uuid_near_task_word() {
        let text = "this PR closes task 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea";
        let ids = extract_task_ids(text);
        assert!(ids.contains("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
    }

    #[test]
    fn is_task_id_validates_shape() {
        assert!(is_task_id("5298055b"));
        assert!(is_task_id("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
        assert!(!is_task_id("5298055G"));
        assert!(!is_task_id("not-a-uuid-12345678901234567890123456"));
        assert!(!is_task_id(""));
    }

    #[test]
    fn parse_gh_output_round_trips() {
        let json = r#"[{"number":12,"title":"feat","body":"Tracks: 5298055b"}]"#;
        let prs = parse_gh_output(json).expect("ok");
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 12);
        assert_eq!(prs[0].body, "Tracks: 5298055b");
    }

    #[test]
    fn check_id_and_label_are_stable() {
        let c = PlanPrDriftCheck;
        assert_eq!(c.id(), "check.plan_pr_drift");
        assert!(c.label().contains("drift"));
    }
}
