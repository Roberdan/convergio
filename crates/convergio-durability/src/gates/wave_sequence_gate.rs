//! `WaveSequenceGate` — refuses `in_progress` claims for tasks in a
//! wave whose predecessor wave is not yet fully complete.
//!
//! Terminal states (`done`, `failed`) do not block subsequent waves:
//! a `failed` task is already off the critical path, and the plan can
//! either retry it (creating a fresh task in the same wave) or accept
//! the failure and move on. Treating `failed` as "still open" would
//! deadlock plans whose wave 1 contained an intentional probe or any
//! permanently-rejected task.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;

/// Tasks in wave N+1 cannot start until every task in wave N is in a
/// terminal state (`done` or `failed`).
pub struct WaveSequenceGate;

#[async_trait::async_trait]
impl Gate for WaveSequenceGate {
    fn name(&self) -> &'static str {
        "wave_sequence"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::InProgress) {
            return Ok(());
        }
        if ctx.task.wave <= 1 {
            return Ok(());
        }

        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM tasks \
             WHERE plan_id = ? AND wave < ? AND status NOT IN ('done', 'failed')",
        )
        .bind(&ctx.task.plan_id)
        .bind(ctx.task.wave)
        .fetch_one(ctx.pool.inner())
        .await?;

        if row.0 > 0 {
            Err(DurabilityError::GateRefused {
                gate: "wave_sequence",
                reason: format!("{} task(s) in earlier waves still open", row.0),
            })
        } else {
            Ok(())
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "wave_sequence",
            requires_evidence_kinds: vec![],
            active_target_status: vec!["in_progress"],
            refusal_reasons: vec!["earlier_wave_tasks_still_open"],
        }
    }
}
