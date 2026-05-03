//! Planner integration tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_planner::{Planner, PlannerError, PlannerMode};
use tempfile::tempdir;

async fn fresh() -> (Planner, Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    (planner, dur, dir)
}

#[tokio::test]
async fn solve_creates_plan_and_one_task_per_line() {
    let (planner, dur, _dir) = fresh().await;
    let id = planner
        .solve("ship the mvp\nwrite docs\nopen the source")
        .await
        .unwrap();

    let plan = dur.plans().get(&id).await.unwrap();
    assert_eq!(plan.title, "ship the mvp");
    assert!(plan.description.unwrap().contains("write docs"));

    let tasks = dur.tasks().list_by_plan(&id).await.unwrap();
    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].title, "ship the mvp");
    assert_eq!(tasks[2].title, "open the source");
    for t in &tasks {
        assert_eq!(t.wave, 1);
    }
}

#[tokio::test]
async fn solve_handles_single_line_mission() {
    let (planner, dur, _dir) = fresh().await;
    let id = planner.solve("just one task").await.unwrap();
    let plan = dur.plans().get(&id).await.unwrap();
    assert_eq!(plan.title, "just one task");
    assert!(plan.description.is_none());

    let tasks = dur.tasks().list_by_plan(&id).await.unwrap();
    assert_eq!(tasks.len(), 1);
}

#[tokio::test]
async fn solve_strips_blank_lines() {
    let (planner, _dur, _dir) = fresh().await;
    let id = planner.solve("\n  \nfirst\n\nsecond\n  ").await.unwrap();
    let tasks = _dur.tasks().list_by_plan(&id).await.unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].title, "first");
    assert_eq!(tasks[1].title, "second");
}

#[tokio::test]
async fn solve_empty_mission_errors() {
    let (planner, _dur, _dir) = fresh().await;
    let err = planner.solve("\n  \n\n").await.unwrap_err();
    matches!(err, PlannerError::EmptyMission);
}

#[tokio::test]
async fn solve_writes_audit_chain_that_verifies() {
    let (planner, dur, _dir) = fresh().await;
    planner.solve("a\nb\nc").await.unwrap();
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok);
    // 1 plan.created + 3 task.created = 4 rows.
    assert!(report.checked >= 4);
}
