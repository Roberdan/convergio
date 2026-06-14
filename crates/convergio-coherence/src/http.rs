//! Shared HTTP client helper for daemon-backed coherence verifiers.
//!
//! Fixes the post-#443 regression where verifiers used `reqwest::Client::new()`
//! without the `x-purpose-id` header and received 400 from the daemon.

use anyhow::{Context, Result};
use std::time::Duration;

/// Nil-UUID default for `x-purpose-id` when `CONVERGIO_PURPOSE_ID` is not set.
/// Mirrors `convergio_api::DEFAULT_PURPOSE_ID`; inlined to avoid adding a new
/// crate dependency.
const DEFAULT_PURPOSE_ID: &str = "00000000-0000-0000-0000-000000000000";

/// HTTP header name. Mirrors `convergio_api::PURPOSE_ID_HEADER`.
const PURPOSE_ID_HEADER: &str = "x-purpose-id";

/// Build a `reqwest::Client` with the given timeout and the default
/// purpose-binding header pre-applied.
///
/// All daemon-backed verifiers should build their clients through this
/// function so the purpose-enforcement middleware accepts their requests.
pub(crate) fn daemon_client(timeout: Duration) -> Result<reqwest::Client> {
    let purpose = std::env::var("CONVERGIO_PURPOSE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PURPOSE_ID.to_string());
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&purpose) {
        headers.insert(PURPOSE_ID_HEADER, v);
    }
    reqwest::Client::builder()
        .timeout(timeout)
        .default_headers(headers)
        .build()
        .with_context(|| "build http client")
}
