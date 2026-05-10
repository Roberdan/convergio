//! Shared agent-facing action contract for Convergio integrations.
//!
//! The HTTP daemon remains the source of truth. This crate only defines
//! the compact, versioned request/response shapes used by adapters such
//! as the MCP bridge.

#![forbid(unsafe_code)]

mod action;

pub use action::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current major schema version for agent actions.
///
/// Bumped to `2` in v0.1.x: removed `complete_task`. Agents that
/// previously called `convergio.act complete_task` must now call
/// `validate_plan` after submitting; the validator promotes
/// `submitted` to `done` (CONSTITUTION §6, ADR-0011).
pub const SCHEMA_VERSION: &str = "2";

/// Stable MCP help tool name.
pub const HELP_TOOL: &str = "convergio.help";

/// Stable MCP action tool name.
pub const ACT_TOOL: &str = "convergio.act";

/// Action capabilities exposed to agents.
pub const CAPABILITIES: &[&str] = &[
    "status",
    "plans",
    "tasks",
    "evidence",
    "audit",
    "agents",
    "capabilities",
    "crdt",
    "workspace",
    "validation",
];

/// Read-only help topics served by `convergio.help`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HelpTopic {
    /// Minimal session bootstrap.
    Quickstart,
    /// Action catalog.
    Actions,
    /// Details for one action.
    Action,
    /// Evidence payload conventions.
    EvidenceSchema,
    /// How agents should handle gate refusals.
    GateRefusal,
    /// Local setup instructions.
    Setup,
    /// Canonical agent prompt addendum.
    Prompt,
}

/// Help output verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HelpVerbosity {
    /// Compact default for agent context.
    Short,
    /// Include machine-readable schemas or examples.
    Schema,
    /// Full explanatory content for humans/debugging.
    Full,
}

/// Request accepted by `convergio.help`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HelpRequest {
    /// Requested help topic.
    #[serde(default = "default_help_topic")]
    pub topic: HelpTopic,
    /// Optional action name when `topic` is `Action`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    /// Requested verbosity.
    #[serde(default = "default_help_verbosity")]
    pub verbosity: HelpVerbosity,
}

/// Request accepted by `convergio.act`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActRequest {
    /// Schema major version the agent used when building this request.
    pub schema_version: String,
    /// Constrained action name.
    pub action: Action,
    /// Action-specific input object.
    #[serde(default)]
    pub params: Value,
}

/// Stable response envelope returned to agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentResponse {
    /// Whether the action succeeded.
    pub ok: bool,
    /// Stable machine-readable response code.
    pub code: AgentCode,
    /// Short stable English message.
    pub message: String,
    /// Optional structured payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Optional next action hint for agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<NextHint>,
}

/// Stable response codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentCode {
    /// Action completed.
    Ok,
    /// Daemon is unreachable.
    DaemonUnavailable,
    /// The action schema version is incompatible.
    SchemaVersionMismatch,
    /// Request shape is invalid.
    InvalidRequest,
    /// A Convergio gate refused the transition.
    GateRefused,
    /// Requested resource was not found.
    NotFound,
    /// Any other daemon or bridge failure.
    Error,
}

/// Stable next-step hints for agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextHint {
    /// Call `convergio.help` again before retrying.
    RefreshHelp,
    /// Start the local daemon.
    StartDaemon,
    /// Fix the issue, attach new evidence, then retry submit.
    FixAddEvidenceRetrySubmit,
    /// Continue with task completion.
    CompleteTask,
    /// Verify audit state.
    VerifyAudit,
}

/// Compact catalog returned by help implementations.
#[derive(Debug, Clone, Serialize)]
pub struct ActionCatalog {
    /// Current schema version.
    pub schema_version: &'static str,
    /// Stable MCP tool names.
    pub tools: ToolNames,
    /// Available capabilities.
    pub capabilities: &'static [&'static str],
    /// Supported actions.
    pub actions: Vec<&'static str>,
}

/// Discoverable per-action metadata (P3-1 — Palantir-inspired
/// Action type registry).
///
/// Each action ships with a stable name + the human-readable summary
/// captured from its `Action` enum doc-comment. The MCP bridge and
/// any external tool can read this without re-implementing the
/// catalog: see [`actions_registry`] and the
/// `GET /v1/api/actions` daemon route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// Stable action slug (e.g. `"submit_task"`).
    pub name: String,
    /// Snake-case capability bucket the action belongs to (one of
    /// [`CAPABILITIES`]).
    pub capability: &'static str,
    /// One-line summary derived from the `Action` enum doc-comment.
    pub summary: &'static str,
}

/// Full registry: every supported [`Action`] enriched with
/// capability + summary. Stable order = enum declaration order.
pub fn actions_registry() -> Vec<ActionMetadata> {
    Action::ALL
        .iter()
        .map(|a| ActionMetadata {
            name: a.as_str().to_string(),
            capability: a.capability(),
            summary: a.summary(),
        })
        .collect()
}

/// Canonical generated JSON action registry document (P3-1).
///
/// This is generated at build time by `build.rs` and embedded into the
/// crate so every consumer (daemon HTTP and MCP bridge) can return the
/// exact same bytes.
pub fn actions_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/actions.json"))
}

/// Raw bytes for the canonical generated action registry JSON.
pub fn actions_json_bytes() -> &'static [u8] {
    actions_json().as_bytes()
}

/// Stable MCP tool names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNames {
    /// Read-only help tool.
    pub help: &'static str,
    /// Action dispatcher tool.
    pub act: &'static str,
}

impl ActionCatalog {
    /// Build the default action catalog.
    pub fn current() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            tools: ToolNames {
                help: HELP_TOOL,
                act: ACT_TOOL,
            },
            capabilities: CAPABILITIES,
            actions: Action::ALL.iter().map(|a| a.as_str()).collect(),
        }
    }
}

fn default_help_topic() -> HelpTopic {
    HelpTopic::Quickstart
}

fn default_help_verbosity() -> HelpVerbosity {
    HelpVerbosity::Short
}

#[cfg(test)]
mod tests;
