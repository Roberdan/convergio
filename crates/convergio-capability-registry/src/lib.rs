//! Remote capability registry fetcher + trust store (W9-F1).
//!
//! This crate is the **F1 slice** of the design captured in
//! [ADR-0072](../../../docs/adr/0072-remote-capability-registry.md):
//! read-only HTTPS resolution of capability bundles plus a versioned
//! Ed25519 trust store. The signature verifier (F2), CLI surface (F3),
//! and reference registry (F4) land in follow-up PRs.
//!
//! ## Boundaries
//!
//! - **No DB, no audit, no daemon coupling.** The crate is a pure
//!   library so it can be reused by both `convergio-cli` (for
//!   `cvg capability install`) and `convergio-server` (when the
//!   daemon resolves a capability on behalf of a spawned agent).
//! - **No live network in tests.** All HTTP I/O goes through the
//!   [`RegistryFetcher`] trait; tests inject [`MockFetcher`].
//!
//! ## Public surface
//!
//! - [`fetcher::RegistryFetcher`] — trait over `index.json`,
//!   `manifest.json`, `.cap`, and `.cap.sig` fetches.
//! - [`fetcher::HttpsRegistryFetcher`] — production impl backed by
//!   `reqwest` + `rustls-tls`.
//! - [`fetcher::MockFetcher`] — in-memory impl for tests.
//! - [`trust_store::TrustStore`] — load + validate operator-overridable
//!   Ed25519 trust roots.
//! - [`manifest::RegistryIndex`] / [`manifest::CapabilityManifest`] —
//!   serde schemas for the JSON documents on the wire.

#![deny(missing_docs)]

pub mod error;
pub mod fetcher;
pub mod manifest;
pub mod trust_store;

mod base64;

pub use error::{RegistryError, Result};
pub use fetcher::{HttpsRegistryFetcher, MockFetcher, RegistryFetcher};
pub use manifest::{CapabilityManifest, IndexEntry, RegistryIndex, VersionEntry};
pub use trust_store::{TrustStore, TrustStoreEntry};
