//! Layer 3 integration tests — spawn `/bin/echo` and friends.

use convergio_db::Pool;
use convergio_lifecycle::{init, LifecycleError, ProcessStatus, SpawnSpec, Supervisor};
use std::time::Duration;
use tempfile::tempdir;

async fn fresh_supervisor() -> (Supervisor, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Supervisor::new(pool), dir)
}

#[tokio::test]
async fn spawn_records_running_process_with_pid() {
    let (sup, _dir) = fresh_supervisor().await;

    let proc = sup
        .spawn(SpawnSpec {
            kind: "shell".into(),
            command: "/bin/echo".into(),
            args: vec!["hello-from-test".into()],
            env: vec![],
            plan_id: Some("plan-1".into()),
            task_id: None,
            cwd: None,
            stdin_payload: None,
        })
        .await
        .unwrap();

    assert_eq!(proc.kind, "shell");
    assert_eq!(proc.status, ProcessStatus::Running);
    assert!(proc.pid.is_some());

    // Round-trip via the DB.
    let same = sup.get(&proc.id).await.unwrap();
    assert_eq!(same.id, proc.id);
    assert_eq!(same.command, "/bin/echo");
}

#[tokio::test]
async fn spawn_failure_returns_error() {
    let (sup, _dir) = fresh_supervisor().await;
    let err = sup
        .spawn(SpawnSpec {
            kind: "shell".into(),
            command: "/no/such/binary/please".into(),
            args: vec![],
            env: vec![],
            plan_id: None,
            task_id: None,
            cwd: None,
            stdin_payload: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, LifecycleError::SpawnFailed(_)));
    let (status, ended_at): (String, Option<String>) =
        sqlx::query_as("SELECT status, ended_at FROM agent_processes WHERE command = ? LIMIT 1")
            .bind("/no/such/binary/please")
            .fetch_one(sup.pool().inner())
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(ended_at.is_some());
}

#[tokio::test]
async fn spawn_timeout_returns_error_before_recording_process() {
    let (sup, _dir) = fresh_supervisor().await;
    let err = sup
        .spawn_with_timeout(
            SpawnSpec {
                kind: "shell".into(),
                command: "/bin/echo".into(),
                args: vec!["timeout".into()],
                env: vec![],
                plan_id: None,
                task_id: None,
                cwd: None,
                stdin_payload: None,
            },
            Duration::ZERO,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, LifecycleError::SpawnTimedOut { .. }));
    let (rows,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_processes")
        .fetch_one(sup.pool().inner())
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn heartbeat_touches_row() {
    let (sup, _dir) = fresh_supervisor().await;
    let proc = sup
        .spawn(SpawnSpec {
            kind: "shell".into(),
            command: "/bin/echo".into(),
            args: vec!["hb".into()],
            env: vec![],
            plan_id: None,
            task_id: None,
            cwd: None,
            stdin_payload: None,
        })
        .await
        .unwrap();

    sup.heartbeat(&proc.id).await.unwrap();
    let after = sup.get(&proc.id).await.unwrap();
    assert!(after.last_heartbeat_at.is_some());

    // Unknown id is NotFound.
    let err = sup.heartbeat("nope").await.unwrap_err();
    assert!(matches!(err, LifecycleError::NotFound(_)));
}

#[tokio::test]
async fn mark_exited_records_exit_code() {
    let (sup, _dir) = fresh_supervisor().await;
    let proc = sup
        .spawn(SpawnSpec {
            kind: "shell".into(),
            command: "/bin/echo".into(),
            args: vec!["bye".into()],
            env: vec![],
            plan_id: None,
            task_id: None,
            cwd: None,
            stdin_payload: None,
        })
        .await
        .unwrap();

    sup.mark_exited(&proc.id, Some(0), true).await.unwrap();
    let after = sup.get(&proc.id).await.unwrap();
    assert_eq!(after.status, ProcessStatus::Exited);
    assert_eq!(after.exit_code, Some(0));
    assert!(after.ended_at.is_some());
}

#[tokio::test]
async fn invalid_started_timestamp_is_data_error_not_not_found() {
    let (sup, _dir) = fresh_supervisor().await;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agent_processes (id, kind, command, plan_id, task_id, pid, \
         status, exit_code, last_heartbeat_at, started_at, ended_at) \
         VALUES (?, 'shell', '/bin/echo', NULL, NULL, NULL, 'running', NULL, NULL, ?, NULL)",
    )
    .bind(&id)
    .bind("not-a-timestamp")
    .execute(sup.pool().inner())
    .await
    .unwrap();

    let err = sup.get(&id).await.unwrap_err();
    assert!(matches!(
        err,
        LifecycleError::InvalidTimestamp {
            field: "started_at",
            ..
        }
    ));
}
