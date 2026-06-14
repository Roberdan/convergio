//! Shared HTTP helpers for `cvg` daemon clients.
//!
//! Centralises purpose-id resolution and header construction so every
//! subcommand that talks to the daemon carries `x-purpose-id` consistently.
//! Fixes the post-#443 regression where several callers used a bare
//! `reqwest::Client` and received 400 from the purpose-binding middleware.

/// Resolve the GDPR processing purpose id for outbound daemon requests.
///
/// Reads `CONVERGIO_PURPOSE_ID` from the environment. Falls back to
/// [`convergio_api::DEFAULT_PURPOSE_ID`] (the nil UUID) when the variable is
/// absent or empty.
pub fn purpose_id() -> String {
    std::env::var("CONVERGIO_PURPOSE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| convergio_api::DEFAULT_PURPOSE_ID.to_string())
}

/// Build a `HeaderMap` containing the resolved `x-purpose-id` header.
///
/// Returns an empty map on the (unlikely) event that the resolved string is
/// not valid HTTP header value bytes — so callers building a `reqwest::Client`
/// silently degrade to sending no header rather than panicking.
pub fn purpose_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&purpose_id()) {
        headers.insert(convergio_api::PURPOSE_ID_HEADER, v);
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the nil-UUID fallback when no env var is set.
    ///
    /// NOTE: mutates the process environment; run with `--test-threads=1` if
    /// another test in the same binary sets `CONVERGIO_PURPOSE_ID`.
    #[test]
    fn purpose_headers_default_is_nil_uuid() {
        let was = std::env::var("CONVERGIO_PURPOSE_ID").ok();
        std::env::remove_var("CONVERGIO_PURPOSE_ID");

        let headers = purpose_headers();
        let value = headers
            .get(convergio_api::PURPOSE_ID_HEADER)
            .expect("header must be present");
        assert_eq!(
            value.to_str().unwrap(),
            convergio_api::DEFAULT_PURPOSE_ID,
            "default must be the nil UUID"
        );

        // Restore original env state.
        match was {
            Some(v) => std::env::set_var("CONVERGIO_PURPOSE_ID", v),
            None => std::env::remove_var("CONVERGIO_PURPOSE_ID"),
        }
    }

    /// Verify that `CONVERGIO_PURPOSE_ID` overrides the nil-UUID default.
    #[test]
    fn purpose_headers_honours_env_override() {
        let custom = "12345678-1234-1234-1234-123456789abc";
        let was = std::env::var("CONVERGIO_PURPOSE_ID").ok();
        std::env::set_var("CONVERGIO_PURPOSE_ID", custom);

        let headers = purpose_headers();
        let value = headers
            .get(convergio_api::PURPOSE_ID_HEADER)
            .expect("header must be present");
        assert_eq!(value.to_str().unwrap(), custom);

        // Restore original env state.
        match was {
            Some(v) => std::env::set_var("CONVERGIO_PURPOSE_ID", v),
            None => std::env::remove_var("CONVERGIO_PURPOSE_ID"),
        }
    }
}
