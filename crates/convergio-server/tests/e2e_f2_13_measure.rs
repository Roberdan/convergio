//! F2-13 go/no-go measurement: cross-repo patterns + duplicate FP rate.
//!
//! Boots the full daemon in-process, registers the 3-repo fleet
//! (convergio + convergio-edu + convergio-ui-framework, falling back to
//! plan-fsm-* fixture proxies when the real repos are missing), builds
//! embeddings, runs the similarity batch, then classifies all duplicate
//! pairs to compute the false-positive rate.
//!
//! Run with:
//!   cargo test -p convergio-server --features fastembed \
//!       --test e2e_f2_13_measure -- --ignored --nocapture
//!
//! Without `--features fastembed` the test uses DeterministicTestEmbedder
//! and the numbers are only structural (not semantic).
//!
//! Methodology + acceptance criteria: docs/spec/f2-13-measurement.md.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "e2e_f2_13_common.rs"]
mod common;

use convergio_fleet::{find_duplicates, find_patterns};
use serde_json::Value;

/// P0-6 / finding H4: long-running e2e tests must fail loud, not
/// run forever. The body is wrapped in `tokio::time::timeout` with
/// a 3-minute default; override via `CONVERGIO_E2E_F2_13_TIMEOUT_SECS`.
fn timeout_secs() -> u64 {
    std::env::var("CONVERGIO_E2E_F2_13_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}

#[tokio::test]
#[ignore = "F2-13 measurement — slow (embeds real repos); run with --ignored --nocapture"]
async fn f2_13_measure_cross_repo_patterns_and_fp_rate() {
    let timeout = std::time::Duration::from_secs(timeout_secs());
    tokio::time::timeout(timeout, run_measurement())
        .await
        .expect("F2-13 measurement timed out — increase CONVERGIO_E2E_F2_13_TIMEOUT_SECS");
}

async fn run_measurement() {
    let embedder = common::make_embedder();
    let model_id = embedder.model_id().to_owned();
    let (base, fleet_store, _embed, _dir) = common::boot_with_embedder(embedder).await;
    let client = common::client();

    let ws = common::workspace_root();
    let (convergio_path, convergio_real) = (ws.clone(), true);
    let (edu_path, edu_real) =
        common::resolve_repo("/Users/Roberdan/GitHub/convergio-edu", "plan-fsm-ts");
    let (ui_path, ui_real) = common::resolve_repo(
        "/Users/Roberdan/GitHub/convergio-ui-framework",
        "plan-fsm-py",
    );

    eprintln!("\n=== F2-13 MEASUREMENT ===\nmodel: {model_id}");
    eprintln!(
        "convergio: {} ({})",
        convergio_path.display(),
        if convergio_real { "real" } else { "fixture" }
    );
    eprintln!(
        "convergio-edu: {} ({})",
        edu_path.display(),
        if edu_real { "real" } else { "fixture proxy" }
    );
    eprintln!(
        "convergio-ui-framework: {} ({})\n",
        ui_path.display(),
        if ui_real { "real" } else { "fixture proxy" }
    );

    if convergio_path.is_dir() {
        common::register_repo(
            &base,
            "convergio",
            &convergio_path.to_string_lossy(),
            "rust",
        )
        .await;
    }
    if edu_path.is_dir() {
        common::register_repo(
            &base,
            "convergio-edu",
            &edu_path.to_string_lossy(),
            "typescript",
        )
        .await;
    }
    if ui_path.is_dir() {
        let lang = if ui_real { "typescript" } else { "python" };
        common::register_repo(
            &base,
            "convergio-ui-framework",
            &ui_path.to_string_lossy(),
            lang,
        )
        .await;
    }

    let t0 = std::time::Instant::now();
    let build_resp: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({ "refresh_similarity": true }))
        .send()
        .await
        .expect("build send")
        .json()
        .await
        .expect("build json");
    let elapsed = t0.elapsed();

    eprintln!("Build report (elapsed {elapsed:.1?}):");
    eprintln!("  repos_processed: {}", build_resp["repos_processed"]);
    eprintln!("  repos_skipped:   {}", build_resp["repos_skipped"]);
    eprintln!("  embed.considered:{}", build_resp["embed"]["considered"]);
    eprintln!("  embed.embedded:  {}", build_resp["embed"]["embedded"]);
    eprintln!(
        "  similarity.pairs_checked: {}",
        build_resp["similarity"]["pairs_checked"]
    );
    eprintln!(
        "  similarity.similar_to:    {}",
        build_resp["similarity"]["similar_to"]
    );
    eprintln!(
        "  similarity.duplicates:    {}\n",
        build_resp["similarity"]["duplicates"]
    );

    let patterns = find_patterns(&fleet_store, 2).await.expect("patterns");
    eprintln!("=== CROSS-REPO PATTERNS (min_repos=2) ===");
    eprintln!("Total clusters: {}", patterns.len());
    for (i, c) in patterns.iter().enumerate() {
        eprintln!(
            "Cluster {} [{:.1}%] → '{}'",
            i + 1,
            c.confidence * 100.0,
            c.hoist_target
        );
        for m in &c.members {
            eprintln!("  {} :: {} ({})", m.repo, m.name, m.kind);
        }
    }
    let patterns_ge3 = find_patterns(&fleet_store, 3).await.expect("patterns-3");
    eprintln!("\nClusters spanning ≥3 repos: {}\n", patterns_ge3.len());

    let all_pairs = find_duplicates(&fleet_store, 0.95, None, true)
        .await
        .expect("duplicates");
    eprintln!(
        "=== DUPLICATE PAIRS (cosine ≥ 0.95) ===\nTotal: {}",
        all_pairs.len()
    );

    let sample_size = all_pairs.len().min(50);
    let sample = &all_pairs[..sample_size];
    let (mut tp, mut fp) = (0usize, 0usize);

    eprintln!("\nSample of {sample_size} pairs (heuristic classification):");
    for (i, p) in sample.iter().enumerate() {
        let is_tp = common::classify_pair_tp(&p.name_a, &p.kind_a, &p.name_b, &p.kind_b, p.score);
        if is_tp {
            tp += 1;
        } else {
            fp += 1;
        }
        let verdict = if is_tp { "TP" } else { "FP" };
        eprintln!(
            "  [{:02}] {verdict} score={:.3}  {}/{}({})  ↔  {}/{}({})",
            i + 1,
            p.score,
            p.repo_a,
            p.name_a,
            p.kind_a,
            p.repo_b,
            p.name_b,
            p.kind_b,
        );
        for line in &p.diff_preview {
            eprintln!("         {line}");
        }
    }

    let fp_rate = if sample_size == 0 {
        0.0
    } else {
        fp as f64 / sample_size as f64
    };
    eprintln!("\n=== F2-13 RESULTS ===");
    eprintln!("Cross-repo clusters (min_repos=2): {}", patterns.len());
    eprintln!("Cross-repo clusters (min_repos=3): {}", patterns_ge3.len());
    eprintln!("Total duplicate pairs (cosine ≥ 0.95): {}", all_pairs.len());
    eprintln!("Sample size (≤50): {sample_size}");
    eprintln!("  True positives:  {tp}");
    eprintln!("  False positives: {fp}");
    eprintln!("  FP rate:         {:.1}%\n", fp_rate * 100.0);

    if !edu_real || !ui_real {
        eprintln!("NOTE: using fixture proxies for missing repos.");
        eprintln!("      Numbers are indicative, not the final gate measurement.");
        eprintln!("      Re-run with real convergio-edu and convergio-ui-framework.");
    } else {
        assert!(
            patterns_ge3.len() >= 3,
            "F2 gate FAIL: found {} clusters spanning ≥3 repos, need ≥3",
            patterns_ge3.len()
        );
        if sample_size >= 10 {
            assert!(
                fp_rate < 0.20,
                "F2 gate FAIL: FP rate {:.1}% ≥ 20% on {sample_size} pairs",
                fp_rate * 100.0,
            );
        }
    }
}
