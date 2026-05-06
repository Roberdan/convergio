//! HTTP client + GitHub shell-out for the dashboard. Read-only by
//! design — actions go through `cvg` subcommands. Endpoints:
//! `GET /v1/plans`, `/v1/plans/{id}/tasks`, `/v1/agents`,
//! `/v1/plans/{id}/messages/tail`, `/v1/audit/verify`, plus `gh pr list` (skipped when
//! `CONVERGIO_DASH_NO_GH=1`).

use crate::client_gh::fetch_prs;
use anyhow::Result;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use crate::plan_counts::PlanCounts;
pub use crate::types::{AgentProcess, BusMessage, Plan, PrSummary, RegistryAgent, TaskSummary};

/// Snapshot of every dataset the dashboard renders.
#[derive(Debug, Default)]
pub struct Snapshot {
    /// Plans.
    pub plans: Vec<Plan>,
    /// Tasks across every loaded plan.
    pub tasks: Vec<TaskSummary>,
    /// Registered agents.
    pub agents: Vec<RegistryAgent>,
    /// Supervised agent processes.
    pub agent_processes: Vec<AgentProcess>,
    /// PRs via `gh pr list` (empty when disabled).
    pub prs: Vec<PrSummary>,
    /// Recent bus messages.
    pub messages: Vec<BusMessage>,
    /// Audit chain verifies / not / unreachable.
    pub audit_ok: Option<bool>,
    /// Daemon version from `/v1/health`, compared with binary's
    /// `CARGO_PKG_VERSION` to surface drift in the header.
    pub daemon_version: Option<String>,
}

/// PR data is cached for this long before the next `gh pr list`
/// shell-out. The dashboard tick is 5s by default; 30s means the
/// `gh` cost is amortised across ~6 refreshes without making the
/// PR pane feel stale (PR state turns over much slower than tasks).
const PR_CACHE_TTL: Duration = Duration::from_secs(30);

/// Time-stamped cache entry for the PR list.
type PrCacheCell = Arc<Mutex<Option<(Instant, Vec<PrSummary>)>>>;

/// Read-only HTTP client. Cloneable.
#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    inner: reqwest::Client,
    enable_gh: bool,
    github_slug: Option<String>,
    pr_cache: PrCacheCell,
}

impl Client {
    /// Build a client targeting `base` (e.g. `http://127.0.0.1:8420`).
    pub fn new(base: String) -> Self {
        let enable_gh = std::env::var("CONVERGIO_DASH_NO_GH").ok().as_deref() != Some("1");
        Self {
            base,
            inner: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            enable_gh,
            github_slug: None,
            pr_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Scope `gh pr list` to `owner/repo` instead of inheriting cwd.
    /// `cvg dash` derives the slug from `origin` so the dashboard
    /// works from any directory.
    pub fn with_github_slug(mut self, slug: Option<String>) -> Self {
        self.github_slug = slug.filter(|s| !s.is_empty());
        self
    }

    /// One-shot fetch of every dataset. Sub-fetches fail soft —
    /// partial data is more useful than blanking the dashboard.
    ///
    /// Per-plan fetches (`tasks` + `messages/tail`) run in parallel
    /// via `futures::future::join_all`; without this fan-out the
    /// loop is N+1 and dashes with 40+ plans block for hundreds of
    /// ms per refresh on loopback. Global fetches (registry,
    /// processes, audit, health) run concurrently with the
    /// per-plan fan-out via `tokio::join!`. The PRs `gh pr list`
    /// shell-out also runs concurrently (it is the dominant cost,
    /// ~600ms on a warm cache) and is additionally memoised for
    /// [`PR_CACHE_TTL`] so most refreshes pay zero gh cost.
    pub async fn snapshot(&self) -> Result<Snapshot> {
        let mut plans: Vec<Plan> = self
            .get_json("/v1/plans")
            .await
            .unwrap_or_else(|_| Vec::new());
        sort_plans_by_status(&mut plans);

        let plan_ids: Vec<String> = plans.iter().map(|p| p.id.clone()).collect();
        let plan_fetches = futures::future::join_all(
            plan_ids
                .iter()
                .map(|id| self.fetch_plan_overview(id.clone())),
        );

        let global_fetches = self.fetch_globals();
        let prs_future = self.fetch_prs_cached();

        let (plan_results, (agents, agent_processes, audit_ok, daemon_version), prs) =
            tokio::join!(plan_fetches, global_fetches, prs_future);

        let mut tasks: Vec<TaskSummary> = Vec::new();
        let mut messages: Vec<BusMessage> = Vec::new();
        for (plan_id, mut plan_tasks, mut plan_messages) in plan_results {
            for t in &mut plan_tasks {
                t.plan_id = plan_id.clone();
            }
            tasks.append(&mut plan_tasks);
            messages.append(&mut plan_messages);
        }
        messages.sort_by_key(|m| std::cmp::Reverse(m.seq));
        messages.truncate(200);

        Ok(Snapshot {
            plans,
            tasks,
            agents,
            agent_processes,
            prs,
            messages,
            audit_ok,
            daemon_version,
        })
    }

    /// PR fetch with a [`PR_CACHE_TTL`] memo. Returns the cached
    /// vector when fresh; otherwise shells out to `gh` (off the
    /// blocking thread, see [`crate::client_gh::fetch_prs`]) and
    /// updates the cache on success. A failed fetch keeps the
    /// stale cache rather than blanking the PRs pane — partial
    /// data beats no data.
    async fn fetch_prs_cached(&self) -> Vec<PrSummary> {
        if !self.enable_gh {
            return Vec::new();
        }
        if let Some((stamped_at, cached)) = self.pr_cache_snapshot() {
            if stamped_at.elapsed() < PR_CACHE_TTL {
                return cached;
            }
        }
        match fetch_prs(self.github_slug.as_deref()).await {
            Ok(prs) => {
                self.pr_cache_store(prs.clone());
                prs
            }
            Err(_) => self.pr_cache_snapshot().map(|(_, v)| v).unwrap_or_default(),
        }
    }

    fn pr_cache_snapshot(&self) -> Option<(Instant, Vec<PrSummary>)> {
        let guard = match self.pr_cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.clone()
    }

    fn pr_cache_store(&self, prs: Vec<PrSummary>) {
        let mut guard = match self.pr_cache.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = Some((Instant::now(), prs));
    }

    async fn fetch_plan_overview(
        &self,
        plan_id: String,
    ) -> (String, Vec<TaskSummary>, Vec<BusMessage>) {
        let tasks_path = format!("/v1/plans/{plan_id}/tasks");
        let messages_path = format!("/v1/plans/{plan_id}/messages/tail?limit=100");
        let (tasks, messages) = tokio::join!(
            self.get_json::<Vec<TaskSummary>>(&tasks_path),
            self.get_json::<Vec<BusMessage>>(&messages_path),
        );
        (
            plan_id,
            tasks.unwrap_or_default(),
            messages.unwrap_or_default(),
        )
    }

    async fn fetch_globals(
        &self,
    ) -> (
        Vec<RegistryAgent>,
        Vec<AgentProcess>,
        Option<bool>,
        Option<String>,
    ) {
        let (agents, processes, audit, health) = tokio::join!(
            self.get_json::<Vec<RegistryAgent>>("/v1/agent-registry/agents"),
            self.get_json::<Vec<AgentProcess>>("/v1/agents?limit=200"),
            self.get_json::<serde_json::Value>("/v1/audit/verify"),
            self.get_json::<serde_json::Value>("/v1/health"),
        );
        let agents = agents.unwrap_or_default();
        let processes = processes.unwrap_or_default();
        let audit_ok = audit
            .ok()
            .and_then(|v| v.get("ok").and_then(|b| b.as_bool()));
        let daemon_version = health.ok().and_then(|v| {
            v.get("running_version")
                .or_else(|| v.get("version"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        });
        (agents, processes, audit_ok, daemon_version)
    }

    /// Fetch *all* tasks for a plan (not the overview's active-only
    /// subset). Used by drill-down so closed tasks are visible too.
    pub async fn fetch_plan_tasks(&self, plan_id: &str) -> Result<Vec<TaskSummary>> {
        let mut tasks: Vec<TaskSummary> =
            self.get_json(&format!("/v1/plans/{plan_id}/tasks")).await?;
        for t in &mut tasks {
            t.plan_id = plan_id.to_string();
        }
        Ok(tasks)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .inner
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?;
        Ok(resp)
    }
}

/// Sort plans for the dashboard: `active < draft < completed <
/// cancelled`, ties broken on `updated_at desc`. Mirrors operator
/// triage order — what's running floats to the top.
pub fn sort_plans_by_status(plans: &mut [Plan]) {
    plans.sort_by(|a, b| {
        plan_status_rank(&a.status)
            .cmp(&plan_status_rank(&b.status))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
}

fn plan_status_rank(status: &str) -> u8 {
    match status {
        "active" => 0,
        "draft" => 1,
        "completed" => 2,
        "cancelled" => 3,
        _ => 4,
    }
}
