//! Schema migration runner (range 500-599, ADR-0003).

use crate::error::Result;
use convergio_db::Pool;

/// Run pending migrations for `convergio-ontology`.
///
/// Uses `set_ignore_missing(true)` so it can coexist with other crates'
/// migrators on the shared `_sqlx_migrations` table.
pub async fn init(pool: &Pool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool.inner()).await?;
    Ok(())
}
