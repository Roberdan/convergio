//! # convergio-tui — terminal dashboard for `cvg dash`
//!
//! Read-only multi-pane console (Plans, Tasks, Agents, PRs, Bus) that
//! refreshes on a tick. Talks to the local Convergio daemon over HTTP
//! and shells out to `gh pr list` for the PRs pane.
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

pub mod agent_filter;
pub mod bus_stream;
pub mod client;
pub mod client_gh;
pub mod client_pr_cache;
pub mod header_banner;
pub(crate) mod http;
pub mod keymap;
pub mod mode;
pub mod navigation;
pub mod plan_counts;
pub mod render;
pub mod scope;
pub mod state;
pub mod state_lifecycle;
pub mod text_util;
pub mod theme;
pub mod tick;
pub mod time_fmt;
pub mod types;

pub mod panes {
    //! Per-pane renderers. Each module is independent and only depends
    //! on [`crate::state`] for input.

    pub mod agents;
    pub mod bus;
    pub mod detail;
    pub mod plans;
    pub mod prs;
    pub mod tasks;
}

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::client::{Client, PrSummary, Snapshot};
use crate::keymap::{Action, KeyMap};
use crate::state::{AppMode, AppState};

/// One step of a progressive snapshot. The dashboard refresh emits
/// these in order: `Core` first (no gh shell-out, ~50ms), then
/// `Prs` once for the open list (~0.5s), then `Prs` again for
/// open+closed combined (~1-3s on busy repos). Each event lets the
/// UI repaint without waiting for the slowest part.
#[derive(Debug)]
pub enum SnapshotEvent {
    /// Core dataset (plans, tasks, agents, messages, audit). PRs
    /// arrive separately via [`SnapshotEvent::Prs`].
    Core(Result<Snapshot>),
    /// Updated PR list — replaces whatever was there. Sent twice
    /// per refresh (open-only, then open+closed).
    Prs(Vec<PrSummary>),
}

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
    // Mouse capture deliberately not enabled — no mouse handler exists,
    // and capturing mouse events stole the terminal's native scroll
    // while spamming the input poll with `Noop`-translated events.
    execute!(stdout, EnterAlternateScreen).context("enter alt screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("ratatui terminal")
}

fn restore_terminal(term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().ok();
    execute!(term.backend_mut(), LeaveAlternateScreen).ok();
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
    let mut state = AppState {
        bus_stream: Some(crate::bus_stream::spawn(daemon_url.to_string())),
        ..AppState::default()
    };
    let keymap = KeyMap;

    // Snapshot events flow back through this channel. Capacity of
    // 4 holds the worst-case in-flight set (Core + 2× Prs from one
    // refresh, plus a margin for an overlapping tick) without ever
    // blocking the producer; we are the single consumer.
    let (snap_tx, mut snap_rx) = mpsc::channel::<SnapshotEvent>(4);
    let mut refresh_in_flight = false;

    // Kick the first refresh off the event loop so the skeleton frame
    // renders immediately. Previously `state.refresh().await` here
    // blocked the first paint behind a ~1s `gh pr list`.
    spawn_refresh(&client, &snap_tx, &mut refresh_in_flight);

    let mut interval = tokio::time::interval(Duration::from_secs(tick_secs));
    interval.tick().await; // first tick fires immediately; consume it
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        if matches!(state.focus, crate::state::Pane::Bus) {
            state.merge_live_bus_pub();
        }
        term.draw(|f| render::root(f, &state))
            .context("render frame")?;

        tokio::select! {
            _ = interval.tick() => {
                spawn_refresh(&client, &snap_tx, &mut refresh_in_flight);
            }
            Some(event) = snap_rx.recv() => {
                match event {
                    SnapshotEvent::Core(snap) => {
                        // Free the in-flight slot as soon as the core
                        // arrives; the trailing PR fetches keep
                        // running but the next tick can already start
                        // a fresh core fetch if the user pressed `r`.
                        refresh_in_flight = false;
                        state.apply_snapshot(snap);
                    }
                    SnapshotEvent::Prs(prs) => state.apply_prs(prs),
                }
            }
            poll = poll_key() => {
                if let Some(action) = poll? {
                    match keymap.translate(action) {
                        Action::Quit => break,
                        Action::RefreshNow => {
                            spawn_refresh(&client, &snap_tx, &mut refresh_in_flight);
                        }
                        Action::PaneNext => state.focus_next(),
                        Action::PanePrev => state.focus_prev(),
                        Action::RowDown => state.row_down(),
                        Action::RowUp => state.row_up(),
                        Action::Drill => {
                            if matches!(state.mode, AppMode::Overview) {
                                if matches!(state.focus, crate::state::Pane::Bus) {
                                    if let Some(target) = state.drill_target() {
                                        state.enter_detail(&client, target).await;
                                    }
                                } else {
                                    state.apply_scope_from_focus();
                                }
                            }
                        }
                        Action::Back => match state.mode {
                            AppMode::Detail(_) => state.back_to_overview(),
                            AppMode::Overview => {
                                if !state.clear_scope() {
                                    break;
                                }
                            }
                        },
                        Action::ToggleHideExited => state.toggle_show_exited_agents(),
                        Action::ToggleShowTerminalTasks => state.toggle_show_terminal_tasks(),
                        Action::Noop => {}
                    }
                }
            }
        }
    }
    Ok(())
}

/// Spawn the progressive snapshot fetch off the event loop.
///
/// Emits three events on `tx` so the dashboard can repaint in
/// stages instead of waiting for the slowest fetch:
///
/// 1. [`SnapshotEvent::Core`] — every dataset except PRs (~50ms
///    on a warm pool). Frees `in_flight` so the next tick can
///    overlap the trailing PR fetches.
/// 2. [`SnapshotEvent::Prs`] with open PRs only (~0.5s).
/// 3. [`SnapshotEvent::Prs`] with open + closed combined
///    (~1-3s on busy repos with `statusCheckRollup`-driven API
///    fan-out). Skipped when `gh` is disabled.
fn spawn_refresh(client: &Client, tx: &mpsc::Sender<SnapshotEvent>, in_flight: &mut bool) {
    if *in_flight {
        return;
    }
    *in_flight = true;
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let snap = client.snapshot_core().await;
        if tx.send(SnapshotEvent::Core(snap)).await.is_err() {
            return;
        }
        if !client.gh_enabled() {
            return;
        }
        let open = client.fetch_prs_open_cached().await;
        if tx.send(SnapshotEvent::Prs(open.clone())).await.is_err() {
            return;
        }
        let closed = client.fetch_prs_closed_cached().await;
        let mut combined = open;
        combined.extend(closed);
        let _ = tx.send(SnapshotEvent::Prs(combined)).await;
    });
}

/// Non-blocking key polling. Returns `None` when the available event
/// is not a key press (e.g. mouse, resize), so the caller's `select!`
/// can keep cycling without busy-waiting. Poll window kept short
/// (50ms) so keystrokes feel snappy — at this granularity the cost
/// is one cheap `spawn_blocking` round-trip per cycle.
async fn poll_key() -> Result<Option<event::KeyEvent>> {
    tokio::task::spawn_blocking(|| -> Result<Option<event::KeyEvent>> {
        if event::poll(Duration::from_millis(50)).context("poll")? {
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
