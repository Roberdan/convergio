//! Workflow interpreter.

use crate::spec::{ActionSpec, NodeId, NodeKind, WorkflowSpec};
use crate::state::{
    CompletedAction, EngineCursor, WorkItem, WorkItemKind, WorkItemStatus, WorkflowInstanceState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

mod tick;

/// An emitted engine event (side effects are executed by callers).
///
/// Note: rustc's `missing_docs` lint can still report variant field docs as missing.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// A new work item was created.
    WorkItemCreated {
        /// Identifier of the created work item.
        work_item_id: String,
    },
    /// Escalation fired for a pending human task.
    EscalationEmitted {
        /// Node that emitted the escalation.
        node_id: NodeId,
        /// Escalation action to emit.
        action: WorkItemKind,
    },
    /// A work item failed; callers decide whether to cancel or compensate.
    WorkItemFailed {
        /// Identifier of the failed work item.
        work_item_id: String,
        /// Node that produced the failure.
        node_id: NodeId,
    },
    /// Instance reached a terminal end.
    Completed,
}

/// Outcome of one engine tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineTickOutcome {
    /// Updated instance state.
    pub state: WorkflowInstanceState,
    /// Emitted events.
    #[serde(default)]
    pub events: Vec<EngineEvent>,
}

/// Interpreter for a workflow spec.
#[derive(Debug, Clone)]
pub struct WorkflowEngine {
    spec: WorkflowSpec,
    pub(super) outgoing: HashMap<NodeId, Vec<NodeId>>,
    pub(super) incoming: HashMap<NodeId, Vec<NodeId>>,
    pub(super) kinds: HashMap<NodeId, NodeKind>,
}

impl WorkflowEngine {
    /// Build the engine for a spec (pre-computes adjacency).
    pub fn new(spec: WorkflowSpec) -> Self {
        let mut outgoing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut incoming: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut kinds: HashMap<NodeId, NodeKind> = HashMap::new();

        for n in &spec.nodes {
            outgoing.insert(n.id.clone(), n.next.clone());
            for to in &n.next {
                incoming.entry(to.clone()).or_default().push(n.id.clone());
            }
            kinds.insert(n.id.clone(), n.kind.clone());
        }

        Self {
            spec,
            outgoing,
            incoming,
            kinds,
        }
    }

    /// Start a new instance with the supplied context.
    pub fn start(&self, instance_id: String, context: serde_json::Value) -> WorkflowInstanceState {
        WorkflowInstanceState {
            instance_id,
            context,
            compensation_target_status: None,
            cursors: vec![EngineCursor {
                node_id: self.spec.start.clone(),
                arrived_from: None,
                due_at: None,
            }],
            work_items: Vec::new(),
            completed_actions: Vec::new(),
            join_memory: HashMap::new(),
        }
    }

    /// Mark a work item completed (or failed) and return the updated state.
    pub fn complete_work_item(
        &self,
        mut state: WorkflowInstanceState,
        work_item_id: &str,
        success: bool,
    ) -> WorkflowInstanceState {
        if let Some(w) = state.work_item_mut(work_item_id) {
            w.status = if success {
                WorkItemStatus::Completed
            } else {
                WorkItemStatus::Failed
            };
        }
        state
    }

    /// Emit compensation work items for completed actions (reverse order).
    pub fn begin_compensation(
        &self,
        mut state: WorkflowInstanceState,
        now: DateTime<Utc>,
    ) -> EngineTickOutcome {
        let mut events = Vec::new();
        for completed in state.completed_actions.iter().rev() {
            let Some(kind) = completed.compensation.clone() else {
                continue;
            };
            let item = WorkItem {
                id: Uuid::new_v4().to_string(),
                node_id: completed.node_id.clone(),
                kind,
                status: WorkItemStatus::Pending,
                created_at: now,
                due_at: None,
                escalated: false,
            };
            events.push(EngineEvent::WorkItemCreated {
                work_item_id: item.id.clone(),
            });
            state.work_items.push(item);
        }
        EngineTickOutcome { state, events }
    }

    /// Advance the instance state by processing all ready cursors.
    pub fn tick(&self, state: WorkflowInstanceState, now: DateTime<Utc>) -> EngineTickOutcome {
        tick::tick(self, state, now)
    }

    pub(super) fn record_completed_action(
        &self,
        state: &mut WorkflowInstanceState,
        node_id: NodeId,
        action: ActionSpec,
    ) {
        state.completed_actions.push(CompletedAction {
            node_id,
            compensation: action.compensation.map(|c| WorkItemKind::Action {
                name: c.name,
                input: c.input,
            }),
        });
    }
}
