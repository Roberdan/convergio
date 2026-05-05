//! Reaper loop. Per tick: (a) releases stale `in_progress` tasks
//! (`task.reaped`); (b) retires agents whose `last_heartbeat_at` is
//! older than [`ReaperConfig::agent_threshold`] via
//! [`Durability::retire_agent`] with a sibling `agent.retired_stale`
//! audit row. `agent_threshold = 0` disables (b). One per daemon —
//! see ARCHITECTURE.md § "Background loops".

use crate::audit::{append_tx, EntityKind};
use crate::error::Result;
use crate::facade::Durability;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Reaper configuration.
#[derive(Debug, Clone)]
pub struct ReaperConfig {
    /// Task heartbeat older than this releases the task.
    pub timeout: Duration,
    /// Loop tick interval.
    pub tick_interval: Duration,
    /// Agent heartbeat older than this retires the agent. Zero disables.
    pub agent_threshold: Duration,
}

impl Default for ReaperConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_threshold: Duration::seconds(3600),
        }
    }
}

/// Spawned-loop handle. Drop or call [`ReaperHandle::abort`] to stop.
pub struct ReaperHandle {
    inner: JoinHandle<()>,
}
impl ReaperHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(self) {
        self.inner.abort();
    }
}

/// Spawn the reaper loop. Tick errors are logged, never kill the loop.
pub fn spawn(durability: Arc<Durability>, config: ReaperConfig) -> ReaperHandle {
    let inner = tokio::spawn(async move {
        info!(?config, "reaper started");
        let interval = tokio_duration(config.tick_interval);
        loop {
            tokio::time::sleep(interval).await;
            match tick(&durability, &config).await {
                Ok((t, a)) if t > 0 || a > 0 => {
                    info!(tasks_reaped = t, agents_retired = a, "reaper tick")
                }
                Ok(_) => debug!("reaper tick: nothing stale"),
                Err(e) => warn!(error = %e, "reaper tick failed"),
            }
        }
    });
    ReaperHandle { inner }
}

/// Run one tick: returns `(tasks_released, agents_retired)`.
pub async fn tick(durability: &Durability, config: &ReaperConfig) -> Result<(usize, usize)> {
    let cutoff = Utc::now() - config.timeout;
    let mut released = 0usize;
    for (id, agent_id) in find_stale(durability, cutoff).await? {
        if release_one(durability, &id, agent_id.as_deref(), &cutoff).await? {
            released += 1;
        }
    }
    let threshold = config.agent_threshold.num_seconds();
    let mut retired = 0usize;
    if threshold > 0 {
        for entry in durability.agents().stale_agents(threshold).await? {
            let record = durability.retire_agent(&entry.agent_id).await?;
            durability
                .audit()
                .append(
                    EntityKind::Agent,
                    &record.id,
                    "agent.retired_stale",
                    &json!({
                        "agent_id": record.id,
                        "last_heartbeat_at": entry.last_heartbeat_at,
                        "threshold_seconds": threshold,
                        "reason": "stale_heartbeat",
                    }),
                    Some(&record.id),
                )
                .await?;
            retired += 1;
        }
    }
    Ok((released, retired))
}

async fn find_stale(
    durability: &Durability,
    cutoff: DateTime<Utc>,
) -> Result<Vec<(String, Option<String>)>> {
    let cutoff_str = cutoff.to_rfc3339();
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' \
           AND last_heartbeat_at < ? \
         UNION ALL \
         SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' \
           AND last_heartbeat_at IS NULL \
           AND updated_at < ?",
    )
    .bind(&cutoff_str)
    .bind(&cutoff_str)
    .fetch_all(durability.pool().inner())
    .await?;
    Ok(rows)
}

async fn release_one(
    durability: &Durability,
    task_id: &str,
    agent_id: Option<&str>,
    cutoff: &DateTime<Utc>,
) -> Result<bool> {
    let mut tx = durability.pool().inner().begin().await?;
    let updated = sqlx::query(
        "UPDATE tasks SET status = 'pending', agent_id = NULL, updated_at = ? \
         WHERE id = ? AND status = 'in_progress'",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(task_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    append_tx(
        &mut tx,
        EntityKind::Task,
        task_id,
        "task.reaped",
        &json!({
            "task_id": task_id,
            "previous_agent_id": agent_id,
            "cutoff": cutoff.to_rfc3339(),
        }),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(true)
}

fn tokio_duration(d: Duration) -> std::time::Duration {
    std::time::Duration::from_millis(d.num_milliseconds().max(1) as u64)
}
