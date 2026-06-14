//! Scan helpers for [`crate::agents`].
//!
//! Two independent inputs:
//!
//! 1. **Merged PRs from git** — `git log --merges` over the requested
//!    window. Parsing lives in [`crate::agents_parse`].
//! 2. **Daemon agent registry** — `GET /v1/agent-registry/agents`. If
//!    the daemon is unreachable we degrade to advisory mode. The
//!    response is mapped to [`AgentSummary`] (a thin shape that does
//!    not require depending on `convergio-durability`).

use crate::agents_parse::{normalise_since, parse_git_log, parse_revision_range};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Best-effort summary of one merged PR pulled out of `git log`.
#[derive(Debug, Clone)]
pub struct MergedPr {
    /// PR number when parseable from the merge subject (`"138"`),
    /// otherwise empty.
    pub number: String,
    /// Author login. We use `committer name` of the merge commit as a
    /// proxy when the GitHub login is not embedded.
    pub author: String,
    /// Branch name (best-effort from `Merge pull request #N from X/Y`).
    pub branch: String,
    /// PR title (best-effort: first non-empty line of merge body, or
    /// merge subject).
    pub title: String,
    /// Merge commit SHA (full).
    pub sha: String,
    /// Merge commit time.
    pub merged_at: DateTime<Utc>,
    /// Author-time of the oldest commit reachable from the merged head
    /// but not from the first parent. Falls back to `merged_at` when
    /// it cannot be computed.
    pub first_commit_at: DateTime<Utc>,
}

impl MergedPr {
    /// Display label: `"#138"` or short SHA when number is unknown.
    pub fn label(&self) -> String {
        if !self.number.is_empty() {
            format!("#{}", self.number)
        } else {
            self.sha.chars().take(8).collect()
        }
    }
}

/// Lite shape of `/v1/agent-registry/agents` rows. Mirrors
/// `convergio_durability::AgentRecord` without the dep.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentSummary {
    /// Stable agent id.
    pub id: String,
    /// Last heartbeat timestamp (optional in the response).
    #[serde(default)]
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

/// Glue passed to per-PR judging.
#[derive(Debug, Clone)]
pub struct ScanContext {
    /// Snapshot of the registry at scan time.
    pub registry: Vec<AgentSummary>,
    /// True if the daemon HTTP endpoint responded.
    pub daemon_reachable: bool,
    /// Daemon base URL (used for follow-up queries that may be added
    /// later — currently held for the bus-coordination check that the
    /// spec marks advisory-only).
    #[allow(dead_code)]
    pub daemon: String,
}

/// Run `git log --merges --pretty=...` over the requested window and
/// return one [`MergedPr`] per merge commit.
pub fn list_merged_prs(root: &Path, since: &str) -> Result<Vec<MergedPr>> {
    let log = run_git_log(root, since)?;
    Ok(parse_git_log(&log))
}

fn run_git_log(root: &Path, since: &str) -> Result<String> {
    // Sentinel-delimited record format. Records are separated by
    // `RECORD\x1e`. Fields by `\x1f`. Embeds: %H, %ct, %an, %s, %b.
    let pretty = "RECORD%x1e%H%x1f%ct%x1f%an%x1f%s%x1f%b";
    let arg = if let Some(rev) = parse_revision_range(since) {
        rev.to_string()
    } else {
        format!("--since={}", normalise_since(since))
    };
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("log")
        .arg("--merges")
        .arg("--first-parent")
        .arg(format!("--pretty=format:{pretty}"))
        .arg(&arg)
        .output()
        .with_context(|| "spawn git log")?;
    if !out.status.success() {
        anyhow::bail!(
            "git log failed (exit={}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Hit the daemon and return the registry. Soft-fails on any error.
pub async fn fetch_agent_registry(daemon: &str) -> Result<Vec<AgentSummary>> {
    let url = format!("{}/v1/agent-registry/agents", daemon.trim_end_matches('/'));
    let client = crate::http::daemon_client(Duration::from_secs(2))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("daemon returned {}", resp.status());
    }
    let agents: Vec<AgentSummary> = resp.json().await.with_context(|| "decode agents")?;
    Ok(agents)
}
