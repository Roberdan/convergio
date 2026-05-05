//! `cvg coherence plan-execution` — per-plan mechanism compliance verifier.
//!
//! Implements the contract defined in ADR-0044. For each closed (`done` or
//! `submitted`) task in a plan, verifies that the required evidence kinds
//! were attached. Produces a compliance score (0–100%).
//!
//! Task types are inferred from evidence present:
//! - `code` — `code` or `merge_record` evidence present → requires `context_pack`,
//!   `ci_run`, `merge_record`.
//! - `doc_only` — `adr` evidence, no `code`/`merge_record` → requires `ci_run`,
//!   `merge_record`.
//! - `analysis` — no `code`, `merge_record`, or `adr` → no evidence requirements.

use crate::plan_execution_scan as scan;
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use std::collections::HashSet;

/// Task-type classification (see ADR-0044 §task-type-contract-table).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Task includes code changes (`code` or `merge_record` evidence).
    Code,
    /// Task includes an ADR commit but no code evidence.
    DocOnly,
    /// Task has neither code nor ADR evidence; treated as research/analysis.
    Analysis,
}

/// Required evidence kinds per task type (see ADR-0044).
fn required_kinds(t: &TaskType) -> &'static [&'static str] {
    match t {
        TaskType::Code => &["context_pack", "ci_run", "merge_record"],
        TaskType::DocOnly => &["ci_run", "merge_record"],
        TaskType::Analysis => &[],
    }
}

fn infer_type(evidence_kinds: &HashSet<String>) -> TaskType {
    if evidence_kinds.contains("code") || evidence_kinds.contains("merge_record") {
        TaskType::Code
    } else if evidence_kinds.contains("adr") {
        TaskType::DocOnly
    } else {
        TaskType::Analysis
    }
}

/// Per-task compliance result.
#[derive(Debug, Serialize)]
pub struct TaskResult {
    /// Full task uuid.
    pub task_id: String,
    /// Task title.
    pub task_title: String,
    /// Current task status (`done` or `submitted`).
    pub task_status: String,
    /// Inferred task type.
    pub task_type: TaskType,
    /// Evidence kinds attached to this task (sorted).
    pub evidence_kinds: Vec<String>,
    /// Required evidence kinds that are absent.
    pub missing_required: Vec<String>,
    /// True when `missing_required` is empty.
    pub compliant: bool,
}

/// Plan-level compliance summary.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Plan uuid.
    pub plan_id: String,
    /// Total closed (done + submitted) tasks evaluated.
    pub tasks_closed: usize,
    /// Tasks that satisfy all required mechanisms.
    pub tasks_compliant: usize,
    /// Compliance score (0–100; 100 = all tasks compliant).
    pub score_pct: u8,
    /// True when the agent registry had at least one active agent.
    pub registry_ok: bool,
    /// True when the plan bus had coordination messages from an agent.
    pub bus_ok: bool,
    /// Per-task breakdown.
    pub tasks: Vec<TaskResult>,
}

/// Run the verifier.
pub async fn run(
    bundle: &Bundle,
    output: OutputMode,
    daemon: &str,
    plan_id: &str,
    strict: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let report = build_report(&client, daemon, plan_id).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(bundle, &report),
    }
    if strict && (report.score_pct < 100 || !report.registry_ok || !report.bus_ok) {
        std::process::exit(1);
    }
    Ok(())
}

async fn build_report(client: &reqwest::Client, daemon: &str, plan_id: &str) -> Result<Report> {
    let all_tasks = scan::fetch_tasks(client, daemon, plan_id).await?;
    let closed: Vec<_> = all_tasks
        .into_iter()
        .filter(|t| t.status == "done" || t.status == "submitted")
        .collect();

    let agents = scan::fetch_agents(client, daemon).await;
    let registry_ok = !agents.is_empty();

    let bus_msgs = scan::fetch_bus_messages(client, daemon, plan_id).await;
    let bus_ok = bus_msgs
        .iter()
        .any(|m| !m.sender.to_lowercase().starts_with("system") && !m.topic.is_empty());

    let mut task_results = Vec::with_capacity(closed.len());
    for task in &closed {
        let ev = scan::fetch_evidence(client, daemon, &task.id).await;
        let kinds: HashSet<String> = ev.iter().map(|e| e.kind.clone()).collect();
        let kind_list: Vec<String> = {
            let mut v: Vec<_> = kinds.iter().cloned().collect();
            v.sort();
            v
        };
        let task_type = infer_type(&kinds);
        let required = required_kinds(&task_type);
        let missing: Vec<String> = required
            .iter()
            .filter(|k| !kinds.contains(**k))
            .map(|k| (*k).to_string())
            .collect();
        task_results.push(TaskResult {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            task_status: task.status.clone(),
            task_type,
            evidence_kinds: kind_list,
            compliant: missing.is_empty(),
            missing_required: missing,
        });
    }

    let tasks_closed = task_results.len();
    let tasks_compliant = task_results.iter().filter(|t| t.compliant).count();
    let score_pct: u8 = if tasks_closed == 0 {
        100
    } else {
        let pct = (tasks_compliant * 100)
            .checked_div(tasks_closed)
            .unwrap_or(100);
        pct.min(100) as u8
    };

    Ok(Report {
        plan_id: plan_id.to_string(),
        tasks_closed,
        tasks_compliant,
        score_pct,
        registry_ok,
        bus_ok,
        tasks: task_results,
    })
}

fn render_human(bundle: &Bundle, report: &Report) {
    println!(
        "{}",
        bundle.t(
            "coherence-plan-execution-summary",
            &[
                ("plan", &report.plan_id[..8.min(report.plan_id.len())]),
                ("closed", &report.tasks_closed.to_string()),
                ("compliant", &report.tasks_compliant.to_string()),
                ("score", &report.score_pct.to_string()),
            ]
        )
    );
    let reg_s = if report.registry_ok { "ok" } else { "FAIL" };
    let bus_s = if report.bus_ok { "ok" } else { "FAIL" };
    println!(
        "{}",
        bundle.t(
            "coherence-plan-execution-plan-checks",
            &[("registry", reg_s), ("bus", bus_s)]
        )
    );
    for t in &report.tasks {
        if t.compliant {
            println!(
                "{}",
                bundle.t(
                    "coherence-plan-execution-task-ok",
                    &[
                        ("id", &t.task_id[..8.min(t.task_id.len())]),
                        ("title", &t.task_title)
                    ]
                )
            );
        } else {
            println!(
                "{}",
                bundle.t(
                    "coherence-plan-execution-task-fail",
                    &[
                        ("id", &t.task_id[..8.min(t.task_id.len())]),
                        ("title", &t.task_title),
                        ("missing", &t.missing_required.join(", "))
                    ]
                )
            );
        }
    }
}

fn render_plain(report: &Report) {
    println!(
        "plan={} closed={} compliant={} score={}%",
        report.plan_id, report.tasks_closed, report.tasks_compliant, report.score_pct
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_type_code_from_code_evidence() {
        let mut kinds = HashSet::new();
        kinds.insert("code".to_string());
        kinds.insert("context_pack".to_string());
        assert_eq!(infer_type(&kinds), TaskType::Code);
    }

    #[test]
    fn infer_type_code_from_merge_record() {
        let mut kinds = HashSet::new();
        kinds.insert("merge_record".to_string());
        assert_eq!(infer_type(&kinds), TaskType::Code);
    }

    #[test]
    fn infer_type_doc_only() {
        let mut kinds = HashSet::new();
        kinds.insert("adr".to_string());
        assert_eq!(infer_type(&kinds), TaskType::DocOnly);
    }

    #[test]
    fn infer_type_analysis_when_empty() {
        let kinds = HashSet::new();
        assert_eq!(infer_type(&kinds), TaskType::Analysis);
    }

    #[test]
    fn code_task_requires_graph_and_ci() {
        let required = required_kinds(&TaskType::Code);
        assert!(required.contains(&"context_pack"));
        assert!(required.contains(&"ci_run"));
        assert!(required.contains(&"merge_record"));
    }

    #[test]
    fn analysis_has_no_requirements() {
        assert!(required_kinds(&TaskType::Analysis).is_empty());
    }
}
