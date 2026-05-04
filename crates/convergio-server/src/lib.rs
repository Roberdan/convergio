//! # convergio-server
//!
//! Routing shell for the Convergio daemon. Holds an axum [`axum::Router`]
//! over the Layer 1 [`convergio_durability::Durability`] facade.
//!
//! See `src/main.rs` for the binary entry point.
//! See [`router`] for how to mount the router into a test harness.

#![forbid(unsafe_code)]

mod app;
mod capability_install;
mod error;
mod routes;
mod sse;

pub use app::{router, AppState};
pub use error::ApiError;
