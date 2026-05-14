//! Task-scoped context packet generation.

use crate::app::AppState;
use crate::error::ApiError;
use crate::routes::limits::validate_message_limit as validate_limit;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use convergio_bus::Message;
use convergio_durability::{AgentRecord, DurabilityError, Evidence, Plan, Task};
use serde::{Deserialize, Serialize};
use std::path::{Path as FsPath, PathBuf};

const MAX_AGENT_DOC_CHARS: usize = 8_000;

/// Mount context packet routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/tasks/:id/context", post(task_context))
}

#[derive(Debug, Deserialize)]
struct ContextRequest {
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default)]
    message_topic: Option<String>,
    #[serde(default)]
    message_cursor: Option<i64>,
    #[serde(default = "default_message_limit")]
    message_limit: i64,
}

#[derive(Debug, Serialize)]
struct ContextPacket {
    schema_version: &'static str,
    plan: Plan,
    task: Task,
    evidence: Vec<Evidence>,
    messages: Vec<Message>,
    agents: Vec<AgentRecord>,
    agent_instructions: Vec<AgentInstruction>,
}

#[derive(Debug, Serialize)]
struct AgentInstruction {
    path: String,
    content: String,
}

async fn task_context(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(body): Json<ContextRequest>,
) -> Result<Json<ContextPacket>, ApiError> {
    let task = state.durability.tasks().get(&task_id).await?;
    let plan = state.durability.plans().get(&task.plan_id).await?;
    let evidence = state.durability.evidence().list_by_task(&task_id).await?;
    let message_limit = validate_message_limit(body.message_limit)?;
    let topic = body
        .message_topic
        .unwrap_or_else(|| format!("task:{task_id}"));
    let messages = state
        .bus
        .poll(
            &task.plan_id,
            &topic,
            body.message_cursor.unwrap_or(0),
            message_limit,
        )
        .await?;
    let agents = state.durability.agents().list().await?;
    let agent_instructions = match body.workspace_path {
        Some(path) => agent_docs(&path)?,
        None => Vec::new(),
    };

    Ok(Json(ContextPacket {
        schema_version: "1",
        plan,
        task,
        evidence,
        messages,
        agents,
        agent_instructions,
    }))
}

fn default_message_limit() -> i64 {
    20
}

fn validate_message_limit(limit: i64) -> Result<i64, ApiError> {
    validate_limit(limit, "invalid_context_limit", "message_limit")
}

fn agent_docs(path: &str) -> Result<Vec<AgentInstruction>, ApiError> {
    let input = PathBuf::from(path);
    if !input.exists() {
        return Err(DurabilityError::NotFound {
            entity: "workspace_path",
            id: path.into(),
        }
        .into());
    }
    let start = if input.is_dir() {
        input.as_path()
    } else {
        input.parent().unwrap_or_else(|| FsPath::new("."))
    };
    let mut docs = Vec::new();
    for dir in start.ancestors() {
        let candidate = dir.join("AGENTS.md");
        if candidate.is_file() {
            let content = read_agent_doc(&candidate)?;
            docs.push(AgentInstruction {
                path: candidate.display().to_string(),
                content,
            });
        }
    }
    Ok(docs)
}

fn read_agent_doc(path: &FsPath) -> Result<String, ApiError> {
    let content = std::fs::read_to_string(path).map_err(|_| DurabilityError::NotFound {
        entity: "agent_doc",
        id: path.display().to_string(),
    })?;
    Ok(content.chars().take(MAX_AGENT_DOC_CHARS).collect())
}
