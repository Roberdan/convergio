//! Per-PR verdict logic for [`crate::agents`].
//!
//! Split out to honour the 300-line per-file cap. Pure: takes a PR
//! plus the registry snapshot, returns a [`Row`].

use crate::agents::{Finding, Row};
use crate::agents_scan::{AgentSummary, MergedPr, ScanContext};

/// Per-PR verdict. Allowlisted PRs are always `Clean`.
pub(crate) fn judge(pr: &MergedPr, ctx: &ScanContext) -> Row {
    if is_allowlisted(pr) {
        return clean(pr, "", "allowlisted (machine-authored)");
    }
    if !ctx.daemon_reachable {
        return clean(pr, "", "daemon unreachable; advisory skip");
    }
    let Some(agent) = match_agent(&ctx.registry, &pr.author) else {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: String::new(),
            finding: Finding::NoRegisteredAgent,
            evidence: format!("no agent_id matches '{}'", pr.author),
        };
    };
    if !heartbeat_in_window(agent, pr) {
        return Row {
            pr: pr.label(),
            author: pr.author.clone(),
            branch: pr.branch.clone(),
            agent_matched: agent.id.clone(),
            finding: Finding::NoHeartbeatInWindow,
            evidence: match agent.last_heartbeat_at.as_ref() {
                Some(ts) => format!("last heartbeat {ts} outside window"),
                None => "no heartbeat ever recorded".into(),
            },
        };
    }
    let active = active_agents_in_window(&ctx.registry, pr);
    let evidence = if active >= 2 {
        format!("heartbeat ok; {active} agents active")
    } else {
        "heartbeat ok".to_string()
    };
    clean(pr, &agent.id, &evidence)
}

fn clean(pr: &MergedPr, agent_id: &str, evidence: &str) -> Row {
    Row {
        pr: pr.label(),
        author: pr.author.clone(),
        branch: pr.branch.clone(),
        agent_matched: agent_id.to_string(),
        finding: Finding::Clean,
        evidence: evidence.to_string(),
    }
}

/// Allowlist: machine-authored PRs that don't claim agent attribution.
pub(crate) fn is_allowlisted(pr: &MergedPr) -> bool {
    let login = pr.author.to_ascii_lowercase();
    if login == "release-please-bot" || login == "dependabot[bot]" || login == "dependabot" {
        return true;
    }
    let title_lc = pr.title.to_ascii_lowercase();
    title_lc.starts_with("chore(deps):") || title_lc.starts_with("chore(release):")
}

/// Find an agent whose id is the canonical `claude-code-${login}`
/// form, falling back to any id that contains the login as a
/// substring. Lower-case match throughout.
pub(crate) fn match_agent<'a>(
    registry: &'a [AgentSummary],
    author: &str,
) -> Option<&'a AgentSummary> {
    let login = author.to_ascii_lowercase();
    if login.is_empty() {
        return None;
    }
    let prefixed = format!("claude-code-{login}");
    if let Some(hit) = registry
        .iter()
        .find(|a| a.id.to_ascii_lowercase() == prefixed)
    {
        return Some(hit);
    }
    registry
        .iter()
        .find(|a| a.id.to_ascii_lowercase().contains(&login))
}

/// True if the matched agent has a heartbeat inside the PR window
/// padded by ±10 minutes.
fn heartbeat_in_window(agent: &AgentSummary, pr: &MergedPr) -> bool {
    let Some(hb) = agent.last_heartbeat_at else {
        return false;
    };
    let window_start = pr.first_commit_at - chrono::Duration::minutes(10);
    let window_end = pr.merged_at + chrono::Duration::minutes(10);
    hb >= window_start && hb <= window_end
}

/// Count agents whose `last_heartbeat_at` falls inside the PR window.
fn active_agents_in_window(registry: &[AgentSummary], pr: &MergedPr) -> usize {
    let window_start = pr.first_commit_at - chrono::Duration::minutes(10);
    let window_end = pr.merged_at + chrono::Duration::minutes(10);
    registry
        .iter()
        .filter(|a| {
            a.last_heartbeat_at
                .map(|hb| hb >= window_start && hb <= window_end)
                .unwrap_or(false)
        })
        .count()
}
