//! # convergio-server-core
//!
//! Shared HTTP-layer primitives extracted from `convergio-server` so that
//! sibling route crates (e.g. [`convergio-fleet-routes`]) can depend on
//! [`AppState`] and [`ApiError`] without creating a cycle with the
//! daemon binary crate.
//!
//! This crate is intentionally a thin seam:
//!
//! | Module     | Owns |
//! |------------|------|
//! | [`state`]  | [`AppState`] — the typed dependency-injection bag every route handler reads from |
//! | [`error`]  | [`ApiError`] + the layer-error `From` impls + the canonical `IntoResponse` mapping |
//!
//! Routing (`Router::new()`), middleware (`TraceLayer`), and the binary
//! entry point stay in `convergio-server`. This crate has no
//! domain logic — it is the small set of types both `convergio-server`
//! and `convergio-fleet-routes` need to agree on.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod state;

pub use error::ApiError;
pub use state::AppState;
