//! HTTP error type — re-export from `convergio-server-core`.
//!
//! The canonical [`ApiError`] enum + `From` + `IntoResponse` impls
//! live in `convergio-server-core` so sibling route crates
//! (`convergio-fleet-routes`) can return it without depending on the
//! daemon crate.

pub use convergio_server_core::ApiError;
