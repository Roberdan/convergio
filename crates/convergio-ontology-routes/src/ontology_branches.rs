//! `/v1/ontology` branch overlay API.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::{OntologyBranch, OntologyBranchStatus, OntologyResolvedEntry};
use convergio_server_core::ApiError;
use convergio_server_core::AppState;
use serde::Deserialize;

/// Mount ontology branch routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/ontology/branches",
            post(create_branch).get(list_branches),
        )
        .route(
            "/v1/ontology/branches/:id/transition",
            post(transition_branch),
        )
        .route(
            "/v1/ontology/entries/:key",
            get(get_entry).put(put_entry).delete(delete_entry),
        )
}

#[derive(Deserialize)]
struct CreateBranchBody {
    name: String,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize)]
struct TransitionBranchBody {
    target: OntologyBranchStatus,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize, Default)]
struct EntryQuery {
    #[serde(default)]
    branch_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize)]
struct PutEntryBody {
    value: serde_json::Value,
    #[serde(default)]
    agent_id: Option<String>,
}

async fn create_branch(
    State(state): State<AppState>,
    Json(body): Json<CreateBranchBody>,
) -> Result<Json<OntologyBranch>, ApiError> {
    let branch = state
        .durability
        .create_ontology_branch(&body.name, body.agent_id.as_deref())
        .await?;
    Ok(Json(branch))
}

async fn list_branches(
    State(state): State<AppState>,
) -> Result<Json<Vec<OntologyBranch>>, ApiError> {
    Ok(Json(state.durability.list_ontology_branches().await?))
}

async fn transition_branch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TransitionBranchBody>,
) -> Result<Json<OntologyBranch>, ApiError> {
    let branch = state
        .durability
        .transition_ontology_branch(&id, body.target, body.agent_id.as_deref())
        .await?;
    Ok(Json(branch))
}

async fn get_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<EntryQuery>,
) -> Result<Json<OntologyResolvedEntry>, ApiError> {
    let entry = state
        .durability
        .resolve_ontology_entry(&key, query.branch_id.as_deref())
        .await?;
    Ok(Json(entry))
}

async fn put_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<EntryQuery>,
    Json(body): Json<PutEntryBody>,
) -> Result<Json<OntologyResolvedEntry>, ApiError> {
    state
        .durability
        .set_ontology_entry(
            &key,
            body.value,
            query.branch_id.as_deref(),
            body.agent_id.as_deref(),
        )
        .await?;
    get_entry(State(state), Path(key), Query(query)).await
}

async fn delete_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<EntryQuery>,
) -> Result<Json<OntologyResolvedEntry>, ApiError> {
    state
        .durability
        .delete_ontology_entry(&key, query.branch_id.as_deref(), query.agent_id.as_deref())
        .await?;
    get_entry(State(state), Path(key), Query(query)).await
}
