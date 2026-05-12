//! Hybrid retrieval: fuse a structural ranked list with a semantic
//! ranked list via Reciprocal Rank Fusion (RRF).
//!
//! ADR-0038 § 5.5: RRF is the default fusion for `cvg graph for-task
//! --semantic`. The two input lists need only stable ids and an
//! ordering — scores are not compared directly because they live in
//! different units (substring static-score vs cosine).
//!
//! Each output hit carries a [`MatchSource`] — `Structural`,
//! `Semantic`, or `Both` — so callers (and the audit log) can see
//! which retrieval path surfaced the file.

use serde::Serialize;
use std::collections::HashMap;

/// Where a [`RetrievalHit`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchSource {
    /// Found by the substring (graph) retriever only.
    Structural,
    /// Found by the semantic (embedding) retriever only.
    Semantic,
    /// Surfaced by both retrievers — the strongest signal.
    Both,
}

/// Per-source contribution to the fused score. Either side may be
/// `None` when the corresponding retriever did not return the id.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ScoreComponents {
    /// RRF contribution from the structural list.
    pub structural: Option<f64>,
    /// RRF contribution from the semantic list.
    pub semantic: Option<f64>,
}

/// One fused hit with provenance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RetrievalHit {
    /// Stable identifier (typically the file path; whatever the
    /// caller used as the join key).
    pub id: String,
    /// Combined RRF score. Higher = more relevant.
    pub score: f64,
    /// Provenance — Structural / Semantic / Both.
    pub match_source: MatchSource,
    /// Per-source breakdown for debugging and audit.
    pub score_components: ScoreComponents,
}

/// Standard RRF constant. Larger `k` flattens differences between
/// adjacent ranks; `60` is the value from the original paper and
/// the current TREC default.
pub const DEFAULT_RRF_K: f64 = 60.0;

/// Default structural weight for [`linear_blend_fuse`] (`0.5` — equal blend).
pub const DEFAULT_LINEAR_ALPHA: f64 = 0.5;

/// Fuse two ranked lists via Reciprocal Rank Fusion.
///
/// Input lists are read in order — index 0 is rank 1. RRF formula:
///
/// ```text
/// score(id) = Σ 1 / (k + rank(id, list))
/// ```
///
/// Returns hits sorted by fused score descending. Ties are broken by
/// the stable order in which ids first appeared.
pub fn rrf_fuse<S: AsRef<str>>(structural: &[S], semantic: &[S], k: f64) -> Vec<RetrievalHit> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: HashMap<String, ScoreComponents> = HashMap::new();

    for (i, id) in structural.iter().enumerate() {
        let rank = (i + 1) as f64;
        let id = id.as_ref().to_owned();
        let entry = by_id.entry(id.clone()).or_default();
        if entry.structural.is_none() && entry.semantic.is_none() {
            order.push(id);
        }
        entry.structural = Some(1.0 / (k + rank));
    }
    for (i, id) in semantic.iter().enumerate() {
        let rank = (i + 1) as f64;
        let id = id.as_ref().to_owned();
        let entry = by_id.entry(id.clone()).or_default();
        if entry.structural.is_none() && entry.semantic.is_none() {
            order.push(id);
        }
        entry.semantic = Some(1.0 / (k + rank));
    }

    let mut hits: Vec<RetrievalHit> = order
        .into_iter()
        .map(|id| {
            let comps = by_id.remove(&id).unwrap_or_default();
            let score = comps.structural.unwrap_or(0.0) + comps.semantic.unwrap_or(0.0);
            let match_source = match (comps.structural, comps.semantic) {
                (Some(_), Some(_)) => MatchSource::Both,
                (Some(_), None) => MatchSource::Structural,
                (None, Some(_)) => MatchSource::Semantic,
                (None, None) => MatchSource::Structural, // unreachable
            };
            RetrievalHit {
                id,
                score,
                match_source,
                score_components: comps,
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

/// Fuse two ranked lists via rank-normalised linear blend.
///
/// Normalises each list: rank 1 → 1.0, rank N → `1/N`, absent → 0.0.
/// Combined: `score = alpha × structural + (1 − alpha) × semantic`.
/// `alpha` clamped to `[0.0, 1.0]`; `1.0` = pure structural, `0.0` = pure
/// semantic. Mitigates substring saturation (ADR-0038 § 15.7.1).
/// Returns hits sorted descending; ties by stable insertion order.
pub fn linear_blend_fuse<S: AsRef<str>>(
    structural: &[S],
    semantic: &[S],
    alpha: f64,
) -> Vec<RetrievalHit> {
    let alpha = alpha.clamp(0.0, 1.0);
    let s_len = structural.len() as f64;
    let e_len = semantic.len() as f64;

    let mut order: Vec<String> = Vec::new();
    let mut by_id: HashMap<String, ScoreComponents> = HashMap::new();

    for (i, id) in structural.iter().enumerate() {
        let rank_score = if s_len > 0.0 {
            1.0 - (i as f64) / s_len
        } else {
            0.0
        };
        let id = id.as_ref().to_owned();
        let entry = by_id.entry(id.clone()).or_default();
        if entry.structural.is_none() && entry.semantic.is_none() {
            order.push(id);
        }
        entry.structural = Some(rank_score);
    }
    for (i, id) in semantic.iter().enumerate() {
        let rank_score = if e_len > 0.0 {
            1.0 - (i as f64) / e_len
        } else {
            0.0
        };
        let id = id.as_ref().to_owned();
        let entry = by_id.entry(id.clone()).or_default();
        if entry.structural.is_none() && entry.semantic.is_none() {
            order.push(id);
        }
        entry.semantic = Some(rank_score);
    }

    let mut hits: Vec<RetrievalHit> = order
        .into_iter()
        .map(|id| {
            let comps = by_id.remove(&id).unwrap_or_default();
            let s = comps.structural.unwrap_or(0.0);
            let e = comps.semantic.unwrap_or(0.0);
            let score = alpha * s + (1.0 - alpha) * e;
            let match_source = match (comps.structural, comps.semantic) {
                (Some(_), Some(_)) => MatchSource::Both,
                (Some(_), None) => MatchSource::Structural,
                (None, Some(_)) => MatchSource::Semantic,
                (None, None) => MatchSource::Structural, // unreachable
            };
            RetrievalHit {
                id,
                score,
                match_source,
                score_components: comps,
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

#[cfg(test)]
#[path = "hybrid_tests.rs"]
mod tests;
