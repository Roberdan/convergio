//! `cvg coherence close-post-hoc` — surface bypass-the-gate volume.
//!
//! ADR-0026 documents `task.closed_post_hoc` as a deliberate escape
//! hatch: a task can be closed without going through Thor when the
//! work was already done out-of-band (typically because Thor was not
//! yet wired). The hatch is correct by design but invisible — no
//! verifier reports *how often* it is used, *which agent* uses it
//! most, or *what reasons* are claimed.
//!
//! The 2026-05-04 retrospective (finding H5) caught this: 15
//! close-post-hoc closes shipped in one F2 session with zero
//! coherence findings flagging the bypass volume. This module is
//! the fix (P0-4 of the retrospective fix plan).
//!
//! Walks the daemon audit chain via `GET /v1/audit/events`
//! (paginated), filters `transition = task.closed_post_hoc` rows in
//! the requested window, joins task titles best-effort, and emits a
//! row per closure with reason + agent + plan. `--strict` exits
//! non-zero when the count exceeds `--threshold` (default 0 — any
//! close-post-hoc in the window flips strict).

use crate::close_post_hoc_scan::{aggregate, enrich_titles, parse_since, scan_audit};
use crate::OutputMode;
use anyhow::Result;
use chrono::{DateTime, Utc};
use convergio_i18n::Bundle;
use serde::Serialize;

/// One audit row: a single `task.closed_post_hoc` event.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// Daemon-side task uuid.
    pub task_id: String,
    /// Best-effort task title (empty when the task could not be looked up).
    pub task_title: String,
    /// Owning plan uuid.
    pub plan_id: String,
    /// Agent id recorded on the audit row (`""` if missing).
    pub agent_id: String,
    /// Reason field from the audit payload (`""` if missing).
    pub reason: String,
    /// Audit row timestamp.
    pub created_at: DateTime<Utc>,
}

/// Summary report.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Window argument (best-effort interpretation of `--since`).
    pub since: String,
    /// Total `task.closed_post_hoc` rows found in the window.
    pub total: usize,
    /// Rows, oldest-first.
    pub rows: Vec<Row>,
    /// Per-agent count, sorted descending by count then agent_id.
    pub by_agent: Vec<(String, usize)>,
    /// Per-plan count.
    pub by_plan: Vec<(String, usize)>,
}

/// Run the verifier.
pub async fn run(
    bundle: &Bundle,
    output: OutputMode,
    daemon: &str,
    since: &str,
    strict: bool,
    threshold: usize,
) -> Result<()> {
    let cutoff = parse_since(since)?;
    let report = build_report(daemon, since, cutoff).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(&report, bundle),
    }
    if strict && report.total > threshold {
        std::process::exit(1);
    }
    Ok(())
}

async fn build_report(daemon: &str, since: &str, cutoff: DateTime<Utc>) -> Result<Report> {
    let client = reqwest::Client::new();
    let rows = scan_audit(&client, daemon, cutoff).await?;
    let mut report = Report {
        since: since.to_string(),
        total: rows.len(),
        rows,
        ..Report::default()
    };
    report.rows = enrich_titles(&client, daemon, std::mem::take(&mut report.rows)).await;
    report.by_agent = aggregate(&report.rows, |r| r.agent_id.clone());
    report.by_plan = aggregate(&report.rows, |r| r.plan_id.clone());
    Ok(report)
}

fn render_human(report: &Report, bundle: &Bundle) {
    println!(
        "{}",
        bundle.t(
            "coherence-close-post-hoc-header",
            &[
                ("total", &report.total.to_string()),
                ("since", &report.since),
            ]
        )
    );
    if report.rows.is_empty() {
        println!("  {}", bundle.t("coherence-close-post-hoc-clean", &[]));
        return;
    }
    println!();
    println!("  {}", bundle.t("coherence-close-post-hoc-by-agent", &[]));
    for (a, c) in &report.by_agent {
        println!("    {a:<32} {c}");
    }
    println!();
    println!("  {}", bundle.t("coherence-close-post-hoc-by-plan", &[]));
    for (p, c) in &report.by_plan {
        println!("    {:<40} {}", &p[..p.len().min(38)], c);
    }
    println!();
    println!("  {}", bundle.t("coherence-close-post-hoc-rows", &[]));
    for r in &report.rows {
        println!(
            "    {} {} {} ({})",
            r.created_at.format("%Y-%m-%d %H:%M"),
            &r.task_id[..r.task_id.len().min(8)],
            r.task_title,
            r.agent_id
        );
        if !r.reason.is_empty() {
            println!(
                "      {}",
                bundle.t(
                    "coherence-close-post-hoc-row-reason",
                    &[("reason", &r.reason)]
                )
            );
        }
    }
}

fn render_plain(report: &Report) {
    for r in &report.rows {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.created_at.to_rfc3339(),
            r.task_id,
            r.plan_id,
            r.agent_id,
            r.reason
        );
    }
    println!("# total={}", report.total);
}
