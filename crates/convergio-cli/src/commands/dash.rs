//! `cvg dash` — open the TUI dashboard.
//!
//! Tiny shim that hands control to the [`convergio_tui`] crate. The
//! crate boundary is intentional (ADR-0029): keeping ratatui and
//! crossterm out of the daemon and out of every cvg subcommand keeps
//! their dependency tree off the hot CLI path. Read
//! [crate-level AGENTS.md](../../convergio-tui/AGENTS.md) before
//! changing the dashboard surface.

use anyhow::Result;

/// Default heartbeat staleness threshold (in seconds) for the
/// opportunistic `retire-stale` sweep run at dash startup. One hour
/// matches the daemon-side default in `routes::agent_registry`.
const DEFAULT_RETIRE_STALE_THRESHOLD_SECS: i64 = 3600;

/// Entry point for `cvg dash`. Resolves the workspace's GitHub slug
/// (best-effort) so the PRs pane is scoped to this repository
/// regardless of cwd. Forwards everything to
/// [`convergio_tui::run`], which owns terminal setup/teardown.
///
/// Before launching the TUI we issue one best-effort `retire-stale`
/// POST against the daemon (P2-11 step 2). This trims the registry
/// of agents that stopped heart-beating long ago so the dash's
/// Agents pane is not buried under historical rows. Failures are
/// logged via stderr but never block dash startup; setting
/// `CONVERGIO_DASH_NO_RETIRE_STALE=1` skips the sweep entirely.
pub async fn run(daemon_url: &str, tick_secs: u64) -> Result<()> {
    if std::env::var("CONVERGIO_DASH_NO_RETIRE_STALE")
        .ok()
        .as_deref()
        != Some("1")
    {
        opportunistic_retire_stale(daemon_url).await;
    }
    let slug = super::update_repo_root::resolve()
        .ok()
        .and_then(|root| super::update_repo_root::github_slug(&root));
    convergio_tui::run(daemon_url, tick_secs, slug).await
}

async fn opportunistic_retire_stale(daemon_url: &str) {
    let url = format!("{daemon_url}/v1/agent-registry/agents/retire-stale");
    let body = serde_json::json!({
        "threshold_seconds": DEFAULT_RETIRE_STALE_THRESHOLD_SECS,
        "apply": true,
    });
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .default_headers(crate::http::purpose_headers())
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    if let Err(err) = client.post(&url).json(&body).send().await {
        eprintln!("cvg dash: retire-stale skipped ({err})");
    }
}
