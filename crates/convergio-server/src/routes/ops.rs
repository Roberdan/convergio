//! `/v1/ops/...` — workflow engine core routes.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_ops::{OpsWorkflow, OpsWorkflowInstance, WorkflowSpec};
use serde::Deserialize;
use serde_json::Value;

/// Mount ops workflow engine routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ops/workflows", post(create_workflow))
        .route("/v1/ops/workflows/:id", get(get_workflow))
        .route("/v1/ops/workflows/by-key/:key", get(get_workflow_by_key))
        .route(
            "/v1/ops/workflows/:id/versions",
            post(append_workflow_version),
        )
        .route("/v1/ops/instances", post(start_instance))
        .route("/v1/ops/instances/:id", get(get_instance))
        .route("/v1/ops/instances/:id/tick", post(tick_instance))
        .route(
            "/v1/ops/instances/:id/work-items/:work_item_id/complete",
            post(complete_work_item),
        )
        .route("/v1/ops/instances/:id/cancel", post(cancel_instance))
}

#[derive(Deserialize)]
struct CreateWorkflowBody {
    workflow_key: String,
    spec: WorkflowSpec,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn create_workflow(
    State(state): State<AppState>,
    Json(body): Json<CreateWorkflowBody>,
) -> Result<Json<OpsWorkflow>, ApiError> {
    let wf = state
        .ops
        .create_workflow(&body.workflow_key, &body.spec, body.agent_id.as_deref())
        .await?;
    Ok(Json(wf))
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OpsWorkflow>, ApiError> {
    let wf = state.ops.workflows().get_current(&id).await?;
    Ok(Json(wf))
}

async fn get_workflow_by_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<OpsWorkflow>, ApiError> {
    let wf = state.ops.workflows().get_current_by_key(&key).await?;
    Ok(Json(wf))
}

#[derive(Deserialize)]
struct AppendWorkflowVersionBody {
    spec: WorkflowSpec,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn append_workflow_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AppendWorkflowVersionBody>,
) -> Result<Json<OpsWorkflow>, ApiError> {
    let wf = state
        .ops
        .append_workflow_version(&id, &body.spec, body.agent_id.as_deref())
        .await?;
    Ok(Json(wf))
}

#[derive(Deserialize)]
struct StartInstanceBody {
    workflow_id: String,
    #[serde(default)]
    workflow_version: Option<i64>,
    #[serde(default)]
    context: Value,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn start_instance(
    State(state): State<AppState>,
    Json(body): Json<StartInstanceBody>,
) -> Result<Json<OpsWorkflowInstance>, ApiError> {
    let inst = state
        .ops
        .start_instance(
            &body.workflow_id,
            body.workflow_version,
            body.context,
            body.agent_id.as_deref(),
        )
        .await?;
    Ok(Json(inst))
}

async fn get_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OpsWorkflowInstance>, ApiError> {
    let inst = state.ops.instances().get_current(&id).await?;
    Ok(Json(inst))
}

#[derive(Deserialize, Default)]
struct TickBody {
    #[serde(default)]
    agent_id: Option<String>,
}

async fn tick_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<TickBody>>,
) -> Result<Json<OpsWorkflowInstance>, ApiError> {
    let agent_id = body.and_then(|Json(b)| b.agent_id);
    let inst = state.ops.tick_instance(&id, agent_id.as_deref()).await?;
    Ok(Json(inst))
}

#[derive(Deserialize)]
struct CompleteWorkItemBody {
    success: bool,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn complete_work_item(
    State(state): State<AppState>,
    Path((id, work_item_id)): Path<(String, String)>,
    Json(body): Json<CompleteWorkItemBody>,
) -> Result<Json<OpsWorkflowInstance>, ApiError> {
    let inst = state
        .ops
        .complete_work_item(&id, &work_item_id, body.success, body.agent_id.as_deref())
        .await?;
    Ok(Json(inst))
}

#[derive(Deserialize, Default)]
struct CancelBody {
    #[serde(default)]
    agent_id: Option<String>,
}

async fn cancel_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<CancelBody>>,
) -> Result<Json<OpsWorkflowInstance>, ApiError> {
    let agent_id = body.and_then(|Json(b)| b.agent_id);
    let inst = state.ops.cancel_instance(&id, agent_id.as_deref()).await?;
    Ok(Json(inst))
}
