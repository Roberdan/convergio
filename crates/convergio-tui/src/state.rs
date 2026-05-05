//! Aggregate state for the dashboard.
//!
//! [`AppState`] owns the datasets the panes render plus the
//! focus + scroll position for each pane. Refreshes are issued by
//! [`AppState::refresh`] which delegates to [`crate::client::Client`].

use crate::bus_stream::{BusStreamHandle, Transport as BusTransport};
use crate::client::{
    AgentProcess, BusMessage, Client, Plan, PrSummary, RegistryAgent, TaskSummary,
};
pub use crate::mode::{AppMode, DetailTarget, Scope};

/// The panes rendered by the dashboard, in tab order.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// Plans (top-left). Default focus on startup.
    #[default]
    Plans,
    /// Tasks across plans (top-right).
    Tasks,
    /// Registered agents (bottom-left).
    Agents,
    /// Open and closed pull requests.
    Prs,
    /// Agent bus messages.
    Bus,
}

impl Pane {
    /// All panes in display order.
    pub const ALL: [Pane; 5] = [Pane::Plans, Pane::Tasks, Pane::Agents, Pane::Prs, Pane::Bus];

    /// Short label rendered as the pane title.
    pub fn label(&self) -> &'static str {
        match self {
            Pane::Plans => "Plans",
            Pane::Tasks => "Tasks",
            Pane::Agents => "Agents",
            Pane::Prs => "PRs",
            Pane::Bus => "Bus",
        }
    }
}

/// Per-pane scroll offset.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cursor {
    /// First row index visible in the pane.
    pub offset: usize,
    /// Selected row, relative to all rows in the pane (NOT to offset).
    pub selected: usize,
}

impl Cursor {
    /// Move the selection one row down, capped at `max_idx`. Adjusts
    /// the offset only enough to keep the selection visible.
    pub fn down(&mut self, max_idx: usize, page: usize) {
        if max_idx == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(max_idx - 1);
        if self.selected >= self.offset + page {
            self.offset = self.selected + 1 - page;
        }
    }

    /// Move the selection one row up.
    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }
}

/// Connection / refresh status surfaced in the footer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    /// First refresh has not completed yet.
    #[default]
    Initial,
    /// Last refresh succeeded.
    Connected,
    /// Last refresh failed (network or 4xx/5xx).
    Disconnected,
}

/// Aggregate dashboard state.
///
/// `Default` produces an empty state (no plans, etc.). Call
/// [`AppState::refresh`] to populate.
#[derive(Debug, Default)]
pub struct AppState {
    /// Plans returned by the daemon.
    pub plans: Vec<Plan>,
    /// Tasks across loaded plans.
    pub tasks: Vec<TaskSummary>,
    /// Registered agents.
    pub agents: Vec<RegistryAgent>,
    /// Layer-3 supervised agent processes.
    pub agent_processes: Vec<AgentProcess>,
    /// Open and closed pull requests via `gh pr list`.
    pub prs: Vec<PrSummary>,
    /// Recent plan-scoped bus messages.
    pub messages: Vec<BusMessage>,
    /// Audit chain ok/not.
    pub audit_ok: Option<bool>,
    /// Daemon version reported by `GET /v1/health`. `None` until the
    /// first successful refresh.
    pub daemon_version: Option<String>,
    /// Connection state for the footer.
    pub connection: Connection,
    /// UTC timestamp of the last successful refresh.
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    /// Currently focused pane.
    pub focus: Pane,
    /// Per-pane cursor.
    pub cursor: PaneCursors,
    /// Active UI mode (Overview vs drill-down).
    pub mode: AppMode,
    /// Cross-pane drill-down filter. Defaults to [`Scope::All`].
    pub scope: Scope,
    /// Cached task list for the plan currently being drilled into.
    /// Populated by [`AppState::enter_detail`] for `Plan` targets so
    /// the detail panel shows every task (not only the active subset
    /// that the overview pane carries).
    pub detail_tasks: Vec<TaskSummary>,
    /// Live SSE handle for the Bus pane (P1.3, ADR-0029 addendum).
    /// `None` = supervisor never started (renderer tests).
    #[doc(hidden)]
    pub bus_stream: Option<BusStreamHandle>,
    /// Plan id the Bus pane subscription currently follows.
    pub bus_following: Option<String>,
    /// P2-11: when `true`, exited rows are listed in the Agents pane.
    pub show_exited_agents: bool,
}

/// Cursors for the four panes, addressable by [`Pane`].
#[derive(Debug, Default, Clone, Copy)]
pub struct PaneCursors {
    /// Cursor for the Plans pane.
    pub plans: Cursor,
    /// Cursor for the Active Tasks pane.
    pub tasks: Cursor,
    /// Cursor for the Agents pane.
    pub agents: Cursor,
    /// Cursor for the PRs pane.
    pub prs: Cursor,
    /// Cursor for the bus pane.
    pub bus: Cursor,
}

/// Compile-time version of the `cvg` binary embedding this dashboard.
/// Compared against the live daemon version to surface drift.
pub const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `Some(daemon)` when the daemon and the binary report different
/// versions, `None` when they match or the daemon is unreachable.
pub fn version_drift(daemon: Option<&str>) -> Option<String> {
    let d = daemon?;
    if d == BINARY_VERSION {
        None
    } else {
        Some(d.to_string())
    }
}

impl AppState {
    /// Refresh every dataset. Failures roll up into
    /// [`Connection::Disconnected`] and leave the previous data in
    /// place — the dashboard never blanks itself on a transient
    /// network error.
    pub async fn refresh(&mut self, client: &Client) {
        let snapshot = client.snapshot().await;
        match snapshot {
            Ok(s) => {
                self.plans = s.plans;
                self.tasks = s.tasks;
                self.agents = s.agents;
                self.agent_processes = s.agent_processes;
                self.prs = s.prs;
                self.messages = s.messages;
                self.audit_ok = s.audit_ok;
                self.daemon_version = s.daemon_version;
                self.connection = Connection::Connected;
                self.last_refresh = Some(chrono::Utc::now());
            }
            Err(_) => {
                self.connection = Connection::Disconnected;
            }
        }
        self.update_bus_subscription();
        self.merge_live_bus();
    }

    /// Re-point the Bus pane SSE subscription at the currently-scoped
    /// plan, falling back to the first plan if no scope is set. No-op
    /// when the live handle was never installed.
    pub fn update_bus_subscription(&mut self) {
        let Some(handle) = &self.bus_stream else {
            return;
        };
        let target = self
            .scoped_plan_id()
            .map(|s| s.to_string())
            .or_else(|| self.plans.first().map(|p| p.id.clone()));
        if target != self.bus_following {
            handle.set_plan(target.clone());
            self.bus_following = target;
        }
    }

    /// Public re-export of [`AppState::merge_live_bus`] for callers
    /// that want to refresh the Bus pane between full snapshots.
    pub fn merge_live_bus_pub(&mut self) {
        self.merge_live_bus();
    }

    /// Fold the live SSE buffer into [`AppState::messages`] so
    /// scope-filtered helpers still work without changes. Live
    /// rows replace any stale poll snapshot for the same `seq`.
    fn merge_live_bus(&mut self) {
        let Some(handle) = &self.bus_stream else {
            return;
        };
        let live = handle.snapshot();
        if live.is_empty() {
            return;
        }
        // Newest-first incoming; merge by seq dedup, drop oldest.
        let mut merged: Vec<BusMessage> = live;
        let known: std::collections::HashSet<i64> = merged.iter().map(|m| m.seq).collect();
        for m in self.messages.drain(..) {
            if !known.contains(&m.seq) {
                merged.push(m);
            }
        }
        merged.sort_by_key(|m| std::cmp::Reverse(m.seq));
        merged.truncate(crate::bus_stream::BUFFER_CAP);
        self.messages = merged;
    }

    /// Toggle visibility of exited agents in the Agents pane (P2-11).
    pub fn toggle_show_exited_agents(&mut self) {
        self.show_exited_agents = !self.show_exited_agents;
    }

    /// Active transport for the Bus pane footer hint.
    pub fn bus_transport(&self) -> BusTransport {
        self.bus_stream
            .as_ref()
            .map(|h| h.transport())
            .unwrap_or(BusTransport::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_all_covers_four_panes() {
        assert_eq!(Pane::ALL.len(), 5);
    }

    #[test]
    fn focus_cycles_forward_and_backward() {
        let mut s = AppState::default();
        assert_eq!(s.focus, Pane::Plans);
        s.focus_next();
        s.focus_next();
        s.focus_next();
        s.focus_next();
        s.focus_next();
        assert_eq!(s.focus, Pane::Plans, "wraps after 5 hops");
        s.focus_prev();
        assert_eq!(s.focus, Pane::Bus);
    }

    #[test]
    fn cursor_down_caps_at_last_row_and_noop_on_empty() {
        let mut c = Cursor::default();
        for _ in 0..4 {
            c.down(3, 2);
        }
        assert_eq!(c.selected, 2);
        let mut c2 = Cursor::default();
        c2.down(0, 5);
        assert_eq!((c2.selected, c2.offset), (0, 0));
    }

    #[test]
    fn cursor_up_does_not_underflow() {
        let mut c = Cursor::default();
        c.up();
        assert_eq!(c.selected, 0);
    }
}
