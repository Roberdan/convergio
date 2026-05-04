//! `cvg agent show <id>` — rich, multi-section view of one agent.
//!
//! Pulls `/v1/agent-registry/agents/:id/details`, which already
//! aggregates current task, plan, leases, recent audit and recent
//! PR links server-side. We just render.

use super::agent_format::{color_status, relative, relative_ago_opt, truncate};
use super::{Client, OutputMode};
use anyhow::Result;
use chrono::{DateTime, Utc};
use convergio_i18n::Bundle;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Details {
    id: String,
    kind: String,
    status: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    last_heartbeat_at: Option<DateTime<Utc>>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    current_task_id: Option<String>,
    #[serde(default)]
    current_task_title: Option<String>,
    #[serde(default)]
    current_task_status: Option<String>,
    #[serde(default)]
    current_task_plan_id: Option<String>,
    #[serde(default)]
    current_task_plan_title: Option<String>,
    #[serde(default)]
    current_task_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    leases: Vec<LeaseRow>,
    #[serde(default)]
    recent_audit: Vec<AuditRow>,
    #[serde(default)]
    recent_prs: Vec<PrRow>,
}

#[derive(Debug, Deserialize)]
struct LeaseRow {
    resource_label: String,
    #[serde(default)]
    purpose: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct AuditRow {
    seq: i64,
    transition: String,
    entity_type: String,
    entity_id: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct PrRow {
    repo_slug: String,
    pr_number: i64,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    plan_id: Option<String>,
}

/// Entry point.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode, id: &str) -> Result<()> {
    match client
        .get::<Value>(&format!("/v1/agent-registry/agents/{id}/details"))
        .await
    {
        Ok(v) => {
            let parsed: Details = serde_json::from_value(v.clone())?;
            match output {
                OutputMode::Human => render(bundle, &parsed),
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&v)?),
                OutputMode::Plain => render_plain(&parsed),
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", bundle.t("agent-not-found", &[("id", id)]));
            Err(e)
        }
    }
}

fn render(bundle: &Bundle, d: &Details) {
    let now = Utc::now();
    println!(
        "{}",
        bundle.t("agent-show-header", &[("id", d.id.as_str())])
    );
    let registered = d
        .created_at
        .map(|c| c.to_rfc3339())
        .unwrap_or_else(|| "-".into());
    println!(
        "{}: {} ({})",
        bundle.t("agent-show-kind", &[]),
        d.kind,
        bundle.t("agent-show-registered", &[("at", &registered)])
    );
    let hb = relative_ago_opt(d.last_heartbeat_at.as_ref(), &now);
    println!(
        "{}: {} ({})",
        bundle.t("agent-show-status", &[]),
        color_status(&d.status),
        hb
    );
    if !d.capabilities.is_empty() {
        println!(
            "{}: {}",
            bundle.t("agent-show-capabilities", &[]),
            d.capabilities.join(", ")
        );
    }
    println!();
    render_current_task(bundle, d, &now);
    println!();
    render_leases(bundle, d, &now);
    println!();
    render_audit(bundle, d, &now);
    println!();
    render_prs(bundle, d);
}

fn render_current_task(bundle: &Bundle, d: &Details, now: &DateTime<Utc>) {
    println!("{}:", bundle.t("agent-show-current-task", &[]));
    let Some(task_id) = &d.current_task_id else {
        println!("  {}", bundle.t("agent-show-no-current-task", &[]));
        return;
    };
    let title = d.current_task_title.as_deref().unwrap_or(task_id);
    println!("  {}", truncate(title, 80));
    if let Some(plan_title) = &d.current_task_plan_title {
        let plan_id = d.current_task_plan_id.as_deref().unwrap_or("-");
        println!(
            "  {}: {} ({})",
            bundle.t("agent-show-plan", &[]),
            truncate(plan_title, 60),
            short_id(plan_id)
        );
    }
    if let Some(status) = &d.current_task_status {
        let started = d
            .current_task_started_at
            .map(|t| relative_ago_opt(Some(&t), now))
            .unwrap_or_else(|| "-".into());
        println!(
            "  {}: {} ({})",
            bundle.t("agent-show-task-status", &[]),
            status,
            started
        );
    }
}

fn render_leases(bundle: &Bundle, d: &Details, now: &DateTime<Utc>) {
    println!(
        "{} ({}):",
        bundle.t("agent-show-leases", &[]),
        d.leases.len()
    );
    if d.leases.is_empty() {
        println!("  {}", bundle.t("agent-show-no-leases", &[]));
        return;
    }
    for l in &d.leases {
        let held = relative(&l.created_at, now);
        let expires = relative(now, &l.expires_at);
        let purpose = l.purpose.clone().unwrap_or_else(|| "-".into());
        println!(
            "  {} ({}, held {held}, expires in {expires})",
            l.resource_label, purpose
        );
    }
}

fn render_audit(bundle: &Bundle, d: &Details, now: &DateTime<Utc>) {
    println!(
        "{} ({}):",
        bundle.t("agent-show-recent-audit", &[]),
        d.recent_audit.len()
    );
    if d.recent_audit.is_empty() {
        println!("  {}", bundle.t("agent-show-no-recent-audit", &[]));
        return;
    }
    for a in &d.recent_audit {
        let when = relative(&a.created_at, now);
        println!(
            "  #{:<5} {when:<6} {:<24} {}:{}",
            a.seq,
            truncate(&a.transition, 22),
            a.entity_type,
            short_id(&a.entity_id)
        );
    }
}

fn render_prs(bundle: &Bundle, d: &Details) {
    println!(
        "{} ({}):",
        bundle.t("agent-show-recent-prs", &[]),
        d.recent_prs.len()
    );
    if d.recent_prs.is_empty() {
        println!("  {}", bundle.t("agent-show-no-recent-prs", &[]));
        return;
    }
    for p in &d.recent_prs {
        let branch = p.branch.clone().unwrap_or_else(|| "-".into());
        println!(
            "  #{} {}/{} {}",
            p.pr_number,
            p.repo_slug,
            branch,
            plan_short(p)
        );
    }
}

fn plan_short(p: &PrRow) -> String {
    p.plan_id
        .as_deref()
        .map(short_id)
        .map(|s| format!("plan:{s}"))
        .unwrap_or_default()
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn render_plain(d: &Details) {
    let task = d.current_task_title.clone().unwrap_or_else(|| "-".into());
    let branch = d
        .recent_prs
        .first()
        .and_then(|p| p.branch.clone())
        .unwrap_or_else(|| "-".into());
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        d.id,
        d.kind,
        d.status,
        d.current_task_id.clone().unwrap_or_else(|| "-".into()),
        task,
        branch
    );
}
