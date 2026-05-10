//! `GET /v1/gates/preconditions` — discoverable gate precondition
//! catalog (P3-2 — Palantir-inspired declarative gate
//! preconditions).
//!
//! Returns the result of `gates::describe_pipeline()` so the MCP
//! bridge and any external tool can surface gate inputs / refusal
//! reasons without re-implementing them.

use crate::app::AppState;
use axum::routing::get;
use axum::{Json, Router};
use convergio_api::GatePreconditionsCatalog;
use convergio_durability::gates::{default_pipeline, describe_pipeline};

/// Mount the route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/gates/preconditions", get(preconditions))
}

async fn preconditions() -> Json<GatePreconditionsCatalog> {
    let pipeline = default_pipeline();
    Json(GatePreconditionsCatalog {
        schema_version: convergio_api::GATE_PRECONDITIONS_SCHEMA_VERSION.to_string(),
        preconditions: describe_pipeline(&pipeline),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preconditions_response_lists_every_default_gate() {
        let resp = preconditions().await;
        assert_eq!(
            resp.0.schema_version,
            convergio_api::GATE_PRECONDITIONS_SCHEMA_VERSION
        );
        assert!(!resp.0.preconditions.is_empty());
        let names: Vec<&str> = resp
            .0
            .preconditions
            .iter()
            .map(|p| p.gate.as_str())
            .collect();
        // The 9 gates from default_pipeline appear in stable order.
        for expected in [
            "plan_status",
            "evidence",
            "crdt_conflict",
            "no_debt",
            "no_stub",
            "wire_check",
            "no_secrets",
            "zero_warnings",
            "wave_sequence",
        ] {
            assert!(names.contains(&expected), "missing {expected}");
        }
    }

    #[tokio::test]
    async fn preconditions_overrides_describe_for_known_gates() {
        let resp = preconditions().await;
        let plan = resp
            .0
            .preconditions
            .iter()
            .find(|p| p.gate == "plan_status")
            .expect("plan_status present");
        assert!(plan
            .refusal_reasons
            .iter()
            .any(|r| r == "plan_is_cancelled"));
        let ev = resp
            .0
            .preconditions
            .iter()
            .find(|p| p.gate == "evidence")
            .expect("evidence present");
        assert_eq!(
            ev.active_target_status,
            vec!["submitted".to_string(), "done".to_string()]
        );
        assert!(ev.enforces_task_evidence_required);
    }
}
