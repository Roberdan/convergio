//! Unit tests for [`crate::handshake`] — payload validation, skip
//! padding, label coverage, and finalize bookkeeping. Kept in a
//! separate module so `handshake.rs` itself stays under the
//! 300-line per-file cap (CONSTITUTION § 13).

use crate::handshake::{
    skip_remaining, test_finalize, test_phase_label, test_short_id, validate_pong_payload, Phase,
    PhaseOutcome,
};
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn validate_pong_accepts_matching_payload() {
    let payload = json!({
        "type": "pong",
        "from": "B",
        "nonce": "abc123",
        "replying_to": "msg-1",
    });
    assert!(validate_pong_payload(&payload, "abc123", "msg-1"));
}

#[test]
fn validate_pong_rejects_wrong_nonce() {
    let payload = json!({"type": "pong", "nonce": "wrong", "replying_to": "msg-1"});
    assert!(!validate_pong_payload(&payload, "abc123", "msg-1"));
}

#[test]
fn validate_pong_rejects_wrong_type() {
    let payload = json!({"type": "ping", "nonce": "abc123", "replying_to": "msg-1"});
    assert!(!validate_pong_payload(&payload, "abc123", "msg-1"));
}

#[test]
fn validate_pong_rejects_wrong_reply() {
    let payload = json!({"type": "pong", "nonce": "abc123", "replying_to": "msg-2"});
    assert!(!validate_pong_payload(&payload, "abc123", "msg-1"));
}

#[test]
fn skip_remaining_pads_to_six() {
    let mut phases: Vec<Phase> = Vec::new();
    skip_remaining(&mut phases, 3);
    assert_eq!(phases.len(), 4);
    assert_eq!(phases[0].n, 3);
    assert_eq!(phases.last().expect("populated").n, 6);
    assert!(phases.iter().all(|p| p.outcome == PhaseOutcome::Skipped));
}

#[test]
fn skip_remaining_at_seven_is_noop() {
    let mut phases: Vec<Phase> = Vec::new();
    skip_remaining(&mut phases, 7);
    assert!(phases.is_empty());
}

#[test]
fn phase_label_covers_six() {
    for n in 1..=6 {
        assert!(!test_phase_label(n).is_empty());
        assert_ne!(test_phase_label(n), "?");
    }
}

#[test]
fn phase_label_unknown_is_question_mark() {
    assert_eq!(test_phase_label(0), "?");
    assert_eq!(test_phase_label(7), "?");
}

#[test]
fn short_id_is_eight_hex() {
    let id = test_short_id();
    assert_eq!(id.len(), 8);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn finalize_marks_success_only_when_six_ok() {
    let started = Instant::now();
    let mut phases = Vec::new();
    for n in 1..=6 {
        phases.push(Phase {
            n,
            label: "x".into(),
            outcome: PhaseOutcome::Ok,
            elapsed_ms: 0,
            detail: String::new(),
        });
    }
    let r = test_finalize(
        "http://x",
        Duration::from_secs(1),
        started,
        phases,
        "p".into(),
        ("a".into(), "b".into()),
    );
    assert!(r.success);
}

#[test]
fn finalize_failure_when_a_phase_skipped() {
    let started = Instant::now();
    let mut phases = Vec::new();
    for n in 1..=5 {
        phases.push(Phase {
            n,
            label: "x".into(),
            outcome: PhaseOutcome::Ok,
            elapsed_ms: 0,
            detail: String::new(),
        });
    }
    skip_remaining(&mut phases, 6);
    let r = test_finalize(
        "http://x",
        Duration::from_secs(1),
        started,
        phases,
        "p".into(),
        ("a".into(), "b".into()),
    );
    assert!(!r.success);
}
