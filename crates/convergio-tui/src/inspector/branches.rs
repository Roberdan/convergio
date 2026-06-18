//! Inspector Branches panel.
//!
//! Renders `GET /v1/ontology/branches`: scenario branch overlays with
//! lifecycle status, id, name and timestamps. Status is conveyed by a
//! glyph as well as a label so it reads without colour (P3).

use crate::client_ontology::OntologyBranchRow;
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

/// Render the Branches pane.
pub fn render(f: &mut Frame, area: Rect, state: &InspectorState, focused: bool) {
    let count = state.branches.value().map(|b| b.len()).unwrap_or(0);
    let title = format!(" Branches ({count}) [{}] ", state.branches.status_word());
    let block = pane_block(&title, focused);
    if !render_state(f, area, block.clone(), &state.branches) {
        return;
    }
    let Some(branches) = state.branches.value() else {
        return;
    };
    if branches.is_empty() {
        let hint = "no scenario branches";
        f.render_widget(
            Paragraph::new(Line::styled(hint, theme::dim())).block(block),
            area,
        );
        return;
    }

    let last = branches.len().saturating_sub(1);
    let selected = state.branches_cursor.selected.min(last);
    let items: Vec<ListItem> = branches
        .iter()
        .enumerate()
        .map(|(idx, b)| ListItem::new(branch_line(b, idx == selected)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

/// `(glyph, colour)` for a branch lifecycle status. The glyph encodes
/// the state so colour is never the only signal.
fn status_badge(status: &str) -> (&'static str, Style) {
    match status {
        "draft" => ("◐", Style::default().fg(theme::WARNING)),
        "review" => ("◑", Style::default().fg(theme::INFO)),
        "merged" => ("✓", Style::default().fg(theme::SUCCESS)),
        "discarded" => ("⊘", Style::default().fg(theme::MUTED)),
        _ => ("·", theme::dim()),
    }
}

fn branch_line(b: &OntologyBranchRow, selected: bool) -> Line<'static> {
    let accent = if selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    let (glyph, style) = status_badge(&b.status);
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(glyph, style),
        Span::raw(" "),
        Span::styled(format!("{:9}", short_status(&b.status)), style),
        Span::raw(" "),
        Span::styled(short_id(&b.id), theme::dim()),
        Span::raw(" "),
        Span::styled(truncate(&b.name, 22).to_string(), theme::text()),
        Span::raw(" "),
        Span::styled(short_time(&b.updated_at), theme::dim()),
    ])
}

fn short_status(status: &str) -> String {
    truncate(status, 9).to_string()
}

fn short_id(id: &str) -> String {
    truncate(id, 8).to_string()
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

    fn branch(name: &str, status: &str) -> OntologyBranchRow {
        OntologyBranchRow {
            id: "branch-uuid-1234".into(),
            name: name.into(),
            status: status.into(),
            created_at: "2026-06-01T10:00:00Z".into(),
            updated_at: "2026-06-02T11:00:00Z".into(),
        }
    }

    #[test]
    fn status_badge_maps_each_lifecycle() {
        assert_eq!(status_badge("draft").0, "◐");
        assert_eq!(status_badge("review").0, "◑");
        assert_eq!(status_badge("merged").0, "✓");
        assert_eq!(status_badge("discarded").0, "⊘");
        assert_eq!(status_badge("???").0, "·");
    }

    #[test]
    fn render_lists_branches_with_status() {
        let backend = TestBackend::new(80, 6);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            branches: Load::Loaded(vec![branch("scenario-a", "review")]),
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
        assert!(dump.contains("Branches (1)"));
        assert!(dump.contains("scenario-a"));
        assert!(dump.contains("review"));
    }

    #[test]
    fn render_empty_shows_hint() {
        let backend = TestBackend::new(60, 4);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            branches: Load::Loaded(vec![]),
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
        assert!(dump.contains("no scenario branches"));
    }
}
