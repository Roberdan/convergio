//! Key dispatch.
//!
//! Translates a [`crossterm::event::KeyEvent`] into a high-level
//! [`Action`] that the event loop in `lib.rs` consumes. Centralising
//! the mapping keeps `lib.rs` short and makes per-key behaviour
//! testable without spinning a terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// High-level action produced by a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Quit the dashboard and restore the terminal.
    Quit,
    /// Skip the tick wait and refresh now.
    RefreshNow,
    /// Move focus to the next pane in tab order.
    PaneNext,
    /// Move focus to the previous pane.
    PanePrev,
    /// Cursor down within the focused pane.
    RowDown,
    /// Cursor up within the focused pane.
    RowUp,
    /// Drill into the row selected in the focused pane.
    Drill,
    /// Pop one level back: leave detail mode, or quit when already
    /// at the overview. The overview/detail split lives in
    /// [`crate::state::AppState`]; the keymap only emits the intent.
    Back,
    /// Toggle whether `terminated` / `retired` agents are listed in
    /// the Agents pane. Default is to hide; pressing `e` reveals
    /// them so an operator can audit historical runs.
    ToggleHideExited,
    /// Toggle whether terminal-status tasks (`done`/`failed`) are
    /// listed in the Tasks pane. Default is to hide; pressing `t`
    /// reveals them so an operator can audit historical task runs.
    ToggleShowTerminalTasks,
    /// Key was bound to no action — caller ignores.
    Noop,
}

/// Default key binding. Stateless. Cloneable.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyMap;

impl KeyMap {
    /// Map a key event to an [`Action`].
    pub fn translate(&self, key: KeyEvent) -> Action {
        // Ctrl+C is always quit, regardless of focus.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Drill,
            KeyCode::Char('r') => Action::RefreshNow,
            KeyCode::Tab => Action::PaneNext,
            KeyCode::BackTab => Action::PanePrev,
            KeyCode::Char('j') | KeyCode::Down => Action::RowDown,
            KeyCode::Char('k') | KeyCode::Up => Action::RowUp,
            KeyCode::Char('e') => Action::ToggleHideExited,
            KeyCode::Char('t') => Action::ToggleShowTerminalTasks,
            _ => Action::Noop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_quits_unconditionally() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Char('q'))), Action::Quit);
    }

    #[test]
    fn esc_emits_back_for_dispatcher_to_resolve() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Esc)), Action::Back);
    }

    #[test]
    fn enter_emits_drill() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Enter)), Action::Drill);
    }

    #[test]
    fn ctrl_c_quits_even_if_inside_input() {
        let km = KeyMap;
        assert_eq!(km.translate(ctrl(Char('c'))), Action::Quit);
    }

    #[test]
    fn r_refreshes() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Char('r'))), Action::RefreshNow);
    }

    #[test]
    fn tab_moves_pane_forward_and_shift_tab_back() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Tab)), Action::PaneNext);
        assert_eq!(km.translate(key(BackTab)), Action::PanePrev);
    }

    #[test]
    fn j_k_arrows_move_rows() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Char('j'))), Action::RowDown);
        assert_eq!(km.translate(key(Down)), Action::RowDown);
        assert_eq!(km.translate(key(Char('k'))), Action::RowUp);
        assert_eq!(km.translate(key(Up)), Action::RowUp);
    }

    #[test]
    fn unbound_keys_are_noop() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Char('x'))), Action::Noop);
    }

    #[test]
    fn e_toggles_hide_exited() {
        let km = KeyMap;
        assert_eq!(km.translate(key(Char('e'))), Action::ToggleHideExited);
    }

    #[test]
    fn t_toggles_show_terminal_tasks() {
        let km = KeyMap;
        assert_eq!(
            km.translate(key(Char('t'))),
            Action::ToggleShowTerminalTasks
        );
    }
}
