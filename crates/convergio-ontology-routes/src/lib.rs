//! # convergio-ontology-routes
//!
//! Ontology Runtime HTTP routes (ADR-0053 / ADR-0054 / ADR-0060) —
//! extracted from `convergio-server` to keep the daemon crate under its
//! per-crate context-budget cap and to give future ontology/purpose route
//! additions a home (same rationale and pattern as
//! `convergio-fleet-routes`, ADR-0049 follow-up).
//!
//! All routes share the canonical [`convergio_server_core::AppState`] and
//! return [`convergio_server_core::ApiError`]. Behaviour is byte-identical
//! to the pre-extraction routes; the move was structural.
//!
//! Mount with [`router`]; `convergio-server` calls it once during daemon
//! assembly.
//!
//! | Module               | Routes |
//! |----------------------|--------|
//! | [`ontology`]         | `/v1/ontology/types`, `export`, `import` |
//! | [`events`]           | `/v1/ontology/events*` bitemporal as-of reads |
//! | [`ontology_branches`]| `/v1/ontology` branch overlay |
//! | [`ontology_graph`]   | `/v1/ontology/diff\|lineage\|branch-diff` |
//! | [`purposes`]         | `/v1/purposes` registry (ADR-0054 §B) |

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// `/v1/ontology/events*` bitemporal as-of read surface (ADR-0053, W3).
pub mod events;
/// `/v1/ontology/types`, `/v1/ontology/export/*`, `/v1/ontology/import`.
pub mod ontology;
/// `/v1/ontology` branch overlay API (create/list/resolve).
pub mod ontology_branches;
/// `/v1/ontology/diff|lineage|branch-diff` graph output surface (ADR-0060).
pub mod ontology_graph;
/// `/v1/purposes` immutable purpose registry surface (ADR-0054 §B).
pub mod purposes;

use axum::Router;
use convergio_server_core::AppState;

/// Top-level router mounting the ontology runtime HTTP surface.
/// `convergio-server` merges this once during daemon assembly.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(ontology::router())
        .merge(events::router())
        .merge(ontology_branches::router())
        .merge(ontology_graph::router())
        .merge(purposes::router())
}
