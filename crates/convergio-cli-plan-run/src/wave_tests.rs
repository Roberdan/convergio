//! Unit tests for the wave orchestrator. Kept in a sibling file so
//! `wave.rs` stays under the per-file Rust cap.

use super::*;
use anyhow::anyhow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn meta(wave: i64, seq: i64) -> TaskMeta {
    TaskMeta {
        id: format!("t{wave}{seq}"),
        title: "x".into(),
        wave,
        sequence: seq,
    }
}

#[test]
fn group_by_wave_partitions_and_keeps_order() {
    let waves = group_by_wave(vec![meta(1, 1), meta(1, 2), meta(2, 1)]);
    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0].len(), 2);
    assert_eq!(waves[1][0].sequence, 1);
}

#[test]
fn collect_pending_filters_and_sorts() {
    let raw = json!([
        {"id":"a","title":"A","status":"done","wave":1,"sequence":1},
        {"id":"b","title":"B","status":"pending","wave":2,"sequence":1},
        {"id":"c","title":"C","status":"pending","wave":1,"sequence":2},
    ]);
    let ids: Vec<String> = collect_pending_in_wave_order(&raw)
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(ids, ["c", "b"]);
}

#[test]
fn transition_body_carries_agent_id_when_present() {
    assert!(transition_body(None, "submitted").get("agent_id").is_none());
    assert_eq!(transition_body(Some("a"), "in_progress")["agent_id"], "a");
}

/// Regression test for audit finding `src/runner.rs:183` (medium):
/// when one in-flight submission fails, every other already-scheduled
/// submission must still complete before the wave returns, so no
/// daemon-side task is stranded mid-transition.
#[tokio::test]
async fn run_wave_with_drains_in_flight_submissions_after_failure() {
    let completed = Arc::new(AtomicUsize::new(0));
    let tasks: Vec<TaskMeta> = (0..3).map(|i| meta(1, i)).collect();
    let completed_for_fn = completed.clone();
    let outcomes = run_wave_with(3, tasks, move |task| {
        let completed = completed_for_fn.clone();
        async move {
            // task seq 0 fails fast; the other two run longer than the
            // failure latency. A correct drain awaits them.
            if task.sequence == 0 {
                completed.fetch_add(1, Ordering::SeqCst);
                return SubmitOutcome {
                    task,
                    transition: Err(anyhow!("boom")),
                    bus_warning: None,
                };
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            completed.fetch_add(1, Ordering::SeqCst);
            SubmitOutcome {
                task,
                transition: Ok(()),
                bus_warning: None,
            }
        }
    })
    .await;

    assert_eq!(
        completed.load(Ordering::SeqCst),
        3,
        "every in-flight submission must complete (no abandoned in_progress claims)"
    );
    assert_eq!(
        outcomes.len(),
        3,
        "the wave must surface every in-flight outcome to the caller"
    );
}

/// Regression test for audit finding `src/runner.rs:230` (low):
/// when the plan-scoped bus publish step fails, `submit_one_inner`
/// must surface the error as a non-fatal warning instead of silently
/// dropping it. Daemon transitions still succeed.
#[tokio::test]
async fn submit_one_inner_surfaces_bus_publish_failure_as_warning() {
    let (transition, bus_warning) = submit_one_inner(
        || async { anyhow::Ok(json!({})) },
        || async { anyhow::Ok(json!({})) },
        || async { Err::<Value, _>(anyhow!("publish 500")) },
    )
    .await;
    assert!(transition.is_ok(), "transition succeeded");
    let warning = bus_warning.expect("bus publish failure must surface as a warning");
    assert!(warning.to_string().contains("publish 500"));
}
