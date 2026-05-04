//! Router assembly + shared state.

use axum::Router;
use convergio_bus::Bus;
use convergio_durability::audit::VerifyReport;
use convergio_durability::Durability;
use convergio_embed::{EmbedStore, Embedder};
use convergio_fleet::FleetStore;
use convergio_graph::Store as GraphStore;
use convergio_lifecycle::Supervisor;
use std::sync::{Arc, Mutex};
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
    /// Tier-3 semantic embeddings store (ADR-0038, F1).
    pub embed: Arc<EmbedStore>,
    /// Embedder used by the daemon for ingest + semantic queries.
    /// The default at startup is `DeterministicTestEmbedder` (no
    /// network); set `CONVERGIO_EMBED_MODEL=multilingual-e5-small`
    /// (and build with `--features fastembed`) to swap in the real
    /// ONNX model. ADR-0038 § F1-β.
    pub embedder: Arc<dyn Embedder>,
    /// Fleet repo registry (ADR-0038, F2-6).
    pub fleet: Arc<FleetStore>,
    /// Memoised full-chain audit verify result. Keyed by tail `seq`; auto-
    /// invalidated when a new audit row is appended (tail advances). Shared
    /// across all clones via `Arc` — one warm call benefits every concurrent
    /// request. Only applies to the parameter-free `/v1/audit/verify` call.
    pub audit_verify_cache: Arc<Mutex<Option<(i64, VerifyReport)>>>,
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
        .merge(crate::routes::fleet::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
