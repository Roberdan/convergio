//! Shared route-input validators.
//!
//! Several bus/context routes accept a `limit` (or `message_limit`)
//! query/body field bounded by the same `[1, 100]` window. The
//! helpers below keep that bound and its error envelope in one
//! place while still allowing each route to surface a stable,
//! route-specific error `code` and field name.

use crate::error::ApiError;

/// Maximum number of messages a single poll/tail may return.
///
/// Shared between `routes::messages`, `routes::system_messages`, and
/// `routes::context` — the audit found this constant duplicated
/// across all three (advisory low-severity finding, 2026-05-12).
pub const MAX_MESSAGE_LIMIT: i64 = 100;

/// Validate that `limit` is in `[1, MAX_MESSAGE_LIMIT]`.
///
/// On rejection returns an `ApiError::BadRequest` whose `code` is the
/// caller-supplied stable code and whose `message` quotes the
/// caller-supplied field name. Keeping `code` and `field` per-route
/// preserves the existing client contract while sharing the bound.
pub fn validate_message_limit(
    limit: i64,
    code: &'static str,
    field: &'static str,
) -> Result<i64, ApiError> {
    if (1..=MAX_MESSAGE_LIMIT).contains(&limit) {
        Ok(limit)
    } else {
        Err(ApiError::BadRequest {
            code,
            message: format!("{field} must be between 1 and {MAX_MESSAGE_LIMIT}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounds() {
        assert_eq!(
            validate_message_limit(1, "invalid_message_limit", "limit").unwrap(),
            1
        );
        assert_eq!(
            validate_message_limit(MAX_MESSAGE_LIMIT, "invalid_message_limit", "limit").unwrap(),
            MAX_MESSAGE_LIMIT
        );
    }

    #[test]
    fn rejects_below_one() {
        let err = validate_message_limit(0, "invalid_message_limit", "limit").unwrap_err();
        match err {
            ApiError::BadRequest { code, message } => {
                assert_eq!(code, "invalid_message_limit");
                assert!(message.contains("limit"));
                assert!(message.contains("100"));
            }
            _ => panic!("expected BadRequest, got something else"),
        }
    }

    #[test]
    fn rejects_above_max() {
        let err = validate_message_limit(
            MAX_MESSAGE_LIMIT + 1,
            "invalid_context_limit",
            "message_limit",
        )
        .unwrap_err();
        match err {
            ApiError::BadRequest { code, message } => {
                assert_eq!(code, "invalid_context_limit");
                assert!(message.starts_with("message_limit must"));
            }
            _ => panic!("expected BadRequest"),
        }
    }

    #[test]
    fn rejects_negative() {
        assert!(validate_message_limit(-5, "invalid_message_limit", "limit").is_err());
    }
}
