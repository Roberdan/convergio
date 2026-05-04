//! Agent bus pane.

use crate::client::BusMessage;
use crate::render::pane_block;
use crate::state::AppState;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Render the bus messages pane.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let scoped = state.scoped_messages();
    let scope_crumb = state
        .scoped_plan_title()
        .map(|t| format!(" · {}", short(t, 24)))
        .unwrap_or_default();
    let title = format!(" Bus ({}){scope_crumb} ", scoped.len());
    let block = pane_block(&title, focused);

    let selected_idx = state
        .cursor
        .bus
        .selected
        .min(scoped.len().saturating_sub(1));
    let items: Vec<ListItem> = scoped
        .iter()
        .enumerate()
        .map(|(idx, msg)| ListItem::new(message_line(msg, idx == selected_idx)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected_idx));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

fn message_line(msg: &BusMessage, is_selected: bool) -> Line<'static> {
    let accent = if is_selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(format!("#{: <5}", msg.seq), theme::heading()),
        Span::raw(" "),
        Span::styled(format!("{:16}", short(&msg.topic, 16)), theme::dim()),
        Span::raw(" "),
        Span::styled(
            format!(
                "{:18}",
                short(msg.sender.as_deref().unwrap_or("system"), 18)
            ),
            theme::dim(),
        ),
        Span::raw(" "),
        Span::styled(short_time(&msg.created_at), theme::dim()),
        Span::raw(" "),
        Span::raw(short(&payload_summary(&msg.payload), 80).to_string()),
    ])
}

fn payload_summary(payload: &serde_json::Value) -> String {
    match payload {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn short_time(raw: &str) -> String {
    raw.get(..16).unwrap_or(raw).replace('T', " ")
}

fn short(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_bus_includes_topic_and_sender() {
        let backend = TestBackend::new(120, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState {
            messages: vec![BusMessage {
                id: "m1".into(),
                seq: 7,
                plan_id: Some("p1".into()),
                topic: "agent.status".into(),
                sender: Some("alpha".into()),
                payload: serde_json::json!({"text": "hello"}),
                created_at: "2026-05-02T20:11:00Z".into(),
                ..BusMessage::default()
            }],
            ..AppState::default()
        };
        term.draw(|f| render(f, f.area(), &state, true)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("Bus"));
        assert!(dump.contains("agent.status"));
        assert!(dump.contains("alpha"));
    }
}
