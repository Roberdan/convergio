//! `POST /v1/dispatch` — one executor tick.
//!
//! In the MVP the executor loop runs in the background; this endpoint
//! exposes a manual tick for tests, CLI smoke and ops.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use convergio_executor::{Executor, RunnerDefaults, SpawnTemplate};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Default, Deserialize)]
struct DispatchRequest {
    #[serde(default)]
    no_dispatch: bool,
    #[serde(default)]
    executor: Option<String>,
    #[serde(default)]
    repo: Option<String>,
}

/// Mount the dispatch route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/dispatch", post(dispatch))
}

async fn dispatch(
    State(state): State<AppState>,
    body: Option<Json<DispatchRequest>>,
) -> Result<Json<Value>, ApiError> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    let executor = body.executor.as_deref().unwrap_or("default");
    if body.no_dispatch || executor == "none" {
        return Ok(Json(json!({
            "dispatched": 0,
            "executor": "none",
            "tracker_only": true
        })));
    }
    if executor != "default" {
        return Err(ApiError::BadRequest {
            code: "unknown_executor",
            message: format!("unknown executor mode: {executor}"),
        });
    }
    let mut exec = Executor::new(
        (*state.durability).clone(),
        (*state.supervisor).clone(),
        SpawnTemplate::default(),
    )
    .with_defaults(RunnerDefaults::from_env());
    if let Some(repo) = body.repo.as_deref() {
        exec = exec.with_repo_path(std::path::PathBuf::from(
            state.fleet.get_repo(repo).await?.path,
        ));
    } else if let Some(p) = std::env::var_os("CONVERGIO_REPO_PATH") {
        exec = exec.with_repo_path(std::path::PathBuf::from(p));
    }
    let n = exec.tick().await.map_err(map_exec)?;
    Ok(Json(json!({"dispatched": n, "executor": "default"})))
}

fn map_exec(e: convergio_executor::ExecutorError) -> ApiError {
    match e {
        convergio_executor::ExecutorError::Durability(d) => ApiError::Durability(d),
        convergio_executor::ExecutorError::Lifecycle(l) => ApiError::Lifecycle(l),
        convergio_executor::ExecutorError::Runner(r) => ApiError::BadRequest {
            code: "runner_error",
            message: r.to_string(),
        },
        convergio_executor::ExecutorError::Worktree(msg) => ApiError::BadRequest {
            code: "worktree_error",
            message: msg,
        },
    }
}
