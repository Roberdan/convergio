//! Fleet plan routes (ADR-0038, F3-2).
//!
//! Routes:
//! - `POST   /v1/fleet/plans`                                — create
//! - `GET    /v1/fleet/plans`                                — list
//! - `GET    /v1/fleet/plans/:id`                            — show + links
//! - `POST   /v1/fleet/plans/:id/repos`                      — link a repo
//! - `POST   /v1/fleet/plans/:id/repos/:repo/tasks`          — add per-repo task
//!
//! Status rollup is derived in `show` from the durability layer — no
//! aggregate column on `fleet_plans`. The link insertion is
//! idempotent on `(fleet_plan_id, repo)`.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::NewTask;
use convergio_fleet::{FleetError, FleetPlanRepoLink, FleetPlanView, NewFleetPlan};
use serde::Deserialize;
use serde_json::{json, Value};

/// Mount the fleet-plan routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/fleet/plans", post(create).get(list))
        .route("/v1/fleet/plans/:id", get(show))
        .route("/v1/fleet/plans/:id/repos", post(link_repo))
        .route("/v1/fleet/plans/:id/repos/:repo/tasks", post(add_task))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<NewFleetPlan>,
) -> Result<Json<Value>, ApiError> {
    let plan = state.fleet_plans.create(input).await?;
    Ok(Json(json!(plan)))
}

#[derive(Deserialize)]
struct ListQuery {
    scope: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<convergio_fleet::FleetPlan>>, ApiError> {
    let plans = state.fleet_plans.list(q.scope.as_deref()).await?;
    Ok(Json(plans))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FleetPlanView>, ApiError> {
    Ok(Json(state.fleet_plans.show(&id).await?))
}

#[derive(Deserialize)]
struct LinkRepoInput {
    /// Repo name (matches `fleet_repos.name`).
    repo: String,
    /// Per-repo plan id in `convergio-durability`.
    repo_plan_id: String,
}

async fn link_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<LinkRepoInput>,
) -> Result<Json<Value>, ApiError> {
    // The fleet plan must exist; surface 404 here rather than letting
    // the link table accept a parent-less link (no FK to fleet_plans
    // on the row schema; the integrity check lives at this layer).
    state.fleet_plans.get(&id).await?;
    // The per-repo plan must exist in convergio-durability. Without
    // this check a typo would produce a dangling link that only
    // surfaces later when `add-task` tries to create the task on a
    // non-existent plan. fleet_plan_repos has no FK to plans, so
    // validate here.
    state.durability.plans().get(&input.repo_plan_id).await?;
    let link = FleetPlanRepoLink {
        fleet_plan_id: id,
        repo: input.repo,
        repo_plan_id: input.repo_plan_id,
    };
    state.fleet_plans.link_repo(&link).await?;
    Ok(Json(json!(link)))
}

#[derive(Deserialize)]
struct AddTaskInput {
    /// Task title.
    title: String,
    /// Optional task description.
    #[serde(default)]
    description: Option<String>,
    /// Wave (defaults to 1).
    #[serde(default = "default_wave")]
    wave: i64,
    /// Sequence within the wave (defaults to next).
    #[serde(default = "default_seq")]
    sequence: i64,
    /// Evidence kinds required for the task to submit.
    #[serde(default)]
    evidence_required: Vec<String>,
}

fn default_wave() -> i64 {
    1
}
fn default_seq() -> i64 {
    1
}

async fn add_task(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
    Json(input): Json<AddTaskInput>,
) -> Result<Json<Value>, ApiError> {
    // Resolve the per-repo plan id from the fleet-plan links.
    let links = state.fleet_plans.links(&id).await?;
    let link = links.iter().find(|l| l.repo == repo).ok_or_else(|| {
        ApiError::Fleet(FleetError::NotFound(format!(
            "repo '{repo}' not linked to fleet plan '{id}'"
        )))
    })?;
    let task = state
        .durability
        .create_task(
            &link.repo_plan_id,
            NewTask {
                title: input.title,
                description: input.description,
                wave: input.wave,
                sequence: input.sequence,
                evidence_required: input.evidence_required,
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await?;
    Ok(Json(json!({
        "fleet_plan_id": id,
        "repo": repo,
        "repo_plan_id": link.repo_plan_id,
        "task": task,
    })))
}
