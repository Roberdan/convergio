//! `/v1/fleet/repos` — fleet repo management (ADR-0038, F2-6).
//!
//! Routes:
//! - `POST   /v1/fleet/repos`        — add a repo to the fleet
//! - `GET    /v1/fleet/repos`        — list all fleet repos
//! - `PATCH  /v1/fleet/repos/:name`  — enable / disable a repo

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{patch, post};
use axum::{Json, Router};
use convergio_fleet::{FleetRepo, RepoEntry, RepoRole};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Mount the fleet routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/fleet/repos", post(add).get(list))
        .route("/v1/fleet/repos/:name", patch(update))
}

#[derive(Debug, Deserialize)]
struct AddRequest {
    /// Short slug — unique identifier for this repo in the fleet.
    name: String,
    /// Absolute path on disk.
    path: String,
    /// Primary language (e.g. "rust", "typescript").
    language: String,
    /// Parser backend ("syn" or "tree-sitter").
    #[serde(default = "default_parser")]
    parser: String,
    /// Role in the fleet (defaults to "downstream").
    #[serde(default)]
    role: Option<String>,
    /// Parent repo this one derives from (read from convergio.yaml by CLI).
    #[serde(default)]
    derives_from: Option<String>,
}

fn default_parser() -> String {
    "tree-sitter".to_owned()
}

#[derive(Debug, Deserialize)]
struct UpdateRequest {
    /// Set `true` to enable, `false` to disable.
    enabled: Option<bool>,
}

/// Shape returned for every fleet repo.
#[derive(Debug, Serialize)]
struct RepoResponse {
    /// Short slug.
    name: String,
    /// Absolute path on disk.
    path: String,
    /// Primary language.
    language: String,
    /// Parser backend.
    parser: String,
    /// Role string (engine | library | downstream | sandbox).
    role: String,
    /// Parent repo name, if any.
    derives_from: Option<String>,
    /// ISO-8601 timestamp of last graph build, if any.
    last_built_at: Option<String>,
    /// Whether the repo is active.
    enabled: bool,
    /// Fraction of files with stored embeddings (F3 placeholder).
    embed_coverage: Option<f64>,
}

fn to_response(r: FleetRepo) -> RepoResponse {
    RepoResponse {
        name: r.name,
        path: r.path,
        language: r.language,
        parser: r.parser,
        role: r.role,
        derives_from: r.derives_from,
        last_built_at: r.last_built_at,
        enabled: r.enabled,
        embed_coverage: None,
    }
}

/// `POST /v1/fleet/repos` — register a new repo.
async fn add(
    State(state): State<AppState>,
    Json(req): Json<AddRequest>,
) -> Result<Json<Value>, ApiError> {
    let role: RepoRole = req
        .role
        .as_deref()
        .unwrap_or("downstream")
        .parse()
        .map_err(|msg: String| ApiError::BadRequest {
            code: "invalid_role",
            message: msg,
        })?;

    let entry = RepoEntry {
        name: req.name.clone(),
        path: req.path,
        language: req.language,
        parser: req.parser,
        role,
        derives_from: req.derives_from,
    };

    state
        .fleet
        .add_repo(&entry)
        .await
        .map_err(ApiError::Fleet)?;
    let repo = state
        .fleet
        .get_repo(&req.name)
        .await
        .map_err(ApiError::Fleet)?;
    serde_json::to_value(to_response(repo))
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

/// `GET /v1/fleet/repos` — list all repos (enabled and disabled).
async fn list(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let repos = state.fleet.list_repos().await.map_err(ApiError::Fleet)?;
    let items: Vec<RepoResponse> = repos.into_iter().map(to_response).collect();
    Ok(Json(json!({ "repos": items })))
}

/// `PATCH /v1/fleet/repos/:name` — toggle enabled/disabled.
async fn update(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(enabled) = req.enabled {
        state
            .fleet
            .set_enabled(&name, enabled)
            .await
            .map_err(ApiError::Fleet)?;
    }
    let repo = state.fleet.get_repo(&name).await.map_err(ApiError::Fleet)?;
    serde_json::to_value(to_response(repo))
        .map(Json)
        .map_err(|e| ApiError::Internal(e.to_string()))
}
