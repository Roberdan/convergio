//! `POST /v1/audit/append` — agent-emitted custom audit rows.
//!
//! See [P2-2 retro A8] and [ADR-0002 § Custom kinds] for the rationale.
//! Agents that need to emit operational signals the daemon does not
//! natively model (pre-stop checks, coherence scans, retro boomerangs)
//! call this route instead of falling back to `tracing::info!` and
//! losing the hash-chain signal.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use convergio_durability::audit::EntityKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Body accepted by `POST /v1/audit/append`.
#[derive(Debug, Deserialize)]
pub(super) struct AppendBody {
    /// Stable dotted kind, e.g. `myapp.session.pre_stop.check.1`.
    pub kind: String,
    /// Logical entity affected — closed enum.
    pub entity_kind: AppendEntityKind,
    /// Opaque correlation key. Required, non-empty.
    pub entity_id: String,
    /// Agent that produced the row, when known.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Caller-controlled JSON object. Must be an object — scalars and
    /// arrays are rejected so payloads stay machine-readable.
    pub payload: Value,
}

/// Closed enum mirroring the wire contract spelled in P2-2:
/// `agent | task | plan | evidence | free`. Projects onto
/// [`EntityKind`].
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AppendEntityKind {
    /// Audit row keyed off an agent identity.
    Agent,
    /// Audit row keyed off a task.
    Task,
    /// Audit row keyed off a plan.
    Plan,
    /// Audit row keyed off an evidence row.
    Evidence,
    /// Free-form correlation key (agent picks the convention).
    Free,
}

impl From<AppendEntityKind> for EntityKind {
    fn from(value: AppendEntityKind) -> Self {
        match value {
            AppendEntityKind::Agent => EntityKind::Agent,
            AppendEntityKind::Task => EntityKind::Task,
            AppendEntityKind::Plan => EntityKind::Plan,
            AppendEntityKind::Evidence => EntityKind::Evidence,
            AppendEntityKind::Free => EntityKind::Free,
        }
    }
}

/// Response returned by `POST /v1/audit/append` on success.
#[derive(Debug, Serialize)]
pub(super) struct AppendResponse {
    /// New row sequence number.
    pub seq: i64,
    /// `sha256(prev_hash || canonical_json(payload))` hex.
    pub hash: String,
}

/// Reserved prefixes — daemon-owned audit row families. Any `kind`
/// starting with one of these is rejected with 422 `kind_reserved`.
const RESERVED_KIND_PREFIXES: &[&str] = &[
    "task.",
    "plan.",
    "evidence.",
    "crdt.",
    "workspace.",
    "capability.",
];

/// Reserved exact kind names — agent-lifecycle rows the daemon writes.
/// Agents must use a different kind (e.g. `myapp.session.started`) to
/// avoid colliding with daemon semantics.
const RESERVED_KIND_NAMES: &[&str] = &[
    "agent.session_started",
    "agent.retired",
    "agent.retired_stale",
];

/// Validate the dotted-kind grammar. Returns 400 `kind_invalid` on
/// shape mismatch.
///
/// Pattern: `^[a-z][a-z0-9_]*\.[a-z0-9_]+(\.[a-z0-9_]+)*$`. Hand-rolled
/// to avoid pulling `regex` into the server crate for one shape.
pub(super) fn validate_kind(kind: &str) -> Result<(), ApiError> {
    let mut segments = kind.split('.');
    let first = segments.next().unwrap_or("");
    if !is_valid_first_segment(first) {
        return Err(invalid_kind());
    }
    let mut tail_count = 0;
    for seg in segments {
        tail_count += 1;
        if !is_valid_tail_segment(seg) {
            return Err(invalid_kind());
        }
    }
    if tail_count == 0 {
        return Err(invalid_kind());
    }
    Ok(())
}

fn invalid_kind() -> ApiError {
    ApiError::BadRequest {
        code: "kind_invalid",
        message: "kind must match ^[a-z][a-z0-9_]*\\.[a-z0-9_]+(\\.[a-z0-9_]+)*$".into(),
    }
}

fn is_valid_first_segment(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_valid_tail_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Reject reserved daemon-owned kinds with 422 `kind_reserved`.
pub(super) fn check_reserved(kind: &str) -> Result<(), ApiError> {
    let reserved = RESERVED_KIND_NAMES.contains(&kind)
        || RESERVED_KIND_PREFIXES.iter().any(|p| kind.starts_with(p));
    if reserved {
        return Err(ApiError::Validation {
            code: "kind_reserved",
            message: format!(
                "kind '{kind}' is reserved by the daemon — pick a vendor-prefixed kind \
                 (e.g. 'myapp.session.pre_stop')"
            ),
        });
    }
    Ok(())
}

/// Append handler. Validates shape, then writes one row through the
/// existing hash-chained `AuditLog::append` — the chain works the same.
pub(super) async fn append(
    State(state): State<AppState>,
    Json(body): Json<AppendBody>,
) -> Result<(StatusCode, Json<AppendResponse>), ApiError> {
    validate_kind(&body.kind)?;
    check_reserved(&body.kind)?;
    if body.entity_id.trim().is_empty() {
        return Err(ApiError::BadRequest {
            code: "entity_id_empty",
            message: "entity_id must be a non-empty string".into(),
        });
    }
    if !body.payload.is_object() {
        return Err(ApiError::Validation {
            code: "payload_not_object",
            message: "payload must be a JSON object".into(),
        });
    }

    let entry = state
        .durability
        .audit()
        .append(
            body.entity_kind.into(),
            &body.entity_id,
            &body.kind,
            &body.payload,
            body.agent_id.as_deref(),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(AppendResponse {
            seq: entry.seq,
            hash: entry.hash,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_well_formed_kinds() {
        for ok in &[
            "myapp.foo",
            "myapp.session.pre_stop.check.1",
            "cvg.coherence.scan",
            "x.y_z.0_9",
        ] {
            assert!(validate_kind(ok).is_ok(), "expected ok: {ok}");
        }
    }

    #[test]
    fn rejects_malformed_kinds() {
        for bad in &[
            "",
            "noNamespace",
            ".leading",
            "trailing.",
            "0bad.start",
            "myapp..double",
            "myapp.WithCaps",
            "myapp.with-dash",
            "myapp.with space",
        ] {
            assert!(validate_kind(bad).is_err(), "expected err: {bad:?}");
        }
    }

    #[test]
    fn flags_reserved_prefixes_and_names() {
        for r in &[
            "task.foo",
            "plan.created",
            "evidence.attached",
            "crdt.merged",
            "workspace.lease.claimed",
            "capability.installed",
            "agent.session_started",
            "agent.retired",
            "agent.retired_stale",
        ] {
            assert!(check_reserved(r).is_err(), "expected reserved: {r}");
        }
    }

    #[test]
    fn allows_vendor_prefixed_agent_kinds() {
        assert!(check_reserved("agent.heartbeat_jitter").is_ok());
        assert!(check_reserved("myapp.session.pre_stop.check.1").is_ok());
    }
}
