//! Pull-request lifecycle commands for Convergio: `cvg pr stack`,
//! `cvg pr sync`, `cvg pr merge`, `cvg pr link`, `cvg pr who`.
//! Extracted from `convergio-cli` to honour the per-crate hard cap
//! (CONSTITUTION § Agent context budget); same pattern as ADR-0041
//! for `cvg session`.
//!
//! The `convergio-cli` binary delegates `cvg pr ...` here through a
//! thin shim that translates its own `Client` / `OutputMode` to the
//! local types defined below.

pub mod pr;
pub mod pr_diff;
pub mod pr_link;
pub mod pr_merge;
pub mod pr_merge_io;
pub mod pr_parse;
pub mod pr_render;
pub mod pr_sync;
pub mod pr_sync_parse;
pub mod pr_who;

pub use pr::{run, PrCommand};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Output rendering mode for pr commands. Mirrors
/// `convergio_cli::commands::OutputMode` so this crate has no
/// dependency back on the CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// Localized human output.
    Human,
    /// Pretty JSON for scripts and agents.
    Json,
    /// Minimal plain text for shell pipelines.
    Plain,
}

/// Tiny HTTP helper used by pr subcommands. Mirrors the shape of
/// `convergio_cli::commands::Client` but lives here so this crate
/// has no back-edge on the CLI.
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

    /// Daemon base URL.
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
