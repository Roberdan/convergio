//! Drill-down panel.
//!
//! Renders the full body when [`crate::state::AppState::mode`] is
//! [`crate::state::AppMode::Detail`]. One block per entity kind. Each
//! variant shows what the dashboard already knows from the last
//! refresh, plus any extra data the [`crate::state::AppState`]
//! pre-fetched on [`crate::state::AppState::enter_detail`] (today:
//! the full task list for a Plan).
//!
//! No HTTP calls happen from the renderer — that is the rule the rest
//! of the panes follow and we keep it.

use crate::client::{PrSummary, TaskSummary};
use crate::render::pane_block;
use crate::state::{AppState, DetailTarget};
use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the drill-down panel for `target` into `area`.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, target: &DetailTarget) {
    let (title, lines) = match target {
        DetailTarget::Plan { id, title } => (
            format!(" Plan · {} ", short(title, 60)),
            plan_lines(state, id, title),
        ),
        DetailTarget::Task { id, plan_id, title } => (
            format!(" Task · {} ", short(title, 60)),
            task_lines(state, id, plan_id, title),
        ),
        DetailTarget::Agent { id } => (format!(" Agent · {id} "), agent_lines(state, id)),
        DetailTarget::Pr { number, title } => (
            format!(" PR #{number} · {} ", short(title, 60)),
            pr_lines(state, *number, title),
        ),
        DetailTarget::BusMessage { id, seq, topic } => (
            format!(" Bus #{seq} · {} ", short(topic, 60)),
            bus_message_lines(state, id),
        ),
    };
    let block = pane_block(&title, true);
    let inner_h = area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_h);
    let max_scroll_u16 = (max_scroll.min(u16::MAX as usize)) as u16;
    let scroll = state.detail_scroll.min(max_scroll_u16);
    f.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

fn plan_lines(state: &AppState, id: &str, title: &str) -> Vec<Line<'static>> {
    let mut out = vec![header_line(title), Line::raw("")];

    if let Some(plan) = state.plans.iter().find(|p| p.id == id) {
        out.push(kv("status", &plan.status));
        out.push(kv("project", plan.project.as_deref().unwrap_or("-")));
        out.push(kv("updated", &plan.updated_at));
    }
    out.push(kv("id", id));
    out.push(Line::raw(""));

    let tasks = if state.detail_tasks.is_empty() {
        let active: Vec<TaskSummary> = state
            .tasks
            .iter()
            .filter(|t| t.plan_id == id)
            .cloned()
            .collect();
        active
    } else {
        state.detail_tasks.clone()
    };
    out.push(section_heading(&format!("Tasks ({})", tasks.len())));
    if tasks.is_empty() {
        out.push(dim_line("  (no tasks for this plan)"));
    } else {
        for t in &tasks {
            out.push(task_line(t));
        }
    }
    out
}

fn task_lines(state: &AppState, id: &str, plan_id: &str, title: &str) -> Vec<Line<'static>> {
    let mut out = vec![header_line(title), Line::raw("")];
    let task = state.tasks.iter().find(|t| t.id == id);
    if let Some(t) = task {
        out.push(kv("status", &t.status));
        if let Some(a) = &t.agent_id {
            out.push(kv("agent", a));
        } else {
            out.push(kv("agent", "—"));
        }
    }
    out.push(kv("plan", plan_id));
    out.push(kv("id", id));
    out
}

fn agent_lines(state: &AppState, id: &str) -> Vec<Line<'static>> {
    let mut out = vec![header_line(id), Line::raw("")];
    if let Some(a) = state.agents.iter().find(|a| a.id == id) {
        out.push(kv("kind", &a.kind));
        out.push(kv("status", a.status.as_deref().unwrap_or("?")));
        out.push(kv(
            "last_heartbeat",
            a.last_heartbeat_at.as_deref().unwrap_or("—"),
        ));
    }
    out.push(Line::raw(""));
    let owned: Vec<&TaskSummary> = state
        .tasks
        .iter()
        .filter(|t| t.agent_id.as_deref() == Some(id))
        .collect();
    out.push(section_heading(&format!("Active tasks ({})", owned.len())));
    if owned.is_empty() {
        out.push(dim_line("  (agent has no active tasks)"));
    } else {
        for t in owned {
            out.push(task_line(t));
        }
    }
    out
}

fn pr_lines(state: &AppState, number: i64, title: &str) -> Vec<Line<'static>> {
    let mut out = vec![header_line(title), Line::raw("")];
    if let Some(pr) = state.prs.iter().find(|p| p.number == number) {
        out.extend(pr_meta(pr));
    } else {
        out.push(dim_line("  (PR no longer in the open list)"));
    }
    out
}

fn bus_message_lines(state: &AppState, id: &str) -> Vec<Line<'static>> {
    let Some(msg) = state.messages.iter().find(|m| m.id == id) else {
        return vec![dim_line("  (message rolled out of the buffer)")];
    };
    let pretty =
        serde_json::to_string_pretty(&msg.payload).unwrap_or_else(|_| msg.payload.to_string());
    let mut out = vec![
        kv("seq", &msg.seq.to_string()),
        kv("topic", &msg.topic),
        kv("sender", msg.sender.as_deref().unwrap_or("system")),
        kv("plan", msg.plan_id.as_deref().unwrap_or("-")),
        kv("created_at", &msg.created_at),
        Line::raw(""),
        section_heading("Payload"),
    ];
    for line in pretty.lines() {
        out.push(Line::from(Span::styled(
            format!("  {}", line),
            theme::text(),
        )));
    }
    out
}

fn pr_meta(pr: &PrSummary) -> Vec<Line<'static>> {
    let ci = if pr.ci.is_empty() {
        "—"
    } else {
        pr.ci.as_str()
    };
    vec![
        kv("number", &format!("#{}", pr.number)),
        kv("branch", &pr.head_ref_name),
        kv("ci", ci),
    ]
}

fn task_line(t: &TaskSummary) -> Line<'static> {
    let (status_glyph, status_style) = theme::status_pill(&t.status);
    Line::from(vec![
        Span::raw("  "),
        status_glyph,
        Span::raw(" "),
        Span::styled(format!("{:<11}", t.status), status_style),
        Span::raw(" "),
        Span::styled(short(&t.id, 8).to_string(), theme::dim()),
        Span::raw(" "),
        Span::raw(short(&t.title, 70).to_string()),
    ])
}

fn header_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(title.to_string(), theme::heading()))
}

fn section_heading(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        ratatui::style::Style::default()
            .fg(theme::FOCUS)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ))
}

fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<14} ", k), theme::dim()),
        Span::styled(v.to_string(), theme::text()),
    ])
}

fn dim_line(s: &str) -> Line<'static> {
    Line::from(Span::styled(s.to_string(), theme::dim()))
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
    use crate::client::TaskSummary;

    #[test]
    fn short_handles_unicode_safely() {
        let s = "abcdèfgh";
        let t = short(s, 4);
        assert!(s.starts_with(t));
    }

    #[test]
    fn task_line_uses_status_color_class() {
        let t = TaskSummary {
            id: "tx".into(),
            plan_id: "p".into(),
            title: "go".into(),
            status: "done".into(),
            agent_id: None,
            created_at: "2026-05-02T20:11:00Z".into(),
            updated_at: "2026-05-02T20:11:00Z".into(),
            ..TaskSummary::default()
        };
        let line = task_line(&t);
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("done"));
        assert!(text.contains("go"));
    }
}
