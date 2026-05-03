//! `/v1/agents/...` — Layer 3 process supervision.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::{AgentRecord, DurabilityError, NewAgent, Task, TaskStatus};
use convergio_lifecycle::{AgentProcess, SpawnSpec};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Mount Layer 3 routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agents/spawn", post(spawn))
        .route("/v1/agents/spawn-runner", post(spawn_runner))
        .route("/v1/agents/:id", get(get_one))
        .route("/v1/agents/:id/heartbeat", post(heartbeat))
}

#[derive(Debug, Deserialize)]
struct SpawnRunnerRequest {
    agent_id: String,
    kind: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: Vec<(String, String)>,
    #[serde(default)]
    plan_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SpawnRunnerResponse {
    agent: AgentRecord,
    process: AgentProcess,
    task: Option<Task>,
}

async fn spawn(
    State(state): State<AppState>,
    Json(spec): Json<SpawnSpec>,
) -> Result<Json<AgentProcess>, ApiError> {
    let proc = state.supervisor.spawn(spec).await?;
    Ok(Json(proc))
}

/// Allow-list of `kind` values accepted by `POST /v1/agents/spawn-runner`.
///
/// All kinds dispatch through the same shell-style supervisor at
/// `convergio-lifecycle::Supervisor::spawn`. The kind label is
/// informational — it shows up in `agent_processes.kind` and in the
/// `runner` field of the agent registry metadata so observers can tell
/// what wrapper produced the process. Per ADR-0028 the convention is
/// to point `command` at `~/.convergio/adapters/<kind>/run.sh` for
/// non-shell kinds; the daemon does not enforce that path and an
/// operator may point elsewhere as long as the binary is local.
const KNOWN_RUNNER_KINDS: &[&str] = &["shell", "claude", "copilot"];

fn runner_label(kind: &str) -> &'static str {
    match kind {
        "claude" => "claude-shell-wrapper",
        "copilot" => "copilot-shell-wrapper",
        // "shell" or any other (unreachable due to validation) — keep
        // the legacy label for backwards compatibility with
        // pre-0.4.0 observers.
        _ => "local-shell",
    }
}

async fn spawn_runner(
    State(state): State<AppState>,
    Json(request): Json<SpawnRunnerRequest>,
) -> Result<Json<SpawnRunnerResponse>, ApiError> {
    if !KNOWN_RUNNER_KINDS.contains(&request.kind.as_str()) {
        return Err(DurabilityError::InvalidAgent {
            reason: format!(
                "unknown runner kind {:?}; expected one of {:?}",
                request.kind, KNOWN_RUNNER_KINDS
            ),
        }
        .into());
    }
    let agent = state
        .durability
        .register_agent(NewAgent {
            id: request.agent_id.clone(),
            kind: request.kind.clone(),
            name: None,
            host: Some("local".into()),
            capabilities: request.capabilities.clone(),
            metadata: json!({"runner": runner_label(&request.kind)}),
        })
        .await?;
    let mut env = request.env;
    env.push(("CONVERGIO_AGENT_ID".into(), request.agent_id.clone()));
    if let Some(plan_id) = &request.plan_id {
        env.push(("CONVERGIO_PLAN_ID".into(), plan_id.clone()));
    }
    if let Some(task_id) = &request.task_id {
        env.push(("CONVERGIO_TASK_ID".into(), task_id.clone()));
    }
    let process = state
        .supervisor
        .spawn(SpawnSpec {
            kind: request.kind,
            command: request.command,
            args: request.args,
            env,
            plan_id: request.plan_id,
            task_id: request.task_id.clone(),
            cwd: None,
            stdin_payload: None,
        })
        .await?;
    let task = match request.task_id {
        Some(task_id) => Some(
            state
                .durability
                .transition_task(&task_id, TaskStatus::InProgress, Some(&request.agent_id))
                .await?,
        ),
        None => None,
    };
    Ok(Json(SpawnRunnerResponse {
        agent,
        process,
        task,
    }))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AgentProcess>, ApiError> {
    Ok(Json(state.supervisor.get(&id).await?))
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.supervisor.heartbeat(&id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
