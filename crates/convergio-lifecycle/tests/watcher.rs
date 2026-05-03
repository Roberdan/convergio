//! OS-watcher integration tests.

use convergio_db::Pool;
use convergio_lifecycle::{init, watcher, ProcessStatus, SpawnSpec, Supervisor};
use tempfile::tempdir;

async fn fresh() -> (Supervisor, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Supervisor::new(pool), dir)
}

#[tokio::test]
async fn watcher_flips_dead_pid_to_exited() {
    let (sup, _dir) = fresh().await;

    // Spawn /bin/echo — it exits immediately with status 0. Wait a beat
    // so the OS reaps it, then run a tick.
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

    // Give the OS time to actually exit the child.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let n = watcher::tick(&sup).await.unwrap();
    assert_eq!(n, 1);

    let after = sup.get(&proc.id).await.unwrap();
    assert_eq!(after.status, ProcessStatus::Exited);
    assert!(after.ended_at.is_some());
}

#[tokio::test]
async fn watcher_does_not_flip_self_pid() {
    let (sup, _dir) = fresh().await;

    // Insert a fake "running" row whose pid is our own — definitely alive.
    let pid = std::process::id() as i64;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agent_processes (id, kind, command, plan_id, task_id, pid, \
         status, exit_code, last_heartbeat_at, started_at, ended_at) \
         VALUES (?, 'shell', '/bin/sleep', NULL, NULL, ?, 'running', NULL, NULL, ?, NULL)",
    )
    .bind(&id)
    .bind(pid)
    .bind(&now)
    .execute(sup.pool().inner())
    .await
    .unwrap();

    let n = watcher::tick(&sup).await.unwrap();
    assert_eq!(n, 0, "watcher must not flip its own process");

    let row = sup.get(&id).await.unwrap();
    assert_eq!(row.status, ProcessStatus::Running);
}

#[tokio::test]
async fn watcher_skips_processes_without_pid() {
    let (sup, _dir) = fresh().await;

    // Insert a row with pid=NULL still in 'running' (shouldn't really
    // happen — spawn always sets pid before flipping to running — but
    // the watcher must be robust).
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agent_processes (id, kind, command, plan_id, task_id, pid, \
         status, exit_code, last_heartbeat_at, started_at, ended_at) \
         VALUES (?, 'shell', '/bin/sleep', NULL, NULL, NULL, 'running', NULL, NULL, ?, NULL)",
    )
    .bind(&id)
    .bind(&now)
    .execute(sup.pool().inner())
    .await
    .unwrap();

    // Without a pid we cannot probe — the watcher considers it dead
    // and flips it.
    let n = watcher::tick(&sup).await.unwrap();
    assert_eq!(n, 1);

    let row = sup.get(&id).await.unwrap();
    assert_eq!(row.status, ProcessStatus::Exited);
}
