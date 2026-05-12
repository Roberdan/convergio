//! System-scoped bus operations (ADR-0025).
//!
//! `system.*` topics live outside any single plan; their messages
//! carry `plan_id IS NULL` and represent presence/coordination
//! signals (`agent.attached`, `agent.heartbeat`, `agent.idle`,
//! `agent.detached`, …) that have no single plan home.
//!
//! Kept in a separate file from [`crate::bus`] to respect the
//! 300-line cap and to keep the system topic surface visually
//! distinct from the plan-scoped publish/poll/ack/tail/topics
//! cluster.

use crate::bus::{clamp_limit, Bus, MessageRow, MESSAGE_COLUMNS, SYSTEM_TOPIC_PREFIX};
use crate::error::{BusError, Result};
use crate::model::{Message, NewSystemMessage};
use chrono::Utc;
use uuid::Uuid;

impl Bus {
    /// Append a system-scoped message (ADR-0025). The topic MUST start
    /// with `system.`; rejects otherwise. Stored with `plan_id IS NULL`.
    pub async fn publish_system(&self, msg: NewSystemMessage) -> Result<Message> {
        if !msg.topic.starts_with(SYSTEM_TOPIC_PREFIX) {
            return Err(BusError::InvalidTopicScope(format!(
                "topic '{}' is not system-scoped; use publish",
                msg.topic
            )));
        }
        let payload = serde_json::to_string(&msg.payload)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let mut tx = self.pool().inner().begin().await?;
        let next_seq = crate::bus::next_seq(&mut tx).await?;

        sqlx::query(
            "INSERT INTO agent_messages \
             (id, seq, plan_id, topic, sender, payload, consumed_at, consumed_by, created_at) \
             VALUES (?, ?, NULL, ?, ?, ?, NULL, NULL, ?)",
        )
        .bind(&id)
        .bind(next_seq)
        .bind(&msg.topic)
        .bind(&msg.sender)
        .bind(&payload)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(Message {
            id,
            seq: next_seq,
            plan_id: None,
            topic: msg.topic,
            sender: msg.sender,
            payload: msg.payload,
            consumed_at: None,
            consumed_by: None,
            created_at: now,
        })
    }

    /// Poll unconsumed system-scoped messages for `topic` since
    /// `cursor` (exclusive). The topic MUST start with `system.`;
    /// rejects otherwise. See ADR-0025.
    pub async fn poll_system(&self, topic: &str, cursor: i64, limit: i64) -> Result<Vec<Message>> {
        if !topic.starts_with(SYSTEM_TOPIC_PREFIX) {
            return Err(BusError::InvalidTopicScope(format!(
                "topic '{topic}' is not system-scoped; use poll"
            )));
        }
        let limit = clamp_limit(limit)?;
        let rows = sqlx::query_as::<_, MessageRow>(&format!(
            "SELECT {MESSAGE_COLUMNS} \
             FROM agent_messages \
             WHERE plan_id IS NULL AND topic = ? AND seq > ? \
                   AND consumed_at IS NULL \
             ORDER BY seq ASC LIMIT ?"
        ))
        .bind(topic)
        .bind(cursor)
        .bind(limit)
        .fetch_all(self.pool().inner())
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
