//! Top-level layout and footer.
//!
//! Splits the frame into header (title bar), body (multi-pane grid), and
//! footer (status line). Each pane delegates to its own renderer in
//! [`crate::panes`].

use crate::header_banner;
use crate::panes;
use crate::state::{version_drift, AppMode, AppState, Connection, Pane, BINARY_VERSION};
use crate::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Draw one frame.
pub fn root(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let header_h = header_banner::header_height(area.width, area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h), // header
            Constraint::Min(8),           // body
            Constraint::Length(1),        // footer
        ])
        .split(area);
    draw_header(f, chunks[0], state);
    draw_body(f, chunks[1], state);
    draw_footer(f, chunks[2], state);
}

fn draw_header(f: &mut Frame, area: Rect, state: &AppState) {
    header_banner::render(f, area, &header_stats(state));
}

fn header_stats(state: &AppState) -> Vec<String> {
    let mut stats = vec![
        format!("plans:{}", state.plans.len()),
        format!("tasks:{}", state.tasks.len()),
        format!(
            "agents:{}",
            state.agent_processes.len().max(state.agents.len())
        ),
        format!("prs:{}", state.prs.len()),
        format!("bus:{}", state.messages.len()),
    ];
    match version_drift(state.daemon_version.as_deref()) {
        Some(daemon) => {
            stats.push(format!("⚠ v{BINARY_VERSION} ≠ v{daemon}"));
            stats.push("run cvg update".into());
        }
        None => match state.daemon_version.as_deref() {
            Some(d) => stats.push(format!("v{d}")),
            None => stats.push(format!("v{BINARY_VERSION}")),
        },
    }
    stats
}

fn draw_body(f: &mut Frame, area: Rect, state: &AppState) {
    if let AppMode::Detail(target) = &state.mode {
        panes::detail::render(f, area, state, target);
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Percentage(38),
            Constraint::Percentage(20),
        ])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    panes::plans::render(f, top[0], state, focused(state, Pane::Plans));
    panes::tasks::render(f, top[1], state, focused(state, Pane::Tasks));
    panes::agents::render(f, bot[0], state, focused(state, Pane::Agents));
    panes::prs::render(f, bot[1], state, focused(state, Pane::Prs));
    panes::bus::render(f, rows[2], state, focused(state, Pane::Bus));
}

fn focused(state: &AppState, pane: Pane) -> bool {
    state.focus == pane
}

fn draw_footer(f: &mut Frame, area: Rect, state: &AppState) {
    let conn = match state.connection {
        Connection::Initial => Span::styled("connecting", Style::default().fg(theme::WARNING)),
        Connection::Connected => Span::styled("connected", Style::default().fg(theme::SUCCESS)),
        Connection::Disconnected => {
            Span::styled("disconnected", Style::default().fg(theme::DANGER))
        }
    };
    let audit = match state.audit_ok {
        Some(true) => Span::styled("audit ✓", Style::default().fg(theme::SUCCESS)),
        Some(false) => Span::styled("audit ✗", Style::default().fg(theme::DANGER)),
        None => Span::styled("audit ?", theme::dim()),
    };
    let last = match state.last_refresh {
        Some(t) => format!("last {}", crate::time_fmt::clock_utc(t)),
        None => "last –".into(),
    };
    let pane_name = format!("pane: {}", state.focus.label());
    let help = match state.mode {
        AppMode::Overview => {
            "q quit  Enter scope  Esc clear  r refresh  Tab pane  j/k row  e exited"
        }
        AppMode::Detail(_) => "Esc back  q quit  r refresh  j/k scroll",
    };
    let line = Line::from(vec![
        conn,
        Span::styled(" · ", theme::dim()),
        audit,
        Span::styled(" · ", theme::dim()),
        Span::styled(last, theme::text()),
        Span::styled(" · ", theme::dim()),
        Span::styled(pane_name, theme::row_highlight()),
        Span::styled(" · ", theme::dim()),
        Span::styled(help, theme::dim()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// Common helper: build a bordered block with a title that signals
/// focus three independent ways (CONSTITUTION P3 — never rely on
/// colour alone):
/// 1. A `▶` glyph prefix in the title.
/// 2. An explicit fg+bg pair on the title (focus highlight palette).
/// 3. The focus colour on the border.
pub fn pane_block(title: &str, focused: bool) -> Block<'static> {
    let prefix = if focused { "▶ " } else { "  " };
    let owned_title = format!("{prefix}{title}");
    let title_style = if focused {
        theme::row_highlight()
    } else {
        Style::default().fg(theme::TEXT)
    };
    let border_style = if focused {
        Style::default()
            .fg(theme::FOCUS)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::DIM)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(owned_title, title_style))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn root_renders_without_panic_on_empty_state() {
        let backend = TestBackend::new(120, 40);
        let mut term = Terminal::new(backend).unwrap();
        let state = AppState::default();
        term.draw(|f| root(f, &state)).unwrap();
        let buf = term.backend().buffer();
        let dump = buf.content().iter().map(|c| c.symbol()).collect::<String>();
        assert!(
            dump.contains("█"),
            "banner block glyphs should be rendered: {dump:?}"
        );
        assert!(
            dump.contains(BINARY_VERSION),
            "subtitle should mention the binary version: {dump:?}"
        );
    }
}
