//! Inspector key dispatch and async data loading.
//!
//! These methods bridge the read-only ontology fetchers in
//! [`crate::client_ontology`] into [`crate::state::AppState`]. The
//! event loop in [`crate::run`] routes a key through
//! [`AppState::handle_inspector_action`] whenever the inspector
//! section is active; everything here mutates inspector state only and
//! never issues a write request.

use crate::client::Client;
use crate::inspector::{InspectorPane, Load, Section};
use crate::keymap::Action;
use crate::state::AppState;

impl AppState {
    /// Enter the inspector section and kick off the initial fetch.
    pub async fn enter_inspector(&mut self, client: &Client) {
        self.section = Section::Inspector;
        self.refresh_inspector(client).await;
    }

    /// Leave the inspector and return to the ops dashboard.
    pub fn exit_inspector(&mut self) {
        self.section = Section::Ops;
    }

    /// Fetch the type registry, branch list and event snapshot (and
    /// re-fetch the active lineage, if any). Each dataset records its
    /// own loading / error state so a single failed endpoint never
    /// blanks the others.
    pub async fn refresh_inspector(&mut self, client: &Client) {
        self.inspector.types = Load::Loading;
        self.inspector.branches = Load::Loading;
        self.inspector.events = Load::Loading;
        let (types, branches, events) = tokio::join!(
            client.fetch_ontology_types(),
            client.fetch_ontology_branches(),
            client.fetch_ontology_events()
        );
        self.inspector.types = match types {
            Ok(t) => Load::Loaded(t),
            Err(e) => Load::Error(short_err(&e)),
        };
        self.inspector.branches = match branches {
            Ok(b) => Load::Loaded(b),
            Err(e) => Load::Error(short_err(&e)),
        };
        self.inspector.events = match events {
            Ok(ev) => Load::Loaded(ev),
            Err(e) => Load::Error(short_err(&e)),
        };
        if let Some(name) = self.inspector.lineage_object.clone() {
            self.load_lineage(client, &name).await;
        }
    }

    /// Drill the focused inspector row. From the Types pane this loads
    /// the lineage of the selected object and jumps to the Lineage
    /// pane; links have no lineage so the action is a no-op.
    pub async fn inspector_drill(&mut self, client: &Client) {
        if self.inspector.focus != InspectorPane::Types {
            return;
        }
        let Some(row) = self.inspector.selected_type() else {
            return;
        };
        if row.kind != "object" {
            return;
        }
        let name = row.name.clone();
        self.load_lineage(client, &name).await;
        self.inspector.focus = InspectorPane::Lineage;
        self.inspector.lineage_cursor = Default::default();
    }

    async fn load_lineage(&mut self, client: &Client, name: &str) {
        self.inspector.lineage = Load::Loading;
        self.inspector.lineage_object = Some(name.to_string());
        self.inspector.lineage = match client.fetch_ontology_lineage(name).await {
            Ok(l) => Load::Loaded(l),
            Err(e) => Load::Error(short_err(&e)),
        };
    }

    /// Handle a key while the inspector section is active. Returns
    /// `true` when the action was consumed; `false` lets the caller's
    /// global match run (used for `Quit`).
    pub async fn handle_inspector_action(&mut self, client: &Client, action: Action) -> bool {
        match action {
            Action::Quit => false,
            Action::Back | Action::ToggleInspector => {
                self.exit_inspector();
                true
            }
            Action::PaneNext => {
                self.inspector.focus_next();
                true
            }
            Action::PanePrev => {
                self.inspector.focus_prev();
                true
            }
            Action::RowDown => {
                self.inspector.row_down();
                true
            }
            Action::RowUp => {
                self.inspector.row_up();
                true
            }
            Action::Drill => {
                self.inspector_drill(client).await;
                true
            }
            Action::RefreshNow => {
                self.refresh_inspector(client).await;
                true
            }
            _ => true,
        }
    }
}

/// Compress an error chain into a single short line for the pane.
fn short_err(e: &anyhow::Error) -> String {
    let s = e.to_string();
    s.lines().next().unwrap_or("request failed").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_ontology::{OntologyTypeRow, OntologyTypes};
    use crate::inspector::InspectorState;

    fn object(name: &str) -> OntologyTypeRow {
        OntologyTypeRow {
            kind: "object".into(),
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn exit_inspector_returns_to_ops() {
        let mut s = AppState {
            section: Section::Inspector,
            ..AppState::default()
        };
        s.exit_inspector();
        assert_eq!(s.section, Section::Ops);
    }

    #[tokio::test]
    async fn back_action_exits_and_is_consumed() {
        let client = Client::new("http://127.0.0.1:0".into());
        let mut s = AppState {
            section: Section::Inspector,
            ..AppState::default()
        };
        let handled = s.handle_inspector_action(&client, Action::Back).await;
        assert!(handled);
        assert_eq!(s.section, Section::Ops);
    }

    #[tokio::test]
    async fn quit_action_is_not_consumed() {
        let client = Client::new("http://127.0.0.1:0".into());
        let mut s = AppState::default();
        let handled = s.handle_inspector_action(&client, Action::Quit).await;
        assert!(!handled, "Quit must fall through to the global match");
    }

    #[tokio::test]
    async fn pane_next_action_moves_focus() {
        let client = Client::new("http://127.0.0.1:0".into());
        let mut s = AppState::default();
        s.handle_inspector_action(&client, Action::PaneNext).await;
        assert_eq!(s.inspector.focus, InspectorPane::Lineage);
    }

    #[tokio::test]
    async fn drill_on_link_is_noop() {
        let client = Client::new("http://127.0.0.1:0".into());
        let mut s = AppState {
            inspector: InspectorState {
                types: Load::Loaded(OntologyTypes {
                    objects: vec![],
                    links: vec![OntologyTypeRow {
                        kind: "link".into(),
                        name: "WorksAt".into(),
                        ..Default::default()
                    }],
                }),
                ..Default::default()
            },
            ..AppState::default()
        };
        s.inspector_drill(&client).await;
        assert_eq!(s.inspector.focus, InspectorPane::Types);
        assert!(matches!(s.inspector.lineage, Load::Idle));
    }

    #[test]
    fn selected_object_resolves_for_drill() {
        let s = InspectorState {
            types: Load::Loaded(OntologyTypes {
                objects: vec![object("Person")],
                links: vec![],
            }),
            ..Default::default()
        };
        assert_eq!(s.selected_type().map(|r| r.name.as_str()), Some("Person"));
    }
}
