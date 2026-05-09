//! Post-hoc close reason validation.
//!
//! `task.closed_post_hoc` is a deliberate escape hatch (ADR-0026), but
//! it must remain auditable: every close must carry a reason that
//! points to something verifiable (PR, commit, UUID, ADR, friction id).

use crate::error::DurabilityError;
use regex::{Regex, RegexSet};
use std::sync::OnceLock;

/// Hard cap on the trimmed reason length (in Unicode scalar values).
///
/// The reason is stored inside the audit payload; we keep it bounded so
/// one operator mistake cannot bloat the chain.
pub const MAX_POST_HOC_REASON_CHARS: usize = 400;

fn provenance_tokens() -> &'static RegexSet {
    static SET: OnceLock<RegexSet> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"(?i)\bPR\s*#\d+\b",
            r"(?i)\bcommit\s*`?[0-9a-f]{7,40}`?\b",
            r"(?i)\bsha\s*`?[0-9a-f]{7,40}`?\b",
            r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
            r"(?i)\bF\d{1,3}\b",
            r"(?i)\bADR[-\s]*0*\d{1,4}\b",
        ])
        .expect("post-hoc provenance regex set must compile")
    })
}

fn control_chars() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\x00-\x1F\x7F]").expect("control-char regex must compile"))
}

/// Validate a trimmed reason for `task.closed_post_hoc`.
///
/// The caller is responsible for trimming and for returning
/// [`DurabilityError::PostHocReasonMissing`] when the trimmed string is
/// empty.
pub fn validate_post_hoc_reason(reason: &str) -> Result<(), DurabilityError> {
    if control_chars().is_match(reason) {
        return Err(DurabilityError::PostHocReasonInvalid {
            reason: "must be a single line (no control characters)".to_string(),
        });
    }

    if reason.chars().count() > MAX_POST_HOC_REASON_CHARS {
        return Err(DurabilityError::PostHocReasonInvalid {
            reason: format!("must be <= {MAX_POST_HOC_REASON_CHARS} characters"),
        });
    }

    if !provenance_tokens().is_match(reason) {
        return Err(DurabilityError::PostHocReasonInvalid {
            reason:
                "must reference a verifiable anchor (PR #123, commit abc1234, task UUID, ADR-0026, or F42)"
                    .to_string(),
        });
    }

    Ok(())
}
