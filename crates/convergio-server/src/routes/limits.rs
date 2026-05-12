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

    fn assert_bad_request(
        result: Result<i64, ApiError>,
        expected_code: &str,
        expected_field: &str,
    ) {
        match result {
            Err(ApiError::BadRequest { code, message }) => {
                assert_eq!(code, expected_code);
                assert!(
                    message.starts_with(&format!("{expected_field} must")),
                    "got message {message:?}",
                );
                assert!(message.contains("100"));
            }
            Err(_) => panic!("expected BadRequest"),
            Ok(v) => panic!("expected BadRequest, got Ok({v})"),
        }
    }

    fn assert_ok(result: Result<i64, ApiError>, expected: i64) {
        match result {
            Ok(v) => assert_eq!(v, expected),
            Err(_) => panic!("expected Ok({expected})"),
        }
    }

    #[test]
    fn accepts_bounds() {
        assert_ok(
            validate_message_limit(1, "invalid_message_limit", "limit"),
            1,
        );
        assert_ok(
            validate_message_limit(MAX_MESSAGE_LIMIT, "invalid_message_limit", "limit"),
            MAX_MESSAGE_LIMIT,
        );
    }

    #[test]
    fn rejects_below_one() {
        assert_bad_request(
            validate_message_limit(0, "invalid_message_limit", "limit"),
            "invalid_message_limit",
            "limit",
        );
    }

    #[test]
    fn rejects_above_max() {
        assert_bad_request(
            validate_message_limit(
                MAX_MESSAGE_LIMIT + 1,
                "invalid_context_limit",
                "message_limit",
            ),
            "invalid_context_limit",
            "message_limit",
        );
    }

    #[test]
    fn rejects_negative() {
        assert_bad_request(
            validate_message_limit(-5, "invalid_message_limit", "limit"),
            "invalid_message_limit",
            "limit",
        );
    }
}
