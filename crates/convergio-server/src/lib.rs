//! # convergio-server
//!
//! Routing shell for the Convergio daemon. Holds an axum [`axum::Router`]
//! over the full daemon surface: Layer 1 [`convergio_durability::Durability`],
//! the Layer 2 bus facade, Layer 3 lifecycle, the Tier-3 graph and embed
//! facades, and the fleet repository registry — all bundled in
//! [`AppState`]. Domain rules live in the owning crates; this crate
//! only translates HTTP into layer calls and shapes the JSON envelope.
//!
//! See `src/main.rs` for the binary entry point.
//! See [`router`] for how to mount the router into a test harness.

#![forbid(unsafe_code)]

mod app;
mod capability_install;
mod error;
mod purpose;
mod routes;
mod sse;

pub use app::{router, AppState};
pub use error::ApiError;
