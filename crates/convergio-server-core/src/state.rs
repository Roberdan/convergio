//! Shared `AppState` — typed dependency-injection bag for every route handler.
//!
//! Lives here so route crates outside `convergio-server` (today only
//! `convergio-fleet-routes`) can take `State<AppState>` without
//! depending on the binary crate.

use convergio_bus::Bus;
use convergio_durability::audit::VerifyReport;
use convergio_durability::Durability;
use convergio_embed::{EmbedStore, Embedder};
use convergio_fleet::{FleetPlanStore, FleetStore};
use convergio_graph::Store as GraphStore;
use convergio_lifecycle::Supervisor;
use convergio_ontology::Store as OntologyStore;
use std::sync::{Arc, Mutex};

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
    /// network); set `CONVERGIO_EMBED_MODEL=bge-m3-small-int8` (or
    /// `multilingual-e5-small`) and build with `--features fastembed`
    /// to swap in the real ONNX embedder.
    pub embedder: Arc<dyn Embedder>,
    /// Fleet repo registry (ADR-0038, F2-6).
    pub fleet: Arc<FleetStore>,
    /// Fleet plan store (ADR-0038, F3-2): cross-repo plans with
    /// per-repo plan links.
    pub fleet_plans: Arc<FleetPlanStore>,
    /// Ontology Runtime Core schema registry (ADR-0053).
    pub ontology: Arc<OntologyStore>,
    /// Memoised full-chain audit verify result. Keyed by tail `seq`; auto-
    /// invalidated when a new audit row is appended (tail advances). Shared
    /// across all clones via `Arc` — one warm call benefits every concurrent
    /// request. Only applies to the parameter-free `/v1/audit/verify` call.
    pub audit_verify_cache: Arc<Mutex<Option<(i64, VerifyReport)>>>,
}
