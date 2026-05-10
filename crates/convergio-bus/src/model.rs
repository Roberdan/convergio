//! Domain types for Layer 2.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A persisted message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// UUID v4.
    pub id: String,
    /// Monotonic global sequence (1-based).
    pub seq: i64,
    /// Plan that owns this message; `None` for `system.*` topics
    /// per ADR-0025.
    pub plan_id: Option<String>,
    /// Free-form topic (e.g. `task.done`, `agent:foo` for direct,
    /// or a `system.*` family member).
    pub topic: String,
    /// Agent id of the publisher, or `None` for system messages.
    pub sender: Option<String>,
    /// Caller-supplied JSON payload.
    pub payload: serde_json::Value,
    /// When the consumer acked this message; `None` if still in flight.
    pub consumed_at: Option<DateTime<Utc>>,
    /// Agent id of the consumer that acked, if any.
    pub consumed_by: Option<String>,
    /// Publish timestamp.
    pub created_at: DateTime<Utc>,
}

/// Last bus-topic activity for a given agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLastTopic {
    /// Plan that owns this message; `None` for `system.*` topics.
    pub plan_id: Option<String>,
    /// Topic name.
    pub topic: String,
    /// Activity kind: `sent` or `consumed`.
    pub kind: String,
    /// Timestamp of the activity.
    pub at: DateTime<Utc>,
}

/// Per-topic summary returned by [`crate::Bus::topics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    /// Topic name.
    pub topic: String,
    /// Total message count for this topic in the plan.
    pub count: i64,
    /// Highest `seq` observed for this topic.
    pub last_seq: i64,
    /// `created_at` of the latest message (RFC 3339 string for stable JSON).
    pub last_at: String,
}

/// Input for [`crate::Bus::publish`] — plan-scoped messages.
///
/// For system-scoped messages (`system.*` topic family, ADR-0025)
/// use [`NewSystemMessage`] via [`crate::Bus::publish_system`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessage {
    /// Plan that owns this message.
    pub plan_id: String,
    /// Topic.
    pub topic: String,
    /// Sender (publisher) agent id, if any.
    #[serde(default)]
    pub sender: Option<String>,
    /// Payload.
    pub payload: serde_json::Value,
}

/// Input for [`crate::Bus::publish_system`] — system-scoped messages
/// with `plan_id IS NULL`. The `topic` must start with `system.`
/// per ADR-0025; the bus rejects otherwise with
/// [`crate::error::BusError::InvalidTopicScope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSystemMessage {
    /// Topic (must start with `system.`).
    pub topic: String,
    /// Sender (publisher) agent id, if any.
    #[serde(default)]
    pub sender: Option<String>,
    /// Payload.
    pub payload: serde_json::Value,
}
