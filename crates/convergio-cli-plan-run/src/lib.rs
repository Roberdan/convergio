//! Plan-run orchestrator for Convergio: `cvg plan run`. Extracted from
//! `convergio-cli` to honour the per-crate hard cap (CONSTITUTION
//! § Agent context budget); same pattern as ADR-0040 for `cvg pr`
//! and ADR-0041 for `cvg session`.
//!
//! The `convergio-cli` binary delegates `cvg plan run` here through a
//! thin shim that translates its own `Client` / `OutputMode` to the
//! local mirrors defined below.

pub mod runner;
pub(crate) mod wave;

pub use runner::run;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Output rendering mode. Mirrors `convergio_cli::commands::OutputMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// Localized human output.
    Human,
    /// Pretty JSON for scripts and agents.
    Json,
    /// Minimal plain text for shell pipelines.
    Plain,
}

/// Tiny HTTP helper. Mirrors `convergio_cli::commands::Client` so this
/// crate has no back-edge on the CLI.
pub struct Client {
    base: String,
    inner: reqwest::Client,
}

impl Client {
    /// Build with the daemon base URL.
    pub fn new(base: String) -> Self {
        let purpose = std::env::var("CONVERGIO_PURPOSE_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());

        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&purpose) {
            headers.insert(convergio_api::PURPOSE_ID_HEADER, v);
        }

        let inner = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest client");

        Self { base, inner }
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
