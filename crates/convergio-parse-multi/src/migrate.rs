//! Migration runner for convergio-parse-multi (range 900-999, ADR-0003).
//!
//! Currently no tables are needed for the bootstrap phase (F2-1).
//! This module exists as the migration entry point so future F2 tasks
//! can add tables without plumbing changes.

/// Run pending migrations in the 900-999 range.
///
/// No-op in F2-1 — the first real migration (0900_parse_cache) lands in F2-2.
pub async fn init() {
    // No migrations in this phase. The runner is wired so future
    // migrations in range 900-999 can be added here.
    tracing::debug!("convergio-parse-multi: no migrations to run (F2-1 bootstrap)");
}
