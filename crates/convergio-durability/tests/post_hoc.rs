//! `Durability::close_task_post_hoc` — ADR-0026 second exception
//! to ADR-0011 (Thor-only-done).
//!
//! Verifies:
//! 1. Pending → done with audit row of kind `task.closed_post_hoc`,
//!    reason recorded, audit chain remains valid.
//! 2. Failed → done works (the typical triage path).
//! 3. Empty / whitespace reason refused with
//!    `DurabilityError::PostHocReasonMissing`.
//! 4. Already-done task refused with `DurabilityError::AlreadyDone`
//!    (idempotency guard — no duplicate audit rows).

use convergio_db::Pool;
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn make_task(dur: &Durability, title: &str) -> String {
    let plan = dur
        .create_plan(NewPlan {
            title: title.into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    dur.create_task(
        &plan.id,
        NewTask {
            wave: 1,
            sequence: 1,
            title: "t".into(),
            description: None,
            evidence_required: vec![],
            runner_kind: None,
            profile: None,
            max_budget_usd: None,
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn pending_closes_to_done_with_audit_row() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p1").await;

    let task = dur
        .close_task_post_hoc(&task_id, "shipped in PR #71", Some("operator"))
        .await
        .unwrap();
    assert!(matches!(task.status, TaskStatus::Done));

    let row: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(payload), '') FROM audit_log \
         WHERE entity_type = 'task' AND entity_id = ? AND transition = 'task.closed_post_hoc'",
    )
    .bind(&task_id)
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(row.0, 1);
    assert!(row.1.contains("\"reason\":\"shipped in PR #71\""));
    assert!(row.1.contains("\"from\":\"pending\""));
    assert!(row.1.contains("\"agent_id\":\"operator\""));

    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok, "audit chain must remain valid: {report:?}");
}

#[tokio::test]
async fn failed_closes_to_done() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p2").await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Failed, Some("a"))
        .await
        .unwrap();

    let task = dur
        .close_task_post_hoc(&task_id, "obsolete; superseded by F44", None)
        .await
        .unwrap();
    assert!(matches!(task.status, TaskStatus::Done));
}

#[tokio::test]
async fn empty_reason_rejected() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p3").await;

    for blank in ["", "   ", "\t\n"] {
        let err = dur
            .close_task_post_hoc(&task_id, blank, None)
            .await
            .unwrap_err();
        assert!(matches!(err, DurabilityError::PostHocReasonMissing));
    }
}

#[tokio::test]
async fn already_done_is_refused_idempotency_guard() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p4").await;
    dur.close_task_post_hoc(&task_id, "first close", None)
        .await
        .unwrap();

    let err = dur
        .close_task_post_hoc(&task_id, "second close", None)
        .await
        .unwrap_err();
    assert!(matches!(err, DurabilityError::AlreadyDone { .. }));

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_log WHERE entity_id = ? AND transition = 'task.closed_post_hoc'",
    )
    .bind(&task_id)
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(count.0, 1, "idempotency: only one audit row");
}
