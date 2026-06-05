//! Network-facing surface for the remote capability registry.
//!
//! The [`RegistryFetcher`] trait abstracts every remote read so the rest
//! of the daemon (verifier, audit hook, CLI) can drive the registry
//! protocol without depending on `reqwest`. Production code uses
//! [`HttpsRegistryFetcher`]; tests use [`MockFetcher`].
//!
//! Discipline (ADR-0072 § 5):
//!
//! - HTTPS-only — `http://`, `file://`, empty, or any other scheme is
//!   rejected at construction time.
//! - 10 s connect timeout, 30 s read timeout.
//! - 50 MB bundle cap by default; override per-instance for tests.
//! - All cross-origin redirects refused (`redirect::Policy::none()`).
//! - No live network in tests — `MockFetcher` is what the F1 unit
//!   tests exercise, and what F2 will inject into the install path.

use crate::error::{RegistryError, Result};
use crate::manifest::{CapabilityManifest, RegistryIndex};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Default per-bundle byte cap (ADR-0072 § 5).
pub const DEFAULT_BUNDLE_CAP_BYTES: u64 = 50 * 1024 * 1024;

/// Abstract registry I/O. Implementors **must** enforce per-call
/// resource caps; the cap is also re-checked by callers that decode
/// bundle bytes into capabilities.
#[async_trait]
pub trait RegistryFetcher: Send + Sync {
    /// `GET /v1/index.json`
    async fn index(&self) -> Result<RegistryIndex>;

    /// `GET /v1/<name>/manifest.json`
    async fn manifest(&self, name: &str) -> Result<CapabilityManifest>;

    /// `GET /v1/<name>/<version>.cap` — raw bundle bytes.
    async fn bundle(&self, name: &str, version: &str) -> Result<Vec<u8>>;

    /// `GET /v1/<name>/<version>.cap.sig` — detached Ed25519 signature.
    async fn signature(&self, name: &str, version: &str) -> Result<Vec<u8>>;

    /// Stable display URL for the registry root (used in audit rows
    /// and `cvg capability search` output).
    fn endpoint(&self) -> &str;
}

/// `reqwest`-backed [`RegistryFetcher`] suitable for production.
#[derive(Debug, Clone)]
pub struct HttpsRegistryFetcher {
    client: reqwest::Client,
    base: Url,
    endpoint_display: String,
    bundle_cap_bytes: u64,
}

impl HttpsRegistryFetcher {
    /// Construct a fetcher rooted at `base_url`. Refuses any non-HTTPS
    /// URL (including `file://`, empty strings, and `http://`).
    pub fn new(base_url: &str) -> Result<Self> {
        let parsed = Url::parse(base_url)
            .map_err(|e| RegistryError::InvalidUrl(format!("{}: {}", base_url, e)))?;
        if parsed.scheme() != "https" {
            return Err(RegistryError::InvalidUrl(format!(
                "{}: only https:// is supported",
                base_url
            )));
        }
        if parsed.host().is_none() {
            return Err(RegistryError::InvalidUrl(format!(
                "{}: missing host",
                base_url
            )));
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .use_rustls_tls()
            .build()
            .map_err(|e| RegistryError::network(format!("client build: {}", e)))?;
        Ok(Self {
            client,
            endpoint_display: parsed.as_str().trim_end_matches('/').to_string(),
            base: parsed,
            bundle_cap_bytes: DEFAULT_BUNDLE_CAP_BYTES,
        })
    }

    /// Override the per-bundle byte cap. Useful for tests and for
    /// operators who want a tighter local limit.
    pub fn with_bundle_cap_bytes(mut self, cap: u64) -> Self {
        self.bundle_cap_bytes = cap;
        self
    }

    fn join(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .map_err(|e| RegistryError::InvalidUrl(format!("{}: {}", path, e)))
    }

    async fn get_bytes_capped(&self, url: Url, cap: u64) -> Result<Vec<u8>> {
        let endpoint = url.as_str().to_string();
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| RegistryError::network(e.to_string()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::NotFound(endpoint.to_string()));
        }
        if !status.is_success() {
            return Err(RegistryError::invalid_response(
                endpoint,
                format!("HTTP {}", status),
            ));
        }
        if let Some(len) = resp.content_length() {
            if len > cap {
                return Err(RegistryError::BundleTooLarge { size: len, cap });
            }
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| RegistryError::network(e.to_string()))?;
        if bytes.len() as u64 > cap {
            return Err(RegistryError::BundleTooLarge {
                size: bytes.len() as u64,
                cap,
            });
        }
        Ok(bytes.to_vec())
    }
}

#[async_trait]
impl RegistryFetcher for HttpsRegistryFetcher {
    async fn index(&self) -> Result<RegistryIndex> {
        let url = self.join("v1/index.json")?;
        // 4 MB cap for index — the wire format is meant to stay small.
        let bytes = self.get_bytes_capped(url, 4 * 1024 * 1024).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn manifest(&self, name: &str) -> Result<CapabilityManifest> {
        let url = self.join(&format!("v1/{}/manifest.json", name))?;
        let bytes = self.get_bytes_capped(url, 1024 * 1024).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn bundle(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let url = self.join(&format!("v1/{}/{}.cap", name, version))?;
        self.get_bytes_capped(url, self.bundle_cap_bytes).await
    }

    async fn signature(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let url = self.join(&format!("v1/{}/{}.cap.sig", name, version))?;
        // Ed25519 signatures are 64 bytes; allow a small constant cap.
        self.get_bytes_capped(url, 4 * 1024).await
    }

    fn endpoint(&self) -> &str {
        &self.endpoint_display
    }
}

/// In-process [`RegistryFetcher`] for tests. Build with the seeded
/// builder API and inject into the install path under test.
#[derive(Debug, Default, Clone)]
pub struct MockFetcher {
    inner: Arc<MockState>,
}

#[derive(Debug, Default)]
struct MockState {
    endpoint: String,
    index: Option<RegistryIndex>,
    manifests: HashMap<String, CapabilityManifest>,
    bundles: HashMap<(String, String), Vec<u8>>,
    signatures: HashMap<(String, String), Vec<u8>>,
}

impl MockFetcher {
    /// Builder. Endpoint string is purely cosmetic for tests.
    pub fn builder() -> MockFetcherBuilder {
        MockFetcherBuilder::default()
    }
}

/// Builder for [`MockFetcher`].
#[derive(Debug, Default)]
pub struct MockFetcherBuilder {
    state: MockState,
}

impl MockFetcherBuilder {
    /// Set the display endpoint returned by [`RegistryFetcher::endpoint`].
    pub fn endpoint(mut self, e: impl Into<String>) -> Self {
        self.state.endpoint = e.into();
        self
    }

    /// Seed the registry-wide index document.
    pub fn index(mut self, index: RegistryIndex) -> Self {
        self.state.index = Some(index);
        self
    }

    /// Seed a per-capability manifest.
    pub fn manifest(mut self, name: impl Into<String>, m: CapabilityManifest) -> Self {
        self.state.manifests.insert(name.into(), m);
        self
    }

    /// Seed bundle bytes for `(name, version)`.
    pub fn bundle(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Self {
        self.state
            .bundles
            .insert((name.into(), version.into()), bytes);
        self
    }

    /// Seed signature bytes for `(name, version)`.
    pub fn signature(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Self {
        self.state
            .signatures
            .insert((name.into(), version.into()), bytes);
        self
    }

    /// Finalize.
    pub fn build(self) -> MockFetcher {
        MockFetcher {
            inner: Arc::new(self.state),
        }
    }
}

#[async_trait]
impl RegistryFetcher for MockFetcher {
    async fn index(&self) -> Result<RegistryIndex> {
        self.inner
            .index
            .clone()
            .ok_or_else(|| RegistryError::NotFound("index".into()))
    }

    async fn manifest(&self, name: &str) -> Result<CapabilityManifest> {
        self.inner
            .manifests
            .get(name)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(format!("manifest:{name}")))
    }

    async fn bundle(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        self.inner
            .bundles
            .get(&(name.to_string(), version.to_string()))
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(format!("bundle:{name}@{version}")))
    }

    async fn signature(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        self.inner
            .signatures
            .get(&(name.to_string(), version.to_string()))
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(format!("signature:{name}@{version}")))
    }

    fn endpoint(&self) -> &str {
        &self.inner.endpoint
    }
}

// Tests live in `tests/fetcher.rs` so this file stays under the
// 300-line cap.
