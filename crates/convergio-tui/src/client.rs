//! HTTP client + GitHub shell-out for the dashboard. Read-only by
//! design — actions go through `cvg` subcommands. Endpoints:
//! `GET /v1/plans`, `/v1/plans/{id}/tasks`, `/v1/agents`,
//! `/v1/plans/{id}/messages/tail`, `/v1/audit/verify`, plus `gh pr list` (skipped when
//! `CONVERGIO_DASH_NO_GH=1`).

use crate::client_gh::fetch_prs;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

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

/// Read-only HTTP client. Cloneable.
#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    inner: reqwest::Client,
    enable_gh: bool,
    github_slug: Option<String>,
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
    pub async fn snapshot(&self) -> Result<Snapshot> {
        let mut plans: Vec<Plan> = self
            .get_json("/v1/plans")
            .await
            .unwrap_or_else(|_| Vec::new());
        sort_plans_by_status(&mut plans);

        let mut tasks: Vec<TaskSummary> = Vec::new();
        let mut messages: Vec<BusMessage> = Vec::new();
        for p in &plans {
            if let Ok(mut ts) = self
                .get_json::<Vec<TaskSummary>>(&format!("/v1/plans/{}/tasks", p.id))
                .await
            {
                for t in &mut ts {
                    t.plan_id = p.id.clone();
                }
                tasks.extend(ts);
            }
            if let Ok(mut ms) = self
                .get_json::<Vec<BusMessage>>(&format!("/v1/plans/{}/messages/tail?limit=100", p.id))
                .await
            {
                messages.append(&mut ms);
            }
        }
        messages.sort_by_key(|m| std::cmp::Reverse(m.seq));
        messages.truncate(200);

        let agents: Vec<RegistryAgent> = self
            .get_json("/v1/agent-registry/agents")
            .await
            .unwrap_or_else(|_| Vec::new());

        let agent_processes: Vec<AgentProcess> = self
            .get_json("/v1/agents?limit=200")
            .await
            .unwrap_or_default();

        let audit_ok = self
            .get_json::<serde_json::Value>("/v1/audit/verify")
            .await
            .ok()
            .and_then(|v| v.get("ok").and_then(|b| b.as_bool()));

        let daemon_version = self
            .get_json::<serde_json::Value>("/v1/health")
            .await
            .ok()
            .and_then(|v| {
                v.get("running_version")
                    .or_else(|| v.get("version"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            });

        let prs = if self.enable_gh {
            fetch_prs(self.github_slug.as_deref()).unwrap_or_default()
        } else {
            Vec::new()
        };

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
