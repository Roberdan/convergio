//! Session lifecycle commands for Convergio.
//!
//! This crate hosts the `cvg session` suite extracted from
//! `convergio-cli` per ADR-0041 to honour the per-crate hard cap
//! (CONSTITUTION § 13). The two subcommands are:
//!
//! - [`session::SessionCommand::Resume`] — print a cold-start brief
//!   (daemon health, audit chain, the active plan, top pending
//!   tasks, open PRs, and an optional Tier-3 graph context-pack).
//! - [`session::SessionCommand::PreStop`] — end-of-session safety
//!   net (PRD-001 § Artefact 4): walks a registry of cheap checks
//!   and refuses to detach when findings are present unless
//!   `--force` is supplied.
//!
//! The verifiers and renderer are agent-callable from any binary
//! that adds `convergio-cli-session` to its `Cargo.toml` — skills
//! no longer need to shell out to `cvg`.

pub mod checks;
pub mod pre_stop;
pub mod pre_stop_run;
pub mod register_and_poll;
pub mod register_and_poll_render;
pub mod render;
pub mod session;

pub use session::{run, SessionCommand};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Output rendering mode for session commands.
///
/// Mirrors `convergio_cli::commands::OutputMode` so this crate has
/// no dependency back on the CLI. The CLI's enum is converted at
/// the shim boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// Localized human-readable output.
    Human,
    /// Pretty JSON for scripts and agents.
    Json,
    /// Minimal plain text for shell pipelines.
    Plain,
}

/// Tiny HTTP helper used by session subcommands.
///
/// Mirrors the shape of `convergio_cli::commands::Client` but lives
/// here so this crate has no back-edge on the CLI. The CLI shim
/// constructs one of these from its own `Client::base()`.
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

    /// Daemon base URL — exposed so checks can pass it down to
    /// shell-out tooling (`curl` etc.).
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
}

async fn json_or_err<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    let text = resp.text().await.context("reading response body")?;
    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {text}");
    }
    serde_json::from_str(&text).with_context(|| format!("parsing JSON: {text}"))
}
