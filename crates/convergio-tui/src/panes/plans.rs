//! Plans pane.
//!
//! Master pane in the lazygit-style scoped master/detail layout
//! (ADR-0029). Whichever plan the cursor sits on is the *scope* the
//! Tasks / Agents / PRs panes filter against, so as the user moves
//! up/down here the rest of the dashboard re-renders.

use crate::client::Plan;
use crate::plan_counts::PlanCounts;
use crate::render::pane_block;
use crate::state::AppState;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Render the Plans pane into `area`.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let title = format!(" Plans ({}) ", state.plans.len());
    let block = pane_block(&title, focused);

    let selected_idx = state
        .cursor
        .plans
        .selected
        .min(state.plans.len().saturating_sub(1));
    let items: Vec<ListItem> = state
        .plans
        .iter()
        .enumerate()
        .map(|(idx, p)| ListItem::new(plan_lines(p, state, idx == selected_idx)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected_idx));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

fn plan_lines(p: &Plan, state: &AppState, is_selected: bool) -> Vec<Line<'static>> {
    let tasks = state
        .tasks
        .iter()
        .filter(|t| t.plan_id == p.id)
        .cloned()
        .collect::<Vec<_>>();
    let counts = PlanCounts::from_tasks(&tasks);
    let agents = plan_agent_count(p, state);
    let (added, removed) = plan_diff(p, state);
    let (status_glyph, status_style) = theme::status_pill(&p.status);
    let accent = if is_selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };

    let title_line = Line::from(vec![
        accent.clone(),
        Span::raw(" "),
        status_glyph,
        Span::raw(" "),
        Span::styled(truncate(&p.title, 48).to_string(), theme::heading()),
        Span::raw("  "),
        Span::styled(format!("[{}]", p.status), status_style),
        Span::raw("  "),
        Span::styled(
            format!(
                "tasks:{}/{} agents:{} +{} -{}",
                counts.done, counts.total, agents, added, removed
            ),
            theme::dim(),
        ),
    ]);

    let project = p.project.as_deref().unwrap_or("-").to_string();
    let started = p
        .started_at
        .as_deref()
        .map(short_time)
        .unwrap_or_else(|| "not-started".into());
    let ended = p
        .ended_at
        .as_deref()
        .map(short_time)
        .unwrap_or_else(|| "open".into());
    let meta_line = Line::from(vec![
        accent,
        Span::raw("   "),
        Span::styled(
            format!(
                "project: {project}  start:{started}  end:{ended}  dur:{}",
                duration_text(p.duration_ms)
            ),
            theme::dim(),
        ),
    ]);

    vec![title_line, meta_line]
}

fn plan_agent_count(p: &Plan, state: &AppState) -> usize {
    let task_ids = state
        .tasks
        .iter()
        .filter(|t| t.plan_id == p.id)
        .map(|t| t.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut ids = state
        .agent_processes
        .iter()
        .filter(|a| {
            a.plan_id.as_deref() == Some(p.id.as_str())
                || a.task_id
                    .as_deref()
                    .map(|id| task_ids.contains(id))
                    .unwrap_or(false)
        })
        .map(|a| a.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    ids.extend(
        state
            .tasks
            .iter()
            .filter(|t| t.plan_id == p.id)
            .filter_map(|t| t.agent_id.as_deref()),
    );
    ids.len()
}

fn plan_diff(p: &Plan, state: &AppState) -> (i64, i64) {
    let task_ids = state
        .tasks
        .iter()
        .filter(|t| t.plan_id == p.id)
        .map(|t| t.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    state
        .prs
        .iter()
        .filter(|pr| {
            pr.tracked_task_ids
                .iter()
                .any(|t| task_ids.contains(t.as_str()))
        })
        .fold((0, 0), |(a, d), pr| (a + pr.additions, d + pr.deletions))
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

fn short_time(raw: &str) -> String {
    raw.get(..16).unwrap_or(raw).replace('T', " ")
}

use crate::text_util::truncate;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{Plan, TaskSummary};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn state_with(plans: Vec<Plan>, tasks: Vec<TaskSummary>) -> AppState {
        AppState {
            plans,
            tasks,
            ..AppState::default()
        }
    }

    #[test]
    fn truncate_handles_unicode_safely() {
        let s = "abcdèfgh";
        let t = truncate(s, 4);
        assert!(s.starts_with(t));
    }

    #[test]
    fn render_plans_marks_active_count_per_plan() {
        let backend = TestBackend::new(100, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = state_with(
            vec![Plan {
                id: "p1".into(),
                title: "p1".into(),
                project: None,
                status: "draft".into(),
                created_at: "2026-05-02T20:11:00Z".into(),
                updated_at: "2026-05-02".into(),
                ..Plan::default()
            }],
            vec![TaskSummary {
                id: "t1".into(),
                plan_id: "p1".into(),
                title: "do".into(),
                status: "in_progress".into(),
                agent_id: None,
                created_at: "2026-05-02T20:11:00Z".into(),
                updated_at: "2026-05-02T20:11:00Z".into(),
                ..TaskSummary::default()
            }],
        );
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("tasks:0/1"));
        assert!(dump.contains("Plans"));
    }
}
