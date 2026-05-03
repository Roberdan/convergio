//! `Durability::retry_task` — F49 / friction-log F38.
//!
//! Verifies that:
//! 1. A `failed` task can be moved back to `pending` with a
//!    dedicated `task.retried` audit row.
//! 2. The previous owner is cleared so a new agent can claim it.
//! 3. Calling `retry_task` on any non-`failed` status returns
//!    `DurabilityError::NotFailed` with the actual current state.

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

async fn drive_to_failed(dur: &Durability, task_id: &str, agent: &str) {
    dur.transition_task(task_id, TaskStatus::InProgress, Some(agent))
        .await
        .unwrap();
    dur.transition_task(task_id, TaskStatus::Failed, Some(agent))
        .await
        .unwrap();
}

#[tokio::test]
async fn retry_failed_task_returns_to_pending_and_clears_owner() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p1").await;
    drive_to_failed(&dur, &task_id, "alpha").await;

    let task = dur.retry_task(&task_id, Some("alpha")).await.unwrap();

    assert!(matches!(task.status, TaskStatus::Pending));
    assert!(
        task.agent_id.is_none(),
        "retry must clear the previous owner so any agent can re-claim"
    );
}

#[tokio::test]
async fn retry_writes_task_retried_audit_row() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p2").await;
    drive_to_failed(&dur, &task_id, "alpha").await;

    dur.retry_task(&task_id, Some("alpha")).await.unwrap();

    let row: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(payload), '') FROM audit_log \
         WHERE entity_type = 'task' AND entity_id = ? AND transition = 'task.retried'",
    )
    .bind(&task_id)
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(row.0, 1, "exactly one task.retried row expected");
    assert!(row.1.contains("\"from\":\"failed\""));
    assert!(row.1.contains("\"to\":\"pending\""));
    assert!(row.1.contains("\"agent_id\":\"alpha\""));

    // Verify chain integrity after the retry write.
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(
        report.ok,
        "audit chain must remain valid after retry: {report:?}"
    );
}

#[tokio::test]
async fn retry_rejects_pending_task() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p3").await;

    let err = dur.retry_task(&task_id, None).await.unwrap_err();

    match err {
        DurabilityError::NotFailed { id, actual } => {
            assert_eq!(id, task_id);
            assert_eq!(actual, "pending");
        }
        other => panic!("expected NotFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn retry_rejects_in_progress_task() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p4").await;
    dur.transition_task(&task_id, TaskStatus::InProgress, Some("alpha"))
        .await
        .unwrap();

    let err = dur.retry_task(&task_id, Some("alpha")).await.unwrap_err();
    assert!(matches!(
        err,
        DurabilityError::NotFailed {
            actual: "in_progress",
            ..
        }
    ));
}

#[tokio::test]
async fn retry_after_retry_walks_full_lifecycle_again() {
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p5").await;
    drive_to_failed(&dur, &task_id, "alpha").await;

    dur.retry_task(&task_id, Some("alpha")).await.unwrap();

    // Fresh agent picks it up after retry.
    let task = dur
        .transition_task(&task_id, TaskStatus::InProgress, Some("beta"))
        .await
        .unwrap();
    assert!(matches!(task.status, TaskStatus::InProgress));
    assert_eq!(task.agent_id.as_deref(), Some("beta"));
}
