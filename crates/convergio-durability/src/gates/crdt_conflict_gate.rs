//! `CrdtConflictGate` — unresolved CRDT conflicts block task completion.

use super::{Gate, GateContext, GateEvidenceInput, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::CrdtStore;

/// Refuses `submitted`/`done` while the task has materialized CRDT conflicts.
pub struct CrdtConflictGate;

#[async_trait::async_trait]
impl Gate for CrdtConflictGate {
    fn name(&self) -> &'static str {
        "crdt_conflict"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let conflicts = CrdtStore::new(ctx.pool.clone())
            .list_conflicts_for_entity("task", &ctx.task.id)
            .await?;
        if conflicts.is_empty() {
            return Ok(());
        }

        let fields = conflicts
            .iter()
            .map(|cell| cell.field_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Err(DurabilityError::GateRefused {
            gate: "crdt_conflict",
            reason: format!("unresolved CRDT conflicts on fields: {fields}"),
        })
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "crdt_conflict",
            evidence: GateEvidenceInput::None,
            active_target_status: vec!["submitted", "done"],
            refusal_reasons: vec!["crdt_conflicts_present"],
        }
    }
}
