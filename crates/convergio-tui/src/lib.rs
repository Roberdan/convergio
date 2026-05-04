//! # convergio-tui — terminal dashboard for `cvg dash`
//!
//! Read-only 4-pane console (Plans, Active Tasks, Agents, PRs) that
//! refreshes on a tick. Talks to the local Convergio daemon over HTTP
//! and shells out to `gh pr list` for the open-PRs pane.
//!
//! Consumed only by the `cvg` binary (`convergio-cli`). Never imported
//! by the daemon, MCP bridge, or any other agent-facing surface.
//!
//! See [ADR-0029](../../docs/adr/0029-tui-dashboard-crate-separation.md)
//! for the boundary rationale, and `AGENTS.md` for invariants.
//!
//! ## Quickstart
//!
//! ```no_run
//! # async fn demo() -> anyhow::Result<()> {
//! convergio_tui::run("http://127.0.0.1:8420", 5, None).await
//! # }
//! ```
//!
//! Quit with `q`, refresh with `r`, change pane with `Tab`, scroll with
//! `j` / `k`.

pub mod client;
pub mod client_gh;
pub mod header_banner;
pub mod keymap;
pub mod mode;
pub mod plan_counts;
pub mod render;
pub mod scope;
pub mod state;
pub mod theme;
pub mod tick;

pub mod panes {
    //! Per-pane renderers. Each module is independent and only depends
    //! on [`crate::state`] for input.

    pub mod agents;
    pub mod detail;
    pub mod plans;
    pub mod prs;
    pub mod tasks;
}

use anyhow::{Context, Result};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::client::Client;
use crate::keymap::{Action, KeyMap};
use crate::state::{AppMode, AppState};

/// Tick interval bounds. Outside this band the dashboard is either
/// hammering the daemon (too fast) or sleeping past usefulness (too
/// slow); we clamp.
const TICK_BOUNDS: std::ops::RangeInclusive<u64> = 1..=300;

/// Entry point.
///
/// `daemon_url` is the base URL of the local Convergio daemon (e.g.
/// `http://127.0.0.1:8420`). `tick_secs` is the refresh interval in
/// seconds, clamped to `[1, 300]`. `github_slug`, when supplied,
/// scopes `gh pr list` to that `owner/repo` instead of inheriting
/// the operator's cwd — `cvg dash` derives it from the workspace's
/// `origin` remote.
pub async fn run(daemon_url: &str, tick_secs: u64, github_slug: Option<String>) -> Result<()> {
    let tick = tick_secs.clamp(*TICK_BOUNDS.start(), *TICK_BOUNDS.end());
    let mut term = setup_terminal().context("setup terminal")?;
    let result = event_loop(&mut term, daemon_url, tick, github_slug).await;
    restore_terminal(&mut term).ok();
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).context("enter alt screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("ratatui terminal")
}

fn restore_terminal(term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().ok();
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .ok();
    term.show_cursor().ok();
    Ok(())
}

async fn event_loop(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    daemon_url: &str,
    tick_secs: u64,
    github_slug: Option<String>,
) -> Result<()> {
    let client = Client::new(daemon_url.to_string()).with_github_slug(github_slug);
    let mut state = AppState::default();
    let keymap = KeyMap;
    state.refresh(&client).await;

    let mut interval = tokio::time::interval(Duration::from_secs(tick_secs));
    interval.tick().await; // first tick fires immediately; consume it
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        term.draw(|f| render::root(f, &state))
            .context("render frame")?;

        tokio::select! {
            _ = interval.tick() => {
                state.refresh(&client).await;
            }
            poll = poll_key() => {
                if let Some(action) = poll? {
                    match keymap.translate(action) {
                        Action::Quit => break,
                        Action::RefreshNow => state.refresh(&client).await,
                        Action::PaneNext => state.focus_next(),
                        Action::PanePrev => state.focus_prev(),
                        Action::RowDown => state.row_down(),
                        Action::RowUp => state.row_up(),
                        Action::Drill => {
                            if matches!(state.mode, AppMode::Overview) {
                                if let Some(target) = state.drill_target() {
                                    state.enter_detail(&client, target).await;
                                }
                            }
                        }
                        Action::Back => match state.mode {
                            AppMode::Detail(_) => state.back_to_overview(),
                            AppMode::Overview => break,
                        },
                        Action::Noop => {}
                    }
                }
            }
        }
    }
    Ok(())
}

/// Non-blocking key polling. Returns `None` when the available event
/// is not a key press (e.g. mouse, resize), so the caller's `select!`
/// can keep cycling without busy-waiting.
async fn poll_key() -> Result<Option<event::KeyEvent>> {
    tokio::task::spawn_blocking(|| -> Result<Option<event::KeyEvent>> {
        if event::poll(Duration::from_millis(200)).context("poll")? {
            if let Event::Key(k) = event::read().context("read")? {
                return Ok(Some(k));
            }
        }
        Ok(None)
    })
    .await
    .context("join blocking poll")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_secs_is_clamped_into_bounds() {
        assert!(TICK_BOUNDS.contains(&1));
        assert!(TICK_BOUNDS.contains(&300));
        assert!(!TICK_BOUNDS.contains(&0));
        assert!(!TICK_BOUNDS.contains(&301));
    }
}
