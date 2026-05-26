//! Persisted workflow instance execution state.

use crate::spec::NodeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Cursor for an engine token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCursor {
    /// Current node id.
    pub node_id: NodeId,
    /// Optional arrival edge/source node for join bookkeeping.
    #[serde(default)]
    pub arrived_from: Option<NodeId>,
    /// For timer nodes: computed due timestamp stored as UTC.
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
}

/// One unit of work the engine is waiting on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    /// Work item id (UUID v4 string).
    pub id: String,
    /// Node that generated the work.
    pub node_id: NodeId,
    /// Kind of work.
    pub kind: WorkItemKind,
    /// Status.
    pub status: WorkItemStatus,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// Optional due date (for escalation/timers).
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    /// Escalation emitted already (idempotency guard).
    #[serde(default)]
    pub escalated: bool,
}

/// Work item type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkItemKind {
    /// Typed action request.
    Action {
        /// Action identifier.
        name: String,
        /// Input payload.
        #[serde(default)]
        input: Value,
    },
    /// Human task.
    Human {
        /// Display title.
        title: String,
    },
}

/// Work item status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    /// Pending completion.
    Pending,
    /// Completed successfully.
    Completed,
    /// Failed (terminal for the work item).
    Failed,
}

/// Completed action with its compensation reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedAction {
    /// Node id that completed.
    pub node_id: NodeId,
    /// Optional compensation action.
    #[serde(default)]
    pub compensation: Option<WorkItemKind>,
}

/// Full workflow instance state (persisted as JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstanceState {
    /// Instance id.
    pub instance_id: String,
    /// Arbitrary context object used by gateway conditions.
    #[serde(default)]
    pub context: Value,
    /// When running compensations, the terminal status we should land in once all compensation work items complete.
    #[serde(default)]
    pub compensation_target_status: Option<String>,
    /// Active engine cursors (tokens).
    #[serde(default)]
    pub cursors: Vec<EngineCursor>,
    /// Outstanding work items.
    #[serde(default)]
    pub work_items: Vec<WorkItem>,
    /// Stack of completed actions (used for compensation).
    #[serde(default)]
    pub completed_actions: Vec<CompletedAction>,
    /// Parallel-join bookkeeping: for each join-node id, predecessor node ids that have arrived.
    #[serde(default)]
    pub join_memory: HashMap<NodeId, Vec<NodeId>>,
}

impl WorkflowInstanceState {
    /// True iff any work item is still pending.
    pub fn has_pending_work(&self) -> bool {
        self.work_items
            .iter()
            .any(|w| w.status == WorkItemStatus::Pending)
    }

    /// Return a mutable reference to a work item by id.
    pub fn work_item_mut(&mut self, id: &str) -> Option<&mut WorkItem> {
        self.work_items.iter_mut().find(|w| w.id == id)
    }

    /// Return a reference to a work item by id.
    pub fn work_item(&self, id: &str) -> Option<&WorkItem> {
        self.work_items.iter().find(|w| w.id == id)
    }
}
