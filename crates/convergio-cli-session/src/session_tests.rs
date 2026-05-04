//! Unit tests for [`super::session`]. Split out to keep `session.rs`
//! under the 300-line cap mandated by lefthook.
//!
//! Wired from `session.rs` via
//! `#[cfg(test)] #[path = "session_tests.rs"] mod tests;` so the
//! `use super::*;` import below pulls in the surrounding module's
//! private items (e.g. the free function `top_pending`).

use super::*;

fn task(status: &str, wave: i64, sequence: i64) -> Task {
    Task {
        id: format!("id-{wave}-{sequence}"),
        title: format!("t{wave}.{sequence}"),
        status: status.into(),
        wave,
        sequence,
        created_at: "2026-01-01T00:00:00Z".into(),
    }
}

#[test]
fn counts_groups_by_status() {
    let tasks = vec![
        task("done", 1, 1),
        task("pending", 1, 2),
        task("pending", 2, 1),
        task("in_progress", 1, 3),
        task("submitted", 1, 4),
        task("failed", 3, 1),
    ];
    let c = TaskCounts::from(tasks.as_slice());
    assert_eq!(c.total, 6);
    assert_eq!(c.done, 1);
    assert_eq!(c.pending, 2);
    assert_eq!(c.in_progress, 1);
    assert_eq!(c.submitted, 1);
    assert_eq!(c.failed, 1);
}

#[test]
fn top_pending_orders_by_wave_then_sequence() {
    let tasks = vec![
        task("pending", 2, 1),
        task("done", 1, 1),
        task("pending", 1, 5),
        task("pending", 1, 2),
    ];
    let next = top_pending(&tasks, 10);
    let order: Vec<String> = next.iter().map(|t| t.title.clone()).collect();
    assert_eq!(order, vec!["t1.2", "t1.5", "t2.1"]);
}

#[test]
fn top_pending_respects_limit() {
    let tasks: Vec<Task> = (0..10).map(|i| task("pending", 1, i)).collect();
    let next = top_pending(&tasks, 3);
    assert_eq!(next.len(), 3);
}
