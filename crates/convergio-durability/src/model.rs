//! Domain types serialized to/from the API and persisted in the DB.
//!
//! Every type here is `Serialize + Deserialize` so it can flow through
//! the HTTP boundary without conversion shims.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Created but not yet started.
    Draft,
    /// Tasks may be claimed.
    Active,
    /// All tasks complete and validated.
    Completed,
    /// Abandoned.
    Cancelled,
}

impl PlanStatus {
    /// String tag persisted in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from the DB.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// A persistent plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// UUID v4.
    pub id: String,
    /// Short human title.
    pub title: String,
    /// Optional long description.
    pub description: Option<String>,
    /// Optional project or repository this plan belongs to.
    pub project: Option<String>,
    /// Current status.
    pub status: PlanStatus,
    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp (UTC).
    pub updated_at: DateTime<Utc>,
    /// Materialised cache (ADR-0031): first transition into `active`.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// Materialised cache (ADR-0031): most recent transition into
    /// `completed` / `cancelled`.
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
    /// Materialised cache (ADR-0031): `ended_at - started_at` in ms.
    #[serde(default)]
    pub duration_ms: Option<i64>,
}

/// Input for [`crate::store::PlanStore::create`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlan {
    /// Short human title.
    pub title: String,
    /// Optional long description.
    pub description: Option<String>,
    /// Optional project or repository this plan belongs to.
    #[serde(default)]
    pub project: Option<String>,
}

/// Lifecycle of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Awaiting an agent to claim.
    Pending,
    /// Claimed and being worked on.
    InProgress,
    /// Agent claims completion; awaiting validator.
    Submitted,
    /// Validated and closed.
    Done,
    /// Failed; reopenable via [`Durability::retry_task`] which moves
    /// it back to `pending` so a new agent can claim it.
    Failed,
}

impl TaskStatus {
    /// String tag persisted in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Submitted => "submitted",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    /// Parse from the DB.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "submitted" => Some(Self::Submitted),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// A persistent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// UUID v4.
    pub id: String,
    /// Owning plan.
    pub plan_id: String,
    /// Parallel-execution wave (1 = first wave).
    pub wave: i64,
    /// Order within the wave.
    pub sequence: i64,
    /// Short human title.
    pub title: String,
    /// Optional details.
    pub description: Option<String>,
    /// Current status.
    pub status: TaskStatus,
    /// Agent currently holding the task, if any.
    pub agent_id: Option<String>,
    /// Names of evidence the validator requires.
    pub evidence_required: Vec<String>,
    /// Last heartbeat received from `agent_id`.
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Materialised cache (ADR-0031): first time the task entered
    /// `in_progress`, RFC3339. `None` until then.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// Materialised cache (ADR-0031): most recent transition into a
    /// terminal state (`done`/`failed`/`cancelled`). `None` until then.
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
    /// Materialised cache (ADR-0031): `ended_at - started_at` in
    /// milliseconds. `None` until ended.
    #[serde(default)]
    pub duration_ms: Option<i64>,
    /// Per-task runner kind (ADR-0034) in the wire format
    /// `<vendor>:<model>` — e.g. `claude:sonnet`, `claude:opus`,
    /// `copilot:gpt-5.2`, `qwen:qwen3-coder`. `None` ⇒ executor
    /// uses the daemon default (`CONVERGIO_RUNNER_DEFAULT`).
    #[serde(default)]
    pub runner_kind: Option<String>,
    /// Per-task permission profile (ADR-0033). One of `standard`,
    /// `read_only`, `sandbox`. `None` ⇒ daemon default.
    #[serde(default)]
    pub profile: Option<String>,
    /// Per-task session budget cap (USD), forwarded as
    /// `claude --max-budget-usd` when the runner is Claude. `None`
    /// ⇒ no cap.
    #[serde(default)]
    pub max_budget_usd: Option<f32>,
}

/// Recently completed task with plan context for dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentCompletedTask {
    /// Task UUID.
    pub id: String,
    /// Task title.
    pub title: String,
    /// Owning plan id.
    pub plan_id: String,
    /// Owning plan title.
    pub plan_title: String,
    /// Optional project or repository this plan belongs to.
    pub project: Option<String>,
    /// Last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Input for [`crate::store::TaskStore::create`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    /// Wave number (defaults to 1).
    #[serde(default = "default_one")]
    pub wave: i64,
    /// Sequence within the wave (defaults to 1).
    #[serde(default = "default_one")]
    pub sequence: i64,
    /// Short human title.
    pub title: String,
    /// Optional details.
    pub description: Option<String>,
    /// Required evidence kinds.
    #[serde(default)]
    pub evidence_required: Vec<String>,
    /// Optional runner kind override (ADR-0034). `None` ⇒ daemon default.
    #[serde(default)]
    pub runner_kind: Option<String>,
    /// Optional permission profile override. `None` ⇒ daemon default.
    #[serde(default)]
    pub profile: Option<String>,
    /// Optional session budget cap (USD).
    #[serde(default)]
    pub max_budget_usd: Option<f32>,
}

fn default_one() -> i64 {
    1
}

/// A piece of evidence attached to a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// UUID v4.
    pub id: String,
    /// Task this evidence belongs to.
    pub task_id: String,
    /// Logical kind (`test_pass`, `pr_url`, ...).
    pub kind: String,
    /// Caller-supplied JSON payload.
    pub payload: serde_json::Value,
    /// Optional process exit code.
    pub exit_code: Option<i64>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
