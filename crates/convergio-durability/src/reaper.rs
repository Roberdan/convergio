//! Reaper loop.
//!
//! The reaper periodically scans `tasks` for rows in `in_progress`
//! whose `last_heartbeat_at` is older than [`ReaperConfig::timeout`],
//! moves them back to `pending`, clears `agent_id`, mirrors the
//! release into the previous owner's `agents` row (matches the
//! transition-time sync — P2-1), and writes one `task.reaped`
//! audit row per release.
//!
//! Optionally, callers can register an [`OnReap`] hook that fires
//! after each successful release — the server uses this to remove
//! the reaped task's git worktree from disk (issue #408). The hook
//! is best-effort and called outside the release transaction so an
//! error there never undoes the audit row.
//!
//! There is **exactly one** of these per daemon. If you find yourself
//! adding a second background loop in Layer 1, stop and consider
//! whether it belongs in a Layer 4 crate instead — see
//! [ARCHITECTURE.md](../../../ARCHITECTURE.md) § "Background loops".

use crate::audit::{append_tx, EntityKind};
use crate::error::Result;
use crate::facade::Durability;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Hook fired once per successfully-released task. Receives the
/// task id. Used by the server to remove the reaped worktree from
/// disk; Layer 1 stays git-agnostic.
pub type OnReap = Arc<dyn Fn(&str) + Send + Sync + 'static>;

/// Reaper configuration.
#[derive(Clone)]
pub struct ReaperConfig {
    /// Task heartbeat older than this releases the task.
    pub timeout: Duration,
    /// How often the loop ticks.
    pub tick_interval: Duration,
    /// When true, the reaper also retires agents with stale heartbeats.
    pub agent_reaper_enabled: bool,
    /// Agent heartbeat older than this triggers retirement.
    pub agent_threshold: Duration,
    /// Optional best-effort hook invoked once per reaped task,
    /// outside the release transaction. The server wires this to
    /// remove the agent's git worktree from disk.
    pub on_reap: Option<OnReap>,
}

impl std::fmt::Debug for ReaperConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReaperConfig")
            .field("timeout", &self.timeout)
            .field("tick_interval", &self.tick_interval)
            .field("agent_reaper_enabled", &self.agent_reaper_enabled)
            .field("agent_threshold", &self.agent_threshold)
            .field("on_reap", &self.on_reap.is_some())
            .finish()
    }
}

impl Default for ReaperConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_reaper_enabled: true,
            agent_threshold: Duration::seconds(3600),
            on_reap: None,
        }
    }
}

/// Spawned-loop handle. Drop the handle to abort the loop.
pub struct ReaperHandle {
    inner: JoinHandle<()>,
}

impl ReaperHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(self) {
        self.inner.abort();
    }
}

/// Spawn the reaper loop and return its handle.
///
/// The loop is fire-and-forget: errors during a tick are logged at
/// `warn!` and do **not** kill the loop. Persistent issues should
/// surface via metrics, not by silent loop death.
pub fn spawn(durability: Arc<Durability>, config: ReaperConfig) -> ReaperHandle {
    let inner = tokio::spawn(async move {
        info!(?config, "reaper started");
        let interval = tokio_duration(config.tick_interval);
        loop {
            tokio::time::sleep(interval).await;
            match tick(&durability, &config).await {
                Ok(TickResult { tasks, agents }) if tasks > 0 || agents > 0 => {
                    // Promoted to warn! (#408) — a reaped task is an
                    // operator-actionable signal (dead agent, dropped
                    // heartbeat), not routine noise. `info!` was
                    // getting filtered out in default log configs and
                    // operators only noticed after the dispatch budget
                    // starved.
                    warn!(tasks_reaped = tasks, agents_retired = agents, "reaper tick")
                }
                Ok(_) => debug!("reaper tick: nothing stale"),
                Err(e) => warn!(error = %e, "reaper tick failed"),
            }
        }
    });
    ReaperHandle { inner }
}

/// Counts returned by one reaper tick.
#[derive(Debug, Default)]
pub struct TickResult {
    /// Number of tasks moved back to `pending`.
    pub tasks: usize,
    /// Number of agents marked `terminated` due to stale heartbeat.
    pub agents: usize,
}

/// Run one tick. Returns counts of tasks released and agents retired.
///
/// Exposed for tests and for callers that want to drive the loop on
/// their own schedule (e.g. a manual `cvg doctor reap`).
pub async fn tick(durability: &Durability, config: &ReaperConfig) -> Result<TickResult> {
    let cutoff = Utc::now() - config.timeout;
    let stale = find_stale(durability, cutoff).await?;

    let mut tasks = 0usize;
    for (id, agent_id) in stale {
        if release_one(durability, &id, agent_id.as_deref(), &cutoff).await? {
            tasks += 1;
            // Hook is best-effort and runs outside the release
            // transaction. A panic here would kill the reaper loop,
            // so the hook itself should not unwind — server callers
            // wrap their cleanup in `catch_unwind`.
            if let Some(hook) = &config.on_reap {
                hook(&id);
            }
        }
    }

    let agents = if config.agent_reaper_enabled {
        let threshold = config.agent_threshold.num_seconds();
        let result = durability.retire_stale_agents(threshold, false).await?;
        let n = result.agents.iter().filter(|r| r.retired).count();
        if n > 0 {
            info!(
                retired = n,
                threshold_secs = threshold,
                "agent reaper: retired stale agents"
            );
        }
        n
    } else {
        0
    };

    Ok(TickResult { tasks, agents })
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
    let now = Utc::now().to_rfc3339();
    let updated = sqlx::query(
        "UPDATE tasks SET status = 'pending', agent_id = NULL, updated_at = ? \
         WHERE id = ? AND status = 'in_progress'",
    )
    .bind(&now)
    .bind(task_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    // Mirror release into the agents row (P2-1, F46b write-side).
    // Only clears if the agent still points at this task — prevents
    // clobbering an agent that has since claimed a different one.
    if let Some(aid) = agent_id {
        sqlx::query(
            "UPDATE agents \
             SET current_task_id = NULL, status = 'idle', updated_at = ? \
             WHERE id = ? AND current_task_id = ?",
        )
        .bind(&now)
        .bind(aid)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;
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
