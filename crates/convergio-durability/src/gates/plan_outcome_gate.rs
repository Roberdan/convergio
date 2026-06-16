//! `PlanOutcomeGate` — refuses the final `task.done` in a plan when
//! too few tasks ended in `done` (rather than `failed`). Implements
//! W4 (ADR-0055).
//!
//! ## When does it fire?
//!
//! The gate scans **task** transitions (the only pipeline available),
//! but it fires only when the current `Done` transition would close the
//! plan — i.e., every other task in the plan is already in a terminal
//! state (`done` or `failed`). Early task-done transitions are always
//! allowed; only the very last one is checked.
//!
//! ## Denominator / numerator
//!
//! - **Total** = all tasks in the plan (whole plan, not just the final
//!   wave) so a plan that hides failures behind a large final wave
//!   cannot circumvent the gate.
//! - **Done count** = tasks with `status = 'done'`, **including the
//!   current task** (which is transitioning to done right now).
//!
//! ## Opt-in
//!
//! Enforcement is gated by `CONVERGIO_REQUIRE_PLAN_OUTCOME=1`.
//! Production deployments should set this; new plans are backfill-safe
//! because the gate is a no-op when the env var is absent.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;

/// Opt-in env var (same pattern as `PlanCoherenceGate`).
const ENABLE_ENV: &str = "CONVERGIO_REQUIRE_PLAN_OUTCOME";
/// Default minimum fraction of `done` tasks required to close a plan.
const DEFAULT_THRESHOLD: f64 = 0.8;

/// Refuses the final `task.done` in a plan when the success rate
/// (done/total) is below `threshold` (default 80 %).
#[derive(Debug, Clone)]
pub struct PlanOutcomeGate {
    /// Minimum fraction of tasks that must be `done` (not `failed`) for
    /// the plan to be considered successful. Range [0.0, 1.0].
    threshold: f64,
}

impl Default for PlanOutcomeGate {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_THRESHOLD,
        }
    }
}

impl PlanOutcomeGate {
    /// New gate with the default 80 % threshold.
    pub fn new() -> Self {
        Self::default()
    }

    /// New gate with a custom threshold in [0.0, 1.0].
    pub fn with_threshold(threshold: f64) -> Self {
        Self { threshold }
    }

    fn enforce_enabled() -> bool {
        matches!(
            std::env::var(ENABLE_ENV).ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE") | Some("yes")
        )
    }
}

#[async_trait::async_trait]
impl Gate for PlanOutcomeGate {
    fn name(&self) -> &'static str {
        "plan_outcome"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !Self::enforce_enabled() {
            return Ok(());
        }
        if ctx.target_status != TaskStatus::Done {
            return Ok(());
        }

        // Count non-terminal tasks excluding the current task.
        // If any remain open the plan is not closing yet — let it pass.
        let (open,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tasks \
             WHERE plan_id = ? \
               AND id != ? \
               AND status NOT IN ('done', 'failed')",
        )
        .bind(&ctx.task.plan_id)
        .bind(&ctx.task.id)
        .fetch_one(ctx.pool.inner())
        .await?;

        if open > 0 {
            return Ok(());
        }

        // This Done transition closes the plan. Evaluate the success rate.
        // Include the current task (+1) in the numerator.
        let (done_others,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tasks \
             WHERE plan_id = ? \
               AND id != ? \
               AND status = 'done'",
        )
        .bind(&ctx.task.plan_id)
        .bind(&ctx.task.id)
        .fetch_one(ctx.pool.inner())
        .await?;

        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE plan_id = ?")
            .bind(&ctx.task.plan_id)
            .fetch_one(ctx.pool.inner())
            .await?;

        if total == 0 {
            return Ok(());
        }

        let done_count = done_others + 1;
        let done_pct = done_count as f64 / total as f64;

        if done_pct >= self.threshold {
            return Ok(());
        }

        let threshold_pct = (self.threshold * 100.0).round() as u8;
        let actual_pct = (done_pct * 100.0).round() as u8;
        Err(DurabilityError::GateRefused {
            gate: "plan_outcome",
            reason: format!(
                "plan_success_rate_too_low: {done_count}/{total} tasks done \
                 ({actual_pct}% < {threshold_pct}% required) \
                 for plan_id={}",
                ctx.task.plan_id
            ),
        })
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "plan_outcome".into(),
            reads_evidence_kinds: vec![],
            enforces_task_evidence_required: false,
            active_target_status: vec!["done".into()],
            refusal_reasons: vec!["plan_success_rate_too_low".into()],
        }
    }
}
