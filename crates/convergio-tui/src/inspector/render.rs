//! Inspector body layout and shared load-state rendering.
//!
//! [`body`] replaces the ops grid when the inspector section is
//! active: Types on the left, Lineage and Branches stacked on the
//! right. Each panel delegates to its own module and renders its own
//! loading / empty / error state via [`render_state`].

use crate::inspector::{InspectorPane, InspectorState, Load};
use crate::state::AppState;
use crate::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Draw the inspector body into `area`.
pub fn body(f: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(cols[1]);

    let insp = &state.inspector;
    super::types::render(f, cols[0], insp, focused(insp, InspectorPane::Types));
    super::lineage::render(f, right[0], insp, focused(insp, InspectorPane::Lineage));
    super::branches::render(f, right[1], insp, focused(insp, InspectorPane::Branches));
}

fn focused(insp: &InspectorState, pane: InspectorPane) -> bool {
    insp.focus == pane
}

/// Render the placeholder body for a non-`Loaded` dataset. Returns
/// `true` when the dataset is loaded and the caller should render its
/// own content into `block`; `false` once a placeholder has been
/// drawn (idle / loading / error).
pub fn render_state<T>(f: &mut Frame, area: Rect, block: Block<'static>, load: &Load<T>) -> bool {
    let line = match load {
        Load::Loaded(_) => return true,
        Load::Idle => Line::styled("idle — press r to load", theme::dim()),
        Load::Loading => Line::styled("loading…", Style::default().fg(theme::INFO)),
        Load::Error(e) => Line::styled(format!("⚠ error: {e}"), Style::default().fg(theme::DANGER)),
    };
    f.render_widget(Paragraph::new(line).block(block), area);
    false
}

/// Footer help string for the inspector section.
pub fn footer_help() -> &'static str {
    "q quit  o ops  Tab pane  j/k row  Enter lineage  r refresh  Esc back"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_ontology::OntologyTypes;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_state_returns_true_when_loaded() {
        let backend = TestBackend::new(20, 3);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let loaded: Load<OntologyTypes> = Load::Loaded(OntologyTypes::default());
            let go = render_state(f, f.area(), Block::default(), &loaded);
            assert!(go);
        })
        .unwrap();
    }

    #[test]
    fn render_state_draws_error_placeholder() {
        let backend = TestBackend::new(40, 3);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let err: Load<OntologyTypes> = Load::Error("boom".into());
            let go = render_state(f, f.area(), Block::default(), &err);
            assert!(!go);
        })
        .unwrap();
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
