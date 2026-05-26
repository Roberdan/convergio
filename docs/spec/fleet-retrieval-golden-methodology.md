# Fleet retrieval — golden-set methodology

- **Status**: Proposed v1.0
- **Date**: 2026-05-03
- **Companion ADR**: [ADR-0038](../adr/0038-fleet-retrieval-cross-repo-graph.md) § 7.2
- **Companion plan**: [`docs/plans/fleet-retrieval-cross-repo-graph.md`](../plans/fleet-retrieval-cross-repo-graph.md)
- **Audience**: Convergio core, future contributors writing retrieval
  benchmarks, anyone reviewing F1 / F2 / F3 go/no-go evidence

---

## 1. Why this document exists

ADR-0038 introduces a probabilistic component (semantic embeddings)
behind a feature flag. CONSTITUTION § P1 (zero tolerance for tech
debt) requires that probabilistic features ship with a deterministic
quality gate, otherwise we risk shipping noise.

The gate for retrieval quality is **recall@K on a frozen golden set**,
not exact equality. This document specifies how the golden set is
built, how it is consumed by tests and benches, how regressions are
detected, and how cost is contained in CI.

It is referenced by F1-8 / F1-9 / F1-10 in the durable plan and by
ADR-0038 § 7.2.

---

## 2. Definitions

| Term | Definition |
|------|------------|
| **Task fixture** | A historical Convergio task identified by its task ID, with a hand-curated set of files an expert reviewer judged as relevant for completing the task |
| **Golden set** | A frozen collection of task fixtures committed under `tests/fixtures/retrieval-golden/`. Versioned with the repo. |
| **Ground truth** | The `expected_files` array of a task fixture |
| **Retrieved set** | The set of files in the `ContextPack` produced by `cvg graph for-task <id>` (or its fleet equivalent) |
| **K** | The pack-size cutoff; we measure at K ∈ {10, 25} |
| **recall@K** | `\|retrieved ∩ expected\| / \|expected\|` over the top-K retrieved |
| **precision@K** | `\|retrieved ∩ expected\| / K` |
| **Lift** | absolute difference between hybrid and substring-only recall@K on the same task |

Recall is the primary metric. Precision is reported but not gated —
the agent reads the pack, so a slightly noisier pack is acceptable as
long as the relevant files are in there.

---

## 3. Fixture format

Each task fixture is a single JSON file under
`tests/fixtures/retrieval-golden/<repo>/<task-id>.json`:

```json
{
  "task_id": "T-fleet-2026-04-22-i18n-ack",
  "repo": "convergio",
  "title": "wire i18n for cvg session ack messages",
  "task_body": "<the natural-language task description used at the time>",
  "expected_files": [
    "crates/convergio-i18n/src/bundles.rs",
    "crates/convergio-cli/src/commands/session.rs",
    "crates/convergio-cli/locales/en/cli.ftl",
    "crates/convergio-cli/locales/it/cli.ftl"
  ],
  "rationale": "i18n change touches the bundle loader plus the command that surfaces the strings; locale files are evidence the change shipped to both languages",
  "curator": "roberto",
  "curated_at": "2026-04-25",
  "task_completed_at": "2026-04-22",
  "schema_version": 1
}
```

**Constraints**:

- `expected_files` paths are repo-relative and must exist at the
  fixture's curation date in the git history (not necessarily today —
  the test loader resolves blob via `git cat-file` if the path was
  later renamed/removed)
- `rationale` is mandatory and must justify *why* each file is
  expected, in one sentence; this prevents fixture rot
- `curator` must be a real human reviewer (not "agent" or "auto")
- `schema_version` allows forward-compatible loader changes

A fixture without all required fields fails to load — no silent
defaults.

---

## 4. Curating fixtures (the human discipline)

Goal: 30 fixtures for F1, 50 for F2 (additional 20 cross-repo). The
process is intentionally manual; this is the part that cannot be
automated without circular reasoning (you cannot ask the retriever to
grade itself).

**Selection criteria** for a candidate task:

1. The task is **completed** (Thor verdict Pass) and lives in the
   audit log.
2. The task touched **at least 2 distinct files**, no more than 12
   (otherwise the recall denominator is too noisy).
3. The task **diff is committed** so the reviewer can see what
   actually shipped.
4. The task is **not trivial** (not a one-liner formatting fix).

**Per-task curation steps**:

1. Reviewer reads `task_body` cold (no IDE help, no retriever).
2. Reviewer lists the files they would have wanted in their context
   to do the task — this is the `expected_files` ground truth.
3. Reviewer runs `git diff` of the actual completed task; any file
   changed in the diff goes into `expected_files` too if not already
   there.
4. Reviewer adds a one-sentence rationale per file in the JSON
   `rationale` field (one prose paragraph covering all files is fine
   when they cluster).

**Anti-pattern**: do *not* include test files in `expected_files`
unless the task is specifically a testing task. The retriever will
naturally over-rank tests because they share vocabulary; we don't
want to give it an artificial bonus.

**Fleet fixtures** (F2 onward): the same shape, but `expected_files`
paths are prefixed with the repo name (`convergio-edu/src/...`) and
the `repo` field denotes the *primary* repo of the task — fixture is
counted in cross-repo recall only when `expected_files` references at
least two distinct repos.

---

## 5. Measuring recall

The bench harness lives at `crates/convergio-embed/tests/recall_bench.rs`
(F1) and is wrapped by a `cargo test` integration test for CI.

```rust
#[tokio::test]
async fn hybrid_recall_meets_f1_threshold() -> Result<()> {
    let fixtures = load_golden_set("tests/fixtures/retrieval-golden")?;
    let mut substring_total = 0.0;
    let mut hybrid_total = 0.0;
    for fx in &fixtures {
        let baseline = pool.for_task_substring(&fx.task_id).await?;
        let hybrid = pool.for_task_hybrid(&fx.task_id, alpha = 0.5).await?;
        substring_total += recall_at_k(&baseline.files, &fx.expected_files, 10);
        hybrid_total += recall_at_k(&hybrid.files, &fx.expected_files, 10);
    }
    let baseline_avg = substring_total / fixtures.len() as f64;
    let hybrid_avg = hybrid_total / fixtures.len() as f64;
    let lift = hybrid_avg - baseline_avg;
    assert!(
        lift >= 0.15,
        "F1 gate: hybrid recall@10 = {hybrid_avg}, baseline = {baseline_avg}, lift = {lift} < 0.15"
    );
    assert!(hybrid_avg >= 0.85, "absolute recall@10 floor 0.85 not met: {hybrid_avg}");
    Ok(())
}
```

Two assertions:

- **Lift** ≥ 0.15 (absolute) — the hybrid retriever must measurably
  beat substring-only on the same fixtures
- **Floor** ≥ 0.85 (absolute hybrid recall@10) — the hybrid must be
  good in absolute terms, not just better than a weak baseline

Both assertions come from ADR-0038 § 6 F1 go/no-go criteria.

---

## 6. Determinism (P1 compliance)

Probabilistic ≠ flaky. The harness pins:

| Source of variance | How it is pinned |
|--------------------|------------------|
| Embedding model | Pinned to a single hash in a `convergio-embed` constant; CI verifies the hash on download |
| Quantisation | int8 default — deterministic for the same input on the same hardware family |
| Tokeniser | Pinned to model's tokeniser; no fallback path |
| Random seed | Not used by the embedding pipeline; the only randomness is in tie-breaking RRF, which is deterministic given a stable input ordering |
| File ordering on disk | The fixture loader sorts `expected_files` lexicographically before comparison |
| Hardware | The recall@K metric is robust to ±1% variance in cosine values caused by different SIMD paths; the ≥0.15 lift threshold is set well above this noise floor |

If a CI run produces a different recall number than a local run on
identical inputs, that is a bug in the harness, not in the model. The
test must investigate it before merging.

---

## 7. CI cost control

Embeddings cost CPU. CI runs cannot afford to embed 30K nodes on
every PR. Strategy:

| Trigger | Subset | Wall time budget |
|---------|--------|------------------|
| PR (any) | 50-task subset of golden, model loaded from `~/.cache/convergio/embed-models/` (cached across CI runs) | ≤ 90 s |
| Push to `main` | full 30-task golden + bench harness | ≤ 4 min |
| Nightly on `main` | full 30-task golden + recall trend ingestion + storage size check + p95 latency bench | ≤ 8 min |

The 50-task PR subset is `tests/fixtures/retrieval-golden/_subset.json`
— a deterministic list of fixture IDs maintained by hand to maximise
diversity (different crates, different task sizes, different
languages once F2 lands). Subset selection is itself committed and
reviewable.

---

## 8. Regression detection

Every golden run writes a JSON report to
`target/retrieval-bench/<git-sha>.json`:

```json
{
  "git_sha": "abc1234",
  "model": "bge-m3-small-int8",
  "fixtures": 30,
  "recall_at_10_substring": 0.71,
  "recall_at_10_hybrid": 0.89,
  "lift": 0.18,
  "p95_query_ms": 740,
  "storage_mb": 38.4,
  "incremental_rebuild_s": 22.1
}
```

Nightly job appends to `target/retrieval-bench/trend.csv` (a single
file under VCS so trend is reviewable). Any 5-fixture subset showing
`>0.05` regression in recall@10 vs the last green main flags the next
PR as **needs review**.

---

## 9. What this document does *not* cover

- **Drift detection methodology** (semantic doc-drift). Lives in F3
  scope and gets its own spec under
  `docs/adr/0041-split-session-into-its-own-crate.md` if/when F3 starts.
- **Cluster naming evaluation** (F3, decision D-5). Separate spec.
- **Cross-language fixture generation tooling**. F2-6 in the durable
  plan tracks this; methodology to follow when F2 starts.

This document is intentionally narrow: it specifies *only* the
recall@K gate that decides F1 go/no-go and continues to be the
quality floor for F2/F3 retrieval changes.

---

## 10. Open questions for the F1 reviewer

- Should the 30 F1 fixtures be skewed toward tasks the team got
  *wrong* historically (where retrieval failed and the agent
  hallucinated)? Trade-off: more useful signal, but risks training a
  retriever that is good at hard cases and worse at average ones.
  *Default answer*: 70% normal, 30% historically-failed.
- Should we publish anonymised golden fixtures in the public repo?
  *Default answer*: yes — fixtures derive from open-source Convergio
  tasks and there is no PII; the rationale field is reviewed by
  Roberto before commit.

---

*End of golden-set methodology spec.*
