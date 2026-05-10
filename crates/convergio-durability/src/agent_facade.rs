//! Audited agent registry facade operations.

use crate::audit::EntityKind;
use crate::store::{
    AgentHeartbeat, AgentRecord, AgentStore, NewAgent, RetireStaleResult, StaleAgentReport,
};
use crate::{Durability, Result};
use chrono::{Duration, Utc};
use serde_json::json;

/// Re-registration within this window suppresses `agent.session_started`.
const SESSION_STARTED_DEDUP_MINUTES: i64 = 30;

impl Durability {
    /// Agent registry store accessor.
    pub fn agents(&self) -> AgentStore {
        AgentStore::new(self.pool().clone())
    }

    /// Register or refresh an agent identity. Emits `agent.registered`
    /// every call and `agent.session_started` only when the agent is
    /// new or its previous heartbeat is older than
    /// [`SESSION_STARTED_DEDUP_MINUTES`] (the telemetry signal
    /// `/v1/status.telemetry.sessions_started_24h` counts).
    pub async fn register_agent(&self, input: NewAgent) -> Result<AgentRecord> {
        let prior = self.agents().get(&input.id).await.ok();
        let is_session_start = match &prior {
            None => true,
            Some(prev) => match prev.last_heartbeat_at {
                None => true,
                Some(ts) => {
                    Utc::now().signed_duration_since(ts)
                        > Duration::minutes(SESSION_STARTED_DEDUP_MINUTES)
                }
            },
        };
        let host = input.host.clone();
        let metadata = input.metadata.clone();
        let agent = self.agents().register(input).await?;
        self.audit()
            .append(
                EntityKind::Agent,
                &agent.id,
                "agent.registered",
                &json!({
                    "agent_id": agent.id,
                    "kind": agent.kind,
                    "capabilities": agent.capabilities,
                }),
                Some(&agent.id),
            )
            .await?;
        if is_session_start {
            let repo_root = metadata
                .get("repo_root")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            self.audit()
                .append(
                    EntityKind::Agent,
                    &agent.id,
                    "agent.session_started",
                    &json!({
                        "agent_id": agent.id,
                        "host": host,
                        "kind": agent.kind,
                        "capabilities": agent.capabilities,
                        "repo_root": repo_root,
                    }),
                    Some(&agent.id),
                )
                .await?;
        }
        Ok(agent)
    }

    /// Record an agent heartbeat and write an audit row.
    pub async fn heartbeat_agent(
        &self,
        agent_id: &str,
        input: AgentHeartbeat,
    ) -> Result<AgentRecord> {
        let agent = self.agents().heartbeat(agent_id, input).await?;
        self.audit()
            .append(
                EntityKind::Agent,
                &agent.id,
                "agent.heartbeat",
                &json!({
                    "agent_id": agent.id,
                    "status": agent.status,
                    "current_task_id": agent.current_task_id,
                }),
                Some(&agent.id),
            )
            .await?;
        Ok(agent)
    }

    /// Retire an agent identity and write an audit row.
    pub async fn retire_agent(&self, agent_id: &str) -> Result<AgentRecord> {
        let agent = self.agents().retire(agent_id).await?;
        self.audit()
            .append(
                EntityKind::Agent,
                &agent.id,
                "agent.retired",
                &json!({"agent_id": agent.id}),
                Some(&agent.id),
            )
            .await?;
        Ok(agent)
    }

    /// Re-register a terminated agent identity: flips status back to
    /// `idle` without mutating identity metadata. Writes one audit row
    /// of kind `agent.re_registered`.
    pub async fn reregister_agent(&self, agent_id: &str) -> Result<AgentRecord> {
        let agent = self.agents().get(agent_id).await?;
        if agent.status != "terminated" {
            return Err(crate::error::DurabilityError::AgentNotTerminated {
                id: agent_id.to_string(),
                actual: agent.status,
            });
        }
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE agents SET status = 'idle', current_task_id = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(agent_id)
        .execute(self.pool().inner())
        .await?;

        self.audit()
            .append(
                EntityKind::Agent,
                agent_id,
                "agent.re_registered",
                &json!({"agent_id": agent_id}),
                Some(agent_id),
            )
            .await?;
        self.agents().get(agent_id).await
    }

    /// Retire all agents whose last heartbeat is older than
    /// `threshold_secs`. Writes one `agent.retired_stale` audit row
    /// per agent retired. When `dry_run` is true the method only
    /// reports which agents *would* be retired without touching the DB.
    pub async fn retire_stale_agents(
        &self,
        threshold_secs: i64,
        dry_run: bool,
    ) -> Result<RetireStaleResult> {
        let reports = self.agents().stale_agents(threshold_secs).await?;
        let mut out = Vec::with_capacity(reports.len());
        for report in reports {
            let retired = if dry_run {
                false
            } else {
                self.retire_one_stale(&report.agent_id, report.last_heartbeat_at)
                    .await?
            };
            out.push(StaleAgentReport { retired, ..report });
        }
        Ok(RetireStaleResult {
            threshold_seconds: threshold_secs,
            applied: !dry_run,
            agents: out,
        })
    }

    async fn retire_one_stale(
        &self,
        agent_id: &str,
        last_heartbeat_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<bool> {
        let rows = sqlx::query(
            "UPDATE agents SET status = 'retired', current_task_id = NULL, \
             updated_at = ? WHERE id = ? AND status NOT IN ('terminated', 'retired')",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(agent_id)
        .execute(self.pool().inner())
        .await?
        .rows_affected();

        if rows == 0 {
            return Ok(false);
        }

        self.audit()
            .append(
                EntityKind::Agent,
                agent_id,
                "agent.retired_stale",
                &json!({
                    "agent_id": agent_id,
                    "last_heartbeat_at": last_heartbeat_at.map(|t| t.to_rfc3339()),
                }),
                None,
            )
            .await?;
        Ok(true)
    }
}
