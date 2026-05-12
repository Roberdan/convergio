//! Tests for [`crate::checks::check_1_plan_pr_drift`].
//!
//! Hosted in a sibling file so the implementation module stays under
//! the 300-line cap (CONSTITUTION § 13) and has room for additional
//! parser/extractor cases without an immediate split.

use super::*;

#[test]
fn extract_task_ids_recognises_tracks_line() {
    let text = "Some preamble.\nTracks: 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea\nMore.";
    let ids = extract_task_ids(text);
    assert!(ids.contains("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
}

#[test]
fn extract_task_ids_recognises_short_prefix_tracks() {
    let text = "Tracks: T5298055b\nMore.";
    let ids = extract_task_ids(text);
    assert!(ids.contains("5298055b"));
}

#[test]
fn extract_task_ids_ignores_non_hex_garbage() {
    let text = "Tracks: notauuid here-is-junk-not-hex 12345678";
    let ids = extract_task_ids(text);
    assert!(ids.contains("12345678"));
    assert_eq!(ids.len(), 1);
}

#[test]
fn extract_task_ids_finds_uuid_near_task_word() {
    let text = "this PR closes task 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea";
    let ids = extract_task_ids(text);
    assert!(ids.contains("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
}

#[test]
fn is_task_id_validates_shape() {
    assert!(is_task_id("5298055b"));
    assert!(is_task_id("5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"));
    assert!(!is_task_id("5298055G"));
    assert!(!is_task_id("not-a-uuid-12345678901234567890123456"));
    assert!(!is_task_id(""));
}

#[test]
fn parse_gh_output_round_trips() {
    let json = r#"[{"number":12,"title":"feat","body":"Tracks: 5298055b"}]"#;
    let prs = parse_gh_output(json).expect("ok");
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 12);
    assert_eq!(prs[0].body, "Tracks: 5298055b");
}

#[test]
fn check_id_and_label_are_stable() {
    let c = PlanPrDriftCheck;
    assert_eq!(c.id(), "check.plan_pr_drift");
    assert!(c.label().contains("drift"));
}
