//! `PlanStatusGate` — refuses task transitions when the owning plan is
//! not in a state that accepts work.

use super::{Gate, GateContext, GateEvidenceInput, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::{PlanStatus, TaskStatus};

/// Refuses if the plan is `cancelled` or `completed` and the caller
/// wants to move the task into `in_progress`/`submitted`.
pub struct PlanStatusGate;

#[async_trait::async_trait]
impl Gate for PlanStatusGate {
    fn name(&self) -> &'static str {
        "plan_status"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        let row = sqlx::query_as::<_, (String,)>("SELECT status FROM plans WHERE id = ? LIMIT 1")
            .bind(&ctx.task.plan_id)
            .fetch_optional(ctx.pool.inner())
            .await?;

        let Some((status,)) = row else {
            return Err(DurabilityError::GateRefused {
                gate: "plan_status",
                reason: format!("plan {} not found", ctx.task.plan_id),
            });
        };

        let plan_status = PlanStatus::parse(&status).unwrap_or(PlanStatus::Draft);
        if matches!(plan_status, PlanStatus::Cancelled | PlanStatus::Completed)
            && matches!(
                ctx.target_status,
                TaskStatus::InProgress | TaskStatus::Submitted
            )
        {
            return Err(DurabilityError::GateRefused {
                gate: "plan_status",
                reason: format!("plan is {}", plan_status.as_str()),
            });
        }

        Ok(())
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "plan_status",
            evidence: GateEvidenceInput::None,
            active_target_status: vec!["in_progress", "submitted"],
            refusal_reasons: vec!["plan_not_found", "plan_is_cancelled", "plan_is_completed"],
        }
    }
}
