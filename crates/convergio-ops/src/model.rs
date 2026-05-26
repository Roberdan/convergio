//! Persisted workflow engine domain types.

use crate::spec::WorkflowSpec;
use crate::state::WorkflowInstanceState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One persisted workflow definition snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsWorkflow {
    /// Stable workflow identifier (UUID v4 string).
    pub workflow_id: String,
    /// Human/keyed identifier for lookup (e.g. "edu.student.intake").
    pub workflow_key: String,
    /// Monotonic version number within the key.
    pub version: i64,
    /// Declarative workflow spec.
    pub spec: WorkflowSpec,
    /// Bitemporal axis: valid time interval.
    pub valid_from: DateTime<Utc>,
    /// Bitemporal axis: valid time end (open when None).
    pub valid_to: Option<DateTime<Utc>>,
    /// Bitemporal axis: system time interval.
    pub system_from: DateTime<Utc>,
    /// Bitemporal axis: system time end (open when None).
    pub system_to: Option<DateTime<Utc>>,
    /// Row insertion timestamp.
    pub created_at: DateTime<Utc>,
    /// Optional agent that authored the version.
    pub created_by_agent: Option<String>,
}

/// Lifecycle of a workflow instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpsWorkflowInstanceStatus {
    /// Instance is running with tokens and/or work items.
    Running,
    /// Running compensations after cancellation/failure.
    Compensating,
    /// Completed successfully.
    Completed,
    /// Failed (terminal).
    Failed,
    /// Cancelled (terminal).
    Cancelled,
}

impl OpsWorkflowInstanceStatus {
    /// Persisted string tag.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Compensating => "compensating",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from DB tag.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "compensating" => Some(Self::Compensating),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// One persisted workflow instance snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsWorkflowInstance {
    /// Stable instance identifier (UUID v4 string).
    pub instance_id: String,
    /// Workflow definition identifier.
    pub workflow_id: String,
    /// Workflow definition version.
    pub workflow_version: i64,
    /// Current instance status.
    pub status: OpsWorkflowInstanceStatus,
    /// Engine state (tokens, work items, context).
    pub state: WorkflowInstanceState,
    /// Bitemporal axis: valid time interval.
    pub valid_from: DateTime<Utc>,
    /// Bitemporal axis: valid time end (open when None).
    pub valid_to: Option<DateTime<Utc>>,
    /// Bitemporal axis: system time interval.
    pub system_from: DateTime<Utc>,
    /// Bitemporal axis: system time end (open when None).
    pub system_to: Option<DateTime<Utc>>,
    /// Row insertion timestamp.
    pub created_at: DateTime<Utc>,
    /// Optional author agent.
    pub created_by_agent: Option<String>,
}
