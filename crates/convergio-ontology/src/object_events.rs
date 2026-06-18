//! SQLite data-access layer for the `object_events` bitemporal log.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use convergio_durability::audit::{append_tx, canonical_json, EntityKind};
use serde_json::json;
use sqlx::{Row, Sqlite, Transaction};

/// One persisted ontology object event.
#[derive(Debug, Clone)]
pub struct ObjectEvent {
    /// Opaque ontology object id.
    pub object_id: String,
    /// Operation string (e.g. "upsert", "delete").
    pub op: String,
    /// Canonical JSON payload.
    pub payload: serde_json::Value,
    /// Valid-time start.
    pub valid_from: DateTime<Utc>,
    /// Valid-time end.
    pub valid_to: Option<DateTime<Utc>>,
    /// Transaction-time start.
    pub tx_from: DateTime<Utc>,
    /// Transaction-time end.
    pub tx_to: Option<DateTime<Utc>>,
}

/// Input for appending an event.
#[derive(Debug, Clone)]
pub struct NewObjectEvent {
    /// Opaque ontology object id.
    pub object_id: String,
    /// Operation string (e.g. "upsert", "delete").
    pub op: String,
    /// JSON payload (will be canonicalized before persistence).
    pub payload: serde_json::Value,
    /// Valid-time start.
    pub valid_from: DateTime<Utc>,
    /// Valid-time end.
    pub valid_to: Option<DateTime<Utc>>,
}

/// Store handle for `object_events`.
#[derive(Clone)]
pub struct ObjectEventsStore {
    pub(crate) pool: Pool,
}

impl ObjectEventsStore {
    /// Bind to an existing SQLite pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Append one event and atomically write the matching hash-chained audit row.
    ///
    /// The write is bitemporal by transaction-time: the previous open row for
    /// `object_id` (where `tx_to IS NULL`) is closed in the same transaction.
    pub async fn append_event(
        &self,
        input: NewObjectEvent,
        agent_id: Option<&str>,
    ) -> Result<ObjectEvent> {
        let now = Utc::now();
        let now_s = now.to_rfc3339();
        let payload_canon = canonical_json(&input.payload)?;

        let mut tx: Transaction<'_, Sqlite> = self.pool.inner().begin().await?;

        // Close the previous open system-time row, if any.
        sqlx::query("UPDATE object_events SET tx_to = ? WHERE object_id = ? AND tx_to IS NULL")
            .bind(&now_s)
            .bind(&input.object_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO object_events (object_id, op, payload, valid_from, valid_to, tx_from, tx_to) \
             VALUES (?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(&input.object_id)
        .bind(&input.op)
        .bind(&payload_canon)
        .bind(input.valid_from.to_rfc3339())
        .bind(input.valid_to.as_ref().map(|t| t.to_rfc3339()))
        .bind(&now_s)
        .execute(&mut *tx)
        .await?;

        // Hash-chain the same event through the shared audit log.
        append_tx(
            &mut tx,
            EntityKind::Free,
            &input.object_id,
            "ontology.object_event.appended",
            &json!({
                "object_id": input.object_id,
                "op": input.op,
                "payload": input.payload,
                "payload_canonical": payload_canon,
                "valid_from": input.valid_from.to_rfc3339(),
                "valid_to": input.valid_to.as_ref().map(|t| t.to_rfc3339()),
                "tx_from": now_s,
            }),
            agent_id,
        )
        .await?;

        tx.commit().await?;

        Ok(ObjectEvent {
            object_id: input.object_id,
            op: input.op,
            payload: input.payload,
            valid_from: input.valid_from,
            valid_to: input.valid_to,
            tx_from: now,
            tx_to: None,
        })
    }

    /// Return the transaction-current row for `object_id`.
    pub async fn get_tx_current(&self, object_id: &str) -> Result<Option<ObjectEvent>> {
        let row = sqlx::query(
            "SELECT object_id, op, payload, valid_from, valid_to, tx_from, tx_to \
             FROM object_events_tx_current WHERE object_id = ? LIMIT 1",
        )
        .bind(object_id)
        .fetch_optional(self.pool.inner())
        .await?;

        row.map(row_to_event).transpose()
    }

    /// List all rows that are current by transaction-time (i.e. `tx_to IS NULL`).
    pub async fn list_tx_current(&self) -> Result<Vec<ObjectEvent>> {
        let rows = sqlx::query(
            "SELECT object_id, op, payload, valid_from, valid_to, tx_from, tx_to \
             FROM object_events_tx_current ORDER BY object_id",
        )
        .fetch_all(self.pool.inner())
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }
}

pub(crate) fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<ObjectEvent> {
    let payload_s: String = row.get("payload");
    let payload: serde_json::Value = serde_json::from_str(&payload_s)?;
    Ok(ObjectEvent {
        object_id: row.get("object_id"),
        op: row.get("op"),
        payload,
        valid_from: parse_ts(row.get("valid_from"))?,
        valid_to: parse_opt_ts(row.get("valid_to"))?,
        tx_from: parse_ts(row.get("tx_from"))?,
        tx_to: parse_opt_ts(row.get("tx_to"))?,
    })
}

pub(crate) fn parse_ts(s: String) -> Result<DateTime<Utc>> {
    let dt =
        DateTime::parse_from_rfc3339(&s).map_err(|e| Error::TimestampParse(format!("{e}: {s}")))?;
    Ok(dt.with_timezone(&Utc))
}

fn parse_opt_ts(s: Option<String>) -> Result<Option<DateTime<Utc>>> {
    match s {
        None => Ok(None),
        Some(v) => Ok(Some(parse_ts(v)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    async fn pool() -> (Pool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}", tmp.path().display());
        let pool = Pool::connect(&url).await.unwrap();
        (pool, tmp)
    }

    #[tokio::test]
    async fn append_event_closes_previous_tx_row() {
        let (pool, _tmp) = pool().await;
        convergio_durability::init(&pool).await.unwrap();
        Store::new(pool.clone()).migrate().await.unwrap();

        let store = ObjectEventsStore::new(pool.clone());
        let t0 = Utc::now();

        store
            .append_event(
                NewObjectEvent {
                    object_id: "o1".to_owned(),
                    op: "upsert".to_owned(),
                    payload: json!({"v": 1}),
                    valid_from: t0,
                    valid_to: None,
                },
                Some("agent-1"),
            )
            .await
            .unwrap();

        store
            .append_event(
                NewObjectEvent {
                    object_id: "o1".to_owned(),
                    op: "upsert".to_owned(),
                    payload: json!({"v": 2}),
                    valid_from: t0,
                    valid_to: None,
                },
                Some("agent-1"),
            )
            .await
            .unwrap();

        let current = store.get_tx_current("o1").await.unwrap().unwrap();
        assert_eq!(current.payload, json!({"v": 2}));

        let rows: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT tx_to FROM object_events WHERE object_id = ? ORDER BY tx_from ASC",
        )
        .bind("o1")
        .fetch_all(store.pool.inner())
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].0.is_some());
        assert!(rows[1].0.is_none());

        let audit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
            .fetch_one(store.pool.inner())
            .await
            .unwrap();
        assert_eq!(audit_count, 2);

        let report = convergio_durability::audit::AuditLog::new(store.pool.clone())
            .verify(None, None)
            .await
            .unwrap();
        assert!(report.ok);
    }

    #[tokio::test]
    async fn audit_failure_rolls_back_object_events_insert() {
        let (pool, _tmp) = pool().await;
        // Only ontology migrations: no audit tables.
        Store::new(pool.clone()).migrate().await.unwrap();

        let store = ObjectEventsStore::new(pool.clone());
        let err = store
            .append_event(
                NewObjectEvent {
                    object_id: "o2".to_owned(),
                    op: "upsert".to_owned(),
                    payload: json!({"v": 1}),
                    valid_from: Utc::now(),
                    valid_to: None,
                },
                None,
            )
            .await
            .unwrap_err();

        // We only assert it failed somewhere in the durability/audit path.
        let _ = format!("{err}");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM object_events")
            .fetch_one(store.pool.inner())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
