//! `EvidenceGate` — refuses `submitted`/`done` transitions when the
//! task's `evidence_required` set is not fully covered.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;

/// Server-enforced rule: a task cannot move to `submitted` (or beyond)
/// without at least one evidence row of every required kind.
pub struct EvidenceGate;

#[async_trait::async_trait]
impl Gate for EvidenceGate {
    fn name(&self) -> &'static str {
        "evidence"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }
        if ctx.task.evidence_required.is_empty() {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let present = store.kinds_for(&ctx.task.id).await?;
        let mut missing: Vec<&str> = Vec::new();
        for required in &ctx.task.evidence_required {
            if !present.iter().any(|p| p == required) {
                missing.push(required);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(DurabilityError::GateRefused {
                gate: "evidence",
                reason_code: "missing_evidence_kind",
                reason: format!("missing evidence kinds: {}", missing.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "evidence",
            // Per-task evidence kinds are dynamic; agents must read
            // task.evidence_required at runtime. The default here
            // signals "this gate consumes evidence" without claiming
            // a static list.
            requires_evidence_kinds: vec![],
            active_target_status: vec!["submitted", "done"],
            refusal_reasons: vec!["missing_evidence_kind"],
        }
    }
}
