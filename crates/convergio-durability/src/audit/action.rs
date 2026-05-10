//! Typed audit actions derived from persisted audit rows.
//!
//! P3-3: Palantir-inspired compensating actions.
//!
//! The audit log stores `transition` as an opaque dotted string plus a
//! canonical JSON payload. For a small, curated subset of daemon-owned
//! transitions we can recover a typed `Action` and compute a mechanical
//! compensating action (when possible).

use super::AuditEntry;
use crate::model::TaskStatus;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Typed action inferred from an audit row.
///
/// This enum is intentionally narrow: it models only actions that the
/// daemon itself emits (not arbitrary `audit.append` rows) and that are
/// useful for mechanical compensation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// `agent.retired` — marks an agent terminated.
    AgentRetire {
        /// Agent id.
        agent_id: String,
    },
    /// `agent.re_registered` — compensating action that un-terminates an agent.
    AgentReregister {
        /// Agent id.
        agent_id: String,
    },

    /// `plan.renamed` — human title changed in place.
    PlanRename {
        /// Plan id.
        plan_id: String,
        /// Previous title.
        from: String,
        /// New title.
        to: String,
        /// Agent responsible, when known.
        #[serde(default)]
        agent_id: Option<String>,
    },

    /// `task.<status>` — a gate-approved task status transition.
    TaskTransition {
        /// Task id.
        task_id: String,
        /// Previous status.
        from: TaskStatus,
        /// Target status.
        to: TaskStatus,
        /// Agent responsible, when known.
        #[serde(default)]
        agent_id: Option<String>,
    },

    /// `task.completed_by_thor` — Thor promoted `submitted -> done`.
    TaskCompletedByThor {
        /// Task id.
        task_id: String,
    },

    /// `task.closed_post_hoc` — operator promoted a task to `done`.
    TaskClosedPostHoc {
        /// Task id.
        task_id: String,
        /// Previous status.
        from: TaskStatus,
        /// Operator-supplied provenance reason.
        reason: String,
        /// Agent responsible, when known.
        #[serde(default)]
        agent_id: Option<String>,
    },

    /// `task.reopened` — admin reopen out of `done`.
    TaskReopen {
        /// Task id.
        task_id: String,
        /// Target status to reopen to.
        to: TaskStatus,
        /// Optional provenance reason.
        #[serde(default)]
        reason: Option<String>,
        /// Source audit sequence number being compensated, when known.
        #[serde(default)]
        source_seq: Option<i64>,
        /// Agent responsible, when known.
        #[serde(default)]
        agent_id: Option<String>,
    },

    /// `evidence.attached` — new evidence row created.
    EvidenceAttach {
        /// Evidence id.
        evidence_id: String,
    },
    /// `evidence.removed` — evidence row deleted.
    EvidenceRemove {
        /// Evidence id.
        evidence_id: String,
    },

    /// `plan.created` — plan row inserted.
    PlanCreated {
        /// Plan id.
        plan_id: String,
    },
    /// `task.created` — task row inserted.
    TaskCreated {
        /// Task id.
        task_id: String,
    },
    /// `agent.heartbeat` — time-series update.
    AgentHeartbeat {
        /// Agent id.
        agent_id: String,
    },
}

impl Action {
    /// Recover a typed action from an audit row.
    pub fn try_from_entry(entry: &AuditEntry) -> Result<Self, String> {
        let payload: Value = serde_json::from_str(&entry.payload)
            .map_err(|e| format!("invalid audit payload json: {e}"))?;
        match entry.transition.as_str() {
            "agent.retired" => Ok(Self::AgentRetire {
                agent_id: read_str(&payload, "agent_id")?.to_string(),
            }),
            "agent.re_registered" => Ok(Self::AgentReregister {
                agent_id: read_str(&payload, "agent_id")?.to_string(),
            }),
            "agent.heartbeat" => Ok(Self::AgentHeartbeat {
                agent_id: read_str(&payload, "agent_id")?.to_string(),
            }),
            "plan.created" => Ok(Self::PlanCreated {
                plan_id: read_str(&payload, "plan_id")?.to_string(),
            }),
            "plan.renamed" => Ok(Self::PlanRename {
                plan_id: read_str(&payload, "plan_id")?.to_string(),
                from: read_str(&payload, "from")?.to_string(),
                to: read_str(&payload, "to")?.to_string(),
                agent_id: payload
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            }),
            "task.created" => Ok(Self::TaskCreated {
                task_id: read_str(&payload, "task_id")?.to_string(),
            }),
            "task.completed_by_thor" => Ok(Self::TaskCompletedByThor {
                task_id: read_str(&payload, "task_id")?.to_string(),
            }),
            "task.closed_post_hoc" => Ok(Self::TaskClosedPostHoc {
                task_id: read_str(&payload, "task_id")?.to_string(),
                from: read_task_status(&payload, "from")?,
                reason: read_str(&payload, "reason")?.to_string(),
                agent_id: payload
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            }),
            "task.reopened" => Ok(Self::TaskReopen {
                task_id: read_str(&payload, "task_id")?.to_string(),
                to: read_task_status(&payload, "to")?,
                reason: payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                source_seq: payload.get("source_seq").and_then(Value::as_i64),
                agent_id: payload
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            }),
            "evidence.attached" => Ok(Self::EvidenceAttach {
                evidence_id: read_str(&payload, "evidence_id")?.to_string(),
            }),
            "evidence.removed" => Ok(Self::EvidenceRemove {
                evidence_id: read_str(&payload, "evidence_id")?.to_string(),
            }),
            other if other.starts_with("task.") => {
                let to = other
                    .strip_prefix("task.")
                    .ok_or_else(|| "missing task. prefix".to_string())?;
                let to = TaskStatus::parse(to)
                    .ok_or_else(|| format!("unknown task status in transition: {other}"))?;
                Ok(Self::TaskTransition {
                    task_id: read_str(&payload, "task_id")?.to_string(),
                    from: read_task_status(&payload, "from")?,
                    to,
                    agent_id: payload
                        .get("agent_id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                })
            }
            other => Err(format!(
                "unsupported audit transition for action inference: {other}"
            )),
        }
    }

    /// Compensating action that undoes this action's side effects when
    /// a clean inverse exists.
    ///
    /// Returns `None` when the daemon does not expose a safe mechanical
    /// inverse (see [`Self::compensate_rationale`]).
    pub fn compensate(&self) -> Option<Self> {
        match self {
            Self::AgentRetire { agent_id } => Some(Self::AgentReregister {
                agent_id: agent_id.clone(),
            }),
            Self::AgentReregister { agent_id } => Some(Self::AgentRetire {
                agent_id: agent_id.clone(),
            }),
            Self::PlanRename {
                plan_id,
                from,
                to,
                agent_id,
            } => Some(Self::PlanRename {
                plan_id: plan_id.clone(),
                from: to.clone(),
                to: from.clone(),
                agent_id: agent_id.clone(),
            }),
            Self::TaskTransition {
                task_id,
                from,
                to,
                agent_id,
            } => Some(Self::TaskTransition {
                task_id: task_id.clone(),
                from: *to,
                to: *from,
                agent_id: agent_id.clone(),
            }),
            Self::TaskCompletedByThor { task_id } => Some(Self::TaskReopen {
                task_id: task_id.clone(),
                to: TaskStatus::Submitted,
                reason: None,
                source_seq: None,
                agent_id: None,
            }),
            Self::TaskClosedPostHoc { task_id, from, .. } => Some(Self::TaskReopen {
                task_id: task_id.clone(),
                to: *from,
                reason: None,
                source_seq: None,
                agent_id: None,
            }),
            Self::EvidenceAttach { evidence_id } => Some(Self::EvidenceRemove {
                evidence_id: evidence_id.clone(),
            }),
            // Explicit non-reversible actions — rationale is part of the
            // contract (see `compensate_rationale`).
            Self::EvidenceRemove { .. }
            | Self::PlanCreated { .. }
            | Self::TaskCreated { .. }
            | Self::AgentHeartbeat { .. }
            | Self::TaskReopen { .. } => None,
        }
    }

    /// Short rationale for why [`Self::compensate`] is `None`.
    pub fn compensate_rationale(&self) -> Option<&'static str> {
        match self {
            Self::PlanCreated { .. } => Some("plan deletion is not a supported daemon action"),
            Self::TaskCreated { .. } => Some("task deletion is not a supported daemon action"),
            Self::EvidenceRemove { .. } => Some(
                "evidence removal is destructive; restoring would require persisting the original payload in the audit row",
            ),
            Self::AgentHeartbeat { .. } => {
                Some("heartbeats are time-series signals and cannot be meaningfully undone")
            }
            Self::TaskReopen { .. } => Some("re-open operations are administrative; no canonical inverse exists"),
            _ => None,
        }
    }
}

fn read_str<'a>(payload: &'a Value, field: &str) -> Result<&'a str, String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or invalid '{field}'"))
}

fn read_task_status(payload: &Value, field: &str) -> Result<TaskStatus, String> {
    let raw = read_str(payload, field)?;
    TaskStatus::parse(raw).ok_or_else(|| format!("unknown task status '{raw}' in '{field}'"))
}
