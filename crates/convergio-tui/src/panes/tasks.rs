//! Tasks pane.
//!
//! Filtered against [`AppState::scoped_plan_id`]: when the Plans
//! pane has a plan under its cursor, this pane shows only that
//! plan's active tasks. Title carries the scope crumb.

use crate::client::TaskSummary;
use crate::render::pane_block;
use crate::state::AppState;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Render the Tasks pane.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let scoped: Vec<TaskSummary> = state.scoped_tasks().into_iter().cloned().collect();
    let total = scoped.len();
    let mut sorted: Vec<TaskSummary> = if state.show_terminal_tasks {
        scoped
    } else {
        scoped
            .into_iter()
            .filter(|t| !is_terminal(&t.status))
            .collect()
    };
    sorted.sort_by_key(|t| status_priority(&t.status));

    let scope_crumb = state
        .scoped_plan_title()
        .map(|t| format!(" · {}", short(t, 24)))
        .unwrap_or_default();
    let filter_crumb = if state.show_terminal_tasks {
        String::new()
    } else {
        format!("/{total} · t:show all")
    };
    let title = format!(" Tasks ({}{filter_crumb}){scope_crumb} ", sorted.len());
    let block = pane_block(&title, focused);

    let selected_idx = state
        .cursor
        .tasks
        .selected
        .min(sorted.len().saturating_sub(1));
    let items: Vec<ListItem> = sorted
        .iter()
        .enumerate()
        .map(|(idx, t)| ListItem::new(task_line(t, idx == selected_idx)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected_idx));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

fn task_line(t: &TaskSummary, is_selected: bool) -> Line<'static> {
    let owner = t.agent_id.as_deref().unwrap_or("-");
    let (status_glyph, status_style) = theme::status_pill(&t.status);
    let accent = if is_selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(short(&t.id, 8).to_string(), theme::dim()),
        Span::raw(" "),
        status_glyph,
        Span::raw(" "),
        Span::styled(format!("{:12}", &t.status), status_style),
        Span::raw(" "),
        Span::styled(format!("{:18}", short(owner, 18)), theme::dim()),
        Span::raw(" "),
        Span::raw(short(&task_title(t), 42).to_string()),
        Span::raw(" "),
        Span::styled(
            format!(
                "w{}.{:<2} start:{} end:{} dur:{}",
                t.wave,
                t.sequence,
                time_or_dash(t.started_at.as_deref()),
                time_or_dash(t.ended_at.as_deref()),
                duration_text(t.duration_ms)
            ),
            theme::dim(),
        ),
    ])
}

fn task_title(t: &TaskSummary) -> String {
    match t.description.as_deref().filter(|d| !d.trim().is_empty()) {
        Some(d) => format!("{} - {}", t.title, d.trim()),
        None => t.title.clone(),
    }
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "done" | "failed")
}

fn status_priority(status: &str) -> u8 {
    match status {
        "in_progress" => 0,
        "submitted" => 1,
        "pending" => 2,
        "failed" => 3,
        "done" => 4,
        _ => 5,
    }
}

fn time_or_dash(raw: Option<&str>) -> String {
    raw.map(|s| s.get(..16).unwrap_or(s).replace('T', " "))
        .unwrap_or_else(|| "-".into())
}

fn duration_text(ms: Option<i64>) -> String {
    let Some(ms) = ms else {
        return "-".into();
    };
    let secs = ms.max(0) / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
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

    fn task(id: &str, status: &str) -> TaskSummary {
        TaskSummary {
            id: id.into(),
            plan_id: "p".into(),
            title: format!("title-{id}"),
            status: status.into(),
            agent_id: Some("claude-code-roberdan".into()),
            created_at: "2026-05-02T20:11:00Z".into(),
            updated_at: "2026-05-02T20:11:00Z".into(),
            ..TaskSummary::default()
        }
    }

    #[test]
    fn status_priority_orders_in_progress_first() {
        let mut v = vec!["done", "in_progress", "submitted", "pending"];
        v.sort_by_key(|s| status_priority(s));
        assert_eq!(v, vec!["in_progress", "submitted", "pending", "done"]);
    }

    #[test]
    fn hide_terminal_filters_done_and_failed_by_default() {
        let backend = TestBackend::new(140, 8);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState {
            tasks: vec![
                task("aaaaaaaa11", "in_progress"),
                task("bbbbbbbb22", "done"),
                task("cccccccc33", "failed"),
            ],
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
        assert!(dump.contains("in_progress"));
        assert!(!dump.contains("done"));
        assert!(!dump.contains("failed"));
        // Title carries (1/3 · t:show all).
        assert!(dump.contains("(1/3"));
    }

    #[test]
    fn toggle_reveals_terminal_tasks() {
        let backend = TestBackend::new(140, 8);
        let mut term = Terminal::new(backend).unwrap();
        let mut state = AppState {
            tasks: vec![
                task("aaaaaaaa11", "in_progress"),
                task("bbbbbbbb22", "done"),
            ],
            ..AppState::default()
        };
        state.toggle_show_terminal_tasks();
        term.draw(|f| render(f, f.area(), &state, true)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("done"));
    }

    #[test]
    fn render_tasks_includes_status_and_owner() {
        let backend = TestBackend::new(140, 8);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState {
            tasks: vec![
                task("aaaaaaaa11", "in_progress"),
                task("bbbbbbbb22", "submitted"),
            ],
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
        assert!(dump.contains("Tasks"));
        assert!(dump.contains("in_progress"));
        assert!(dump.contains("submitted"));
    }
}
