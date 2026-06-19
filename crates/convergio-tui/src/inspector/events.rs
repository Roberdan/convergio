//! Inspector Events panel.
//!
//! Renders `GET /v1/ontology/events` with no params: the
//! transaction-current bitemporal snapshot, one event per ontology
//! object. Each row shows the operation, object id, valid-time start
//! and transaction-time start. The operation is conveyed by a glyph as
//! well as a label so it reads without colour (CONSTITUTION P3). The
//! panel is strictly read-only — the TUI never writes events.

use crate::client_ontology::OntologyEventRow;
use crate::inspector::render::render_state;
use crate::inspector::InspectorState;
use crate::render::pane_block;
use crate::text_util::truncate;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Render the Events pane.
pub fn render(f: &mut Frame, area: Rect, state: &InspectorState, focused: bool) {
    let count = state.events.value().map(|e| e.len()).unwrap_or(0);
    let title = format!(" Events ({count}) [{}] ", state.events.status_word());
    let block = pane_block(&title, focused);
    if !render_state(f, area, block.clone(), &state.events) {
        return;
    }
    let Some(events) = state.events.value() else {
        return;
    };
    if events.is_empty() {
        let hint = "no ontology events";
        f.render_widget(
            Paragraph::new(Line::styled(hint, theme::dim())).block(block),
            area,
        );
        return;
    }

    let last = events.len().saturating_sub(1);
    let selected = state.events_cursor.selected.min(last);
    let items: Vec<ListItem> = events
        .iter()
        .enumerate()
        .map(|(idx, e)| ListItem::new(event_line(e, idx == selected)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

/// `(glyph, colour)` for an event operation. The glyph encodes the
/// operation so colour is never the only signal (CONSTITUTION P3).
fn op_badge(op: &str) -> (&'static str, Style) {
    match op {
        "upsert" => ("✚", Style::default().fg(theme::SUCCESS)),
        "delete" => ("✖", Style::default().fg(theme::DANGER)),
        _ => ("·", theme::dim()),
    }
}

/// Pure mapping of one event into its four display columns:
/// `(object_id, op, valid_from, tx_from)`. Timestamps are trimmed to
/// minute precision and `T` is swapped for a space, matching the other
/// panels. Kept separate from ratatui so it stays unit-testable.
fn event_cells(e: &OntologyEventRow) -> (String, String, String, String) {
    (
        truncate(&e.object_id, 18).to_string(),
        truncate(&e.op, 8).to_string(),
        short_time(&e.valid_from),
        short_time(&e.tx_from),
    )
}

fn event_line(e: &OntologyEventRow, selected: bool) -> Line<'static> {
    let accent = if selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    let (glyph, style) = op_badge(&e.op);
    let (object_id, op, valid_from, tx_from) = event_cells(e);
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(glyph, style),
        Span::raw(" "),
        Span::styled(format!("{op:8}"), style),
        Span::raw(" "),
        Span::styled(format!("{object_id:18}"), theme::text()),
        Span::raw(" "),
        Span::styled(format!("v:{valid_from}"), theme::dim()),
        Span::raw(" "),
        Span::styled(format!("t:{tx_from}"), theme::dim()),
    ])
}

fn short_time(raw: &str) -> String {
    raw.get(..16).unwrap_or(raw).replace('T', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::Load;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn event(object_id: &str, op: &str) -> OntologyEventRow {
        OntologyEventRow {
            object_id: object_id.into(),
            op: op.into(),
            payload: serde_json::json!({"v": 1}),
            valid_from: "2026-06-01T10:00:00+00:00".into(),
            valid_to: None,
            tx_from: "2026-06-01T10:00:01+00:00".into(),
            tx_to: None,
        }
    }

    #[test]
    fn op_badge_maps_each_operation() {
        assert_eq!(op_badge("upsert").0, "✚");
        assert_eq!(op_badge("delete").0, "✖");
        assert_eq!(op_badge("???").0, "·");
    }

    #[test]
    fn events_pane_label_and_focused_len() {
        use crate::inspector::InspectorPane;
        assert_eq!(InspectorPane::Events.label(), "Events");
        let s = InspectorState {
            focus: InspectorPane::Events,
            events: Load::Loaded(vec![event("a", "upsert"), event("b", "delete")]),
            ..Default::default()
        };
        assert_eq!(s.focused_len(), 2);
    }

    #[test]
    fn event_cells_maps_columns_from_sample_event() {
        let (object_id, op, valid_from, tx_from) = event_cells(&event("person:42", "upsert"));
        assert_eq!(object_id, "person:42");
        assert_eq!(op, "upsert");
        assert_eq!(valid_from, "2026-06-01 10:00");
        assert_eq!(tx_from, "2026-06-01 10:00");
    }

    #[test]
    fn event_cells_truncate_long_object_id() {
        let (object_id, _, _, _) = event_cells(&event("0123456789abcdefghij", "upsert"));
        assert_eq!(object_id.chars().count(), 18);
    }

    #[test]
    fn render_lists_events_with_op() {
        let backend = TestBackend::new(80, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            events: Load::Loaded(vec![event("person:42", "upsert")]),
            ..Default::default()
        };
        term.draw(|f| render(f, f.area(), &state, true)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("Events (1)"));
        assert!(dump.contains("person:42"));
        assert!(dump.contains("upsert"));
    }

    #[test]
    fn render_empty_shows_hint() {
        let backend = TestBackend::new(60, 4);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            events: Load::Loaded(vec![]),
            ..Default::default()
        };
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("no ontology events"));
    }

    #[test]
    fn render_shows_error_placeholder() {
        let backend = TestBackend::new(50, 4);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            events: Load::Error("boom".into()),
            ..Default::default()
        };
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("error"));
        assert!(dump.contains("boom"));
    }
}
