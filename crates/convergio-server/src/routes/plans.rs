//! `/v1/plans/...` — create, list, get.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use convergio_durability::{NewPlan, NewPlanPrLink, Plan, PlanStatus, Task};
use serde::Deserialize;

/// Mount `/v1/plans` routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/plans", post(create).get(list))
        .route("/v1/plans/:id", get(by_id).patch(rename))
        .route("/v1/plans/:id/transition", post(transition))
        .route("/v1/plans/:id/triage", get(triage))
        .route("/v1/plans/:id/pr-links", post(add_pr_link))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewPlan>,
) -> Result<Json<Plan>, ApiError> {
    let plan = state.durability.create_plan(body).await?;
    Ok(Json(plan))
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Plan>>, ApiError> {
    let plans = state.durability.plans().list(q.limit).await?;
    Ok(Json(plans))
}

async fn by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Plan>, ApiError> {
    // Accept either a plain integer plan number or a UUID.
    if let Ok(num) = id.parse::<i64>() {
        let plan = state
            .durability
            .plans()
            .find_by_number(num)
            .await?
            .ok_or_else(|| {
                crate::error::ApiError::Durability(
                    convergio_durability::DurabilityError::NotFound {
                        entity: "plan",
                        id: id.clone(),
                    },
                )
            })?;
        return Ok(Json(plan));
    }
    let plan = state.durability.plans().get(&id).await?;
    Ok(Json(plan))
}

#[derive(Deserialize)]
struct RenameBody {
    title: String,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn rename(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RenameBody>,
) -> Result<Json<Plan>, ApiError> {
    let plan = state
        .durability
        .rename_plan(&id, &body.title, body.agent_id.as_deref())
        .await?;
    Ok(Json(plan))
}

#[derive(Deserialize)]
struct TransitionBody {
    target: PlanStatus,
}

async fn transition(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TransitionBody>,
) -> Result<Json<Plan>, ApiError> {
    let plan = state.durability.transition_plan(&id, body.target).await?;
    Ok(Json(plan))
}

#[derive(Deserialize)]
struct TriageQuery {
    #[serde(default = "default_stale_days")]
    stale_days: i64,
}

fn default_stale_days() -> i64 {
    7
}

async fn triage(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TriageQuery>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let before = Utc::now() - Duration::days(q.stale_days);
    let tasks = state
        .durability
        .tasks()
        .list_stale_by_plan(&id, before)
        .await?;
    Ok(Json(tasks))
}

/// Body for `POST /v1/plans/:id/pr-links`.
#[derive(Deserialize)]
struct AddPrLinkBody {
    pr_number: i64,
    repo_slug: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn add_pr_link(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Json(body): Json<AddPrLinkBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .durability
        .plan_pr_links()
        .add(NewPlanPrLink {
            plan_id: plan_id.clone(),
            task_id: body.task_id.clone(),
            pr_number: body.pr_number,
            repo_slug: body.repo_slug.clone(),
            branch: body.branch.clone(),
            agent_id: body.agent_id.clone(),
        })
        .await?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "plan_id": plan_id,
        "pr_number": body.pr_number,
        "repo_slug": body.repo_slug,
        "agent_id": body.agent_id,
    })))
}
