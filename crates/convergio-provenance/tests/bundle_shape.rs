//! Integration tests for `convergio-provenance`.

use convergio_provenance::{emit_bundle, Activity, Agent, Entity};

#[test]
fn emit_bundle_round_trips_through_prov_json() {
    let now = chrono::Utc::now();
    let bundle = emit_bundle(
        Activity {
            id: "act-1".into(),
            kind: "ontology.upsert".into(),
            started_at: now,
            ended_at: Some(now),
        },
        Agent {
            id: "agent-1".into(),
            label: "claude-code".into(),
        },
        Entity {
            id: "ent-1".into(),
            kind: "ontology.object.revision".into(),
        },
    )
    .expect("real impl must succeed");

    let s = serde_json::to_string(&bundle).unwrap();
    let _round: convergio_provenance::ProvBundle = serde_json::from_str(&s).unwrap();
    assert!(s.contains("ontology.upsert"));
}
