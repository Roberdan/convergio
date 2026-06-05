//! `/v1/plans/:plan_id/tasks` and `/v1/tasks/:id/transition`.

use crate::app::AppState;
use crate::error::ApiError;
use crate::routes::bitemporal::BitemporalQuery;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::{NewTask, Task, TaskStatus};
use serde::Deserialize;

/// Mount task routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/plans/:plan_id/tasks", post(create).get(list))
        .route("/v1/tasks/:id", get(by_id))
        .route("/v1/tasks/:id/transition", post(transition))
        .route("/v1/tasks/:id/retry", post(retry))
        .route("/v1/tasks/:id/close-post-hoc", post(close_post_hoc))
        .route("/v1/tasks/:id/heartbeat", post(heartbeat))
}

#[derive(Deserialize)]
struct TransitionBody {
    target: TaskStatus,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize, Default)]
struct RetryBody {
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize)]
struct ClosePostHocBody {
    reason: String,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Json(body): Json<NewTask>,
) -> Result<Json<Task>, ApiError> {
    let task = state.durability.create_task(&plan_id, body).await?;
    Ok(Json(task))
}

async fn list(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Query(bt): Query<BitemporalQuery>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let _ = bt.parse()?;
    let tasks = state.durability.tasks().list_by_plan(&plan_id).await?;
    Ok(Json(tasks))
}

async fn by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(bt): Query<BitemporalQuery>,
) -> Result<Json<Task>, ApiError> {
    let _ = bt.parse()?;
    let task = state.durability.tasks().get(&id).await?;
    Ok(Json(task))
}

async fn transition(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TransitionBody>,
) -> Result<Json<Task>, ApiError> {
    let task = state
        .durability
        .transition_task(&id, body.target, body.agent_id.as_deref())
        .await?;
    Ok(Json(task))
}

async fn retry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<RetryBody>>,
) -> Result<Json<Task>, ApiError> {
    let agent_id = body.and_then(|Json(b)| b.agent_id);
    let task = state
        .durability
        .retry_task(&id, agent_id.as_deref())
        .await?;
    Ok(Json(task))
}

async fn close_post_hoc(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ClosePostHocBody>,
) -> Result<Json<Task>, ApiError> {
    let task = state
        .durability
        .close_task_post_hoc(&id, &body.reason, body.agent_id.as_deref())
        .await?;
    Ok(Json(task))
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.durability.tasks().heartbeat(&id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
