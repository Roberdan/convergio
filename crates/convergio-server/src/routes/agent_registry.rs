//! `/v1/agent-registry/*` durable agent identity routes.
//!
//! The `details` aggregator (`/v1/agent-registry/agents/:id/details`)
//! composes pieces from [`convergio_durability::AgentStore`] —
//! summary, current task plan meta, leases, recent audit, recent
//! PRs — and exposes them as [`AgentDetails`]. The aggregation
//! lives here (server-side) so the durability crate stays under
//! its context-budget cap; the underlying SQL projections still
//! live in `convergio_durability::store::agent_queries`.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use convergio_bus::AgentLastTopic;
use convergio_durability::{
    AgentAuditEntry, AgentHeartbeat, AgentLease, AgentPrLink, AgentRecord, AgentSummary,
    ClaimedTasks, NewAgent, RetireStaleResult,
};
use serde::{Deserialize, Serialize};

/// Rich `cvg agent show` payload — base summary plus current task
/// plan/title, active workspace leases, recent audit and recent
/// PR links.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentDetails {
    #[serde(flatten)]
    pub summary: AgentSummary,
    /// Tasks still owned by the agent (helps when `current_task_id` is missing).
    pub claimed_tasks: ClaimedTasks,
    /// Most recent bus topic activity involving this agent.
    pub last_topic: Option<AgentLastTopic>,
    pub current_task_plan_id: Option<String>,
    pub current_task_plan_title: Option<String>,
    pub current_task_started_at: Option<DateTime<Utc>>,
    pub leases: Vec<AgentLease>,
    pub recent_audit: Vec<AgentAuditEntry>,
    pub recent_prs: Vec<AgentPrLink>,
}

/// Enriched `cvg agent list` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentSummaryEnriched {
    #[serde(flatten)]
    pub summary: AgentSummary,
    pub claimed_tasks: ClaimedTasks,
    pub last_topic: Option<AgentLastTopic>,
}

/// Mount agent registry routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agent-registry/agents", get(list).post(register))
        .route("/v1/agent-registry/agents/summaries", get(summaries))
        .route("/v1/agent-registry/agents/retire-stale", post(retire_stale))
        .route("/v1/agent-registry/agents/:id", get(get_one))
        .route("/v1/agent-registry/agents/:id/details", get(details))
        .route("/v1/agent-registry/agents/:id/heartbeat", post(heartbeat))
        .route("/v1/agent-registry/agents/:id/retire", post(retire))
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<NewAgent>,
) -> Result<Json<AgentRecord>, ApiError> {
    Ok(Json(state.durability.register_agent(body).await?))
}

#[derive(Debug, Default, Deserialize)]
struct AgentListQuery {
    /// Filter by canonical agent status (`working`, `idle`, `ready`, `terminated`, …).
    status: Option<String>,
    /// Maximum number of rows to return. Caps at 1000 server-side.
    limit: Option<i64>,
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<AgentListQuery>,
) -> Result<Json<Vec<AgentRecord>>, ApiError> {
    let limit = q.limit.map(|n| n.clamp(1, 1000));
    Ok(Json(
        state
            .durability
            .agents()
            .list_filtered(q.status.as_deref(), limit)
            .await?,
    ))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AgentRecord>, ApiError> {
    Ok(Json(state.durability.agents().get(&id).await?))
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AgentHeartbeat>,
) -> Result<Json<AgentRecord>, ApiError> {
    Ok(Json(state.durability.heartbeat_agent(&id, body).await?))
}

async fn retire(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AgentRecord>, ApiError> {
    Ok(Json(state.durability.retire_agent(&id).await?))
}

async fn summaries(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentSummaryEnriched>>, ApiError> {
    let store = state.durability.agents();
    let rows = store.summaries().await?;
    let mut out = Vec::with_capacity(rows.len());
    for summary in rows {
        let claimed_tasks = store.claimed_tasks_for_agent(&summary.agent.id, 3).await?;
        let last_topic = state.bus.last_topic_for_agent(&summary.agent.id).await?;
        out.push(AgentSummaryEnriched {
            summary,
            claimed_tasks,
            last_topic,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct DetailsQuery {
    #[serde(default = "default_audit_limit")]
    audit_limit: i64,
    #[serde(default = "default_pr_limit")]
    pr_limit: i64,
}

fn default_audit_limit() -> i64 {
    10
}
fn default_pr_limit() -> i64 {
    5
}

async fn details(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DetailsQuery>,
) -> Result<Json<AgentDetails>, ApiError> {
    let store = state.durability.agents();
    let summary = store.summary(&id).await?;
    let task_id = summary.agent.current_task_id.clone();
    let meta = store.current_task_meta(task_id.as_deref()).await?;
    let plan_id = meta.as_ref().map(|m| m.plan_id.clone());
    let leases = store.leases_for_agent(&id).await?;
    let recent_audit = store.recent_audit_for_agent(&id, q.audit_limit).await?;
    let recent_prs = store
        .recent_prs(plan_id.as_deref(), task_id.as_deref(), q.pr_limit)
        .await?;
    let claimed_tasks = store.claimed_tasks_for_agent(&id, 20).await?;
    let last_topic = state.bus.last_topic_for_agent(&id).await?;
    Ok(Json(AgentDetails {
        summary,
        claimed_tasks,
        last_topic,
        current_task_plan_id: meta.as_ref().map(|m| m.plan_id.clone()),
        current_task_plan_title: meta.as_ref().map(|m| m.plan_title.clone()),
        current_task_started_at: meta.and_then(|m| m.started_at),
        leases,
        recent_audit,
        recent_prs,
    }))
}

#[derive(Deserialize, Default)]
struct RetireStaleBody {
    #[serde(default = "default_retire_threshold")]
    threshold_seconds: i64,
    #[serde(default)]
    apply: bool,
}

fn default_retire_threshold() -> i64 {
    3600
}

async fn retire_stale(
    State(state): State<AppState>,
    body: Option<Json<RetireStaleBody>>,
) -> Result<Json<RetireStaleResult>, ApiError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let stale = state
        .durability
        .agents()
        .stale_agents(body.threshold_seconds)
        .await?;
    let mut agents = Vec::with_capacity(stale.len());
    for mut entry in stale {
        if body.apply {
            let record = state.durability.retire_agent(&entry.agent_id).await?;
            state
                .durability
                .audit()
                .append(
                    convergio_durability::audit::EntityKind::Agent,
                    &record.id,
                    "agent.retired_stale",
                    &serde_json::json!({
                        "agent_id": record.id,
                        "last_heartbeat_at": entry.last_heartbeat_at,
                        "threshold_seconds": body.threshold_seconds,
                        "reason": "stale_heartbeat",
                    }),
                    Some(&record.id),
                )
                .await?;
            entry.retired = true;
        }
        agents.push(entry);
    }
    Ok(Json(RetireStaleResult {
        threshold_seconds: body.threshold_seconds,
        applied: body.apply,
        agents,
    }))
}
