//! Thor pipeline hardening tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use convergio_thor::{Thor, Verdict};
use std::time::Duration;
use tempfile::tempdir;

async fn fresh_with_pipeline(
    cmd: &str,
    timeout: Duration,
) -> (Thor, Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);
    (
        Thor::with_pipeline_timeout(dur.clone(), Some(cmd.to_string()), timeout),
        dur,
        dir,
    )
}

async fn submitted_plan(dur: &Durability) -> (String, String) {
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
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
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::Submitted, Some("a"))
        .await
        .unwrap();
    (plan.id, task.id)
}

#[tokio::test]
async fn pipeline_timeout_blocks_promotion() {
    let (thor, dur, _dir) = fresh_with_pipeline("sleep 1", Duration::from_millis(50)).await;
    let (plan_id, task_id) = submitted_plan(&dur).await;

    let verdict = thor.validate(&plan_id).await.unwrap();

    match verdict {
        Verdict::Fail { reasons } => {
            let joined = reasons.join("\n");
            assert!(joined.contains("timed out after"), "joined: {joined}");
        }
        Verdict::Pass => panic!("timed out pipeline must fail"),
    }
    let task = dur.tasks().get(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Submitted);
}

#[tokio::test]
async fn pipeline_truncation_marker_is_included() {
    let cmd = "yes X | head -n 5000; echo SENTINEL_TAIL >&2; exit 3";
    let (thor, dur, _dir) = fresh_with_pipeline(cmd, Duration::from_secs(5)).await;
    let (plan_id, _task_id) = submitted_plan(&dur).await;

    let verdict = thor.validate(&plan_id).await.unwrap();

    match verdict {
        Verdict::Fail { reasons } => {
            let joined = reasons.join("\n");
            assert!(
                joined.contains("[pipeline output truncated; showing last 4096 bytes]"),
                "joined: {joined}"
            );
            assert!(joined.contains("SENTINEL_TAIL"), "joined: {joined}");
        }
        Verdict::Pass => panic!("failing pipeline must fail"),
    }
}
