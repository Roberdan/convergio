//! Migration runner for `convergio-reports`.
//!
//! Migration range 501–599 reserved by ADR-0003.

use crate::error::Result;
use convergio_db::Pool;

/// Apply pending `convergio-reports` migrations against the supplied pool.
///
/// Idempotent — safe to call on every daemon start.
pub async fn init(pool: &Pool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool.inner()).await?;

    // Ensure the `ReportTemplate` ObjectType exists so templates can be managed
    // as typed domain objects.
    let ontology = convergio_ontology::Store::new(pool.clone());
    ontology
        .upsert_object(
            crate::builtins::REPORT_TEMPLATE_OBJECT_TYPE_ID,
            crate::builtins::REPORT_TEMPLATE_SCHEMA_VERSION,
            false,
            "ReportTemplate",
            "Convergio report template definition",
            crate::builtins::report_template_object_type(),
            None,
        )
        .await
        .map_err(|e| {
            crate::error::ReportError::InvalidInput(format!(
                "failed to ensure ReportTemplate ObjectType: {e}"
            ))
        })?;

    tracing::info!("reports migrations up to date");
    Ok(())
}
