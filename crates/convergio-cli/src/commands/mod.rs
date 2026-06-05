//! CLI subcommand modules — one file per top-level command.

pub mod about;
pub mod actions;
pub mod agent;
mod agent_format;
mod agent_list;
mod agent_retire;
mod agent_retire_one;
mod agent_show;
mod agent_show_render;
mod agent_spawn;
mod agent_spawn_heartbeat;
mod agent_spawn_usage;
mod agent_spawn_wire;
pub mod audit;
pub mod bus;
pub mod bus_extra;
pub mod bus_render;
pub mod bus_tail;
pub mod capability;
mod capability_trust;
mod capability_types;
pub mod coherence;
pub mod crdt;
pub mod dash;
pub mod demo;
pub mod discover;
mod discover_render;
pub mod dispatch;
pub mod docs;
mod docs_generators;
mod docs_generators_crate;
mod docs_merge_driver;
mod docs_rewrite;
pub mod doctor;
mod doctor_env;
mod doctor_zombies;
pub mod embed;
pub mod evidence;
pub mod fleet;
pub(crate) mod fleet_build;
pub(crate) mod fleet_cleanup;
pub(crate) mod fleet_cleanup_render;
pub(crate) mod fleet_detect;
pub(crate) mod fleet_dispatch;
pub(crate) mod fleet_duplicates;
pub(crate) mod fleet_patterns;
pub(crate) mod fleet_plan;
pub mod gates;
pub mod graph;
mod graph_query;
mod graph_render;
pub mod health;
pub mod mcp;
pub mod monitor;
pub mod ontology;
pub mod ontology_diff;
pub mod ontology_types;
pub mod plan;
mod plan_run;
pub mod plan_templates;
mod plan_triage;
pub mod pr;
pub mod service;
pub(crate) mod service_port;
pub(crate) mod service_unit;
pub mod session;
pub mod setup;
mod setup_agent_prompt;
pub mod setup_fleet;
mod setup_prompts;
mod setup_readme;
mod setup_repo_path;
mod setup_scripts;
pub(crate) mod setup_self_check;
pub mod solve;
pub mod status;
pub mod status_agents;
mod status_render;
pub mod task;
mod task_complete;
mod task_pr_url;
mod task_render;
pub mod task_templates;
pub mod update;
mod update_release_notes;
mod update_repo_root;
mod update_run;
pub mod validate;
pub mod workspace;

use anyhow::{Context, Result};
use clap::ValueEnum;
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
        // Default purpose id: callers can override with CONVERGIO_PURPOSE_ID.
        // This keeps `cvg` functional while the server requires purpose-binding.
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

    /// `GET path` and return the raw response bytes. Used by
    /// byte-identical export endpoints (ontology, actions.json) that
    /// must not round-trip through `serde_json::to_string`.
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let url = format!("{}{}", self.base, path);
        let resp = self
            .inner
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = resp.status();
        let bytes = resp.bytes().await.with_context(|| format!("read {url}"))?;
        if !status.is_success() {
            anyhow::bail!("GET {url} → {status}: {}", String::from_utf8_lossy(&bytes));
        }
        Ok(bytes.to_vec())
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

async fn json_or_err<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    let text = resp.text().await.context("reading response body")?;
    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {text}");
    }
    serde_json::from_str(&text).with_context(|| format!("parsing JSON: {text}"))
}
