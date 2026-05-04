//! Persistent store for fleet repos and fleet plans.

use crate::config::RepoEntry;
use crate::error::{FleetError, Result};
use convergio_db::Pool;
use sqlx::Row;

/// Row returned from `fleet_repos`.
#[derive(Debug, Clone)]
pub struct FleetRepo {
    /// Short slug (PRIMARY KEY).
    pub name: String,
    /// Absolute path on disk.
    pub path: String,
    /// Primary language.
    pub language: String,
    /// Parser backend.
    pub parser: String,
    /// Role string ("engine" | "library" | "downstream" | "sandbox").
    pub role: String,
    /// Optional parent repo name.
    pub derives_from: Option<String>,
    /// ISO-8601 timestamp of the last graph build, if any.
    pub last_built_at: Option<String>,
    /// Whether this repo is active.
    pub enabled: bool,
}

fn row_to_repo(row: &sqlx::sqlite::SqliteRow) -> FleetRepo {
    FleetRepo {
        name: row.get("name"),
        path: row.get("path"),
        language: row.get("language"),
        parser: row.get("parser"),
        role: row.get("role"),
        derives_from: row.get("derives_from"),
        last_built_at: row.get("last_built_at"),
        enabled: row.get::<i64, _>("enabled") != 0,
    }
}

/// Provides CRUD operations over `fleet_repos`.
#[derive(Clone)]
pub struct FleetStore {
    pool: Pool,
}

impl FleetStore {
    /// Create a new store bound to the given pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert a repo from a [`RepoEntry`]. Returns
    /// [`FleetError::RepoDuplicate`] if the name already exists.
    pub async fn add_repo(&self, entry: &RepoEntry) -> Result<()> {
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM fleet_repos WHERE name = ?")
            .bind(&entry.name)
            .fetch_one(self.pool.inner())
            .await?
            .get(0);

        if count > 0 {
            return Err(FleetError::RepoDuplicate(entry.name.clone()));
        }

        let role = entry.role.as_str();
        sqlx::query(
            "INSERT INTO fleet_repos (name, path, language, parser, role, derives_from)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.name)
        .bind(&entry.path)
        .bind(&entry.language)
        .bind(&entry.parser)
        .bind(role)
        .bind(&entry.derives_from)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Return all repos (enabled and disabled).
    pub async fn list_repos(&self) -> Result<Vec<FleetRepo>> {
        let rows = sqlx::query(
            "SELECT name, path, language, parser, role, derives_from,
                    last_built_at, enabled
             FROM fleet_repos
             ORDER BY name",
        )
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows.iter().map(row_to_repo).collect())
    }

    /// Return a single repo by name or [`FleetError::RepoNotFound`].
    pub async fn get_repo(&self, name: &str) -> Result<FleetRepo> {
        sqlx::query(
            "SELECT name, path, language, parser, role, derives_from,
                    last_built_at, enabled
             FROM fleet_repos
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(self.pool.inner())
        .await?
        .map(|r| row_to_repo(&r))
        .ok_or_else(|| FleetError::RepoNotFound(name.to_owned()))
    }

    /// Set `last_built_at` to the current UTC timestamp.
    pub async fn mark_built(&self, name: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let affected = sqlx::query("UPDATE fleet_repos SET last_built_at = ? WHERE name = ?")
            .bind(&now)
            .bind(name)
            .execute(self.pool.inner())
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(FleetError::RepoNotFound(name.to_owned()));
        }
        Ok(())
    }

    /// Toggle the `enabled` flag.
    pub async fn set_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let flag: i64 = if enabled { 1 } else { 0 };
        let affected = sqlx::query("UPDATE fleet_repos SET enabled = ? WHERE name = ?")
            .bind(flag)
            .bind(name)
            .execute(self.pool.inner())
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(FleetError::RepoNotFound(name.to_owned()));
        }
        Ok(())
    }

    /// Remove a repo record. Does not fail if the name is absent.
    pub async fn remove_repo(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM fleet_repos WHERE name = ?")
            .bind(name)
            .execute(self.pool.inner())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RepoRole;
    use crate::migrate::init;

    async fn test_store() -> (FleetStore, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}", tmp.path().display());
        let pool = convergio_db::Pool::connect(&url).await.unwrap();
        init(&pool).await.unwrap();
        let store = FleetStore::new(pool);
        (store, tmp)
    }

    fn entry(name: &str, role: RepoRole) -> RepoEntry {
        RepoEntry {
            name: name.to_owned(),
            path: format!("/repos/{name}"),
            language: "rust".to_owned(),
            parser: "syn".to_owned(),
            role,
            derives_from: None,
        }
    }

    #[tokio::test]
    async fn add_and_list() {
        let (store, _tmp) = test_store().await;
        store
            .add_repo(&entry("alpha", RepoRole::Engine))
            .await
            .unwrap();
        store
            .add_repo(&entry("beta", RepoRole::Downstream))
            .await
            .unwrap();
        let repos = store.list_repos().await.unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "alpha");
        assert_eq!(repos[0].role, "engine");
    }

    #[tokio::test]
    async fn duplicate_returns_error() {
        let (store, _tmp) = test_store().await;
        store
            .add_repo(&entry("dup", RepoRole::Sandbox))
            .await
            .unwrap();
        let err = store
            .add_repo(&entry("dup", RepoRole::Sandbox))
            .await
            .unwrap_err();
        assert!(matches!(err, FleetError::RepoDuplicate(_)));
    }

    #[tokio::test]
    async fn get_not_found() {
        let (store, _tmp) = test_store().await;
        let err = store.get_repo("ghost").await.unwrap_err();
        assert!(matches!(err, FleetError::RepoNotFound(_)));
    }

    #[tokio::test]
    async fn mark_built_updates_timestamp() {
        let (store, _tmp) = test_store().await;
        store
            .add_repo(&entry("engine", RepoRole::Engine))
            .await
            .unwrap();
        store.mark_built("engine").await.unwrap();
        let repo = store.get_repo("engine").await.unwrap();
        assert!(repo.last_built_at.is_some());
    }

    #[tokio::test]
    async fn set_enabled_toggles() {
        let (store, _tmp) = test_store().await;
        store
            .add_repo(&entry("lib", RepoRole::Library))
            .await
            .unwrap();
        store.set_enabled("lib", false).await.unwrap();
        let repo = store.get_repo("lib").await.unwrap();
        assert!(!repo.enabled);
        store.set_enabled("lib", true).await.unwrap();
        let repo = store.get_repo("lib").await.unwrap();
        assert!(repo.enabled);
    }

    #[tokio::test]
    async fn remove_repo_is_idempotent() {
        let (store, _tmp) = test_store().await;
        store
            .add_repo(&entry("transient", RepoRole::Sandbox))
            .await
            .unwrap();
        store.remove_repo("transient").await.unwrap();
        store.remove_repo("transient").await.unwrap();
        let repos = store.list_repos().await.unwrap();
        assert!(repos.is_empty());
    }
}
