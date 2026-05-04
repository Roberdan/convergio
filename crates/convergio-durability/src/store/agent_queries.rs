//! Read-only enrichment queries layered on the durable agent
//! registry. Powers `cvg agent list` (`summaries` / `summary`),
//! `cvg agent show <id>` (callers compose), and the dry-run side
//! of `cvg agent retire-stale`. Mutation goes through
//! [`super::AgentStore`] / [`crate::Durability`].

use crate::error::Result;
use crate::store::agent_rows::{AgentRow, AGENT_SELECT};
use crate::store::agent_summary::{
    decode_payload, resource_label, AgentAuditEntry, AgentLease, AgentPrLink, AgentSummary,
    CurrentTaskMeta, StaleAgentReport,
};
use crate::store::{AgentRecord, AgentStore};
use chrono::{DateTime, Utc};
use sqlx::Row;

fn lease_from_row(r: sqlx::sqlite::SqliteRow) -> Result<AgentLease> {
    let kind: String = r.try_get("kind")?;
    let project: Option<String> = r.try_get("project")?;
    let path: String = r.try_get("path")?;
    let symbol: Option<String> = r.try_get("symbol")?;
    Ok(AgentLease {
        id: r.try_get("id")?,
        resource_label: resource_label(&kind, project.as_deref(), &path, symbol.as_deref()),
        purpose: r.try_get("purpose")?,
        created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
        expires_at: parse_ts(&r.try_get::<String, _>("expires_at")?),
    })
}

fn audit_from_row(r: sqlx::sqlite::SqliteRow) -> Result<AgentAuditEntry> {
    let payload: String = r.try_get("payload")?;
    Ok(AgentAuditEntry {
        seq: r.try_get("seq")?,
        transition: r.try_get("transition")?,
        entity_type: r.try_get("entity_type")?,
        entity_id: r.try_get("entity_id")?,
        created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
        payload: decode_payload(&payload),
    })
}

fn pr_from_row(r: sqlx::sqlite::SqliteRow) -> Result<AgentPrLink> {
    Ok(AgentPrLink {
        repo_slug: r.try_get("repo_slug")?,
        pr_number: r.try_get("pr_number")?,
        branch: r.try_get("branch")?,
        plan_id: r.try_get("plan_id")?,
        task_id: r.try_get("task_id")?,
        created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
    })
}

fn parse_ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

impl AgentStore {
    /// Enriched list used by `cvg agent list`.
    pub async fn summaries(&self) -> Result<Vec<AgentSummary>> {
        let agents = self.list().await?;
        let mut out = Vec::with_capacity(agents.len());
        for agent in agents {
            out.push(self.enrich(agent).await?);
        }
        Ok(out)
    }

    /// Single-agent enrichment used by [`Self::summaries`] and the
    /// rich show view.
    pub async fn summary(&self, agent_id: &str) -> Result<AgentSummary> {
        let agent = self.get(agent_id).await?;
        self.enrich(agent).await
    }

    async fn enrich(&self, agent: AgentRecord) -> Result<AgentSummary> {
        let mut current_task_title = None;
        let mut current_task_status = None;
        let mut plan_id: Option<String> = None;
        if let Some(task_id) = &agent.current_task_id {
            if let Some(r) =
                sqlx::query("SELECT title, status, plan_id FROM tasks WHERE id = ? LIMIT 1")
                    .bind(task_id)
                    .fetch_optional(self.pool().inner())
                    .await?
            {
                current_task_title = Some(r.try_get::<String, _>("title")?);
                current_task_status = Some(r.try_get::<String, _>("status")?);
                plan_id = Some(r.try_get::<String, _>("plan_id")?);
            }
        }
        let active_leases: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workspace_leases WHERE agent_id = ? AND status = 'active'",
        )
        .bind(&agent.id)
        .fetch_one(self.pool().inner())
        .await?;
        let (recent_branch, recent_pr_number) = self.recent_pr(&agent, plan_id.as_deref()).await?;
        let (last_audit_kind, last_audit_at) = self.last_audit(&agent.id).await?;
        Ok(AgentSummary {
            agent,
            current_task_title,
            current_task_status,
            recent_branch,
            recent_pr_number,
            active_leases,
            last_audit_kind,
            last_audit_at,
        })
    }

    async fn recent_pr(
        &self,
        agent: &AgentRecord,
        plan_id: Option<&str>,
    ) -> Result<(Option<String>, Option<i64>)> {
        let row = sqlx::query(
            "SELECT pr_number, branch FROM plan_pr_links \
             WHERE (? IS NOT NULL AND task_id = ?) OR (? IS NOT NULL AND plan_id = ?) \
             ORDER BY (task_id = ?) DESC, created_at DESC LIMIT 1",
        )
        .bind(&agent.current_task_id)
        .bind(&agent.current_task_id)
        .bind(plan_id)
        .bind(plan_id)
        .bind(&agent.current_task_id)
        .fetch_optional(self.pool().inner())
        .await?;
        if let Some(r) = row {
            return Ok((r.try_get("branch")?, Some(r.try_get("pr_number")?)));
        }
        let branch = agent
            .metadata
            .get("branch")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        Ok((branch, None))
    }

    async fn last_audit(&self, id: &str) -> Result<(Option<String>, Option<DateTime<Utc>>)> {
        let row = sqlx::query(
            "SELECT transition, created_at FROM audit_log \
             WHERE agent_id = ? ORDER BY seq DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(self.pool().inner())
        .await?;
        let Some(r) = row else {
            return Ok((None, None));
        };
        let ts: String = r.try_get("created_at")?;
        Ok((
            Some(r.try_get("transition")?),
            DateTime::parse_from_rfc3339(&ts)
                .ok()
                .map(|d| d.with_timezone(&Utc)),
        ))
    }

    /// Plan metadata (id + title + started_at) for an agent's
    /// current task. `None` when the task or its plan is missing.
    pub async fn current_task_meta(
        &self,
        task_id: Option<&str>,
    ) -> Result<Option<CurrentTaskMeta>> {
        let Some(task_id) = task_id else {
            return Ok(None);
        };
        let Some(r) = sqlx::query(
            "SELECT t.plan_id, t.started_at, p.title AS plan_title \
             FROM tasks t JOIN plans p ON p.id = t.plan_id WHERE t.id = ? LIMIT 1",
        )
        .bind(task_id)
        .fetch_optional(self.pool().inner())
        .await?
        else {
            return Ok(None);
        };
        let started_at = r
            .try_get::<Option<String>, _>("started_at")
            .ok()
            .flatten()
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        Ok(Some(CurrentTaskMeta {
            plan_id: r.try_get("plan_id")?,
            plan_title: r.try_get("plan_title")?,
            started_at,
        }))
    }

    /// Active workspace leases held by `agent_id`, newest first.
    pub async fn leases_for_agent(&self, agent_id: &str) -> Result<Vec<AgentLease>> {
        let rows = sqlx::query(
            "SELECT l.id, l.purpose, l.expires_at, l.created_at, \
                    r.kind, r.project, r.path, r.symbol \
             FROM workspace_leases l JOIN workspace_resources r ON r.id = l.resource_id \
             WHERE l.agent_id = ? AND l.status = 'active' ORDER BY l.created_at DESC",
        )
        .bind(agent_id)
        .fetch_all(self.pool().inner())
        .await?;
        rows.into_iter().map(lease_from_row).collect()
    }

    /// Most recent `audit_log` entries tagged with `agent_id`.
    pub async fn recent_audit_for_agent(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<AgentAuditEntry>> {
        let limit = limit.clamp(1, 100);
        let rows = sqlx::query(
            "SELECT seq, transition, entity_type, entity_id, payload, created_at \
             FROM audit_log WHERE agent_id = ? ORDER BY seq DESC LIMIT ?",
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(self.pool().inner())
        .await?;
        rows.into_iter().map(audit_from_row).collect()
    }

    /// Recent PRs tied to `plan_id` (preferred) or to a single
    /// `task_id`. Empty when neither key is supplied.
    pub async fn recent_prs(
        &self,
        plan_id: Option<&str>,
        task_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AgentPrLink>> {
        let limit = limit.clamp(1, 50);
        let rows = if let Some(plan) = plan_id {
            sqlx::query(
                "SELECT plan_id, task_id, pr_number, repo_slug, branch, created_at \
                 FROM plan_pr_links WHERE plan_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(plan)
            .bind(limit)
            .fetch_all(self.pool().inner())
            .await?
        } else if let Some(task) = task_id {
            sqlx::query(
                "SELECT plan_id, task_id, pr_number, repo_slug, branch, created_at \
                 FROM plan_pr_links WHERE task_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(task)
            .bind(limit)
            .fetch_all(self.pool().inner())
            .await?
        } else {
            return Ok(Vec::new());
        };
        rows.into_iter().map(pr_from_row).collect()
    }

    /// Find agents whose most recent heartbeat is older than the
    /// threshold and whose status is not already `terminated`/
    /// `retired`. Returns one [`StaleAgentReport`] per match.
    pub async fn stale_agents(&self, threshold_seconds: i64) -> Result<Vec<StaleAgentReport>> {
        let cutoff =
            (Utc::now() - chrono::Duration::seconds(threshold_seconds.max(0))).to_rfc3339();
        let rows = sqlx::query_as::<_, AgentRow>(&format!(
            "{AGENT_SELECT} WHERE status NOT IN ('terminated', 'retired') \
             AND (last_heartbeat_at IS NULL OR last_heartbeat_at < ?) \
             ORDER BY last_heartbeat_at IS NOT NULL, last_heartbeat_at ASC"
        ))
        .bind(&cutoff)
        .fetch_all(self.pool().inner())
        .await?;
        rows.into_iter()
            .map(|row| {
                let r: AgentRecord = row.try_into()?;
                Ok(StaleAgentReport {
                    agent_id: r.id.clone(),
                    last_heartbeat_at: r.last_heartbeat_at,
                    previous_status: r.status,
                    retired: false,
                })
            })
            .collect()
    }
}
