//! Audited agent registry facade operations.

use crate::audit::EntityKind;
use crate::store::{AgentHeartbeat, AgentRecord, AgentStore, NewAgent};
use crate::{Durability, Result};
use chrono::{Duration, Utc};
use serde_json::json;

/// Re-registration within this window does NOT emit a new
/// `agent.session_started` audit row. The SessionStart hook fires on
/// every Claude Code session resume — without this guard we would
/// spam the audit chain on every prompt context restore.
const SESSION_STARTED_DEDUP_MINUTES: i64 = 30;

impl Durability {
    /// Agent registry store accessor.
    pub fn agents(&self) -> AgentStore {
        AgentStore::new(self.pool().clone())
    }

    /// Register or refresh an agent identity and write an audit row.
    ///
    /// Emits two audit kinds:
    /// - `agent.registered` — every call (idempotent identity refresh).
    /// - `agent.session_started` — only when the agent is new, or when
    ///   the previous heartbeat is older than
    ///   [`SESSION_STARTED_DEDUP_MINUTES`]. This is the telemetry
    ///   signal `/v1/status.telemetry.sessions_started_24h` counts.
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
}
