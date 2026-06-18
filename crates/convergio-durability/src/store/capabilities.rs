//! Local capability registry store.

use crate::error::{DurabilityError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Capability registration input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCapability {
    /// Capability name, for example `planner`.
    pub name: String,
    /// Capability package version.
    pub version: String,
    /// Registry status; defaults to `disabled`.
    #[serde(default = "default_status")]
    pub status: String,
    /// Source label; defaults to `local`.
    #[serde(default = "default_source")]
    pub source: String,
    /// Installed root path, if known.
    #[serde(default)]
    pub root_path: Option<String>,
    /// Parsed manifest payload.
    #[serde(default = "default_manifest")]
    pub manifest: Value,
    /// Package checksum, if known.
    #[serde(default)]
    pub checksum: Option<String>,
    /// Package signature, if known.
    #[serde(default)]
    pub signature: Option<String>,
}

/// Persisted capability row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name.
    pub name: String,
    /// Capability package version.
    pub version: String,
    /// Registry status.
    pub status: String,
    /// Source label.
    pub source: String,
    /// Installed root path, if known.
    pub root_path: Option<String>,
    /// Parsed manifest payload.
    pub manifest: Value,
    /// Package checksum, if known.
    pub checksum: Option<String>,
    /// Package signature, if known.
    pub signature: Option<String>,
    /// Installation timestamp.
    pub installed_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Capability {
    /// Declared processing purposes (ADR-0054 §B.2), read from the stored
    /// manifest. Empty when the capability is "ambient" (declares none) or
    /// when `purposes` is absent/malformed. Consumed by purpose-aware gates.
    pub fn declared_purposes(&self) -> Vec<String> {
        self.manifest
            .get("purposes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Access to local capability registry rows.
#[derive(Clone)]
pub struct CapabilityStore {
    pool: convergio_db::Pool,
}

impl CapabilityStore {
    /// Wrap a pool.
    pub fn new(pool: convergio_db::Pool) -> Self {
        Self { pool }
    }

    /// Register or refresh a local capability row.
    pub async fn register(&self, input: NewCapability) -> Result<Capability> {
        validate_name(&input.name)?;
        validate_status(&input.status)?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO capabilities \
             (name, version, status, source, root_path, manifest, checksum, signature, installed_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(name) DO UPDATE SET version = excluded.version, status = excluded.status, \
             source = excluded.source, root_path = excluded.root_path, manifest = excluded.manifest, \
             checksum = excluded.checksum, signature = excluded.signature, updated_at = excluded.updated_at",
        )
        .bind(&input.name)
        .bind(&input.version)
        .bind(&input.status)
        .bind(&input.source)
        .bind(&input.root_path)
        .bind(serde_json::to_string(&input.manifest)?)
        .bind(&input.checksum)
        .bind(&input.signature)
        .bind(&now)
        .bind(&now)
        .execute(self.pool.inner())
        .await?;
        self.get(&input.name).await
    }

    /// Set capability status.
    pub async fn set_status(&self, name: &str, status: &str) -> Result<Capability> {
        validate_status(status)?;
        let rows = sqlx::query("UPDATE capabilities SET status = ?, updated_at = ? WHERE name = ?")
            .bind(status)
            .bind(Utc::now().to_rfc3339())
            .bind(name)
            .execute(self.pool.inner())
            .await?
            .rows_affected();
        if rows == 0 {
            return Err(DurabilityError::NotFound {
                entity: "capability",
                id: name.to_string(),
            });
        }
        self.get(name).await
    }

    /// Fetch one capability.
    pub async fn get(&self, name: &str) -> Result<Capability> {
        let row =
            sqlx::query_as::<_, CapabilityRow>(&format!("{CAP_SELECT} WHERE name = ? LIMIT 1"))
                .bind(name)
                .fetch_optional(self.pool.inner())
                .await?;
        row.map(TryInto::try_into)
            .transpose()?
            .ok_or_else(|| DurabilityError::NotFound {
                entity: "capability",
                id: name.to_string(),
            })
    }

    /// List capabilities by name.
    pub async fn list(&self) -> Result<Vec<Capability>> {
        let rows = sqlx::query_as::<_, CapabilityRow>(&format!("{CAP_SELECT} ORDER BY name ASC"))
            .fetch_all(self.pool.inner())
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Remove one capability registry row and return the removed value.
    pub async fn remove(&self, name: &str) -> Result<Capability> {
        let cap = self.get(name).await?;
        sqlx::query("DELETE FROM capabilities WHERE name = ?")
            .bind(name)
            .execute(self.pool.inner())
            .await?;
        Ok(cap)
    }
}

fn validate_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        return Err(DurabilityError::InvalidCapability {
            reason: "capability name must be lowercase ascii, digit, '-' or '_'".into(),
        });
    }
    Ok(())
}

fn validate_status(status: &str) -> Result<()> {
    if !matches!(status, "installed" | "enabled" | "disabled" | "failed") {
        return Err(DurabilityError::InvalidCapability {
            reason: "unknown capability status".into(),
        });
    }
    Ok(())
}

fn default_status() -> String {
    "disabled".into()
}

fn default_source() -> String {
    "local".into()
}

fn default_manifest() -> Value {
    serde_json::json!({})
}

const CAP_SELECT: &str = "SELECT name, version, status, source, root_path, manifest, \
    checksum, signature, installed_at, updated_at FROM capabilities";

#[derive(sqlx::FromRow)]
struct CapabilityRow {
    name: String,
    version: String,
    status: String,
    source: String,
    root_path: Option<String>,
    manifest: String,
    checksum: Option<String>,
    signature: Option<String>,
    installed_at: String,
    updated_at: String,
}

impl TryFrom<CapabilityRow> for Capability {
    type Error = DurabilityError;
    fn try_from(row: CapabilityRow) -> Result<Self> {
        Ok(Self {
            name: row.name,
            version: row.version,
            status: row.status,
            source: row.source,
            root_path: row.root_path,
            manifest: serde_json::from_str(&row.manifest)?,
            checksum: row.checksum,
            signature: row.signature,
            installed_at: parse_time(&row.installed_at)?,
            updated_at: parse_time(&row.updated_at)?,
        })
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| DurabilityError::NotFound {
            entity: "timestamp",
            id: value.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cap(manifest: serde_json::Value) -> Capability {
        Capability {
            name: "c".into(),
            version: "1".into(),
            status: "installed".into(),
            source: "local".into(),
            root_path: None,
            manifest,
            checksum: None,
            signature: None,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn declared_purposes_reads_manifest_array() {
        let c = cap(json!({"purposes": ["student-records", "billing"]}));
        assert_eq!(
            c.declared_purposes(),
            vec!["student-records".to_string(), "billing".to_string()]
        );
    }

    #[test]
    fn declared_purposes_empty_when_absent_or_wrong_type() {
        assert!(cap(json!({})).declared_purposes().is_empty());
        assert!(cap(json!({"purposes": "nope"}))
            .declared_purposes()
            .is_empty());
    }
}
