//! W10 Cost-of-Pass integration tests.

mod support;

use convergio_executor::cost_tracker::{load_cost_stats, record_cost, TaskCost};
use support::fresh;
use uuid::Uuid;

/// A cost row written via `record_cost` must appear in the audit log and
/// be counted when `load_cost_stats` is queried.
#[tokio::test]
async fn records_cost_on_task_done() {
    let (_exec, dur, _dir) = fresh().await;

    let cost = TaskCost {
        task_id: Uuid::new_v4(),
        runner_kind: "claude:sonnet".into(),
        elapsed_secs: 12.5,
        tokens: Some(1024),
        passed: true,
    };
    record_cost(&dur, cost).await.unwrap();

    let events = dur.audit().list_since(0, 1000).await.unwrap();
    let row = events
        .iter()
        .find(|e| e.transition == "task.cost_recorded")
        .expect("task.cost_recorded audit row should exist");
    let payload: serde_json::Value = serde_json::from_str(&row.payload).unwrap();
    assert_eq!(payload["runner_kind"], "claude:sonnet");
    assert!((payload["elapsed_secs"].as_f64().unwrap() - 12.5).abs() < 1e-9);
    assert_eq!(payload["passed"], true);
}

/// When no cost rows exist, `load_cost_stats` must return an empty map.
#[tokio::test]
async fn load_cost_stats_returns_empty_for_empty_db() {
    let (_exec, dur, _dir) = fresh().await;

    let stats = load_cost_stats(&dur).await.unwrap();
    assert!(
        stats.is_empty(),
        "expected empty map, got {:?} entries",
        stats.len()
    );
}

/// Recording 3 cost rows for the same runner kind (2 passes, 1 fail)
/// should aggregate correctly: pass_rate ≈ 0.667, sample_count = 3.
#[tokio::test]
async fn cost_stats_aggregate_multiple_runs() {
    let (_exec, dur, _dir) = fresh().await;

    for passed in [true, true, false] {
        let cost = TaskCost {
            task_id: Uuid::new_v4(),
            runner_kind: "claude:sonnet".into(),
            elapsed_secs: 10.0,
            tokens: None,
            passed,
        };
        record_cost(&dur, cost).await.unwrap();
    }

    let stats = load_cost_stats(&dur).await.unwrap();
    let s = stats
        .get("claude:sonnet")
        .expect("claude:sonnet should be present");

    assert_eq!(s.sample_count, 3);
    assert!(
        (s.pass_rate - 2.0 / 3.0).abs() < 1e-9,
        "pass_rate should be ≈0.667, got {}",
        s.pass_rate
    );
    assert!(
        (s.avg_elapsed_secs - 10.0).abs() < 1e-9,
        "avg_elapsed_secs should be 10.0, got {}",
        s.avg_elapsed_secs
    );
}
