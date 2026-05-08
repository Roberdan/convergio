//! Pool wrapper.
//!
//! Convergio is local-first and SQLite-only. `Pool` is a thin newtype
//! around [`sqlx::SqlitePool`] so callers don't depend on `sqlx`
//! directly.

use crate::error::{DbError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::{
    path::Path,
    str::FromStr,
    sync::{Once, OnceLock},
    time::Duration,
};
use tracing::info;

/// The database backend currently in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// SQLite — local mode.
    Sqlite,
}

/// SQLite connection pool.
///
/// Created once at daemon startup, cloned (cheaply) into every request
/// extractor and background loop.
#[derive(Clone)]
pub struct Pool {
    inner: SqlitePool,
    backend: Backend,
}

impl Pool {
    /// Connect to the database identified by `url`.
    ///
    /// Only `sqlite://` URLs are accepted.
    pub async fn connect(url: &str) -> Result<Self> {
        let backend = detect_backend(url)?;
        ensure_sqlite_parent(url)?;
        // Register sqlite-vec (`vec0`) so `CREATE VIRTUAL TABLE ... USING vec0`
        // migrations can run at daemon start (ADR-0038 § 5.2.3).
        register_sqlite_vec_auto_extension()?;
        // WAL + busy_timeout: lets concurrent writers serialize through
        // the write-ahead log instead of returning SQLITE_BUSY under
        // contention. Tracks F35 (CI flake on
        // `convergio-bus::concurrent_publish_allocates_contiguous_sequences`).
        let opts = SqliteConnectOptions::from_str(url)
            .map_err(|e| DbError::InvalidUrl(e.to_string()))?
            .busy_timeout(Duration::from_secs(5))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(16)
            .connect_with(opts)
            .await?;

        info!(?backend, "connected to database");
        Ok(Self {
            inner: pool,
            backend,
        })
    }

    /// Backend in use.
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Borrow the underlying [`sqlx::SqlitePool`].
    pub fn inner(&self) -> &SqlitePool {
        &self.inner
    }
}

fn detect_backend(url: &str) -> Result<Backend> {
    let scheme = url
        .split_once("://")
        .map(|(s, _)| s)
        .ok_or_else(|| DbError::InvalidUrl(format!("missing scheme in {url}")))?;
    match scheme {
        "sqlite" => Ok(Backend::Sqlite),
        other => Err(DbError::UnsupportedScheme(other.into())),
    }
}

fn ensure_sqlite_parent(url: &str) -> Result<()> {
    let trimmed = url.trim_start_matches("sqlite://");
    if trimmed.starts_with(":memory:") || trimmed.contains("mode=memory") {
        return Ok(());
    }
    let without_query = trimmed.split_once('?').map(|(p, _)| p).unwrap_or(trimmed);
    let path = Path::new(without_query);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

extern "C" {
    #[link_name = "sqlite3_vec_init"]
    fn sqlite3_vec_init_entrypoint(
        db: *mut libsqlite3_sys::sqlite3,
        pz_err_msg: *mut *mut std::os::raw::c_char,
        api: *const libsqlite3_sys::sqlite3_api_routines,
    ) -> std::os::raw::c_int;
}

fn register_sqlite_vec_auto_extension() -> Result<()> {
    static ONCE: Once = Once::new();
    static RC: OnceLock<i32> = OnceLock::new();

    ONCE.call_once(|| {
        // Ensure the `sqlite-vec` crate is linked, so the `sqlite_vec0`
        // object code (and `sqlite3_vec_init`) is present in this process.
        let _ = sqlite_vec::sqlite3_vec_init as *const ();

        // SAFETY: Registers a process-wide auto-extension; guarded by `Once`.
        let rc =
            unsafe { libsqlite3_sys::sqlite3_auto_extension(Some(sqlite3_vec_init_entrypoint)) };
        let _ = RC.set(rc);
    });

    match RC.get().copied().unwrap_or(libsqlite3_sys::SQLITE_OK) {
        rc if rc == libsqlite3_sys::SQLITE_OK => Ok(()),
        rc => Err(DbError::SqliteVecAutoExtension(rc)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_sqlite_scheme() {
        assert_eq!(detect_backend("sqlite://./x.db").unwrap(), Backend::Sqlite);
    }

    #[test]
    fn rejects_unknown_scheme() {
        assert!(detect_backend("mysql://x").is_err());
        assert!(detect_backend("file://x").is_err());
        assert!(detect_backend("not-a-url").is_err());
    }

    #[tokio::test]
    async fn connect_to_sqlite_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/dirs/state.db");
        let url = format!("sqlite://{}", path.display());
        let pool = Pool::connect(&url).await.unwrap();
        assert_eq!(pool.backend(), Backend::Sqlite);
        assert!(path.exists());

        let version: String = sqlx::query_scalar("SELECT vec_version()")
            .fetch_one(pool.inner())
            .await
            .unwrap();
        assert!(version.starts_with('v'));

        sqlx::query("CREATE VIRTUAL TABLE t USING vec0(embedding float[384]);")
            .execute(pool.inner())
            .await
            .unwrap();
    }
}
