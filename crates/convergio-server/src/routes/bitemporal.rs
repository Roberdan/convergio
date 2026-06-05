//! Shared `as_of` / `tx_as_of` bitemporal query parsing for read routes.

use crate::error::ApiError;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Common bitemporal query params accepted by read routes.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BitemporalQuery {
    /// Valid-time “as of” (ISO-8601 / RFC3339). Defaults to now.
    #[serde(default)]
    pub(crate) as_of: Option<String>,
    /// Transaction-time “as of” (ISO-8601 / RFC3339). Defaults to now.
    #[serde(default)]
    pub(crate) tx_as_of: Option<String>,
}

impl BitemporalQuery {
    /// Validate and parse query params, applying now/now defaults.
    pub(crate) fn parse(&self) -> Result<(DateTime<Utc>, DateTime<Utc>), ApiError> {
        parse_bitemporal(self.as_of.as_deref(), self.tx_as_of.as_deref())
    }
}

/// Parse optional `as_of` and `tx_as_of` timestamps.
///
/// Missing values default to the same `Utc::now()` instant.
pub(crate) fn parse_bitemporal(
    as_of: Option<&str>,
    tx_as_of: Option<&str>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), ApiError> {
    let now = Utc::now();
    let as_of = match as_of {
        Some(v) => parse_one("as_of", v)?,
        None => now,
    };
    let tx_as_of = match tx_as_of {
        Some(v) => parse_one("tx_as_of", v)?,
        None => now,
    };
    Ok((as_of, tx_as_of))
}

fn parse_one(field: &'static str, value: &str) -> Result<DateTime<Utc>, ApiError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(ApiError::Validation {
            code: "invalid_timestamp",
            message: format!("{field} must be a non-empty ISO-8601 timestamp"),
        });
    }
    let parsed = DateTime::parse_from_rfc3339(v).map_err(|_| ApiError::Validation {
        code: "invalid_timestamp",
        message: format!("invalid {field}: expected ISO-8601/RFC3339, got {value:?}"),
    })?;
    Ok(parsed.with_timezone(&Utc))
}
