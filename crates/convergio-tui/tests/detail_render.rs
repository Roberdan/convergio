//! Integration tests for the drill-down panel.
//!
//! Hosted in `tests/` so the (already 250+) `panes/detail.rs` stays
//! under the 300-line cap. Uses only the public surface of
//! `convergio-tui` — `AppState` + `DetailTarget` + `panes::detail::render`.

use convergio_tui::client::{Plan, PrSummary, RegistryAgent, TaskSummary};
use convergio_tui::panes::detail;
use convergio_tui::state::{AppState, DetailTarget};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn task(id: &str, plan_id: &str, status: &str, title: &str) -> TaskSummary {
    TaskSummary {
        id: id.into(),
        plan_id: plan_id.into(),
        title: title.into(),
        status: status.into(),
        agent_id: None,
        created_at: "2026-05-02T18:00:00Z".into(),
        updated_at: "2026-05-02T18:00:00Z".into(),
        ..TaskSummary::default()
    }
}

fn dump(width: u16, height: u16, state: &AppState, target: &DetailTarget) -> String {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| detail::render(f, f.area(), state, target))
        .unwrap();
    term.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn plan_detail_lists_status_and_tasks() {
    let state = AppState {
        plans: vec![Plan {
            id: "p1".into(),
            title: "drill plan".into(),
            project: Some("convergio".into()),
            status: "active".into(),
            created_at: "2026-05-02T18:00:00Z".into(),
            updated_at: "2026-05-02T18:00:00Z".into(),
            ..Plan::default()
        }],
        tasks: vec![task("t1", "p1", "in_progress", "active task")],
        detail_tasks: vec![
            task("t1", "p1", "in_progress", "active task"),
            task("t2", "p1", "done", "closed task"),
        ],
        ..AppState::default()
    };
    let target = DetailTarget::Plan {
        id: "p1".into(),
        title: "drill plan".into(),
    };
    let d = dump(120, 18, &state, &target);
    assert!(d.contains("drill plan"));
    assert!(d.contains("active"));
    assert!(d.contains("Tasks (2)"));
    assert!(d.contains("active task"));
    assert!(d.contains("closed task"));
}

#[test]
fn task_detail_shows_status_and_plan_breadcrumb() {
    let state = AppState {
        tasks: vec![TaskSummary {
            id: "t9".into(),
            plan_id: "p2".into(),
            title: "scaffold".into(),
            status: "submitted".into(),
            agent_id: Some("claude-code".into()),
            created_at: "2026-05-02T18:00:00Z".into(),
            updated_at: "2026-05-02T18:00:00Z".into(),
            ..TaskSummary::default()
        }],
        ..AppState::default()
    };
    let target = DetailTarget::Task {
        id: "t9".into(),
        plan_id: "p2".into(),
        title: "scaffold".into(),
    };
    let d = dump(100, 12, &state, &target);
    assert!(d.contains("submitted"));
    assert!(d.contains("claude-code"));
    assert!(d.contains("p2"));
}

#[test]
fn agent_detail_lists_owned_tasks() {
    let state = AppState {
        agents: vec![RegistryAgent {
            id: "claude-code".into(),
            kind: "claude".into(),
            status: Some("idle".into()),
            last_heartbeat_at: Some("2026-05-02T20:00:00Z".into()),
            ..RegistryAgent::default()
        }],
        tasks: vec![TaskSummary {
            id: "t1".into(),
            plan_id: "p1".into(),
            title: "owned".into(),
            status: "in_progress".into(),
            agent_id: Some("claude-code".into()),
            created_at: "2026-05-02T18:00:00Z".into(),
            updated_at: "2026-05-02T18:00:00Z".into(),
            ..TaskSummary::default()
        }],
        ..AppState::default()
    };
    let target = DetailTarget::Agent {
        id: "claude-code".into(),
    };
    let d = dump(110, 14, &state, &target);
    assert!(d.contains("claude"));
    assert!(d.contains("Active tasks (1)"));
    assert!(d.contains("owned"));
}

#[test]
fn pr_detail_shows_branch_and_ci() {
    let state = AppState {
        prs: vec![PrSummary {
            number: 42,
            title: "header polish".into(),
            head_ref_name: "feat/header".into(),
            ci: "success".into(),
            state: "OPEN".into(),
            ..PrSummary::default()
        }],
        ..AppState::default()
    };
    let target = DetailTarget::Pr {
        number: 42,
        title: "header polish".into(),
    };
    let d = dump(100, 10, &state, &target);
    assert!(d.contains("feat/header"));
    assert!(d.contains("success"));
}
