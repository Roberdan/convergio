//! `/v1/fleet/repos` and `/v1/fleet/build` — fleet management (ADR-0038, F2-6/F2-7/F2-8).
//!
//! Routes:
//! - `POST   /v1/fleet/repos`        — add a repo to the fleet
//! - `GET    /v1/fleet/repos`        — list all fleet repos
//! - `PATCH  /v1/fleet/repos/:name`  — enable / disable a repo
//! - `POST   /v1/fleet/build`        — parse + embed all enabled repos (idempotent)

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{patch, post};
use axum::{Json, Router};
use convergio_embed::{collect_files, ingest, DEFAULT_MAX_LINES};
use convergio_fleet::{run_similarity_batch, FleetRepo, RepoEntry, RepoRole};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Mount the fleet routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/fleet/repos", post(add).get(list))
        .route("/v1/fleet/repos/:name", patch(update))
        .route("/v1/fleet/build", post(build))
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

#[derive(Debug, Deserialize, Default)]
struct BuildBody {
    /// When `true`, recompute cross-repo `similar_to` / `duplicates` edges after ingestion.
    #[serde(default)]
    refresh_similarity: bool,
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

/// `POST /v1/fleet/build` — idempotent parse + embed across all enabled repos.
///
/// For each enabled repo: collect files by language, run the embedder, and
/// stamp `last_built_at`. With `refresh_similarity=true`, also recomputes
/// cross-repo cosine similarity edges (ADR-0038, F2-7).
async fn build(
    State(state): State<AppState>,
    Json(body): Json<BuildBody>,
) -> Result<Json<Value>, ApiError> {
    let repos = state.fleet.list_repos().await.map_err(ApiError::Fleet)?;
    let enabled: Vec<_> = repos.into_iter().filter(|r| r.enabled).collect();

    let (mut considered, mut embedded, mut skipped, mut failed, mut skipped_repos) =
        (0usize, 0usize, 0usize, 0usize, 0usize);

    for repo in &enabled {
        let root = PathBuf::from(&repo.path);
        if !root.is_dir() {
            tracing::warn!(repo = %repo.name, path = %repo.path, "path not found; skipping");
            skipped_repos += 1;
            continue;
        }
        let exts = lang_extensions(&repo.language);
        let nodes = collect_files(&repo.name, &root, exts, DEFAULT_MAX_LINES);
        let report = ingest(&state.embed, state.embedder.as_ref(), nodes)
            .await
            .map_err(|e| ApiError::Internal(format!("ingest failed for {}: {e}", repo.name)))?;
        considered += report.considered;
        embedded += report.embedded;
        skipped += report.skipped_unchanged;
        failed += report.failed;
        state
            .fleet
            .mark_built(&repo.name)
            .await
            .map_err(ApiError::Fleet)?;
        tracing::info!(repo = %repo.name, embedded = report.embedded, "fleet build: repo done");
    }

    let batch_report = if body.refresh_similarity {
        let model = state.embedder.model_id();
        run_similarity_batch(&state.embed, &state.fleet, model)
            .await
            .map_err(|e| ApiError::Internal(format!("similarity batch failed: {e}")))?
    } else {
        convergio_fleet::BatchReport::default()
    };

    Ok(Json(json!({
        "ok": true,
        "repos_processed": enabled.len() - skipped_repos,
        "repos_skipped": skipped_repos,
        "model": state.embedder.model_id(),
        "embed": {
            "considered": considered,
            "embedded": embedded,
            "skipped_unchanged": skipped,
            "failed": failed,
        },
        "similarity": {
            "pairs_checked": batch_report.pairs_checked,
            "similar_to": batch_report.similar_to,
            "duplicates": batch_report.duplicates,
        },
    })))
}

/// File extensions to embed for a given primary language.
fn lang_extensions(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => &["rs"],
        "typescript" | "javascript" => &["ts", "tsx", "js", "jsx"],
        "python" => &["py"],
        "markdown" | "docs" => &["md"],
        _ => convergio_embed::SOURCE_EXTENSIONS,
    }
}
