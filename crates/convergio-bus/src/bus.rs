//! Bus — write/read API for Layer 2.

use crate::error::{BusError, Result};
use crate::model::{Message, NewMessage};
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use uuid::Uuid;

/// Topic prefix that marks the system-scoped family (ADR-0025).
pub(crate) const SYSTEM_TOPIC_PREFIX: &str = "system.";

/// Hard ceiling on page size for every public read API on the bus.
///
/// The HTTP layer already clamps `?limit=` to 1..=100, but the crate
/// is reachable from in-process callers (executor, Layer 4, MCP) that
/// bypass that check, so the same invariant is enforced here as
/// defence-in-depth (CONSTITUTION § Sacred principles — security
/// first). 1000 leaves plenty of headroom for batch consumers while
/// still preventing a single call from streaming the entire table.
pub(crate) const MAX_PAGE_LIMIT: i64 = 1000;

/// Validate and clamp a caller-provided page-size `limit`. Returns the
/// bound to use in the SQL `LIMIT ?` clause.
///
/// Rejects `limit <= 0` outright — SQLite treats negative limits as
/// "no limit", which would turn every read into an unbounded scan.
pub(crate) fn clamp_limit(limit: i64) -> Result<i64> {
    if limit <= 0 {
        return Err(BusError::InvalidLimit { value: limit });
    }
    Ok(limit.min(MAX_PAGE_LIMIT))
}

/// Read/write access to the message bus.
#[derive(Clone)]
pub struct Bus {
    pool: Pool,
}

impl Bus {
    /// Wrap a pool. The caller is responsible for having run
    /// [`crate::init`] at least once.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Pool accessor for sibling impl blocks (e.g. [`crate::bus_system`]).
    pub(crate) fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Append a plan-scoped message to the bus. The topic MUST NOT
    /// start with `system.` — those go through [`Self::publish_system`].
    pub async fn publish(&self, msg: NewMessage) -> Result<Message> {
        if msg.topic.starts_with(SYSTEM_TOPIC_PREFIX) {
            return Err(BusError::InvalidTopicScope(format!(
                "topic '{}' is system-scoped; use publish_system",
                msg.topic
            )));
        }
        let payload = serde_json::to_string(&msg.payload)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let mut tx = self.pool.inner().begin().await?;
        let next_seq = next_seq(&mut tx).await?;

        sqlx::query(
            "INSERT INTO agent_messages \
             (id, seq, plan_id, topic, sender, payload, consumed_at, consumed_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
        )
        .bind(&id)
        .bind(next_seq)
        .bind(&msg.plan_id)
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
            plan_id: Some(msg.plan_id),
            topic: msg.topic,
            sender: msg.sender,
            payload: msg.payload,
            consumed_at: None,
            consumed_by: None,
            created_at: now,
        })
    }

    /// Poll unconsumed messages for `(plan_id, topic)` since `cursor`
    /// (exclusive). Returns up to `limit` rows in `seq` order.
    ///
    /// For system-scoped messages (`plan_id IS NULL`) use
    /// [`Self::poll_system`] instead.
    ///
    /// Pass `cursor = 0` on first call. The next cursor is the highest
    /// `seq` you saw. The bus does **not** auto-ack — call [`Self::ack`]
    /// when you have processed a message.
    ///
    /// `limit` must be `> 0` and is capped at [`MAX_PAGE_LIMIT`];
    /// non-positive values return [`BusError::InvalidLimit`].
    pub async fn poll(
        &self,
        plan_id: &str,
        topic: &str,
        cursor: i64,
        limit: i64,
    ) -> Result<Vec<Message>> {
        if topic.starts_with(SYSTEM_TOPIC_PREFIX) {
            return Err(BusError::InvalidTopicScope(format!(
                "topic '{topic}' is system-scoped; use poll_system"
            )));
        }
        self.poll_filtered(plan_id, topic, cursor, limit, None)
            .await
    }

    /// Same as [`Self::poll`] but skips rows where `sender == exclude_sender`.
    /// Useful when an agent polls a topic it also publishes to and wants to
    /// see only peer traffic — closes the v0.2 friction-log finding "Bus
    /// poll_messages: filter out own published messages by agent_id"
    /// (task `596c6601-…`). Pass `None` for backward-compatible behaviour.
    ///
    /// `exclude_sender` is matched against the literal `sender` column:
    /// system messages (sender NULL) are always returned regardless of
    /// the filter.
    pub async fn poll_filtered(
        &self,
        plan_id: &str,
        topic: &str,
        cursor: i64,
        limit: i64,
        exclude_sender: Option<&str>,
    ) -> Result<Vec<Message>> {
        if topic.starts_with(SYSTEM_TOPIC_PREFIX) {
            return Err(BusError::InvalidTopicScope(format!(
                "topic '{topic}' is system-scoped; use poll_system"
            )));
        }
        let limit = clamp_limit(limit)?;
        let rows = match exclude_sender {
            None => {
                sqlx::query_as::<_, MessageRow>(
                    "SELECT id, seq, plan_id, topic, sender, payload, consumed_at, \
                            consumed_by, created_at \
                     FROM agent_messages \
                     WHERE plan_id = ? AND topic = ? AND seq > ? AND consumed_at IS NULL \
                     ORDER BY seq ASC LIMIT ?",
                )
                .bind(plan_id)
                .bind(topic)
                .bind(cursor)
                .bind(limit)
                .fetch_all(self.pool.inner())
                .await?
            }
            Some(excl) => {
                sqlx::query_as::<_, MessageRow>(
                    "SELECT id, seq, plan_id, topic, sender, payload, consumed_at, \
                            consumed_by, created_at \
                     FROM agent_messages \
                     WHERE plan_id = ? AND topic = ? AND seq > ? AND consumed_at IS NULL \
                       AND (sender IS NULL OR sender != ?) \
                     ORDER BY seq ASC LIMIT ?",
                )
                .bind(plan_id)
                .bind(topic)
                .bind(cursor)
                .bind(excl)
                .bind(limit)
                .fetch_all(self.pool.inner())
                .await?
            }
        };
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Acknowledge consumption of a message. Idempotent — re-acking a
    /// message that's already acked is a no-op.
    pub async fn ack(&self, message_id: &str, consumer: Option<&str>) -> Result<()> {
        let n = sqlx::query(
            "UPDATE agent_messages SET consumed_at = ?, consumed_by = ? \
             WHERE id = ? AND consumed_at IS NULL",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(consumer)
        .bind(message_id)
        .execute(self.pool.inner())
        .await?
        .rows_affected();
        if n == 0 {
            // Either already acked, or nonexistent. Differentiate.
            let exists: Option<(String,)> =
                sqlx::query_as("SELECT id FROM agent_messages WHERE id = ? LIMIT 1")
                    .bind(message_id)
                    .fetch_optional(self.pool.inner())
                    .await?;
            if exists.is_none() {
                return Err(BusError::NotFound(message_id.to_string()));
            }
        }
        Ok(())
    }
}

pub(crate) async fn next_seq(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "UPDATE agent_message_sequence SET next_seq = next_seq + 1 \
         WHERE id = 1 RETURNING next_seq - 1",
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.0)
}

#[derive(sqlx::FromRow)]
pub(crate) struct MessageRow {
    id: String,
    seq: i64,
    plan_id: Option<String>,
    topic: String,
    sender: Option<String>,
    payload: String,
    consumed_at: Option<String>,
    consumed_by: Option<String>,
    created_at: String,
}

impl TryFrom<MessageRow> for Message {
    type Error = BusError;
    fn try_from(r: MessageRow) -> Result<Self> {
        Ok(Message {
            id: r.id,
            seq: r.seq,
            plan_id: r.plan_id,
            topic: r.topic,
            sender: r.sender,
            payload: serde_json::from_str(&r.payload)?,
            consumed_at: parse_ts_opt("consumed_at", r.consumed_at.as_deref())?,
            consumed_by: r.consumed_by,
            created_at: parse_ts("created_at", &r.created_at)?,
        })
    }
}

fn parse_ts(field: &'static str, value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|source| BusError::InvalidTimestamp {
            field,
            value: value.to_string(),
            source,
        })
}

fn parse_ts_opt(field: &'static str, value: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    value.map(|s| parse_ts(field, s)).transpose()
}
