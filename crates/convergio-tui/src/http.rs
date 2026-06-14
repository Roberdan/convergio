//! HTTP helper utilities for TUI daemon clients.
//!
//! Centralises purpose-id resolution so `client.rs` and `bus_stream.rs`
//! both carry `x-purpose-id` without duplicating the env-read logic.
//! Fixes the post-#443 regression where both used bare `reqwest::Client`
//! builders and received 400 from the purpose-binding middleware.

/// Nil-UUID default for `x-purpose-id` when the env var is not set.
/// Mirrors `convergio_api::DEFAULT_PURPOSE_ID`; inlined to avoid adding
/// a new dependency on `convergio-api` to this crate.
const DEFAULT_PURPOSE_ID: &str = "00000000-0000-0000-0000-000000000000";

/// HTTP header name. Mirrors `convergio_api::PURPOSE_ID_HEADER`.
const PURPOSE_ID_HEADER: &str = "x-purpose-id";

/// Build a `HeaderMap` containing the `x-purpose-id` header.
///
/// Reads `CONVERGIO_PURPOSE_ID` from the environment; falls back to the
/// nil-UUID default when absent or empty.
pub(crate) fn purpose_headers() -> reqwest::header::HeaderMap {
    let purpose = std::env::var("CONVERGIO_PURPOSE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PURPOSE_ID.to_string());
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&purpose) {
        headers.insert(PURPOSE_ID_HEADER, v);
    }
    headers
}
