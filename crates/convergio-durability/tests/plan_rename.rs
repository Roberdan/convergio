//! `Durability::rename_plan` — ADR-0026.
//!
//! Verifies title update, audit row, validation, and chain integrity.

use convergio_db::Pool;
use convergio_durability::{init, Durability, DurabilityError, NewPlan};
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn rename_updates_title_and_writes_audit_row() {
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "Wave 0 — Vision".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();

    let renamed = dur
        .rename_plan(&plan.id, "W0 — Vision", Some("operator"))
        .await
        .unwrap();
    assert_eq!(renamed.title, "W0 — Vision");

    let row: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(payload), '') FROM audit_log \
         WHERE entity_type = 'plan' AND entity_id = ? AND transition = 'plan.renamed'",
    )
    .bind(&plan.id)
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(row.0, 1);
    assert!(row.1.contains("\"from\":\"Wave 0 — Vision\""));
    assert!(row.1.contains("\"to\":\"W0 — Vision\""));

    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok, "audit chain must remain valid: {report:?}");
}

#[tokio::test]
async fn rename_trims_whitespace_and_refuses_blank() {
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();

    for blank in ["", "   ", "\t"] {
        let err = dur.rename_plan(&plan.id, blank, None).await.unwrap_err();
        assert!(matches!(err, DurabilityError::PlanTitleEmpty));
    }

    let renamed = dur
        .rename_plan(&plan.id, "  Trimmed  ", None)
        .await
        .unwrap();
    assert_eq!(renamed.title, "Trimmed");
}

#[tokio::test]
async fn rename_unknown_plan_returns_not_found() {
    let (dur, _dir) = fresh().await;
    let err = dur
        .rename_plan("00000000-0000-0000-0000-000000000000", "x", None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DurabilityError::NotFound { entity: "plan", .. }
    ));
}
