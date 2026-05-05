//! CLI subcommand modules — one file per top-level command.

pub mod about;
pub mod agent;
mod agent_format;
mod agent_list;
mod agent_retire;
mod agent_show;
mod agent_spawn;
mod agent_spawn_heartbeat;
mod agent_spawn_wire;
pub mod audit;
pub mod bus;
pub mod bus_extra;
pub mod bus_render;
pub mod bus_tail;
pub mod capability;
mod capability_types;
pub mod coherence;
pub mod crdt;
pub mod dash;
pub mod demo;
pub mod discover;
pub mod dispatch;
pub mod docs;
mod docs_generators;
mod docs_generators_crate;
mod docs_rewrite;
pub mod doctor;
pub mod embed;
pub mod evidence;
pub mod fleet;
pub(crate) mod fleet_build;
pub(crate) mod fleet_duplicates;
pub(crate) mod fleet_patterns;
pub mod graph;
mod graph_render;
pub mod health;
pub mod mcp;
pub mod monitor;
pub mod plan;
mod plan_triage;
pub mod pr;
pub mod service;
pub mod session;
pub mod setup;
mod setup_prompts;
mod setup_repo_path;
pub mod solve;
pub mod status;
mod status_render;
pub mod task;
mod task_render;
pub mod update;
mod update_release_notes;
mod update_repo_root;
mod update_run;
pub mod validate;
pub mod workspace;

use anyhow::{Context, Result};
use clap::ValueEnum;
use convergio_i18n::Bundle;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Global output rendering mode for commands that support multiple views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    /// Localized human output.
    Human,
    /// Pretty JSON for scripts and agents.
    Json,
    /// Minimal plain text for shell pipelines.
    Plain,
}

/// Tiny HTTP helper shared by subcommands.
#[derive(Clone)]
pub struct Client {
    base: String,
    inner: reqwest::Client,
}

impl Client {
    /// Build with the daemon base URL (e.g. `http://127.0.0.1:8420`).
    pub fn new(base: String) -> Self {
        Self {
            base,
            inner: reqwest::Client::new(),
        }
    }

    /// Daemon base URL — used by localized error messages.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// `GET path` and parse the JSON body into `T`.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .inner
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        json_or_err(resp).await
    }

    /// `POST path` with `body` and parse the JSON body into `T`.
    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .inner
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        json_or_err(resp).await
    }

    /// `PATCH path` with `body` and parse the JSON body into `T`.
    pub async fn patch<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .inner
            .patch(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("PATCH {url}"))?;
        json_or_err(resp).await
    }

    /// `DELETE path` and parse the JSON body into `T`.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .inner
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("DELETE {url}"))?;
        json_or_err(resp).await
    }
}

/// `true` when CLI and daemon versions differ and a drift warning should be emitted.
/// An empty `daemon_ver` (stub or unreachable) is treated as no-drift.
pub(crate) fn should_warn_drift(cli_ver: &str, daemon_ver: &str) -> bool {
    !daemon_ver.is_empty() && cli_ver != daemon_ver
}

/// Emit a one-line stderr warning when the CLI binary version differs from the
/// daemon's running version. Suppressed by `CONVERGIO_NO_DRIFT_WARN=1`.
/// Errors (daemon unreachable) are silently ignored — this is a best-effort check.
pub async fn maybe_warn_drift(client: &Client, bundle: &Bundle) {
    if std::env::var("CONVERGIO_NO_DRIFT_WARN").as_deref() == Ok("1") {
        return;
    }
    let Ok(body) = client.get::<serde_json::Value>("/v1/health").await else {
        return;
    };
    let daemon_ver = body
        .get("running_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let cli_ver = env!("CARGO_PKG_VERSION");
    if should_warn_drift(cli_ver, daemon_ver) {
        eprintln!(
            "{}",
            bundle.t(
                "cli-version-drift",
                &[("cli", cli_ver), ("daemon", daemon_ver)]
            )
        );
    }
}

async fn json_or_err<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    let text = resp.text().await.context("reading response body")?;
    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {text}");
    }
    serde_json::from_str(&text).with_context(|| format!("parsing JSON: {text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drift_when_versions_match() {
        assert!(!should_warn_drift("1.2.3", "1.2.3"));
    }

    #[test]
    fn drift_when_cli_newer() {
        assert!(should_warn_drift("1.3.0", "1.2.3"));
    }

    #[test]
    fn drift_when_cli_older() {
        assert!(should_warn_drift("1.2.3", "1.3.0"));
    }

    #[test]
    fn no_drift_when_daemon_ver_empty() {
        assert!(!should_warn_drift("1.2.3", ""));
    }
}
