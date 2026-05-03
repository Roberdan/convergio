//! `Executor::tick` — one-shot dispatch round.

use crate::defaults::{RunnerDefaults, SpawnTemplate};
use crate::error::Result;
use chrono::Duration;
use convergio_durability::{Durability, TaskStatus};
use convergio_lifecycle::{SpawnSpec, Supervisor};
use convergio_runner::{
    for_kind_with_registry, PermissionProfile, RunnerKind, RunnerRegistry, SpawnContext,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Executor handle.
#[derive(Clone)]
pub struct Executor {
    durability: Durability,
    supervisor: Supervisor,
    template: SpawnTemplate,
    defaults: RunnerDefaults,
    graph: Option<convergio_graph::Store>,
    registry: std::sync::Arc<RunnerRegistry>,
}

impl Executor {
    /// Build with the given facades and spawn template. Uses
    /// [`RunnerDefaults::default`] for runner routing — operators
    /// that want to override should call [`Self::with_defaults`].
    pub fn new(durability: Durability, supervisor: Supervisor, template: SpawnTemplate) -> Self {
        Self {
            durability,
            supervisor,
            template,
            defaults: RunnerDefaults::default(),
            graph: None,
            registry: std::sync::Arc::new(RunnerRegistry::empty()),
        }
    }

    /// Override the daemon-wide runner defaults (`runner_kind`,
    /// `profile`, daemon callback URL).
    pub fn with_defaults(mut self, defaults: RunnerDefaults) -> Self {
        self.defaults = defaults;
        self
    }

    /// Attach a graph store so context-pack injection works.
    /// Without it the executor still spawns runners but the prompt
    /// will not carry tier-3 retrieval (best-effort behaviour).
    pub fn with_graph(mut self, graph: convergio_graph::Store) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Attach a runner registry (`~/.convergio/runners.toml`).
    /// Without one the executor only resolves built-in vendors
    /// (`claude`, `copilot`); tasks pointing at a custom vendor
    /// fail with `RunnerError::UnknownVendor`.
    pub fn with_registry(mut self, registry: RunnerRegistry) -> Self {
        self.registry = std::sync::Arc::new(registry);
        self
    }

    /// Run one dispatch round. Returns the number of tasks moved to
    /// `in_progress`.
    pub async fn tick(&self) -> Result<usize> {
        let pending = self.find_dispatchable().await?;
        let mut dispatched = 0usize;
        for (task_id, plan_id) in pending {
            self.dispatch_one(&task_id, &plan_id).await?;
            dispatched += 1;
        }
        Ok(dispatched)
    }

    async fn find_dispatchable(&self) -> Result<Vec<(String, String)>> {
        // Pending tasks whose wave is "ready" — no earlier-wave task
        // is still open in the same plan.
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT t.id, t.plan_id, t.wave \
             FROM tasks t \
             WHERE t.status = 'pending' \
               AND NOT EXISTS ( \
                   SELECT 1 FROM tasks t2 \
                   WHERE t2.plan_id = t.plan_id \
                     AND t2.wave < t.wave \
                      AND t2.status NOT IN ('done', 'failed') \
               ) \
             ORDER BY t.wave ASC, t.sequence ASC",
        )
        .fetch_all(self.durability.pool().inner())
        .await
        .map_err(convergio_durability::DurabilityError::from)?;
        Ok(rows.into_iter().map(|r| (r.0, r.1)).collect())
    }

    async fn dispatch_one(&self, task_id: &str, plan_id: &str) -> Result<()> {
        // Read the task to see if it carries an explicit
        // `runner_kind`. Tasks created by the legacy planner / by
        // older clients leave it `None` → fall back to the legacy
        // shell template (still useful for smoke tests).
        let task = self.durability.tasks().get(task_id).await?;
        let is_legacy_shell =
            task.runner_kind.is_none() && std::env::var("CONVERGIO_EXECUTOR_USE_RUNNER").is_err();
        let proc = if is_legacy_shell {
            self.spawn_legacy(task_id, plan_id).await?
        } else {
            self.spawn_via_runner(&task, plan_id).await?
        };

        self.durability
            .transition_task(task_id, TaskStatus::InProgress, Some(&proc.id))
            .await?;
        Ok(())
    }

    /// Legacy `/bin/echo`-style spawn — the MVP path. Still useful
    /// for shell-runner smoke tests + when `runner_kind` is None.
    async fn spawn_legacy(
        &self,
        task_id: &str,
        plan_id: &str,
    ) -> Result<convergio_lifecycle::AgentProcess> {
        let mut args = self.template.args.clone();
        args.push(task_id.to_string());
        Ok(self
            .supervisor
            .spawn(SpawnSpec {
                kind: self.template.kind.clone(),
                command: self.template.command.clone(),
                args,
                env: vec![],
                plan_id: Some(plan_id.to_string()),
                task_id: Some(task_id.to_string()),
                cwd: None,
                stdin_payload: None,
            })
            .await?)
    }

    /// ADR-0034: per-task runner-based spawn. Picks
    /// `task.runner_kind` (or daemon default), prepares the vendor
    /// CLI argv via `convergio-runner`, fetches the graph context
    /// pack when available, spawns through the supervisor with the
    /// prompt piped on stdin.
    async fn spawn_via_runner(
        &self,
        task: &convergio_durability::Task,
        plan_id: &str,
    ) -> Result<convergio_lifecycle::AgentProcess> {
        let kind = task
            .runner_kind
            .as_deref()
            .and_then(|s| RunnerKind::from_str(s).ok())
            .unwrap_or_else(|| self.defaults.kind.clone());
        let profile = task
            .profile
            .as_deref()
            .and_then(|s| PermissionProfile::from_str(s).ok())
            .unwrap_or(self.defaults.profile);
        let plan_title = self
            .durability
            .plans()
            .get(plan_id)
            .await
            .map(|p| p.title)
            .unwrap_or_else(|_| "(unknown)".into());
        let agent_id = format!("{}-{}", kind.vendor, task.id.get(..7).unwrap_or(&task.id));
        let seed = build_graph_seed(task);
        let graph_context = self.fetch_graph_context(&task.id, &seed).await;
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let ctx = SpawnContext {
            task,
            plan_id,
            plan_title: &plan_title,
            daemon_url: &self.defaults.daemon_url,
            agent_id: &agent_id,
            graph_context: graph_context.as_deref(),
            cwd: &cwd,
            max_budget_usd: task.max_budget_usd,
            profile,
        };
        let prepared =
            for_kind_with_registry(&kind, &self.registry).and_then(|r| r.prepare(&ctx))?;
        let args: Vec<String> = prepared
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        Ok(self
            .supervisor
            .spawn(SpawnSpec {
                kind: kind.to_string(),
                command: prepared.program.to_string_lossy().into_owned(),
                args,
                env: vec![],
                plan_id: Some(plan_id.to_string()),
                task_id: Some(task.id.clone()),
                cwd: Some(prepared.cwd),
                stdin_payload: Some(prepared.stdin_prompt),
            })
            .await?)
    }

    async fn fetch_graph_context(&self, task_id: &str, seed: &str) -> Option<String> {
        let g = self.graph.as_ref()?;
        let pack = convergio_graph::for_task_text(g, task_id, seed, 50, 8_000)
            .await
            .ok()?;
        serde_json::to_string_pretty(&pack).ok()
    }
}

/// Synthesize the seed text the graph layer ranks against. Concat
/// title + description (when present) so both signal sources count.
fn build_graph_seed(task: &convergio_durability::Task) -> String {
    match task.description.as_deref() {
        Some(d) if !d.is_empty() => format!("{}\n\n{}", task.title, d),
        _ => task.title.clone(),
    }
}

/// Spawned-loop handle. Drop the handle to abort.
pub struct ExecutorHandle {
    inner: JoinHandle<()>,
}

impl ExecutorHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(&self) {
        self.inner.abort();
    }
}

/// Spawn the executor loop. Errors during a tick are logged at
/// `warn!` and do not kill the loop.
pub fn spawn_loop(executor: Arc<Executor>, tick_interval: Duration) -> ExecutorHandle {
    let inner = tokio::spawn(async move {
        info!(
            tick_secs = tick_interval.num_seconds(),
            "executor loop started"
        );
        let interval = tokio_duration(tick_interval);
        loop {
            tokio::time::sleep(interval).await;
            match executor.tick().await {
                Ok(n) if n > 0 => info!(dispatched = n, "executor tick"),
                Ok(_) => debug!("executor tick: nothing pending"),
                Err(e) => warn!(error = %e, "executor tick failed"),
            }
        }
    });
    ExecutorHandle { inner }
}

fn tokio_duration(d: Duration) -> std::time::Duration {
    std::time::Duration::from_millis(d.num_milliseconds().max(1) as u64)
}
