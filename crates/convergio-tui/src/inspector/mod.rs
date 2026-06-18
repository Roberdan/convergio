//! Read-only Ontology Inspector for `cvg dash` (W6, ADR-0059).
//!
//! A second top-level section alongside the ops dashboard. Toggled
//! with `o`, it renders data fetched from the ontology HTTP surface
//! the daemon already serves (ADR-0053 / ADR-0060). Like the rest of
//! the TUI it is strictly read-only — every mutation stays in `cvg`
//! subcommands.
//!
//! Implemented panels (each backed by an existing endpoint):
//!
//! - [`types`] — `GET /v1/ontology/types`.
//! - [`lineage`] — `GET /v1/ontology/lineage/object/:name`.
//! - [`branches`] — `GET /v1/ontology/branches`.
//!
//! Panels from ADR-0059 with no backing read endpoint on `main`
//! (live events, ER queue, gateway calls) are intentionally skipped
//! — the TUI never adds server routes.

pub mod actions;
pub mod branches;
pub mod lineage;
pub mod render;
pub mod types;

use crate::client_ontology::{OntologyBranchRow, OntologyLineage, OntologyTypeRow, OntologyTypes};
use crate::state::Cursor;

/// Top-level dashboard section.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// The five-pane ops dashboard (plans / tasks / agents / PRs / bus).
    #[default]
    Ops,
    /// The read-only Ontology Inspector (W6, ADR-0059).
    Inspector,
}

impl Section {
    /// `true` when the inspector section is active.
    pub fn is_inspector(self) -> bool {
        matches!(self, Section::Inspector)
    }
}

/// Panes within the inspector, in tab order.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPane {
    /// Object / link type registry.
    #[default]
    Types,
    /// Schema-version lineage of the selected object.
    Lineage,
    /// Scenario branch overlays.
    Branches,
}

impl InspectorPane {
    /// All panes in display order.
    pub const ALL: [InspectorPane; 3] = [
        InspectorPane::Types,
        InspectorPane::Lineage,
        InspectorPane::Branches,
    ];

    /// Short label rendered in the pane title.
    pub fn label(self) -> &'static str {
        match self {
            InspectorPane::Types => "Types",
            InspectorPane::Lineage => "Lineage",
            InspectorPane::Branches => "Branches",
        }
    }
}

/// Generic fetch lifecycle for one inspector dataset. Keeps the
/// renderer honest about loading / empty / error states (no panics,
/// no stale data masquerading as fresh).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Load<T> {
    /// Never fetched yet.
    #[default]
    Idle,
    /// A fetch is in flight.
    Loading,
    /// Last fetch succeeded.
    Loaded(T),
    /// Last fetch failed, with a short human message.
    Error(String),
}

impl<T> Load<T> {
    /// Borrow the loaded value, if any.
    pub fn value(&self) -> Option<&T> {
        match self {
            Load::Loaded(v) => Some(v),
            _ => None,
        }
    }

    /// Short status word for the pane title / placeholder body.
    pub fn status_word(&self) -> &'static str {
        match self {
            Load::Idle => "idle",
            Load::Loading => "loading…",
            Load::Loaded(_) => "loaded",
            Load::Error(_) => "error",
        }
    }
}

/// Inspector view state: fetched datasets plus focus + cursors.
#[derive(Debug, Default)]
pub struct InspectorState {
    /// Focused pane.
    pub focus: InspectorPane,
    /// `GET /v1/ontology/types` result.
    pub types: Load<OntologyTypes>,
    /// `GET /v1/ontology/lineage/object/:name` result.
    pub lineage: Load<OntologyLineage>,
    /// `GET /v1/ontology/branches` result.
    pub branches: Load<Vec<OntologyBranchRow>>,
    /// Cursor for the Types pane (indexes the flattened object+link list).
    pub types_cursor: Cursor,
    /// Cursor for the Lineage pane.
    pub lineage_cursor: Cursor,
    /// Cursor for the Branches pane.
    pub branches_cursor: Cursor,
    /// Object whose lineage is currently loaded, for the pane crumb.
    pub lineage_object: Option<String>,
}

impl InspectorState {
    /// Flatten the loaded types into one selection list: objects
    /// first, then links — matching the render order. Empty until the
    /// first successful fetch.
    pub fn type_rows(&self) -> Vec<&OntologyTypeRow> {
        match self.types.value() {
            Some(t) => t.objects.iter().chain(t.links.iter()).collect(),
            None => Vec::new(),
        }
    }

    /// The type row under the Types cursor, if any.
    pub fn selected_type(&self) -> Option<&OntologyTypeRow> {
        let rows = self.type_rows();
        rows.get(self.types_cursor.selected.min(rows.len().saturating_sub(1)))
            .copied()
    }

    /// Number of rows in the focused pane (for cursor clamping).
    pub fn focused_len(&self) -> usize {
        match self.focus {
            InspectorPane::Types => self.type_rows().len(),
            InspectorPane::Lineage => self.lineage.value().map(|l| l.nodes.len()).unwrap_or(0),
            InspectorPane::Branches => self.branches.value().map(|b| b.len()).unwrap_or(0),
        }
    }

    /// Move focus to the next inspector pane.
    pub fn focus_next(&mut self) {
        let idx = InspectorPane::ALL
            .iter()
            .position(|p| *p == self.focus)
            .unwrap_or(0);
        self.focus = InspectorPane::ALL[(idx + 1) % InspectorPane::ALL.len()];
    }

    /// Move focus to the previous inspector pane.
    pub fn focus_prev(&mut self) {
        let idx = InspectorPane::ALL
            .iter()
            .position(|p| *p == self.focus)
            .unwrap_or(0);
        let n = InspectorPane::ALL.len();
        self.focus = InspectorPane::ALL[(idx + n - 1) % n];
    }

    /// Cursor down within the focused pane.
    pub fn row_down(&mut self) {
        let len = self.focused_len();
        self.focused_cursor_mut().down(len, 8);
    }

    /// Cursor up within the focused pane.
    pub fn row_up(&mut self) {
        self.focused_cursor_mut().up();
    }

    fn focused_cursor_mut(&mut self) -> &mut Cursor {
        match self.focus {
            InspectorPane::Types => &mut self.types_cursor,
            InspectorPane::Lineage => &mut self.lineage_cursor,
            InspectorPane::Branches => &mut self.branches_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types_fixture() -> OntologyTypes {
        OntologyTypes {
            objects: vec![
                OntologyTypeRow {
                    kind: "object".into(),
                    name: "Person".into(),
                    ..Default::default()
                },
                OntologyTypeRow {
                    kind: "object".into(),
                    name: "Org".into(),
                    ..Default::default()
                },
            ],
            links: vec![OntologyTypeRow {
                kind: "link".into(),
                name: "WorksAt".into(),
                ..Default::default()
            }],
        }
    }

    #[test]
    fn section_toggle_predicate() {
        assert!(!Section::Ops.is_inspector());
        assert!(Section::Inspector.is_inspector());
    }

    #[test]
    fn focus_cycles_through_three_panes() {
        let mut s = InspectorState::default();
        assert_eq!(s.focus, InspectorPane::Types);
        s.focus_next();
        assert_eq!(s.focus, InspectorPane::Lineage);
        s.focus_next();
        assert_eq!(s.focus, InspectorPane::Branches);
        s.focus_next();
        assert_eq!(s.focus, InspectorPane::Types, "wraps after 3 hops");
        s.focus_prev();
        assert_eq!(s.focus, InspectorPane::Branches);
    }

    #[test]
    fn type_rows_flatten_objects_then_links() {
        let s = InspectorState {
            types: Load::Loaded(types_fixture()),
            ..Default::default()
        };
        let names: Vec<&str> = s.type_rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Person", "Org", "WorksAt"]);
    }

    #[test]
    fn selected_type_tracks_cursor() {
        let mut s = InspectorState {
            types: Load::Loaded(types_fixture()),
            ..Default::default()
        };
        assert_eq!(s.selected_type().map(|r| r.name.as_str()), Some("Person"));
        s.row_down();
        assert_eq!(s.selected_type().map(|r| r.name.as_str()), Some("Org"));
    }

    #[test]
    fn row_down_clamps_to_loaded_len() {
        let mut s = InspectorState {
            types: Load::Loaded(types_fixture()),
            ..Default::default()
        };
        for _ in 0..10 {
            s.row_down();
        }
        assert_eq!(s.types_cursor.selected, 2, "3 rows → last index 2");
    }

    #[test]
    fn load_status_words() {
        assert_eq!(Load::<u8>::Idle.status_word(), "idle");
        assert_eq!(Load::<u8>::Loading.status_word(), "loading…");
        assert_eq!(Load::Loaded(1u8).status_word(), "loaded");
        assert_eq!(Load::<u8>::Error("x".into()).status_word(), "error");
    }
}
