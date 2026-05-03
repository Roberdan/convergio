//! Router assembly + shared state.

use axum::Router;
use convergio_bus::Bus;
use convergio_durability::Durability;
use convergio_embed::EmbedStore;
use convergio_graph::Store as GraphStore;
use convergio_lifecycle::Supervisor;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

/// Application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    /// Layer 1 facade.
    pub durability: Arc<Durability>,
    /// Layer 2 facade.
    pub bus: Arc<Bus>,
    /// Layer 3 facade.
    pub supervisor: Arc<Supervisor>,
    /// Tier-3 retrieval store (ADR-0014).
    pub graph: Arc<GraphStore>,
    /// Tier-3 semantic embeddings store (ADR-0035, F1).
    pub embed: Arc<EmbedStore>,
}

/// Build the top-level router. Test harnesses call this directly with
/// tempdir-backed facades.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(crate::routes::health::router())
        .merge(crate::routes::plans::router())
        .merge(crate::routes::tasks::router())
        .merge(crate::routes::evidence::router())
        .merge(crate::routes::audit::router())
        .merge(crate::routes::capabilities::router())
        .merge(crate::routes::context::router())
        .merge(crate::routes::crdt::router())
        .merge(crate::routes::messages::router())
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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
