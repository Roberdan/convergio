//! Snapshot lifecycle for [`crate::state::AppState`].
//!
//! Split out from `state.rs` so the main file stays under the
//! 300-line cap. Both entry points end in
//! [`AppState::apply_snapshot`]; the refresh wrapper exists only
//! for the legacy `Client`-driven test path.

use crate::client::{Client, Snapshot};
use crate::state::{AppState, Connection};

impl AppState {
    /// Refresh every dataset. Failures roll up into
    /// [`Connection::Disconnected`] and leave the previous data in
    /// place — the dashboard never blanks itself on a transient
    /// network error.
    pub async fn refresh(&mut self, client: &Client) {
        let snapshot = client.snapshot().await;
        self.apply_snapshot(snapshot);
    }

    /// Apply a pre-fetched [`Snapshot`] without re-awaiting the
    /// client. Used by the async refresh path in [`crate::run`] so
    /// the event loop never blocks on the snapshot. Failed fetches
    /// keep the previous data and flip the connection indicator.
    ///
    /// The PR list is only overwritten when the snapshot carries a
    /// non-empty vector. This protects the progressive-snapshot
    /// path: `Client::snapshot_core` returns an empty `prs` field
    /// because the gh shell-out runs separately, and we don't want
    /// the cached PR list to flash to empty between the core
    /// arrival and the first PR payload.
    pub fn apply_snapshot(&mut self, snapshot: anyhow::Result<Snapshot>) {
        match snapshot {
            Ok(s) => {
                let partial = s.partial;
                self.plans = s.plans;
                self.tasks = s.tasks;
                self.agents = s.agents;
                self.agent_processes = s.agent_processes;
                if !s.prs.is_empty() {
                    self.prs = s.prs;
                }
                self.messages = s.messages;
                self.audit_ok = s.audit_ok;
                self.daemon_version = s.daemon_version;
                self.connection = if partial {
                    Connection::Degraded
                } else {
                    Connection::Connected
                };
                self.last_refresh = Some(chrono::Utc::now());
            }
            Err(_) => {
                self.connection = Connection::Disconnected;
            }
        }
        self.update_bus_subscription();
        self.merge_live_bus_pub();
    }
}
