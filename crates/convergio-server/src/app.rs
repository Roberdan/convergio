//! Router assembly + shared state re-export.
//!
//! [`AppState`] lives in `convergio-server-core` so sibling route
//! crates can take `State<AppState>` without depending on this
//! binary crate. This module owns the top-level `Router` assembly
//! and the middleware layer.

use axum::Router;
use tower_http::trace::TraceLayer;

pub use convergio_server_core::AppState;

/// Build the top-level router. Test harnesses call this directly with
/// tempdir-backed facades.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(crate::routes::health::router())
        .merge(crate::routes::plans::router())
        .merge(crate::routes::ops::router())
        .merge(crate::routes::pr_links::router())
        .merge(crate::routes::tasks::router())
        .merge(crate::routes::evidence::router())
        .merge(crate::routes::audit::router())
        .merge(crate::routes::capabilities::router())
        .merge(crate::routes::context::router())
        .merge(crate::routes::crdt::router())
        .merge(crate::routes::ontology::router())
        .merge(crate::routes::ontology_branches::router())
        .merge(crate::routes::purposes::router())
        .merge(crate::routes::messages::router())
        .merge(crate::routes::ontology_graph::router())
        .merge(crate::routes::system_messages::router())
        .merge(crate::routes::agent_registry::router())
        .merge(crate::routes::agents::router())
        .merge(crate::routes::solve::router())
        .merge(crate::routes::status::router())
        .merge(crate::routes::validate::router())
        .merge(crate::routes::dispatch::router())
        .merge(crate::routes::workspace::router())
        .merge(crate::routes::graph::router())
        .merge(crate::routes::embed::router())
        .merge(crate::routes::llm_gateway::router())
        .merge(convergio_fleet_routes::router())
        .merge(crate::routes::telemetry::router())
        .merge(crate::routes::api_actions::router())
        .merge(crate::routes::gate_preconditions::router())
        .merge(crate::routes::search::router())
        .merge(crate::routes::gdpr::router())
        .merge(crate::routes::reports::router())
        .layer(axum::middleware::from_fn(crate::purpose::enforce))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
