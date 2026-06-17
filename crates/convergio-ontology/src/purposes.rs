//! Purpose registry (ADR-0054 §B).
//!
//! A **purpose** is a free-form, immutable declaration of *why* data may
//! be read or written. The capability bucket (ADR-0008) says *what* an
//! agent may do; a purpose says *why*. It is the upstream primitive every
//! regulated vertical needs — GDPR Art. 5(1)(b) purpose limitation,
//! FERPA-equivalent education rules, healthcare consent registries.
//!
//! Purposes are registered once via `cvg purpose register` and are
//! **immutable thereafter**: the `purposes` table refuses `UPDATE` and
//! `DELETE` (see `migrations/1003_purposes.sql`), so a declared intent
//! cannot be silently rewritten. The active purpose of an action is later
//! recorded in its PROV bundle (ADR-0054 §A) and checked by the
//! purpose-mismatch gate.

use crate::error::{Error, Result};
use chrono::Utc;
use convergio_db::Pool;
use sqlx::Row;
use uuid::Uuid;

/// A registered purpose. Immutable after creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurposeRecord {
    /// UUID v4.
    pub id: String,
    /// Unique, human-readable identifier (e.g. `student-records`).
    pub label: String,
    /// Free-form description of the declared intent.
    pub description: String,
    /// Plan that declared this purpose, if any (provenance only).
    pub declared_by_plan: Option<String>,
    /// Registration timestamp (RFC3339).
    pub effective_from: String,
}

/// SQLite-backed registry for immutable purpose declarations.
#[derive(Clone)]
pub struct PurposeStore {
    pool: Pool,
}

impl PurposeStore {
    /// Create a new store bound to the given pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Register a new immutable purpose.
    ///
    /// Returns [`Error::InvalidEntry`] when `label` is empty and
    /// [`Error::PurposeAlreadyExists`] when `label` is already declared —
    /// purposes are immutable, so re-declaring is refused rather than
    /// overwriting.
    pub async fn register(
        &self,
        label: &str,
        description: &str,
        declared_by_plan: Option<&str>,
    ) -> Result<PurposeRecord> {
        let label = label.trim();
        if label.is_empty() {
            return Err(Error::InvalidEntry {
                reason: "purpose label must not be empty".to_owned(),
            });
        }
        if self.get_by_label(label).await?.is_some() {
            return Err(Error::PurposeAlreadyExists {
                label: label.to_owned(),
            });
        }
        let id = Uuid::new_v4().to_string();
        let effective_from = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO purposes (id, label, description, declared_by_plan, effective_from) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(label)
        .bind(description)
        .bind(declared_by_plan)
        .bind(&effective_from)
        .execute(self.pool.inner())
        .await?;

        Ok(PurposeRecord {
            id,
            label: label.to_owned(),
            description: description.to_owned(),
            declared_by_plan: declared_by_plan.map(str::to_owned),
            effective_from,
        })
    }

    /// List every registered purpose in stable registration order.
    pub async fn list(&self) -> Result<Vec<PurposeRecord>> {
        let rows = sqlx::query(
            "SELECT id, label, description, declared_by_plan, effective_from FROM purposes ORDER BY effective_from ASC, label ASC",
        )
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows.iter().map(row_to_record).collect())
    }

    /// Fetch a purpose by its UUID.
    pub async fn get(&self, id: &str) -> Result<Option<PurposeRecord>> {
        let row = sqlx::query(
            "SELECT id, label, description, declared_by_plan, effective_from FROM purposes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(row.as_ref().map(row_to_record))
    }

    /// Fetch a purpose by its unique label.
    pub async fn get_by_label(&self, label: &str) -> Result<Option<PurposeRecord>> {
        let row = sqlx::query(
            "SELECT id, label, description, declared_by_plan, effective_from FROM purposes WHERE label = ?",
        )
        .bind(label)
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(row.as_ref().map(row_to_record))
    }
}

fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> PurposeRecord {
    PurposeRecord {
        id: row.get("id"),
        label: row.get("label"),
        description: row.get("description"),
        declared_by_plan: row.get("declared_by_plan"),
        effective_from: row.get("effective_from"),
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
        convergio_durability::init(&pool).await.unwrap();
        Store::new(pool.clone()).migrate().await.unwrap();
        (pool, tmp)
    }

    #[tokio::test]
    async fn register_and_fetch_roundtrip() {
        let (pool, _tmp) = pool().await;
        let store = PurposeStore::new(pool.clone());
        let p = store
            .register("student-records", "Manage student records", Some("plan-1"))
            .await
            .unwrap();
        assert_eq!(p.label, "student-records");
        assert_eq!(p.declared_by_plan.as_deref(), Some("plan-1"));

        let by_id = store.get(&p.id).await.unwrap().unwrap();
        assert_eq!(by_id, p);
        let by_label = store
            .get_by_label("student-records")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_label.id, p.id);
    }

    #[tokio::test]
    async fn duplicate_label_is_refused() {
        let (pool, _tmp) = pool().await;
        let store = PurposeStore::new(pool.clone());
        store.register("billing", "", None).await.unwrap();
        let err = store.register("billing", "other", None).await.unwrap_err();
        assert!(matches!(err, Error::PurposeAlreadyExists { .. }));
    }

    #[tokio::test]
    async fn empty_label_is_refused() {
        let (pool, _tmp) = pool().await;
        let store = PurposeStore::new(pool.clone());
        let err = store.register("   ", "", None).await.unwrap_err();
        assert!(matches!(err, Error::InvalidEntry { .. }));
    }

    #[tokio::test]
    async fn list_returns_all_registered() {
        let (pool, _tmp) = pool().await;
        let store = PurposeStore::new(pool.clone());
        store.register("a-purpose", "", None).await.unwrap();
        store.register("b-purpose", "", None).await.unwrap();
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn immutability_trigger_blocks_update_and_delete() {
        let (pool, _tmp) = pool().await;
        let store = PurposeStore::new(pool.clone());
        let p = store.register("immutable", "", None).await.unwrap();

        let updated = sqlx::query("UPDATE purposes SET description = 'x' WHERE id = ?")
            .bind(&p.id)
            .execute(pool.inner())
            .await;
        assert!(updated.is_err(), "update must be refused by trigger");

        let deleted = sqlx::query("DELETE FROM purposes WHERE id = ?")
            .bind(&p.id)
            .execute(pool.inner())
            .await;
        assert!(deleted.is_err(), "delete must be refused by trigger");
    }
}
