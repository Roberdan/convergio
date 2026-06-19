//! Unit tests for the shared action contract.

use super::*;
use std::collections::BTreeSet;

#[test]
fn action_names_are_stable_snake_case() {
    let names: Vec<&str> = Action::ALL.iter().map(|a| a.as_str()).collect();
    assert_eq!(names[0], "status");
    assert!(names.contains(&"submit_task"));
    assert!(names.contains(&"get_task_context"));
    assert!(names.contains(&"publish_message"));
    assert!(names.contains(&"poll_messages"));
    assert!(names.contains(&"ack_message"));
    assert!(names.contains(&"import_crdt_ops"));
    assert!(names.contains(&"list_crdt_conflicts"));
    assert!(names.contains(&"register_agent"));
    assert!(names.contains(&"heartbeat_agent"));
    assert!(names.contains(&"retire_agent"));
    assert!(names.contains(&"spawn_runner"));
    assert!(names.contains(&"planner.solve"));
    assert!(names.contains(&"list_capabilities"));
    assert!(names.contains(&"claim_workspace_lease"));
    assert!(names.contains(&"list_workspace_leases"));
    assert!(names.contains(&"submit_patch_proposal"));
    assert!(names.contains(&"enqueue_patch_proposal"));
    assert!(names.contains(&"process_merge_queue"));
    assert!(names.contains(&"list_merge_queue"));
    assert!(names.contains(&"explain_last_refusal"));
    assert!(names.contains(&"llm.call"));
}

#[test]
fn action_deserializes_from_snake_case() {
    let action: Action = serde_json::from_str("\"add_evidence\"").unwrap();
    assert_eq!(action, Action::AddEvidence);
    let action: Action = serde_json::from_str("\"planner.solve\"").unwrap();
    assert_eq!(action, Action::PlannerSolve);
}

#[test]
fn action_all_matches_generated_schema_enum() {
    let schema = serde_json::to_value(schemars::schema_for!(Action)).unwrap();
    let schema_names = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|value| value.get("const").and_then(Value::as_str).unwrap())
        .collect::<BTreeSet<_>>();
    let catalog_names = Action::ALL
        .iter()
        .map(|action| action.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(catalog_names.len(), Action::ALL.len());
    assert_eq!(catalog_names, schema_names);
}

#[test]
fn act_request_has_version_action_and_params() {
    let req: ActRequest = serde_json::from_value(serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "action": "status",
        "params": {}
    }))
    .unwrap();
    assert_eq!(req.schema_version, SCHEMA_VERSION);
    assert_eq!(req.action, Action::Status);
}

#[test]
fn complete_task_action_was_removed_in_schema_v2() {
    // ADR-0011: agents may not self-promote to done. The
    // `complete_task` action no longer exists; callers must use
    // `validate_plan` after submitting.
    assert_eq!(SCHEMA_VERSION, "2");
    let err = serde_json::from_str::<Action>("\"complete_task\"");
    assert!(err.is_err(), "complete_task must not deserialize anymore");
}

#[test]
fn catalog_exposes_exact_two_tools() {
    let catalog = ActionCatalog::current();
    assert_eq!(catalog.tools.help, HELP_TOOL);
    assert_eq!(catalog.tools.act, ACT_TOOL);
    assert_eq!(catalog.actions.len(), Action::ALL.len());
}

#[test]
fn register_and_retire_agent_compensate_each_other() {
    assert_eq!(
        Action::RegisterAgent.compensate(),
        Some(Action::RetireAgent)
    );
    assert_eq!(
        Action::RetireAgent.compensate(),
        Some(Action::RegisterAgent)
    );
    assert!(Action::RegisterAgent.is_reversible());
    assert!(Action::RetireAgent.is_reversible());
}

#[test]
fn workspace_lease_claim_compensates_with_release() {
    assert_eq!(
        Action::ClaimWorkspaceLease.compensate(),
        Some(Action::ReleaseWorkspaceLease)
    );
    // ReleaseWorkspaceLease intentionally returns None — re-claiming
    // a released lease is the operator's call, not a mechanical undo.
    assert_eq!(Action::ReleaseWorkspaceLease.compensate(), None);
}

#[test]
fn read_only_actions_are_not_reversible() {
    for action in [
        Action::Status,
        Action::ListTasks,
        Action::AuditVerify,
        Action::ListAgents,
    ] {
        assert!(
            !action.is_reversible(),
            "{} should not be reversible",
            action.as_str()
        );
    }
}

#[test]
fn at_least_three_actions_have_compensating_pairs() {
    let reversible: Vec<&str> = Action::ALL
        .iter()
        .filter(|a| a.is_reversible())
        .map(|a| a.as_str())
        .collect();
    assert!(
        reversible.len() >= 3,
        "expected ≥3 reversible actions, got {reversible:?}"
    );
}
