//! Bus pane — live agent message tail for the selected plan.
//!
//! Subscribes (via [`crate::bus_stream`]) to
//! `/v1/plans/:plan_id/messages/stream` for the currently-scoped
//! plan and renders the buffered events newest-first. When SSE is
//! unavailable the pane footer surfaces a "polling fallback" hint;
//! data still arrives via the same buffer.

use crate::bus_stream::sse_parser::TopicFamily;
use crate::bus_stream::Transport;
use crate::client::BusMessage;
use crate::render::pane_block;
use crate::state::AppState;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Render the bus messages pane.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let scoped = state.scoped_messages();
    let scope_crumb = state
        .scoped_plan_title()
        .map(|t| format!(" · {}", short(t, 24)))
        .unwrap_or_default();
    let transport_tag = match state.bus_transport() {
        Transport::Sse => " · live",
        Transport::Polling => " · polling fallback",
        Transport::Reconnecting => " · reconnecting",
        Transport::Idle => "",
    };
    let title = format!(" Bus ({}){scope_crumb}{transport_tag} ", scoped.len());
    let block = pane_block(&title, focused);

    if scoped.is_empty() {
        let empty = Paragraph::new(empty_state_lines(state)).block(block);
        f.render_widget(empty, area);
        return;
    }

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

/// Build the line for one bus event row.
///
/// Layout: `▎ HH:MM:SS  sender@kind  topic  type  preview`. The
/// topic span is coloured by [`TopicFamily`]; the family glyph
/// repeats the cue without colour for accessibility.
pub fn message_line(msg: &BusMessage, is_selected: bool) -> Line<'static> {
    let accent = if is_selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    let family = TopicFamily::classify(&msg.topic);
    let family_style = theme::topic_family_style(family);
    let family_glyph = theme::topic_family_glyph(family);
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(short_time(&msg.created_at), theme::dim()),
        Span::raw("  "),
        Span::styled(
            format!(
                "{:18}",
                short(msg.sender.as_deref().unwrap_or("system"), 18)
            ),
            theme::dim(),
        ),
        Span::raw(" "),
        Span::styled(family_glyph.to_string(), family_style),
        Span::raw(" "),
        Span::styled(format!("{:24}", short(&msg.topic, 24)), family_style),
        Span::raw(" "),
        Span::styled(
            format!("{:14}", short(&payload_type(&msg.payload), 14)),
            theme::dim(),
        ),
        Span::raw(" "),
        Span::raw(short(&payload_summary(&msg.payload), 80).to_string()),
    ])
}

/// Lines shown when the buffer is empty.
fn empty_state_lines(state: &AppState) -> Vec<Line<'static>> {
    let plan_hint = state
        .scoped_plan_id()
        .or_else(|| state.plans.first().map(|p| p.id.as_str()))
        .map(|id| format!("cvg bus tail --plan {id}"))
        .unwrap_or_else(|| "cvg bus tail --plan <id>".to_string());
    let polling = state.bus_transport() == Transport::Polling;
    let mut out = vec![
        Line::from(Span::styled(
            "  No bus traffic on this plan yet.",
            theme::dim(),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Try "),
            Span::styled(plan_hint, theme::heading()),
            Span::raw(" from another terminal to verify."),
        ]),
    ];
    if polling {
        out.push(Line::raw(""));
        out.push(Line::from(Span::styled(
            "  .. polling fallback (daemon does not advertise streaming)",
            theme::dim(),
        )));
    }
    out
}

fn payload_type(payload: &serde_json::Value) -> String {
    payload
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match payload {
            serde_json::Value::Object(_) => "object".into(),
            serde_json::Value::Array(_) => "array".into(),
            serde_json::Value::String(_) => "string".into(),
            serde_json::Value::Number(_) => "number".into(),
            serde_json::Value::Bool(_) => "bool".into(),
            serde_json::Value::Null => "null".into(),
        })
}

fn payload_summary(payload: &serde_json::Value) -> String {
    if let Some(s) = payload.get("text").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(s) = payload.get("what_just_happened").and_then(|v| v.as_str()) {
        return format!("what_just_happened: {s}");
    }
    if let Some(s) = payload.get("summary").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    match payload {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn short_time(raw: &str) -> String {
    // Expect RFC3339 like `2026-05-04T19:23:45Z` -> `19:23:45`.
    raw.get(11..19)
        .map(|s| s.to_string())
        .unwrap_or_else(|| raw.get(..8).unwrap_or(raw).to_string())
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
        let backend = TestBackend::new(160, 6);
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
        assert!(dump.contains("20:11:00"));
        assert!(dump.contains("hello"));
    }

    #[test]
    fn render_bus_empty_state_shows_hint() {
        let backend = TestBackend::new(120, 8);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState::default();
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("No bus traffic"));
        assert!(dump.contains("cvg bus tail"));
    }

    #[test]
    fn payload_summary_prefers_known_keys() {
        let v = serde_json::json!({"what_just_happened": "P0 complete"});
        assert!(payload_summary(&v).contains("P0 complete"));
    }

    #[test]
    fn render_stdout_relay_message_shown_in_bus_pane() {
        let backend = TestBackend::new(200, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState {
            messages: vec![BusMessage {
                id: "m2".into(),
                seq: 1,
                plan_id: Some("p1".into()),
                topic: "agent:proc-42:stdout".into(),
                sender: Some("proc-42".into()),
                payload: serde_json::json!({"type": "stdout", "text": "hello from agent", "seq": 0}),
                created_at: "2026-05-05T10:00:00Z".into(),
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
        assert!(dump.contains("agent:proc-42:stdout"), "topic not rendered");
        assert!(dump.contains("hello from agent"), "stdout text not shown");
        // TopicFamily::Agent is resolved for agent:* topics.
        assert_eq!(
            TopicFamily::classify("agent:proc-42:stdout"),
            TopicFamily::Agent
        );
    }
}
