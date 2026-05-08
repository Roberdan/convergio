//! Wire/data types rendered by the dashboard.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Plan summary, matching the daemon's `/v1/plans` response shape.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Plan {
    /// Plan id.
    pub id: String,
    /// Plan title shown in the Plans pane.
    pub title: String,
    /// Optional long description.
    #[serde(default)]
    pub description: Option<String>,
    /// Project label (`convergio`, `convergio-local`, ...).
    #[serde(default)]
    pub project: Option<String>,
    /// Plan status (`draft`, `active`, `completed`, ...).
    pub status: String,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
    /// Last-updated timestamp (RFC3339).
    pub updated_at: String,
    /// First active timestamp (RFC3339).
    #[serde(default)]
    pub started_at: Option<String>,
    /// Completed/cancelled timestamp (RFC3339).
    #[serde(default)]
    pub ended_at: Option<String>,
    /// Duration between start and end, in milliseconds.
    #[serde(default)]
    pub duration_ms: Option<i64>,
}

/// One task as displayed in the dashboard.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TaskSummary {
    /// Task id.
    pub id: String,
    /// Owning plan id.
    pub plan_id: String,
    /// Short title.
    pub title: String,
    /// Optional details.
    #[serde(default)]
    pub description: Option<String>,
    /// Status (`pending`, `in_progress`, `submitted`, `done`, `failed`).
    pub status: String,
    /// Parallel wave.
    #[serde(default)]
    pub wave: i64,
    /// Sequence in the wave.
    #[serde(default)]
    pub sequence: i64,
    /// Optional agent id that claimed the task.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
    /// First in-progress timestamp.
    #[serde(default)]
    pub started_at: Option<String>,
    /// Terminal timestamp.
    #[serde(default)]
    pub ended_at: Option<String>,
    /// Duration in milliseconds.
    #[serde(default)]
    pub duration_ms: Option<i64>,
    /// Runner kind override.
    #[serde(default)]
    pub runner_kind: Option<String>,
    /// Permission profile.
    #[serde(default)]
    pub profile: Option<String>,
}

/// Agent registry row.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RegistryAgent {
    /// Stable agent id.
    pub id: String,
    /// Runner kind (`shell`, `claude`, `copilot`, ...).
    pub kind: String,
    /// `idle`, `working`, `terminated`, ... per registry semantics.
    #[serde(default)]
    pub status: Option<String>,
    /// Optional current task id.
    #[serde(default)]
    pub current_task_id: Option<String>,
    /// Free-form metadata.
    #[serde(default)]
    pub metadata: Value,
    /// Last heartbeat (RFC3339), if any.
    #[serde(default)]
    pub last_heartbeat_at: Option<String>,
}

/// Layer-3 process row.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AgentProcess {
    /// Process id.
    pub id: String,
    /// Logical kind.
    pub kind: String,
    /// Command path.
    pub command: String,
    /// Associated plan.
    #[serde(default)]
    pub plan_id: Option<String>,
    /// Associated task.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Process status.
    pub status: String,
    /// Exit code, when known.
    #[serde(default)]
    pub exit_code: Option<i64>,
    /// Last heartbeat.
    #[serde(default)]
    pub last_heartbeat_at: Option<String>,
    /// Spawn timestamp.
    pub started_at: String,
    /// End timestamp.
    #[serde(default)]
    pub ended_at: Option<String>,
}

/// PR row from GitHub.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PrSummary {
    /// PR number.
    pub number: i64,
    /// PR title.
    pub title: String,
    /// Source branch.
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    /// GitHub state (`OPEN`, `CLOSED`, `MERGED`).
    #[serde(default)]
    pub state: String,
    /// Latest CI rollup.
    #[serde(default)]
    pub ci: String,
    /// Lines added.
    #[serde(default)]
    pub additions: i64,
    /// Lines removed.
    #[serde(default)]
    pub deletions: i64,
    /// Changed file count.
    #[serde(default)]
    pub changed_files: i64,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last update timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Closed timestamp.
    #[serde(default)]
    pub closed_at: Option<String>,
    /// Merged timestamp.
    #[serde(default)]
    pub merged_at: Option<String>,
    /// Task ids declared through `Tracks:` lines in the PR body.
    #[serde(default)]
    pub tracked_task_ids: Vec<String>,
}

/// Persisted bus message.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BusMessage {
    /// Message id.
    pub id: String,
    /// Global sequence.
    pub seq: i64,
    /// Owning plan.
    #[serde(default)]
    pub plan_id: Option<String>,
    /// Topic.
    pub topic: String,
    /// Publisher.
    #[serde(default)]
    pub sender: Option<String>,
    /// Payload.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Consumer ack timestamp.
    #[serde(default)]
    pub consumed_at: Option<String>,
    /// Consumer id.
    #[serde(default)]
    pub consumed_by: Option<String>,
    /// Publish timestamp.
    pub created_at: String,
}
