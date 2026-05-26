//! `A11yGate` phase 1 — built-in accessibility checks on evidence payloads.
//!
//! Implements the built-in subset of CONSTITUTION Sacred principle #3
//! (accessibility first).
//!
//! Phase 2 (capability `a11y.axe`) will wrap axe-core for broader WCAG coverage,
//! but is intentionally out of scope for phase 1.
//!
//! These checks intentionally use **no external tooling**. They scan the string
//! leaves of each evidence payload on `submitted` / `done` transitions and
//! refuse with HTTP 409 + a stable reason of the form:
//!
//! `a11y_violation_found: <evidence_kind>#<rule>, ...`
//!
//! See ADR-0061 for rationale and rule definitions.

mod cli;
mod markdown;

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use serde_json::Value;

/// Built-in accessibility gate.
#[derive(Debug, Default)]
pub struct A11yGate;

#[async_trait::async_trait]
impl Gate for A11yGate {
    fn name(&self) -> &'static str {
        "a11y"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;

        let mut violations: Vec<String> = Vec::new();

        for ev in evidence {
            let kind = ev.kind.as_str();
            let mut strings = Vec::new();
            collect_strings(&ev.payload, &mut strings);
            let joined = strings.join("\n");

            // Bidi check fires on every evidence kind.
            if contains_bidi_override(&joined) {
                violations.push(format!("{kind}#bidi_override"));
            }

            if is_markdown_kind(kind) {
                markdown::check(kind, &joined, &mut violations);
            }

            if is_cli_kind(kind) && cli::has_color_only_signal(&strings) {
                violations.push(format!("{kind}#cli_color_only_signal"));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            violations.sort();
            violations.dedup();
            Err(DurabilityError::GateRefused {
                gate: "a11y",
                reason: format!("a11y_violation_found: {}", violations.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "a11y".into(),
            reads_evidence_kinds: vec!["*".into()],
            enforces_task_evidence_required: false,
            active_target_status: vec!["submitted".into(), "done".into()],
            refusal_reasons: vec!["a11y_violation_found".into()],
        }
    }
}

fn is_markdown_kind(kind: &str) -> bool {
    matches!(
        kind,
        "markdown" | "markdown_doc" | "md_doc" | "doc" | "readme"
    )
}

fn is_cli_kind(kind: &str) -> bool {
    matches!(kind, "cli_output" | "terminal" | "tui_snapshot")
}

fn contains_bidi_override(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            '\u{202A}'
                | '\u{202B}'
                | '\u{202C}'
                | '\u{202D}'
                | '\u{202E}'
                | '\u{2066}'
                | '\u{2067}'
                | '\u{2068}'
                | '\u{2069}'
        )
    })
}

fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| collect_strings(v, out)),
        _ => {}
    }
}
