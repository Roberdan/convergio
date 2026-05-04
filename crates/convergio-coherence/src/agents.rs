//! `cvg coherence agents` — flag merged PRs whose author skipped the
//! multi-agent protocol (registry / heartbeat / coordination bus).
//!
//! The verdict per merged PR in the window:
//!
//! - `no_registered_agent` — no agent_id in `agent_registry` matches
//!   the PR author login (strict-failing).
//! - `no_heartbeat_in_window` — matching agent_id, but zero heartbeats
//!   in the [first_commit, merge_time + 10min] window
//!   (strict-failing).
//! - `no_coordination` — matching agent + heartbeat, but no bus
//!   messages on `coordination/agents` topic when ≥ 2 distinct
//!   agent_ids were active during the window (advisory only).
//! - `clean` — all above satisfied, or PR is allowlisted.
//!
//! Daemon HTTP queries are best-effort: if `127.0.0.1:8420` is
//! unreachable we degrade to advisory mode, emit no findings beyond
//! what filesystem/git can resolve, and keep the strict exit clear.
//! This is intentional — the verifier is a guardrail for agent
//! honesty, not a hard CI block until the `SessionStart` hook
//! (sister Agent A) has bedded in.
//!
//! Internationalised through `convergio-i18n` (P5). Keys live in
//! `crates/convergio-i18n/locales/{en,it}/main.ftl` under
//! `# ---------- CLI: coherence agents ----------`.

use crate::agents_judge::judge;
use crate::agents_scan::{fetch_agent_registry, list_merged_prs, ScanContext};
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Emitted finding bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    /// No agent_id in `agent_registry` matches the PR author.
    NoRegisteredAgent,
    /// Matching agent_id but zero heartbeats in the window.
    NoHeartbeatInWindow,
    /// Matching agent + heartbeat but zero coordination messages
    /// during multi-agent activity. Reserved: surfaced once bus
    /// polling lands (advisory-only by spec).
    #[allow(dead_code)]
    NoCoordination,
    /// All checks satisfied (or allowlisted).
    Clean,
}

impl Finding {
    /// Fluent message key for this finding.
    pub fn ftl_key(self) -> &'static str {
        match self {
            Finding::NoRegisteredAgent => "coherence-agents-finding-no-registered-agent",
            Finding::NoHeartbeatInWindow => "coherence-agents-finding-no-heartbeat",
            Finding::NoCoordination => "coherence-agents-finding-no-coordination",
            Finding::Clean => "coherence-agents-finding-clean",
        }
    }

    /// Plain-text key (output mode `plain`, JSON, tests).
    pub fn key(self) -> &'static str {
        match self {
            Finding::NoRegisteredAgent => "no_registered_agent",
            Finding::NoHeartbeatInWindow => "no_heartbeat_in_window",
            Finding::NoCoordination => "no_coordination",
            Finding::Clean => "clean",
        }
    }

    /// True for findings that flip the exit code under `--strict`.
    pub fn is_strict(self) -> bool {
        matches!(
            self,
            Finding::NoRegisteredAgent | Finding::NoHeartbeatInWindow
        )
    }
}

/// One row in the report — one per merged PR in the window.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// PR number (`"#138"`) or merge SHA prefix when number is unknown.
    pub pr: String,
    /// PR author login (best-effort: derived from merge commit).
    pub author: String,
    /// Branch name (PR head), best-effort from merge commit subject.
    pub branch: String,
    /// Matched agent_id in registry (or empty).
    pub agent_matched: String,
    /// Finding bucket.
    pub finding: Finding,
    /// Short evidence hint (e.g. `"1 heartbeat, 2 messages"`).
    pub evidence: String,
}

/// Summary row.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// Window descriptor (e.g. `"7d"`).
    pub since: String,
    /// PRs scanned.
    pub prs_checked: usize,
    /// Findings (rows whose finding != `Clean`).
    pub findings_count: usize,
    /// Per-kind tally.
    pub by_kind: BTreeMap<String, usize>,
    /// True if daemon was reachable; false → degraded mode.
    pub daemon_reachable: bool,
    /// Whether the run would pass under `--strict`.
    pub strict_passes: bool,
}

/// Full report.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Top-level summary.
    pub summary: Summary,
    /// One row per merged PR (including `Clean`).
    pub findings: Vec<Row>,
}

/// CLI entry point. Renders + sets exit code under `--strict`.
pub async fn run(
    bundle: &Bundle,
    output: OutputMode,
    root: &Path,
    since: &str,
    strict: bool,
    daemon: &str,
) -> Result<()> {
    let report = run_check(root, since, daemon).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(&report, bundle),
    }
    tracing::info!(
        target: "convergio.coherence.agents",
        since = %report.summary.since,
        prs_checked = report.summary.prs_checked,
        findings_count = report.summary.findings_count,
        daemon_reachable = report.summary.daemon_reachable,
        "coherence.agents.scan",
    );
    if strict && !report.summary.strict_passes {
        std::process::exit(1);
    }
    Ok(())
}

/// Run the verifier and produce a report. Filesystem + HTTP. Swallows
/// daemon errors → degrades to `daemon_reachable=false`.
pub async fn run_check(root: &Path, since: &str, daemon: &str) -> Result<Report> {
    let prs = list_merged_prs(root, since)?;
    let (registry, daemon_reachable) = match fetch_agent_registry(daemon).await {
        Ok(v) => (v, true),
        Err(_) => (Vec::new(), false),
    };
    let ctx = ScanContext {
        registry,
        daemon_reachable,
        daemon: daemon.to_string(),
    };
    let mut rows = Vec::with_capacity(prs.len());
    for pr in &prs {
        rows.push(judge(pr, &ctx));
    }
    let findings_count = rows.iter().filter(|r| r.finding != Finding::Clean).count();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        *by_kind.entry(r.finding.key().to_string()).or_insert(0) += 1;
    }
    let strict_passes = !rows.iter().any(|r| r.finding.is_strict());
    Ok(Report {
        summary: Summary {
            since: since.to_string(),
            prs_checked: prs.len(),
            findings_count,
            by_kind,
            daemon_reachable,
            strict_passes,
        },
        findings: rows,
    })
}

fn render_human(report: &Report, bundle: &Bundle) {
    println!(
        "{}",
        bundle.t(
            "coherence-agents-summary",
            &[
                ("checked", &report.summary.prs_checked.to_string()),
                ("since", &report.summary.since),
                ("findings", &report.summary.findings_count.to_string()),
                ("strict", &report.summary.strict_passes.to_string()),
            ]
        )
    );
    if report.findings.is_empty() {
        println!("{}", bundle.t("coherence-agents-empty", &[]));
        return;
    }
    println!("{}", bundle.t("coherence-agents-table-header", &[]));
    for r in &report.findings {
        let finding = bundle.t(r.finding.ftl_key(), &[]);
        println!(
            "  {:<6} {:<22} {:<24} {:<26} {}",
            r.pr, r.author, r.agent_matched, finding, r.evidence
        );
    }
}

fn render_plain(report: &Report) {
    println!(
        "checked={} since={} findings={} strict_passes={} daemon_reachable={}",
        report.summary.prs_checked,
        report.summary.since,
        report.summary.findings_count,
        report.summary.strict_passes,
        report.summary.daemon_reachable,
    );
    for r in &report.findings {
        if r.finding == Finding::Clean {
            continue;
        }
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.pr,
            r.author,
            r.agent_matched,
            r.finding.key(),
            r.evidence
        );
    }
}
