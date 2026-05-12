//! Regression tests for the per-crate audit follow-up (2026-05-12):
//! durability projections must never silently normalize corrupt SQLite
//! rows back into fresh-looking values. Each test corrupts one column
//! in a way that should never be written by the daemon, then asserts
//! that the relevant read path surfaces an error rather than masking
//! the corruption with a default (Pending, Utc::now(), [], …).

use convergio_db::Pool;
use convergio_durability::{
    init, AgentStore, Durability, DurabilityError, NewAgent, NewPlan, NewTask,
};
use serde_json::json;
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn seed_task(dur: &Durability) -> String {
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .tasks()
        .create(
            &plan.id,
            NewTask {
                wave: 0,
                sequence: 0,
                title: "t".into(),
                description: None,
                evidence_required: vec!["repo".into()],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    task.id
}

#[tokio::test]
async fn task_get_rejects_invalid_status() {
    let (dur, _dir) = fresh().await;
    let task_id = seed_task(&dur).await;

    sqlx::query("UPDATE tasks SET status = 'bogus-status' WHERE id = ?")
        .bind(&task_id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let err = dur.tasks().get(&task_id).await.expect_err(
        "tasks().get() must refuse to return a row whose persisted status is not a TaskStatus variant",
    );
    assert!(
        matches!(err, DurabilityError::NotFound { entity, .. } if entity == "task_status"),
        "expected a task_status NotFound projection error, got: {err}",
    );
}

#[tokio::test]
async fn task_get_rejects_invalid_evidence_required_json() {
    let (dur, _dir) = fresh().await;
    let task_id = seed_task(&dur).await;

    // Not valid JSON at all.
    sqlx::query("UPDATE tasks SET evidence_required = '{not-json' WHERE id = ?")
        .bind(&task_id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let err = dur.tasks().get(&task_id).await.expect_err(
        "tasks().get() must refuse to silently return [] when evidence_required is corrupted",
    );
    assert!(
        matches!(err, DurabilityError::Json(_)),
        "expected a Json decode error, got: {err}",
    );
}

#[tokio::test]
async fn agent_register_propagates_decode_error_on_corrupt_existing_row() {
    // agents.rs:91 / agent_facade.rs:26 used `.ok()` on get(), which
    // hid every decoder failure as "agent absent" and made the
    // facade silently re-INSERT (then hit a unique-constraint error)
    // or, on the facade side, emit an extra `agent.session_started`
    // audit row from a failed lookup.
    let (dur, _dir) = fresh().await;
    dur.register_agent(NewAgent {
        id: "a1".into(),
        kind: "copilot".into(),
        name: None,
        host: None,
        capabilities: vec!["code".into()],
        metadata: json!({"pid": 1}),
    })
    .await
    .unwrap();

    sqlx::query("UPDATE agents SET metadata = '{broken' WHERE id = 'a1'")
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let err = dur
        .register_agent(NewAgent {
            id: "a1".into(),
            kind: "copilot".into(),
            name: None,
            host: None,
            capabilities: vec!["code".into()],
            metadata: json!({"pid": 2}),
        })
        .await
        .expect_err("register_agent must propagate decoder failures on the prior-agent lookup");
    assert!(
        matches!(err, DurabilityError::Json(_)),
        "expected the JSON decode error to surface, got: {err}",
    );
}

#[tokio::test]
async fn recent_audit_for_agent_rejects_invalid_timestamp() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(NewAgent {
        id: "a1".into(),
        kind: "copilot".into(),
        name: None,
        host: None,
        capabilities: vec![],
        metadata: json!({}),
    })
    .await
    .unwrap();

    // Corrupt the audit row's created_at, which the projection in
    // agent_queries.rs reads via the shared `parse_ts` helper.
    sqlx::query("UPDATE audit_log SET created_at = 'not-a-timestamp' WHERE agent_id = 'a1'")
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let store = AgentStore::new(dur.pool().clone());
    let err = store.recent_audit_for_agent("a1", 5).await.expect_err(
        "recent_audit_for_agent() must refuse to silently substitute Utc::now() for a corrupt audit_log.created_at",
    );
    assert!(
        matches!(err, DurabilityError::NotFound { entity, .. } if entity == "timestamp"),
        "expected a timestamp NotFound projection error, got: {err}",
    );
}

#[tokio::test]
async fn agent_claimed_tasks_rejects_invalid_timestamp() {
    let (dur, _dir) = fresh().await;
    let task_id = seed_task(&dur).await;
    dur.register_agent(NewAgent {
        id: "a1".into(),
        kind: "copilot".into(),
        name: None,
        host: None,
        capabilities: vec![],
        metadata: json!({}),
    })
    .await
    .unwrap();
    // Claim the task by agent in a status the projection counts.
    sqlx::query(
        "UPDATE tasks SET agent_id = 'a1', status = 'in_progress', updated_at = ? WHERE id = ?",
    )
    .bind("not-rfc3339")
    .bind(&task_id)
    .execute(dur.pool().inner())
    .await
    .unwrap();

    let store = AgentStore::new(dur.pool().clone());
    let err = store
        .claimed_tasks_for_agent("a1", 5)
        .await
        .expect_err("claimed_tasks_for_agent() must refuse to silently substitute Utc::now() for a corrupt updated_at");
    assert!(
        matches!(err, DurabilityError::NotFound { entity, .. } if entity == "timestamp"),
        "expected a timestamp NotFound projection error, got: {err}",
    );
}
