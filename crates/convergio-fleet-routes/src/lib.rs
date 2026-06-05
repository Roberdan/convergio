//! # convergio-fleet-routes
//!
//! Fleet HTTP routes (ADR-0038, F2 + F3) — extracted from
//! `convergio-server` to keep the daemon crate under its per-crate
//! context-budget cap and unblock future fleet additions (ADR-0049
//! follow-up).
//!
//! All routes share the canonical [`convergio_server_core::AppState`]
//! and return [`convergio_server_core::ApiError`]. Behaviour is
//! byte-identical to the pre-extraction routes; the move was
//! structural.
//!
//! Mount with [`router`]; `convergio-server` calls it once during
//! daemon assembly.
//!
//! | Module       | Routes |
//! |--------------|--------|
//! | [`fleet`]    | `repos`, `repos/:name`, `build`, `patterns` |
//! | [`fleet_duplicates`] | `GET /v1/fleet/duplicates` |
//! | [`fleet_doc_drift`]  | `GET /v1/fleet/doc-drift` + `POST /v1/fleet/doc-drift/snapshot` |
//! | [`fleet_plans`]      | `POST/GET /v1/fleet/plans`, link table, validate, audit |
//! | [`fleet_rot`]        | `GET /v1/fleet/rot` |

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bitemporal;

/// `POST/GET /v1/fleet/repos`, `PATCH /v1/fleet/repos/:name`,
/// `POST /v1/fleet/build`, `GET /v1/fleet/patterns`.
pub mod fleet;
/// `GET /v1/fleet/doc-drift`, `POST /v1/fleet/doc-drift/snapshot`.
pub mod fleet_doc_drift;
/// `GET /v1/fleet/duplicates`.
pub mod fleet_duplicates;
/// `POST/GET /v1/fleet/plans`, link table, `validate`, audit walk.
pub mod fleet_plans;
/// `GET /v1/fleet/rot`.
pub mod fleet_rot;

use axum::Router;
use convergio_server_core::AppState;

/// Top-level router with every fleet route mounted at `/v1/fleet/*`.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(fleet::router())
        .merge(fleet_plans::router())
}
