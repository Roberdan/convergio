//! Integration tests that verify the help-footer key hints change
//! correctly when the TUI switches between Overview and Detail modes.
//!
//! Uses `ratatui::backend::TestBackend` so no terminal is required.

use convergio_tui::client::Plan;
use convergio_tui::render;
use convergio_tui::state::{AppMode, AppState, DetailTarget};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Render the full root frame and return the cell contents as a flat string.
fn dump_root(width: u16, height: u16, state: &AppState) -> String {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| render::root(f, state)).unwrap();
    term.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn footer_shows_overview_keys_in_overview_mode() {
    let state = AppState::default();
    // AppMode::Overview is the default — no explicit assignment needed.
    let d = dump_root(120, 30, &state);
    assert!(
        d.contains("Enter scope"),
        "Overview footer must contain 'Enter scope': {d:?}"
    );
    assert!(
        d.contains("Tab pane"),
        "Overview footer must contain 'Tab pane': {d:?}"
    );
    assert!(
        d.contains("e exited"),
        "Overview footer must contain 'e exited': {d:?}"
    );
    assert!(
        !d.contains("Esc back"),
        "Overview footer must NOT contain 'Esc back': {d:?}"
    );
}

#[test]
fn footer_shows_detail_keys_in_detail_mode() {
    let state = AppState {
        plans: vec![Plan {
            id: "p1".into(),
            title: "test plan".into(),
            status: "active".into(),
            created_at: "2026-05-07T10:00:00Z".into(),
            updated_at: "2026-05-07T10:00:00Z".into(),
            ..Plan::default()
        }],
        mode: AppMode::Detail(DetailTarget::Plan {
            id: "p1".into(),
            title: "test plan".into(),
        }),
        ..AppState::default()
    };
    let d = dump_root(120, 30, &state);
    assert!(
        d.contains("Esc back"),
        "Detail footer must contain 'Esc back': {d:?}"
    );
    assert!(
        d.contains("j/k scroll"),
        "Detail footer must contain 'j/k scroll': {d:?}"
    );
    assert!(
        !d.contains("Enter scope"),
        "Detail footer must NOT contain 'Enter scope': {d:?}"
    );
    assert!(
        !d.contains("Tab pane"),
        "Detail footer must NOT contain 'Tab pane': {d:?}"
    );
}
