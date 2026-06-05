//! Ontology branch overlay store.

use crate::error::{DurabilityError, Result};
use crate::ontology_branch::{
    OntologyBranch, OntologyBranchStatus, OntologyResolvedEntry, OntologyValueSource,
};
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use serde_json::Value;

/// Read/write access to ontology branch tables.
#[derive(Clone)]
pub struct OntologyBranchStore {
    pool: Pool,
}

impl OntologyBranchStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert a new branch.
    pub async fn insert_branch(&self, branch: &OntologyBranch) -> Result<()> {
        sqlx::query(
            "INSERT INTO ontology_branches (id, name, status, created_at, updated_at, reviewed_at, merged_at, discarded_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&branch.id)
        .bind(&branch.name)
        .bind(branch.status.as_str())
        .bind(branch.created_at.to_rfc3339())
        .bind(branch.updated_at.to_rfc3339())
        .bind(branch.reviewed_at.as_ref().map(DateTime::to_rfc3339))
        .bind(branch.merged_at.as_ref().map(DateTime::to_rfc3339))
        .bind(branch.discarded_at.as_ref().map(DateTime::to_rfc3339))
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Fetch one branch by id.
    pub async fn get_branch(&self, id: &str) -> Result<OntologyBranch> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT id, name, status, created_at, updated_at, reviewed_at, merged_at, discarded_at FROM ontology_branches WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;
        row.map(TryInto::try_into)
            .transpose()?
            .ok_or_else(|| DurabilityError::NotFound {
                entity: "ontology_branch",
                id: id.to_string(),
            })
    }

    /// List all branches (newest first).
    pub async fn list_branches(&self) -> Result<Vec<OntologyBranch>> {
        let rows = sqlx::query_as::<_, BranchRow>(
            "SELECT id, name, status, created_at, updated_at, reviewed_at, merged_at, discarded_at FROM ontology_branches ORDER BY created_at DESC",
        )
        .fetch_all(self.pool.inner())
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Update status and lifecycle timestamps.
    pub async fn update_branch_status(
        &self,
        id: &str,
        status: OntologyBranchStatus,
        now: DateTime<Utc>,
        reviewed_at: Option<DateTime<Utc>>,
        merged_at: Option<DateTime<Utc>>,
        discarded_at: Option<DateTime<Utc>>,
    ) -> Result<OntologyBranch> {
        let result = sqlx::query("UPDATE ontology_branches SET status = ?, updated_at = ?, reviewed_at = COALESCE(reviewed_at, ?), merged_at = COALESCE(merged_at, ?), discarded_at = COALESCE(discarded_at, ?) WHERE id = ?")
            .bind(status.as_str()).bind(now.to_rfc3339())
            .bind(reviewed_at.map(|d| d.to_rfc3339()))
            .bind(merged_at.map(|d| d.to_rfc3339()))
            .bind(discarded_at.map(|d| d.to_rfc3339())).bind(id)
            .execute(self.pool.inner()).await?;
        if result.rows_affected() == 0 {
            return Err(DurabilityError::NotFound {
                entity: "ontology_branch",
                id: id.to_string(),
            });
        }
        self.get_branch(id).await
    }

    /// Resolve an entry key in mainline or branch overlay.
    pub async fn resolve_entry(
        &self,
        key: &str,
        branch_id: Option<&str>,
    ) -> Result<OntologyResolvedEntry> {
        if let Some(branch_id) = branch_id {
            if let Some(overlay) = self.get_overlay_row(branch_id, key).await? {
                match overlay.op_kind.as_str() {
                    "delete" => {
                        return Ok(OntologyResolvedEntry {
                            key: key.to_string(),
                            value: Value::Null,
                            source: OntologyValueSource::None,
                        })
                    }
                    "set" => {
                        let raw =
                            overlay
                                .value
                                .ok_or_else(|| DurabilityError::InvalidOntologyEntry {
                                    reason: "overlay op_kind=set requires value".into(),
                                })?;
                        return Ok(OntologyResolvedEntry {
                            key: key.to_string(),
                            value: serde_json::from_str::<Value>(&raw)?,
                            source: OntologyValueSource::Branch,
                        });
                    }
                    other => {
                        return Err(DurabilityError::InvalidOntologyEntry {
                            reason: format!("invalid overlay op_kind: {other}"),
                        })
                    }
                }
            }
        }
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM ontology_entries WHERE key = ? LIMIT 1",
        )
        .bind(key)
        .fetch_optional(self.pool.inner())
        .await?;
        match row {
            Some((raw,)) => Ok(OntologyResolvedEntry {
                key: key.to_string(),
                value: serde_json::from_str(&raw)?,
                source: OntologyValueSource::Main,
            }),
            None => Ok(OntologyResolvedEntry {
                key: key.to_string(),
                value: Value::Null,
                source: OntologyValueSource::None,
            }),
        }
    }

    /// Upsert a mainline entry.
    pub async fn upsert_main_entry(
        &self,
        key: &str,
        value: &Value,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO ontology_entries (key, value, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at")
            .bind(key).bind(serde_json::to_string(value)?).bind(now.to_rfc3339())
            .execute(self.pool.inner()).await?;
        Ok(())
    }

    /// Delete a mainline entry.
    pub async fn delete_main_entry(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM ontology_entries WHERE key = ?")
            .bind(key)
            .execute(self.pool.inner())
            .await?;
        Ok(())
    }

    /// Upsert a branch overlay entry (`set`).
    pub async fn upsert_branch_entry(
        &self,
        branch_id: &str,
        key: &str,
        value: &Value,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO ontology_branch_entries (branch_id, key, op_kind, value, updated_at) VALUES (?, ?, 'set', ?, ?) ON CONFLICT(branch_id, key) DO UPDATE SET op_kind = 'set', value = excluded.value, updated_at = excluded.updated_at")
            .bind(branch_id).bind(key).bind(serde_json::to_string(value)?).bind(now.to_rfc3339())
            .execute(self.pool.inner()).await?;
        sqlx::query("UPDATE ontology_branches SET updated_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(branch_id)
            .execute(self.pool.inner())
            .await?;
        Ok(())
    }

    /// Upsert a branch overlay deletion (`delete`).
    pub async fn delete_branch_entry(
        &self,
        branch_id: &str,
        key: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO ontology_branch_entries (branch_id, key, op_kind, value, updated_at) VALUES (?, ?, 'delete', NULL, ?) ON CONFLICT(branch_id, key) DO UPDATE SET op_kind = 'delete', value = NULL, updated_at = excluded.updated_at")
            .bind(branch_id).bind(key).bind(now.to_rfc3339()).execute(self.pool.inner()).await?;
        sqlx::query("UPDATE ontology_branches SET updated_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(branch_id)
            .execute(self.pool.inner())
            .await?;
        Ok(())
    }

    async fn get_overlay_row(&self, branch_id: &str, key: &str) -> Result<Option<OverlayRow>> {
        Ok(sqlx::query_as::<_, OverlayRow>("SELECT op_kind, value FROM ontology_branch_entries WHERE branch_id = ? AND key = ? LIMIT 1")
            .bind(branch_id).bind(key).fetch_optional(self.pool.inner()).await?)
    }
}

#[derive(sqlx::FromRow)]
struct BranchRow {
    id: String,
    name: String,
    status: String,
    created_at: String,
    updated_at: String,
    reviewed_at: Option<String>,
    merged_at: Option<String>,
    discarded_at: Option<String>,
}

impl TryFrom<BranchRow> for OntologyBranch {
    type Error = DurabilityError;

    fn try_from(row: BranchRow) -> Result<Self> {
        let status = OntologyBranchStatus::parse(&row.status).ok_or_else(|| {
            DurabilityError::InvalidOntologyBranch {
                reason: format!("invalid status tag: {}", row.status),
            }
        })?;
        Ok(Self {
            id: row.id,
            name: row.name,
            status,
            created_at: parse_ts(&row.created_at)?,
            updated_at: parse_ts(&row.updated_at)?,
            reviewed_at: row.reviewed_at.as_deref().map(parse_ts).transpose()?,
            merged_at: row.merged_at.as_deref().map(parse_ts).transpose()?,
            discarded_at: row.discarded_at.as_deref().map(parse_ts).transpose()?,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OverlayRow {
    op_kind: String,
    value: Option<String>,
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| DurabilityError::InvalidOntologyBranch {
            reason: format!("invalid timestamp: {s}"),
        })
}
