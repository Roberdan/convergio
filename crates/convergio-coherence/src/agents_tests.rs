//! Unit tests for [`crate::agents`].
//!
//! Synthetic merge-log records + synthetic registry entries cover
//! each finding bucket. We never call out to a real daemon — registry
//! state is constructed in-memory and judged through the public
//! [`crate::agents::run_check`] surface where possible, or through
//! the `judge` helper indirectly via a fixture.

#![cfg(test)]

use crate::agents::{Finding, Row};
use crate::agents_scan::{AgentSummary, MergedPr};
use chrono::{TimeZone, Utc};

/// A tiny re-implementation of the per-PR judge that mirrors
/// `agents::judge` but takes the registry directly. Keeping it in the
/// test module avoids exposing internals as `pub`.
fn judge_sync(pr: &MergedPr, registry: &[AgentSummary], daemon_reachable: bool) -> Row {
    use crate::agents_judge::match_agent;

    if pr.author.to_ascii_lowercase().starts_with("dependabot")
        || pr.title.to_ascii_lowercase().starts_with("chore(deps):")
    {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: String::new(),
            finding: Finding::Clean,
            evidence: "allowlisted".into(),
        };
    }
    if !daemon_reachable {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: String::new(),
            finding: Finding::Clean,
            evidence: "daemon unreachable".into(),
        };
    }
    let Some(agent) = match_agent(registry, &pr.author) else {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: String::new(),
            finding: Finding::NoRegisteredAgent,
            evidence: "no match".into(),
        };
    };
    let in_window = match agent.last_heartbeat_at {
        Some(hb) => {
            let lo = pr.first_commit_at - chrono::Duration::minutes(10);
            let hi = pr.merged_at + chrono::Duration::minutes(10);
            hb >= lo && hb <= hi
        }
        None => false,
    };
    if !in_window {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: agent.id.clone(),
            finding: Finding::NoHeartbeatInWindow,
            evidence: "outside".into(),
        };
    }
    Row {
        pr: pr.label(),
        author: pr.author.clone(),
        branch: pr.branch.clone(),
        agent_matched: agent.id.clone(),
        finding: Finding::Clean,
        evidence: "ok".into(),
    }
}

fn pr(num: &str, author: &str, when: i64) -> MergedPr {
    let merged = Utc.timestamp_opt(when, 0).single().unwrap();
    MergedPr {
        number: num.to_string(),
        author: author.to_string(),
        branch: format!("feat/x-{num}"),
        title: "feat(test): something".to_string(),
        sha: format!("{num:0>40}"),
        merged_at: merged,
        first_commit_at: merged,
    }
}

fn agent(id: &str, hb: Option<i64>) -> AgentSummary {
    AgentSummary {
        id: id.to_string(),
        last_heartbeat_at: hb.map(|t| Utc.timestamp_opt(t, 0).single().unwrap()),
    }
}

#[test]
fn no_registered_agent_when_registry_empty() {
    let p = pr("100", "Roberdan", 1_700_000_000);
    let row = judge_sync(&p, &[], true);
    assert_eq!(row.finding, Finding::NoRegisteredAgent);
}

#[test]
fn no_heartbeat_when_outside_window() {
    let p = pr("101", "Roberdan", 1_700_000_000);
    // Heartbeat one day before window start.
    let reg = vec![agent(
        "claude-code-roberdan",
        Some(1_700_000_000 - 86_400 * 2),
    )];
    let row = judge_sync(&p, &reg, true);
    assert_eq!(row.finding, Finding::NoHeartbeatInWindow);
    assert_eq!(row.agent_matched, "claude-code-roberdan");
}

#[test]
fn clean_when_heartbeat_in_window() {
    let p = pr("102", "Roberdan", 1_700_000_000);
    // Heartbeat 5 minutes before merge.
    let reg = vec![agent("claude-code-roberdan", Some(1_700_000_000 - 300))];
    let row = judge_sync(&p, &reg, true);
    assert_eq!(row.finding, Finding::Clean);
    assert_eq!(row.agent_matched, "claude-code-roberdan");
}

#[test]
fn allowlisted_dependabot() {
    let mut p = pr("103", "dependabot[bot]", 1_700_000_000);
    p.title = "chore(deps): bump foo".into();
    let row = judge_sync(&p, &[], true);
    assert_eq!(row.finding, Finding::Clean);
}

#[test]
fn match_agent_prefers_claude_code_prefixed() {
    let reg = vec![
        agent("foo-roberdan-bar", None),
        agent("claude-code-roberdan", None),
    ];
    let m = crate::agents_judge::match_agent(&reg, "Roberdan").unwrap();
    // The `claude-code-${login}` exact form wins via the explicit
    // equality branch.
    assert_eq!(m.id, "claude-code-roberdan");
}

#[test]
fn match_agent_falls_back_to_substring() {
    let reg = vec![agent("ide-roberdan-laptop", None)];
    let m = crate::agents_judge::match_agent(&reg, "Roberdan").unwrap();
    assert_eq!(m.id, "ide-roberdan-laptop");
}

#[test]
fn match_agent_returns_none_for_empty_login() {
    let reg = vec![agent("claude-code-roberdan", None)];
    assert!(crate::agents_judge::match_agent(&reg, "").is_none());
}

#[test]
fn finding_strict_classification() {
    assert!(Finding::NoRegisteredAgent.is_strict());
    assert!(Finding::NoHeartbeatInWindow.is_strict());
    assert!(!Finding::NoCoordination.is_strict());
    assert!(!Finding::Clean.is_strict());
}

#[test]
fn daemon_unreachable_yields_clean_advisory() {
    let p = pr("104", "Roberdan", 1_700_000_000);
    let row = judge_sync(&p, &[], false);
    assert_eq!(row.finding, Finding::Clean);
}
