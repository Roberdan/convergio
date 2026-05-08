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
use convergio_runner::RunnerRegistry;
use serde_json::{json, Value};

/// Mount the dispatch route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/dispatch", post(dispatch))
}

async fn dispatch(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let registry = RunnerRegistry::load_default().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "failed to load runner registry; custom vendors disabled");
        RunnerRegistry::empty()
    });

    let mut exec = Executor::new(
        (*state.durability).clone(),
        (*state.supervisor).clone(),
        SpawnTemplate::default(),
    )
    .with_defaults(RunnerDefaults::from_env())
    .with_graph((*state.graph).clone())
    .with_registry(registry);

    if let Some(repo_path) = crate::resolve_repo_path() {
        exec = exec.with_repo_path(repo_path);
    }

    let n = exec.tick().await.map_err(map_exec)?;
    Ok(Json(json!({"dispatched": n})))
}

fn map_exec(e: convergio_executor::ExecutorError) -> ApiError {
    match e {
        convergio_executor::ExecutorError::Durability(d) => ApiError::Durability(d),
        convergio_executor::ExecutorError::Lifecycle(l) => ApiError::Lifecycle(l),
        convergio_executor::ExecutorError::Runner(r) => ApiError::BadRequest {
            code: "runner_error",
            message: r.to_string(),
        },
        convergio_executor::ExecutorError::Worktree(m) => ApiError::BadRequest {
            code: "worktree_error",
            message: m,
        },
    }
}
