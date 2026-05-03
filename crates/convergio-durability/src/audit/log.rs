//! `AuditLog` writer + verifier.

use super::canonical::canonical_json;
use super::hash::{compute_hash, GENESIS_HASH};
use super::model::{AuditEntry, EntityKind, VerifyReport};
use crate::error::Result;
use chrono::Utc;
use convergio_db::Pool;
use serde::Serialize;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

/// Audit log handle. Cheap to clone (clones the underlying pool).
#[derive(Clone)]
pub struct AuditLog {
    pool: Pool,
}

impl AuditLog {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Append one entry to the log. Reads the previous `hash`, computes
    /// the new one, writes the row.
    pub async fn append<P: Serialize>(
        &self,
        entity: EntityKind,
        entity_id: &str,
        transition: &str,
        payload: &P,
        agent_id: Option<&str>,
    ) -> Result<AuditEntry> {
        let mut tx = self.pool.inner().begin().await?;
        let entry = append_tx(&mut tx, entity, entity_id, transition, payload, agent_id).await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// Entries with `seq > after_seq`, oldest first, capped at
    /// `limit`. Used by `cvg monitor` (and any client that wants a
    /// live tail) — pass `0` on the first call, then use the last
    /// returned `seq` as the next cursor.
    pub async fn list_since(&self, after_seq: i64, limit: i64) -> Result<Vec<AuditEntry>> {
        let limit = limit.clamp(1, 1000);
        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT id, seq, entity_type, entity_id, transition, payload, agent_id, \
             prev_hash, hash, created_at FROM audit_log \
             WHERE seq > ? ORDER BY seq ASC LIMIT ?",
        )
        .bind(after_seq)
        .bind(limit)
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Last entry by `seq`, or `None` if the log is empty.
    pub async fn tail(&self) -> Result<Option<AuditEntry>> {
        let row = sqlx::query_as::<_, AuditRow>(
            "SELECT id, seq, entity_type, entity_id, transition, payload, agent_id, \
             prev_hash, hash, created_at FROM audit_log ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(row.map(Into::into))
    }

    /// Latest gate refusal, optionally scoped to one task.
    pub async fn latest_refusal(&self, task_id: Option<&str>) -> Result<Option<AuditEntry>> {
        let row = if let Some(task_id) = task_id {
            sqlx::query_as::<_, AuditRow>(
                "SELECT id, seq, entity_type, entity_id, transition, payload, agent_id, \
                 prev_hash, hash, created_at FROM audit_log \
                 WHERE transition = 'task.refused' AND entity_id = ? \
                 ORDER BY seq DESC LIMIT 1",
            )
            .bind(task_id)
            .fetch_optional(self.pool.inner())
            .await?
        } else {
            sqlx::query_as::<_, AuditRow>(
                "SELECT id, seq, entity_type, entity_id, transition, payload, agent_id, \
                 prev_hash, hash, created_at FROM audit_log \
                 WHERE transition = 'task.refused' ORDER BY seq DESC LIMIT 1",
            )
            .fetch_optional(self.pool.inner())
            .await?
        };
        Ok(row.map(Into::into))
    }

    /// Verify the chain in `[from, to]` (both inclusive, both optional).
    pub async fn verify(&self, from: Option<i64>, to: Option<i64>) -> Result<VerifyReport> {
        let from_seq = from.unwrap_or(1);
        let to_seq = to.unwrap_or(i64::MAX);

        let mut prev_hash = self.bootstrap_prev_hash(from_seq).await?;
        let rows = sqlx::query_as::<_, (i64, String, String, String)>(
            "SELECT seq, payload, prev_hash, hash FROM audit_log \
             WHERE seq >= ? AND seq <= ? ORDER BY seq ASC",
        )
        .bind(from_seq)
        .bind(to_seq)
        .fetch_all(self.pool.inner())
        .await?;

        let mut checked = 0i64;
        for (seq, payload, row_prev, row_hash) in rows {
            checked += 1;
            if row_prev != prev_hash {
                return Ok(VerifyReport {
                    ok: false,
                    checked,
                    broken_at: Some(seq),
                });
            }
            if compute_hash(&prev_hash, &payload) != row_hash {
                return Ok(VerifyReport {
                    ok: false,
                    checked,
                    broken_at: Some(seq),
                });
            }
            prev_hash = row_hash;
        }
        Ok(VerifyReport {
            ok: true,
            checked,
            broken_at: None,
        })
    }

    async fn bootstrap_prev_hash(&self, from_seq: i64) -> Result<String> {
        if from_seq <= 1 {
            return Ok(GENESIS_HASH.to_string());
        }
        let row =
            sqlx::query_as::<_, (String,)>("SELECT hash FROM audit_log WHERE seq = ? LIMIT 1")
                .bind(from_seq - 1)
                .fetch_optional(self.pool.inner())
                .await?;
        Ok(row.map(|r| r.0).unwrap_or_else(|| GENESIS_HASH.to_string()))
    }
}

/// Append one audit entry inside an existing transaction.
pub async fn append_tx<P: Serialize>(
    tx: &mut Transaction<'_, Sqlite>,
    entity: EntityKind,
    entity_id: &str,
    transition: &str,
    payload: &P,
    agent_id: Option<&str>,
) -> Result<AuditEntry> {
    let next_seq = next_seq_tx(tx).await?;
    let prev_hash = previous_hash_tx(tx, next_seq).await?;
    let payload_str = canonical_json(payload)?;
    let hash = compute_hash(&prev_hash, &payload_str);

    let entry = AuditEntry {
        id: Uuid::new_v4().to_string(),
        seq: next_seq,
        entity_type: entity.as_str().to_string(),
        entity_id: entity_id.to_string(),
        transition: transition.to_string(),
        payload: payload_str,
        agent_id: agent_id.map(str::to_string),
        prev_hash,
        hash,
        created_at: Utc::now().to_rfc3339(),
    };

    sqlx::query(
        "INSERT INTO audit_log (id, seq, entity_type, entity_id, transition, \
         payload, agent_id, prev_hash, hash, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry.id)
    .bind(entry.seq)
    .bind(&entry.entity_type)
    .bind(&entry.entity_id)
    .bind(&entry.transition)
    .bind(&entry.payload)
    .bind(&entry.agent_id)
    .bind(&entry.prev_hash)
    .bind(&entry.hash)
    .bind(&entry.created_at)
    .execute(&mut **tx)
    .await?;

    Ok(entry)
}

async fn next_seq_tx(tx: &mut Transaction<'_, Sqlite>) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "UPDATE audit_sequence SET next_seq = next_seq + 1 \
         WHERE id = 1 RETURNING next_seq - 1",
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.0)
}

async fn previous_hash_tx(tx: &mut Transaction<'_, Sqlite>, next_seq: i64) -> Result<String> {
    if next_seq <= 1 {
        return Ok(GENESIS_HASH.to_string());
    }
    let row: Option<(String,)> = sqlx::query_as("SELECT hash FROM audit_log WHERE seq = ? LIMIT 1")
        .bind(next_seq - 1)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| GENESIS_HASH.to_string()))
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    seq: i64,
    entity_type: String,
    entity_id: String,
    transition: String,
    payload: String,
    agent_id: Option<String>,
    prev_hash: String,
    hash: String,
    created_at: String,
}

impl From<AuditRow> for AuditEntry {
    fn from(r: AuditRow) -> Self {
        Self {
            id: r.id,
            seq: r.seq,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            transition: r.transition,
            payload: r.payload,
            agent_id: r.agent_id,
            prev_hash: r.prev_hash,
            hash: r.hash,
            created_at: r.created_at,
        }
    }
}
