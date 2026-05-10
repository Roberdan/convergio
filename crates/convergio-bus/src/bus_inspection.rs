//! Plan-scoped inspection helpers (PR #63 — `cvg bus tail` / `topics`).
//!
//! Sibling file to [`crate::bus`] that hosts the human-facing read-only
//! surface. Kept separate so [`crate::bus`] stays under the 300-line
//! cap (CONSTITUTION § 13).

use crate::bus::{Bus, MessageRow};
use crate::error::Result;
use crate::model::{AgentLastTopic, Message, TopicSummary};

impl Bus {
    /// Most recent topic activity that involved `agent_id`.
    ///
    /// This is a small operator UX helper used by the agent registry
    /// enrichment surfaces.
    pub async fn last_topic_for_agent(&self, agent_id: &str) -> Result<Option<AgentLastTopic>> {
        let row = sqlx::query_as::<_, MessageRow>(
            "SELECT id, seq, plan_id, topic, sender, payload, consumed_at, \
                    consumed_by, created_at \
             FROM agent_messages \
             WHERE sender = ? OR consumed_by = ? \
             ORDER BY COALESCE(consumed_at, created_at) DESC LIMIT 1",
        )
        .bind(agent_id)
        .bind(agent_id)
        .fetch_optional(self.pool().inner())
        .await?;

        let Some(r) = row else {
            return Ok(None);
        };
        let msg: Message = r.try_into()?;
        let (kind, at) = if msg.sender.as_deref() == Some(agent_id) {
            ("sent".to_string(), msg.created_at)
        } else {
            (
                "consumed".to_string(),
                msg.consumed_at.unwrap_or(msg.created_at),
            )
        };
        Ok(Some(AgentLastTopic {
            plan_id: msg.plan_id,
            topic: msg.topic,
            kind,
            at,
        }))
    }

    /// Read every message for `plan_id` since `cursor` (exclusive),
    /// regardless of consumed status. Optional `topic` filter narrows
    /// the result. Designed for human-facing inspection (`cvg bus
    /// tail`); agents should keep using [`Self::poll`] which only
    /// returns unconsumed rows.
    pub async fn tail(
        &self,
        plan_id: &str,
        topic: Option<&str>,
        cursor: i64,
        limit: i64,
    ) -> Result<Vec<Message>> {
        let rows = if let Some(t) = topic {
            sqlx::query_as::<_, MessageRow>(
                "SELECT id, seq, plan_id, topic, sender, payload, consumed_at, \
                        consumed_by, created_at \
                 FROM agent_messages \
                 WHERE plan_id = ? AND topic = ? AND seq > ? \
                 ORDER BY seq ASC LIMIT ?",
            )
            .bind(plan_id)
            .bind(t)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool().inner())
            .await?
        } else {
            sqlx::query_as::<_, MessageRow>(
                "SELECT id, seq, plan_id, topic, sender, payload, consumed_at, \
                        consumed_by, created_at \
                 FROM agent_messages \
                 WHERE plan_id = ? AND seq > ? \
                 ORDER BY seq ASC LIMIT ?",
            )
            .bind(plan_id)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool().inner())
            .await?
        };
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Per-topic summary for `plan_id`: total message count, the most
    /// recent `seq`, and the last `created_at`. Returned in
    /// alphabetic topic order so output is stable.
    pub async fn topics(&self, plan_id: &str) -> Result<Vec<TopicSummary>> {
        let rows: Vec<(String, i64, i64, String)> = sqlx::query_as(
            "SELECT topic, COUNT(*) AS count, MAX(seq) AS last_seq, MAX(created_at) AS last_at \
             FROM agent_messages \
             WHERE plan_id = ? \
             GROUP BY topic \
             ORDER BY topic ASC",
        )
        .bind(plan_id)
        .fetch_all(self.pool().inner())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(topic, count, last_seq, last_at)| TopicSummary {
                topic,
                count,
                last_seq,
                last_at,
            })
            .collect())
    }
}
