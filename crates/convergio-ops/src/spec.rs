//! Declarative workflow specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Node identifier within a workflow spec.
pub type NodeId = String;

/// A workflow definition as a directed graph of nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    /// Start node id.
    pub start: NodeId,
    /// Nodes in the workflow.
    pub nodes: Vec<NodeSpec>,
}

impl WorkflowSpec {
    /// Fetch a node by id.
    pub fn node(&self, id: &str) -> Option<&NodeSpec> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Validate that the graph is well-formed.
    pub fn validate(&self) -> Result<(), String> {
        if self.start.trim().is_empty() {
            return Err("workflow spec start node must be non-empty".into());
        }
        if self.nodes.is_empty() {
            return Err("workflow spec must contain at least one node".into());
        }

        let mut ids: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (idx, n) in self.nodes.iter().enumerate() {
            let id = n.id.trim();
            if id.is_empty() {
                return Err(format!("workflow node at index {idx} has empty id"));
            }
            if let Some(prev) = ids.insert(id, idx) {
                return Err(format!(
                    "workflow node id '{id}' is duplicated (indexes {prev} and {idx})"
                ));
            }
        }

        if !ids.contains_key(self.start.as_str()) {
            return Err(format!("start node '{}' not found", self.start));
        }

        for n in &self.nodes {
            for to in &n.next {
                if !ids.contains_key(to.as_str()) {
                    return Err(format!(
                        "edge from '{}' points to missing node '{to}'",
                        n.id
                    ));
                }
            }
            if let NodeKind::Timer(t) = &n.kind {
                if t.after_ms < 0 {
                    return Err(format!(
                        "timer node '{}' has negative after_ms ({})",
                        n.id, t.after_ms
                    ));
                }
            }
        }

        Ok(())
    }
}

/// One node in the workflow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    /// Stable node id.
    pub id: NodeId,
    /// Node kind.
    pub kind: NodeKind,
    /// Outgoing edges (node ids).
    #[serde(default)]
    pub next: Vec<NodeId>,
}

/// Supported BPMN-2.0 subset node kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    /// Start event.
    Start,
    /// End event.
    End,
    /// Automatic timer event (blocks until due).
    Timer(TimerSpec),
    /// A typed action step (creates an action work item).
    Action(ActionSpec),
    /// A human task (creates a human work item).
    HumanTask(HumanTaskSpec),
    /// Parallel gateway (fork/join).
    ParallelGateway {
        /// Fork or join.
        kind: GatewayKind,
    },
    /// Exclusive gateway (routes selected by conditions).
    ExclusiveGateway {
        /// Ordered routes; first matching condition wins.
        routes: Vec<ExclusiveRoute>,
    },
}

/// Parallel gateway kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GatewayKind {
    /// Split a token into one per outgoing edge.
    Fork,
    /// Join tokens from all incoming edges.
    Join,
}

/// Timer spec: blocks for `after_ms` relative to first arrival.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSpec {
    /// Delay duration in milliseconds.
    pub after_ms: i64,
}

/// A typed action step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSpec {
    /// Action identifier (typically from `actions.json`).
    pub name: String,
    /// Input payload for the action.
    #[serde(default)]
    pub input: Value,
    /// Optional compensation action (executed in reverse order on rollback).
    #[serde(default)]
    pub compensation: Option<CompensationSpec>,
}

/// One compensation action spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationSpec {
    /// Action identifier.
    pub name: String,
    /// Input payload.
    #[serde(default)]
    pub input: Value,
}

/// A human task step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanTaskSpec {
    /// Human task title.
    pub title: String,
    /// Optional escalation delay; when exceeded the engine emits an escalation action event.
    #[serde(default)]
    pub escalation_after_ms: Option<i64>,
    /// Optional escalation action to emit when escalated.
    #[serde(default)]
    pub escalation_action: Option<ActionSpec>,
}

/// One exclusive-gateway route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusiveRoute {
    /// Condition expression.
    pub when: ConditionExpr,
    /// Destination node id.
    pub to: NodeId,
}

/// Tiny condition language evaluated against the instance context JSON.
///
/// Note: rustc's `missing_docs` lint can still report variant field docs as missing.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ConditionExpr {
    /// Always matches.
    Always,
    /// Context key is truthy (exists and not false/null/empty string).
    Truthy {
        /// Context key to check.
        key: String,
    },
    /// Context key equals a literal JSON value.
    Eq {
        /// Context key to compare.
        key: String,
        /// Expected JSON value.
        value: Value,
    },
    /// Negation.
    Not {
        /// Nested expression.
        expr: Box<ConditionExpr>,
    },
}

impl ConditionExpr {
    /// Evaluate the expression against the supplied context object.
    pub fn eval(&self, ctx: &Value) -> bool {
        match self {
            Self::Always => true,
            Self::Truthy { key } => match ctx.get(key) {
                None | Some(Value::Null) => false,
                Some(Value::Bool(b)) => *b,
                Some(Value::String(s)) => !s.is_empty(),
                Some(Value::Array(a)) => !a.is_empty(),
                Some(Value::Object(o)) => !o.is_empty(),
                Some(Value::Number(_)) => true,
            },
            Self::Eq { key, value } => ctx.get(key).map(|v| v == value).unwrap_or(false),
            Self::Not { expr } => !expr.eval(ctx),
        }
    }
}
