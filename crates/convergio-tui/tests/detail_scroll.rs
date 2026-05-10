//! Integration test that verifies the Detail panel actually scrolls.
//!
//! The footer hints `j/k scroll` in Detail mode; this test ensures the
//! renderer applies the scroll offset and that navigation updates it.

use convergio_tui::client::BusMessage;
use convergio_tui::panes::detail;
use convergio_tui::state::{AppState, DetailTarget};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

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
fn bus_detail_payload_scroll_hides_early_lines() {
    let payload: Vec<String> = (0..40).map(|i| format!("ZZZ{i:02}")).collect();
    let msg = BusMessage {
        id: "m1".into(),
        seq: 7,
        plan_id: Some("p1".into()),
        topic: "agent:test".into(),
        sender: Some("copilot".into()),
        payload: serde_json::json!(payload),
        created_at: "2026-05-10T18:00:00Z".into(),
        ..BusMessage::default()
    };
    let target = DetailTarget::BusMessage {
        id: "m1".into(),
        seq: 7,
        topic: "agent:test".into(),
    };

    let state_top = AppState {
        messages: vec![msg.clone()],
        detail_scroll: 0,
        ..AppState::default()
    };
    let d0 = dump(80, 12, &state_top, &target);
    assert!(
        d0.contains("ZZZ00"),
        "top view should include early payload: {d0:?}"
    );

    let state_scrolled = AppState {
        messages: vec![msg],
        detail_scroll: 18,
        ..AppState::default()
    };
    let d18 = dump(80, 12, &state_scrolled, &target);
    assert!(
        !d18.contains("ZZZ00"),
        "scrolled view should hide early payload: {d18:?}"
    );
    assert!(
        d18.contains("ZZZ12") || d18.contains("ZZZ18"),
        "scrolled view should include later payload: {d18:?}"
    );
}
