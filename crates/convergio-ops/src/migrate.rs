use convergio_db::Pool;
use convergio_durability::Result;

/// Run ops migrations (ADR-0003: per-crate version ranges).
pub async fn init(pool: &Pool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool.inner()).await?;
    Ok(())
}
