//! Integration tests for `convergio_tui::client::sort_plans_by_status`.
//!
//! Lives in `tests/` so `client.rs` stays under the 300-line cap.

use convergio_tui::client::{sort_plans_by_status, Plan};

fn plan(id: &str, status: &str, updated: &str) -> Plan {
    Plan {
        id: id.into(),
        title: id.into(),
        project: None,
        status: status.into(),
        created_at: updated.into(),
        updated_at: updated.into(),
        ..Plan::default()
    }
}

#[test]
fn sort_plans_groups_active_first_completed_then_cancelled() {
    let mut ps = vec![
        plan("p1", "completed", "2026-05-02T10:00:00Z"),
        plan("p2", "active", "2026-05-02T09:00:00Z"),
        plan("p3", "cancelled", "2026-05-02T11:00:00Z"),
        plan("p4", "draft", "2026-05-02T12:00:00Z"),
        plan("p5", "active", "2026-05-02T08:00:00Z"),
    ];
    sort_plans_by_status(&mut ps);
    let order: Vec<&str> = ps.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(order, vec!["p2", "p5", "p4", "p1", "p3"]);
}

#[test]
fn sort_plans_breaks_ties_by_updated_at_desc() {
    let mut ps = vec![
        plan("old", "active", "2026-05-01T00:00:00Z"),
        plan("new", "active", "2026-05-02T00:00:00Z"),
    ];
    sort_plans_by_status(&mut ps);
    assert_eq!(ps[0].id, "new");
}
