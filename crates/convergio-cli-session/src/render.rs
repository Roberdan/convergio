//! Renderers for `cvg session resume` (human / json / plain).
//!
//! Split out of [`crate::session`] to honour the 300-line per-file
//! cap (CONSTITUTION § 13).

use crate::session_models::{Plan, PrSummary, Task, TaskCounts};
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::Value;

/// Aggregated cold-start payload — borrowed view, never owned.
pub(crate) struct Brief<'a> {
    pub(crate) health: &'a Value,
    pub(crate) audit: &'a Value,
    pub(crate) plan: &'a Plan,
    pub(crate) counts: &'a TaskCounts,
    pub(crate) next: &'a [Task],
    pub(crate) prs: Option<&'a [PrSummary]>,
    /// Optional graph context-pack when `--task-id` was given.
    pub(crate) pack: Option<&'a Value>,
}

pub(crate) fn render(bundle: &Bundle, output: OutputMode, brief: &Brief<'_>) -> Result<()> {
    match output {
        OutputMode::Json => render_json(brief),
        OutputMode::Plain => {
            render_plain(brief);
            Ok(())
        }
        OutputMode::Human => {
            render_human(bundle, brief);
            Ok(())
        }
    }
}

fn render_json(brief: &Brief<'_>) -> Result<()> {
    let value = serde_json::json!({
        "health": brief.health,
        "audit": brief.audit,
        "plan": brief.plan,
        "task_counts": brief.counts,
        "next_tasks": brief.next,
        "open_prs": brief.prs,
        "context_pack": brief.pack,
    });
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn render_plain(brief: &Brief<'_>) {
    let pr_count = brief.prs.map(|p| p.len()).unwrap_or(0);
    println!(
        "health={} audit_ok={} plan_id={} done={} total={} in_progress={} submitted={} pending={} next={} open_prs={}",
        bool_field(brief.health, "ok"),
        bool_field(brief.audit, "ok"),
        brief.plan.id,
        brief.counts.done,
        brief.counts.total,
        brief.counts.in_progress,
        brief.counts.submitted,
        brief.counts.pending,
        brief.next.len(),
        pr_count,
    );
}

fn render_human(bundle: &Bundle, brief: &Brief<'_>) {
    println!("{}", bundle.t("session-resume-header", &[]));
    if let Some(pack) = brief.pack {
        render_pack_summary(bundle, pack);
    }

    let health_ok = bool_field(brief.health, "ok");
    let version = brief
        .health
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("?");
    println!(
        "{}",
        bundle.t(
            if health_ok {
                "session-resume-health-ok"
            } else {
                "session-resume-health-down"
            },
            &[("version", version)],
        )
    );

    let audit_ok = bool_field(brief.audit, "ok");
    let checked = brief
        .audit
        .get("checked")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .to_string();
    println!(
        "{}",
        bundle.t(
            if audit_ok {
                "session-resume-audit-ok"
            } else {
                "session-resume-audit-broken"
            },
            &[("count", &checked)],
        )
    );

    println!(
        "{}",
        bundle.t(
            "session-resume-plan-line",
            &[
                ("title", &brief.plan.title),
                ("status", &brief.plan.status),
                ("project", brief.plan.project.as_deref().unwrap_or("-")),
                ("id", &brief.plan.id),
            ],
        )
    );
    println!(
        "{}",
        bundle.t(
            "session-resume-counts-line",
            &[
                ("done", &brief.counts.done.to_string()),
                ("total", &brief.counts.total.to_string()),
                ("in_progress", &brief.counts.in_progress.to_string()),
                ("submitted", &brief.counts.submitted.to_string()),
                ("pending", &brief.counts.pending.to_string()),
            ],
        )
    );

    if brief.next.is_empty() {
        println!("{}", bundle.t("session-resume-next-empty", &[]));
    } else {
        println!("{}", bundle.t("session-resume-next-header", &[]));
        for task in brief.next {
            println!(
                "{}",
                bundle.t(
                    "session-resume-next-line",
                    &[
                        ("wave", &task.wave.to_string()),
                        ("sequence", &task.sequence.to_string()),
                        ("title", &task.title),
                        ("id", &short_id(&task.id)),
                    ],
                )
            );
        }
    }

    match brief.prs {
        None => println!("{}", bundle.t("session-resume-prs-unavailable", &[])),
        Some([]) => println!("{}", bundle.t("session-resume-prs-empty", &[])),
        Some(prs) => {
            println!("{}", bundle.t("session-resume-prs-header", &[]));
            for pr in prs {
                println!(
                    "{}",
                    bundle.t(
                        if pr.is_draft {
                            "session-resume-pr-line-draft"
                        } else {
                            "session-resume-pr-line"
                        },
                        &[
                            ("number", &pr.number.to_string()),
                            ("title", &pr.title),
                            ("branch", &pr.head_ref_name),
                        ],
                    )
                );
            }
        }
    }
}

fn render_pack_summary(bundle: &Bundle, pack: &Value) {
    let task_id = pack.get("task_id").and_then(Value::as_str).unwrap_or("");
    let nodes = pack
        .get("matched_nodes")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let files = pack
        .get("files")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let est = pack
        .get("estimated_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    println!(
        "{}",
        bundle.t(
            "session-resume-pack-line",
            &[
                ("task_id", &short_id(task_id)),
                ("nodes", &nodes.to_string()),
                ("files", &files.to_string()),
                ("est_tokens", &est.to_string()),
            ],
        )
    );
}

fn bool_field(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}
