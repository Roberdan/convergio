//! Pull requests pane.
//!
//! Open and closed PRs from `gh pr list`.

use crate::client::PrSummary;
use crate::render::pane_block;
use crate::state::AppState;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Render the PRs pane.
pub fn render(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let scoped = state.scoped_prs();
    let open = scoped
        .iter()
        .filter(|p| p.state.eq_ignore_ascii_case("open"))
        .count();
    let closed = scoped.len().saturating_sub(open);
    let scope_crumb = state
        .scoped_plan_title()
        .map(|t| format!(" · {}", short(t, 24)))
        .unwrap_or_default();
    let title = format!(
        " PRs ({}) open:{open} closed:{closed}{scope_crumb} ",
        scoped.len()
    );
    let block = pane_block(&title, focused);

    let selected_idx = state
        .cursor
        .prs
        .selected
        .min(scoped.len().saturating_sub(1));
    let items: Vec<ListItem> = scoped
        .iter()
        .enumerate()
        .map(|(idx, pr)| ListItem::new(pr_line(pr, idx == selected_idx)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected_idx));

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

fn pr_line(pr: &PrSummary, is_selected: bool) -> Line<'static> {
    let accent = if is_selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(format!("#{:<4}", pr.number), theme::heading()),
        Span::raw(" "),
        Span::styled(
            format!("{:6}", state_label(&pr.state)),
            state_style(&pr.state),
        ),
        Span::raw(" "),
        Span::styled(ci_glyph(&pr.ci).to_string(), ci_style(&pr.ci)),
        Span::raw(" "),
        Span::styled(format!("{:20}", short(&pr.head_ref_name, 20)), theme::dim()),
        Span::raw(" "),
        Span::styled(
            format!(
                "+{} -{} files:{} t:{}",
                pr.additions,
                pr.deletions,
                pr.changed_files,
                pr.tracked_task_ids.len()
            ),
            theme::dim(),
        ),
        Span::raw(" "),
        Span::styled(
            format!(
                "created:{} closed:{} ",
                time_or_dash(pr.created_at.as_deref()),
                time_or_dash(pr.closed_at.as_deref().or(pr.merged_at.as_deref()))
            ),
            theme::dim(),
        ),
        Span::raw(short(&pr.title, 32).to_string()),
    ])
}

fn state_label(state: &str) -> &'static str {
    match state {
        "OPEN" | "open" => "open",
        "MERGED" | "merged" => "merged",
        "CLOSED" | "closed" => "closed",
        _ => "?",
    }
}

fn state_style(state: &str) -> Style {
    match state {
        "OPEN" | "open" => Style::default().fg(theme::SUCCESS),
        "MERGED" | "merged" => Style::default().fg(theme::INFO),
        "CLOSED" | "closed" => Style::default().fg(theme::MUTED),
        _ => theme::dim(),
    }
}

fn ci_glyph(ci: &str) -> &'static str {
    match ci {
        "success" | "SUCCESS" => "✓",
        "failure" | "FAILURE" => "✗",
        "pending" | "PENDING" => "…",
        _ => "?",
    }
}

fn ci_style(ci: &str) -> Style {
    match ci {
        "success" | "SUCCESS" => Style::default().fg(theme::SUCCESS),
        "failure" | "FAILURE" => Style::default().fg(theme::DANGER),
        "pending" | "PENDING" => Style::default().fg(theme::WARNING),
        _ => theme::dim(),
    }
}

use crate::text_util::truncate as short;

fn time_or_dash(raw: Option<&str>) -> String {
    raw.map(|s| s.get(..10).unwrap_or(s).to_string())
        .unwrap_or_else(|| "-".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn pr(n: i64, branch: &str, title: &str, ci: &str) -> PrSummary {
        PrSummary {
            number: n,
            title: title.into(),
            head_ref_name: branch.into(),
            ci: ci.into(),
            state: "OPEN".into(),
            ..PrSummary::default()
        }
    }

    #[test]
    fn render_prs_includes_number_and_ci_glyph() {
        let backend = TestBackend::new(120, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState {
            prs: vec![
                pr(92, "hardening/mcp-e2e", "test(mcp): coverage", "failure"),
                pr(93, "hardening/lifecycle", "fix(lifecycle): x", "success"),
            ],
            ..AppState::default()
        };
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("PRs"));
        assert!(dump.contains("#92"));
        assert!(dump.contains("#93"));
    }

    #[test]
    fn ci_glyph_unknown_falls_back_to_question_mark() {
        assert_eq!(ci_glyph(""), "?");
        assert_eq!(ci_glyph("weird"), "?");
    }
}
