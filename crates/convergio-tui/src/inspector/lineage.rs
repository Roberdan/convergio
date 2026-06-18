//! Inspector Lineage panel.
//!
//! Renders `GET /v1/ontology/lineage/object/:name` as an ASCII chain
//! of schema revisions, oldest at the top. Breaking revisions are
//! flagged with a `⚠` glyph (not colour alone) per CONSTITUTION P3.

use crate::client_ontology::LineageNode;
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

/// Render the Lineage pane.
pub fn render(f: &mut Frame, area: Rect, state: &InspectorState, focused: bool) {
    let crumb = state
        .lineage_object
        .as_deref()
        .map(|n| format!(" · {}", truncate(n, 24)))
        .unwrap_or_default();
    let count = state.lineage.value().map(|l| l.nodes.len()).unwrap_or(0);
    let title = format!(" Lineage ({count}){crumb} ");
    let block = pane_block(&title, focused);
    if !render_state(f, area, block.clone(), &state.lineage) {
        return;
    }
    let Some(lineage) = state.lineage.value() else {
        return;
    };
    if lineage.nodes.is_empty() {
        let hint = "select an object in Types and press Enter";
        f.render_widget(
            Paragraph::new(Line::styled(hint, theme::dim())).block(block),
            area,
        );
        return;
    }

    let last = lineage.nodes.len().saturating_sub(1);
    let selected = state.lineage_cursor.selected.min(last);
    let items: Vec<ListItem> = lineage
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, n)| ListItem::new(node_line(n, idx == last, idx == selected)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

/// Build one chain row. `is_head` marks the newest revision (●),
/// older revisions use ○; a `│` connector hints at the chain.
fn node_line(n: &LineageNode, is_head: bool, selected: bool) -> Line<'static> {
    let accent = if selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    let node_glyph = if is_head { "●" } else { "○" };
    let (break_glyph, break_style) = if n.breaking {
        ("⚠ breaking", Style::default().fg(theme::DANGER))
    } else {
        ("· additive", theme::dim())
    };
    let title = truncate(&n.title, 22).to_string();
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(node_glyph, Style::default().fg(theme::FOCUS)),
        Span::raw(" "),
        Span::styled(format!("v{:<3}", n.schema_version), theme::heading()),
        Span::raw(" "),
        Span::styled(format!("{break_glyph:11}"), break_style),
        Span::raw(" "),
        Span::styled(short_hash(&n.content_hash), theme::dim()),
        Span::raw(" "),
        Span::styled(title, theme::text()),
    ])
}

fn short_hash(hash: &str) -> String {
    truncate(hash, 8).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_ontology::OntologyLineage;
    use crate::inspector::Load;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn lineage_state() -> InspectorState {
        InspectorState {
            lineage: Load::Loaded(OntologyLineage {
                object_name: "Person".into(),
                nodes: vec![
                    LineageNode {
                        schema_version: 1,
                        content_hash: "aaaaaaaa1111".into(),
                        breaking: false,
                        title: "first".into(),
                    },
                    LineageNode {
                        schema_version: 2,
                        content_hash: "bbbbbbbb2222".into(),
                        breaking: true,
                        title: "rename".into(),
                    },
                ],
            }),
            lineage_object: Some("Person".into()),
            ..Default::default()
        }
    }

    #[test]
    fn short_hash_truncates_to_eight() {
        assert_eq!(short_hash("0123456789abcdef"), "01234567");
        assert_eq!(short_hash("abc"), "abc");
    }

    #[test]
    fn render_shows_versions_and_breaking_flag() {
        let backend = TestBackend::new(70, 8);
        let mut term = Terminal::new(backend).unwrap();
        let state = lineage_state();
        term.draw(|f| render(f, f.area(), &state, true)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("Lineage (2)"));
        assert!(dump.contains("Person"));
        assert!(dump.contains("v1"));
        assert!(dump.contains("v2"));
        assert!(dump.contains("breaking"));
    }

    #[test]
    fn render_idle_prompts_for_selection() {
        let backend = TestBackend::new(60, 4);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState::default();
        term.draw(|f| render(f, f.area(), &state, false)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("idle"));
    }
}
