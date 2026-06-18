//! Inspector Types panel.
//!
//! Renders `GET /v1/ontology/types`: every registered object type
//! followed by every link type, with kind, latest schema version,
//! name and title. `Enter` on an object drills into its lineage.

use crate::client_ontology::OntologyTypeRow;
use crate::inspector::render::render_state;
use crate::inspector::InspectorState;
use crate::render::pane_block;
use crate::text_util::truncate;
use crate::theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Render the Types pane.
pub fn render(f: &mut Frame, area: Rect, state: &InspectorState, focused: bool) {
    let rows = state.type_rows();
    let title = format!(" Types ({}) [{}] ", rows.len(), state.types.status_word());
    let block = pane_block(&title, focused);
    if !render_state(f, area, block.clone(), &state.types) {
        return;
    }
    if rows.is_empty() {
        let line = Line::styled("no registered types", theme::dim());
        f.render_widget(ratatui::widgets::Paragraph::new(line).block(block), area);
        return;
    }

    let selected = state
        .types_cursor
        .selected
        .min(rows.len().saturating_sub(1));
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(idx, r)| ListItem::new(type_line(r, idx == selected)))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::row_highlight());
    f.render_stateful_widget(list, area, &mut list_state);
}

/// `(glyph, label)` for a type kind. Glyph carries the meaning so the
/// row reads without colour (CONSTITUTION P3).
fn kind_badge(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "object" => ("◆", "obj"),
        "link" => ("↔", "lnk"),
        _ => ("·", "?"),
    }
}

fn type_line(r: &OntologyTypeRow, selected: bool) -> Line<'static> {
    let accent = if selected {
        theme::accent_span()
    } else {
        theme::accent_gap()
    };
    let (glyph, label) = kind_badge(&r.kind);
    let title = if r.title.trim().is_empty() {
        String::new()
    } else {
        format!("  {}", truncate(&r.title, 30))
    };
    Line::from(vec![
        accent,
        Span::raw(" "),
        Span::styled(glyph, Style::default().fg(theme::INFO)),
        Span::raw(" "),
        Span::styled(format!("{label:3}"), theme::dim()),
        Span::raw(" "),
        Span::styled(format!("v{:<3}", r.schema_version), theme::dim()),
        Span::raw(" "),
        Span::styled(truncate(&r.name, 28).to_string(), theme::text()),
        Span::styled(title, theme::dim()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_ontology::OntologyTypes;
    use crate::inspector::Load;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn loaded_state() -> InspectorState {
        InspectorState {
            types: Load::Loaded(OntologyTypes {
                objects: vec![OntologyTypeRow {
                    kind: "object".into(),
                    name: "Person".into(),
                    schema_version: 2,
                    title: "A person".into(),
                    ..Default::default()
                }],
                links: vec![OntologyTypeRow {
                    kind: "link".into(),
                    name: "WorksAt".into(),
                    schema_version: 1,
                    ..Default::default()
                }],
            }),
            ..Default::default()
        }
    }

    #[test]
    fn kind_badge_distinguishes_object_and_link() {
        assert_eq!(kind_badge("object").1, "obj");
        assert_eq!(kind_badge("link").1, "lnk");
        assert_eq!(kind_badge("weird").1, "?");
    }

    #[test]
    fn render_lists_objects_and_links() {
        let backend = TestBackend::new(80, 8);
        let mut term = Terminal::new(backend).unwrap();
        let state = loaded_state();
        term.draw(|f| render(f, f.area(), &state, true)).unwrap();
        let dump = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(dump.contains("Types (2)"));
        assert!(dump.contains("Person"));
        assert!(dump.contains("WorksAt"));
    }

    #[test]
    fn render_shows_loading_placeholder() {
        let backend = TestBackend::new(40, 4);
        let mut term = Terminal::new(backend).unwrap();
        let state = InspectorState {
            types: Load::Loading,
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
        assert!(dump.contains("loading"));
    }
}
