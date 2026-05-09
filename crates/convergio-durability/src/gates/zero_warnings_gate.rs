//! `ZeroWarningsGate` — refuses `submitted`/`done` when any
//! "build/lint/compile/test" evidence has `exit_code != 0` or a
//! non-zero `warnings_count` / `warning_count` field in its payload.
//!
//! Sister gate to `NoDebtGate`. Where `NoDebtGate` looks at *what is
//! in the code*, this one looks at *what the toolchain said about the
//! code*. An LLM that ran `cargo clippy`, got 12 warnings and reported
//! them as "build evidence" must NOT be allowed to flip the task to
//! submitted.
//!
//! ## Trigger conditions
//!
//! For each evidence row whose `kind` is one of:
//!
//! - `build`
//! - `lint`
//! - `compile`
//! - `test`
//! - `typecheck`
//!
//! the gate refuses if **any** of these hold:
//!
//! 1. `exit_code` is set and `!= 0`
//! 2. `payload.warnings_count` is a number `> 0`
//! 3. `payload.warning_count` is a number `> 0` (US spelling)
//! 4. `payload.errors_count` / `error_count` is a number `> 0`
//! 5. `payload.failures` is an array with non-zero length
//!
//! Other evidence kinds are ignored — they may legitimately contain
//! free-form text.

use super::{Gate, GateContext, GateEvidenceInput, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::{Evidence, TaskStatus};
use crate::store::EvidenceStore;
use serde_json::Value;

/// Refuses if any quality-signal evidence carries a non-clean signal.
pub struct ZeroWarningsGate;

const QUALITY_KINDS: &[&str] = &["build", "lint", "compile", "test", "typecheck"];

#[async_trait::async_trait]
impl Gate for ZeroWarningsGate {
    fn name(&self) -> &'static str {
        "zero_warnings"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;

        let mut violations: Vec<String> = Vec::new();
        for ev in evidence {
            if !QUALITY_KINDS.contains(&ev.kind.as_str()) {
                continue;
            }
            for v in inspect(&ev) {
                violations.push(format!("{}#{}", ev.kind, v));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            violations.sort();
            violations.dedup();
            Err(DurabilityError::GateRefused {
                gate: "zero_warnings",
                reason: format!("non-clean quality signal: {}", violations.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "zero_warnings",
            evidence: GateEvidenceInput::SpecificKinds {
                kinds: QUALITY_KINDS.to_vec(),
            },
            active_target_status: vec!["submitted", "done"],
            refusal_reasons: vec!["nonzero_exit", "warnings", "errors", "failures"],
        }
    }
}

fn inspect(ev: &Evidence) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    if let Some(code) = ev.exit_code {
        if code != 0 {
            out.push("nonzero_exit");
        }
    }
    for key in ["warnings_count", "warning_count"] {
        if let Some(n) = ev.payload.get(key).and_then(Value::as_i64) {
            if n > 0 {
                out.push("warnings");
                break;
            }
        }
    }
    for key in ["errors_count", "error_count"] {
        if let Some(n) = ev.payload.get(key).and_then(Value::as_i64) {
            if n > 0 {
                out.push("errors");
                break;
            }
        }
    }
    if let Some(arr) = ev.payload.get("failures").and_then(Value::as_array) {
        if !arr.is_empty() {
            out.push("failures");
        }
    }
    out
}
