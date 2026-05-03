//! Migration runner for `convergio-embed`.
//!
//! Migration range 700-799 reserved by ADR-0003.
//! `set_ignore_missing(true)` lets this migrator share the
//! `_sqlx_migrations` table with sibling crates without complaining
//! about rows it did not write itself.

use crate::error::Result;
use convergio_db::Pool;

/// Apply pending `convergio-embed` migrations against the supplied
/// pool. Idempotent — safe to call on every daemon start.
pub async fn init(pool: &Pool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool.inner()).await?;
    tracing::info!("embed migrations up to date");
    Ok(())
}
