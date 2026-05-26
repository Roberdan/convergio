//! `Executor::tick` — one-shot dispatch round.

use crate::defaults::{RunnerDefaults, SpawnTemplate};
use crate::error::{ExecutorError, Result};
use crate::graph_seed::build_graph_seed;
use crate::{heartbeat, worktree};
use convergio_durability::{Durability, TaskStatus};
use convergio_lifecycle::{SpawnSpec, Supervisor};
use convergio_runner::{
    for_kind_with_registry, PermissionProfile, RunnerKind, RunnerRegistry, SpawnContext,
};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{info, warn};

/// Executor handle.
#[derive(Clone)]
pub struct Executor {
    durability: Durability,
    supervisor: Supervisor,
    template: SpawnTemplate,
    defaults: RunnerDefaults,
    graph: Option<convergio_graph::Store>,
    registry: std::sync::Arc<RunnerRegistry>,
    repo_path: Option<PathBuf>,
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
            repo_path: None,
        }
    }

    /// Set the operator's repo root. Required for runner-based
    /// dispatch — the executor pre-creates a git worktree under
    /// `<repo_path>/.claude/worktrees/agent-<task7>` per task.
    pub fn with_repo_path(mut self, repo_path: PathBuf) -> Self {
        self.repo_path = Some(repo_path);
        self
    }

    /// Override the daemon-wide runner defaults (`runner_kind`,
    /// `profile`, daemon callback URL).
    pub fn with_defaults(mut self, defaults: RunnerDefaults) -> Self {
        self.defaults = defaults;
        self
    }

    /// Attach a graph store so context-pack injection works.
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

    /// Run one dispatch round. Respects `CONVERGIO_EXECUTOR_MAX_PARALLEL`
    /// when set; without it dispatches every wave-ready pending task.
    pub async fn tick(&self) -> Result<usize> {
        let pending = self.find_dispatchable().await?;
        let cap = std::env::var("CONVERGIO_EXECUTOR_MAX_PARALLEL")
            .ok()
            .and_then(|s| s.parse::<usize>().ok());
        let budget = match cap {
            Some(max) => {
                let in_flight = self.count_in_progress().await?;
                max.saturating_sub(in_flight)
            }
            None => pending.len(),
        };
        if budget == 0 {
            return Ok(0);
        }
        let mut dispatched = 0usize;
        for (task_id, plan_id) in pending {
            if dispatched >= budget {
                break;
            }
            if let Err(e) = self.dispatch_one(&task_id, &plan_id).await {
                warn!(task_id, plan_id, error = %e, "executor dispatch failed");
                continue;
            }
            dispatched += 1;
        }
        Ok(dispatched)
    }

    async fn count_in_progress(&self) -> Result<usize> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'")
            .fetch_one(self.durability.pool().inner())
            .await
            .map_err(convergio_durability::DurabilityError::from)?;
        Ok(row.0 as usize)
    }

    async fn find_dispatchable(&self) -> Result<Vec<(String, String)>> {
        // Pending tasks whose wave is "ready" — no earlier-wave task
        // is still open in the same plan. `no_dispatch = 0` filters
        // out tracker-only tasks (A.2) so the executor never picks
        // them up; the operator drives them manually.
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT t.id, t.plan_id, t.wave \
             FROM tasks t \
             WHERE t.status = 'pending' \
               AND t.no_dispatch = 0 \
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
        // W1-B atomic claim + pre-check guards: cap-exceeded is
        // transient, so skip claim+compensate to avoid flipping
        // tasks to Failed under disk pressure (convergio-edu bug
        // 2026-05-12). Claim+compensate stays for real spawn errors.
        if let Some(repo_root) = self.repo_path.as_ref() {
            let holders = crate::holders::collect(&self.durability, repo_root).await;
            if let Err(e) = crate::guards::enforce_with_holders(repo_root, &holders) {
                tracing::debug!(task_id, plan_id, error = %e, "skipping dispatch — guard refused");
                return Ok(());
            }
        }
        let task = self.durability.tasks().get(task_id).await?;
        let is_legacy_shell =
            task.runner_kind.is_none() && std::env::var("CONVERGIO_EXECUTOR_USE_RUNNER").is_err();
        let kind = task
            .runner_kind
            .as_deref()
            .and_then(|s| RunnerKind::from_str(s).ok())
            .unwrap_or_else(|| self.defaults.kind.clone());
        let task7 = task.id.get(..7).unwrap_or(&task.id);
        let agent_id = if is_legacy_shell {
            format!("legacy-{task7}")
        } else {
            format!("{}-{}", kind.vendor, task7)
        };
        let Some(task) = self
            .durability
            .try_claim_pending(task_id, &agent_id)
            .await?
        else {
            tracing::debug!(task_id, plan_id, "claim lost — task no longer pending");
            return Ok(());
        };
        let spawn_result = if is_legacy_shell {
            self.spawn_legacy(task_id, plan_id).await
        } else {
            self.spawn_via_runner(&task, plan_id).await
        };
        if let Err(err) = spawn_result {
            warn!(task_id, plan_id, error = %err, "spawn failed — compensating to failed");
            self.durability
                .transition_task(task_id, TaskStatus::Failed, Some(&agent_id))
                .await
                .ok();
            return Err(err);
        }
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

    /// ADR-0034: per-task runner-based spawn.
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
        let repo_path = self.repo_path.as_ref().ok_or_else(|| {
            ExecutorError::Worktree(
                "CONVERGIO_REPO_PATH not configured — refusing to spawn runner".into(),
            )
        })?;
        let holders = crate::holders::collect(&self.durability, repo_path).await;
        let cwd = worktree::prepare(repo_path, &task.id, &holders)?;
        info!(task_id = %task.id, cwd = %cwd.display(), "prepared agent worktree");
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
        let proc = self
            .supervisor
            .spawn(SpawnSpec {
                kind: kind.to_string(),
                command: prepared.program.to_string_lossy().into_owned(),
                args,
                env: prepared.env,
                plan_id: Some(plan_id.to_string()),
                task_id: Some(task.id.clone()),
                cwd: Some(prepared.cwd),
                stdin_payload: Some(prepared.stdin_prompt),
            })
            .await?;
        // Vendor CLIs in non-interactive mode don't tick the
        // task heartbeat themselves; the sidecar does it for them
        // and removes the worktree on terminal exit.
        if let Some(pid) = proc.pid {
            heartbeat::spawn(
                self.durability.clone(),
                self.repo_path.clone(),
                task.id.clone(),
                pid,
            );
        }
        Ok(proc)
    }

    async fn fetch_graph_context(&self, task_id: &str, seed: &str) -> Option<String> {
        let g = self.graph.as_ref()?;
        let pack = convergio_graph::for_task_text(g, task_id, seed, 50, 8_000)
            .await
            .ok()?;
        serde_json::to_string_pretty(&pack).ok()
    }
}
