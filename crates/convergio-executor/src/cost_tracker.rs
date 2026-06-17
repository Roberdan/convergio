//! W10 Cost-of-Pass tracker: records token/time cost per task completion.
//!
//! Called from the heartbeat sidecar after a task's runner process exits
//! and the task has transitioned to `done` or `failed`.
//! Persists one `task.cost_recorded` audit row per completed task.
//! The routing algorithm (W8) can read these rows via [`load_cost_stats`].

use convergio_durability::{audit::EntityKind, Durability};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transition string for cost-recorded audit rows.
const COST_TRANSITION: &str = "task.cost_recorded";

/// Observed cost for one task run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCost {
    /// The task UUID this cost entry belongs to.
    pub task_id: uuid::Uuid,
    /// Wire-format runner kind (e.g. `claude:sonnet`).
    pub runner_kind: String,
    /// Wall-clock seconds from spawn to terminal transition.
    pub elapsed_secs: f64,
    /// Estimated token count (None when runner doesn't report it).
    pub tokens: Option<u64>,
    /// Whether the task reached `done` (true) or `failed` (false).
    pub passed: bool,
}

/// Per-runner cost aggregate returned by [`load_cost_stats`].
#[derive(Debug, Default)]
pub struct RunnerCostStats {
    /// Mean wall-clock seconds across all recorded samples.
    pub avg_elapsed_secs: f64,
    /// Fraction of samples where `passed == true` (range `[0.0, 1.0]`).
    pub pass_rate: f64,
    /// Total number of cost rows counted for this runner.
    pub sample_count: u64,
}

/// Record cost for a completed task into the audit log.
///
/// Uses `EntityKind::Task` and the `task.cost_recorded` transition.
/// Best-effort only — callers should log errors with `tracing::warn!`
/// and not fail the task if this returns `Err`.
pub async fn record_cost(durability: &Durability, cost: TaskCost) -> anyhow::Result<()> {
    let entity_id = cost.task_id.to_string();
    durability
        .audit()
        .append(EntityKind::Task, &entity_id, COST_TRANSITION, &cost, None)
        .await
        .map_err(|e| anyhow::anyhow!("audit append failed: {e}"))?;
    Ok(())
}

/// Load per-runner cost stats from the audit log.
///
/// Scans all `task.cost_recorded` rows (every sample, no deduplication)
/// and returns average `elapsed_secs`, `pass_rate`, and `sample_count`
/// per `runner_kind`. Returns an empty map when no cost rows exist.
pub async fn load_cost_stats(
    durability: &Durability,
) -> anyhow::Result<HashMap<String, RunnerCostStats>> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT payload FROM audit_log WHERE transition = ?")
        .bind(COST_TRANSITION)
        .fetch_all(durability.pool().inner())
        .await
        .map_err(|e| anyhow::anyhow!("db query failed: {e}"))?;

    // (sum_elapsed, passed_count, total_count)
    let mut agg: HashMap<String, (f64, u64, u64)> = HashMap::new();

    for (payload,) in rows {
        let Ok(cost) = serde_json::from_str::<TaskCost>(&payload) else {
            continue;
        };
        let entry = agg.entry(cost.runner_kind).or_insert((0.0, 0, 0));
        entry.0 += cost.elapsed_secs;
        entry.2 += 1;
        if cost.passed {
            entry.1 += 1;
        }
    }

    let stats = agg
        .into_iter()
        .map(|(kind, (sum_elapsed, passed, total))| {
            let avg_elapsed_secs = if total == 0 {
                0.0
            } else {
                sum_elapsed / total as f64
            };
            let pass_rate = if total == 0 {
                0.0
            } else {
                passed as f64 / total as f64
            };
            (
                kind,
                RunnerCostStats {
                    avg_elapsed_secs,
                    pass_rate,
                    sample_count: total,
                },
            )
        })
        .collect();

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_transition_constant_is_stable() {
        assert_eq!(COST_TRANSITION, "task.cost_recorded");
    }
}
