//! Unit tests for [`crate::hybrid`].
//!
//! Lives in a sibling file (referenced via `#[path]` from `hybrid.rs`)
//! so the production module stays comfortably under the 300-line cap
//! (audit LOW · hybrid.rs:298).

use super::*;

#[test]
fn pure_structural_input_yields_structural_only() {
    let structural = vec!["a", "b", "c"];
    let semantic: Vec<&str> = vec![];
    let hits = rrf_fuse(&structural, &semantic, DEFAULT_RRF_K);
    assert_eq!(hits.len(), 3);
    for h in &hits {
        assert_eq!(h.match_source, MatchSource::Structural);
        assert!(h.score_components.semantic.is_none());
        assert!(h.score_components.structural.is_some());
    }
    // Order preserved: rank 1 has the highest score.
    assert_eq!(hits[0].id, "a");
}

#[test]
fn pure_semantic_input_yields_semantic_only() {
    let structural: Vec<&str> = vec![];
    let semantic = vec!["x", "y"];
    let hits = rrf_fuse(&structural, &semantic, DEFAULT_RRF_K);
    assert_eq!(hits.len(), 2);
    for h in &hits {
        assert_eq!(h.match_source, MatchSource::Semantic);
    }
}

#[test]
fn shared_top_rank_lifts_both_above_singletons() {
    // "shared" appears at rank 1 in both lists → should outrank
    // either list's other entries.
    let structural = vec!["shared", "only-s"];
    let semantic = vec!["shared", "only-e"];
    let hits = rrf_fuse(&structural, &semantic, DEFAULT_RRF_K);
    assert_eq!(hits[0].id, "shared");
    assert_eq!(hits[0].match_source, MatchSource::Both);
    assert!(hits[0].score > hits[1].score);
    // "only-s" and "only-e" are both at rank 2 in their list and
    // absent from the other → identical RRF score.
    let s_score = hits.iter().find(|h| h.id == "only-s").unwrap().score;
    let e_score = hits.iter().find(|h| h.id == "only-e").unwrap().score;
    assert!((s_score - e_score).abs() < 1e-12);
}

#[test]
fn higher_rank_wins() {
    let structural = vec!["first", "second", "third"];
    let semantic: Vec<&str> = vec![];
    let hits = rrf_fuse(&structural, &semantic, DEFAULT_RRF_K);
    assert_eq!(hits[0].id, "first");
    assert_eq!(hits[1].id, "second");
    assert_eq!(hits[2].id, "third");
    assert!(hits[0].score > hits[1].score);
    assert!(hits[1].score > hits[2].score);
}

#[test]
fn empty_inputs_return_empty() {
    let v: Vec<&str> = vec![];
    let hits = rrf_fuse::<&str>(&v, &v, DEFAULT_RRF_K);
    assert!(hits.is_empty());
}

#[test]
fn rrf_score_matches_formula() {
    let structural = vec!["a"];
    let semantic = vec!["a"];
    let hits = rrf_fuse(&structural, &semantic, DEFAULT_RRF_K);
    let expected = 1.0 / (DEFAULT_RRF_K + 1.0) + 1.0 / (DEFAULT_RRF_K + 1.0);
    assert!((hits[0].score - expected).abs() < 1e-12);
}

#[test]
fn linear_blend_pure_ends_and_clamp() {
    // alpha=1.0: purely structural, rank-1 item scores 1.0
    let s = vec!["a", "b"];
    let e: Vec<&str> = vec![];
    let hits = linear_blend_fuse(&s, &e, 1.0);
    assert_eq!(hits[0].id, "a");
    assert_eq!(hits[0].match_source, MatchSource::Structural);
    assert!((hits[0].score - 1.0).abs() < 1e-12);
    // alpha=0.0: purely semantic
    let hits2 = linear_blend_fuse::<&str>(&[], &["x", "y"], 0.0);
    assert_eq!(hits2[0].match_source, MatchSource::Semantic);
    // alpha > 1.0 clamped to 1.0
    let hits3 = linear_blend_fuse(&s, &e, 2.0);
    assert!((hits3[0].score - 1.0).abs() < 1e-12);
    // empty inputs
    let empty: Vec<&str> = vec![];
    assert!(linear_blend_fuse::<&str>(&empty, &empty, 0.5).is_empty());
}

#[test]
fn linear_blend_semantic_overrides_saturated_structural() {
    // alpha=0.3: semantic weight 0.7 → "high" (rank 1 semantic) wins
    let hits = linear_blend_fuse(&["low", "high"], &["high", "low"], 0.3);
    assert_eq!(hits[0].id, "high");
}
