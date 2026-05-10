//! Append-only hash-chained audit log.
//!
//! Every state transition in Layer 1 writes one row. Each row's
//! `hash = sha256(prev_hash || canonical_json(payload))`. The chain is
//! verifiable end-to-end via [`AuditLog::verify`], which any external
//! cron can call as `GET /v1/audit/verify`.
//!
//! See [ADR-0002](../../../../docs/adr/0002-audit-hash-chain.md) for the
//! decision and threat model.

mod action;
mod canonical;
mod hash;
mod log;
mod model;

pub use action::Action;
pub use canonical::canonical_json;
pub use hash::{compute_hash, GENESIS_HASH};
pub(crate) use log::append_tx;
pub use log::AuditLog;
pub use model::{AuditEntry, EntityKind, VerifyReport};
