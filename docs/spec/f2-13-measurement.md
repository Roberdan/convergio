# F2-13 measurement: cross-repo patterns + duplicate FP rate

**Status:** in progress (2026-05-04).
**Source of truth for numbers:** `crates/convergio-server/tests/e2e_f2_13_measure.rs`.
**Plan row:** F2-13 in `docs/plans/fleet-retrieval-cross-repo-graph.md`.
**Gate:** ADR-0038 §6 F2 — F2 ships iff ≥3 cross-repo patterns AND
duplicate FP rate <20% on 50 sampled pairs.

## What gets measured

Three numbers, on every run of the e2e test:

1. **Cross-repo pattern clusters (min_repos=2)** — clusters from
   `find_patterns(fleet, 2)` after `cvg fleet build --refresh-similarity`.
2. **Cross-repo pattern clusters (min_repos=3)** — same, with the strict
   ≥3 distinct repos requirement that the F2 gate calls for.
3. **FP rate on a sample of 50 duplicate pairs** — pairs from
   `find_duplicates(fleet, 0.95, all=true)`, each classified by
   `classify_pair_tp` (see § Classifier below).

## Fleet under measurement

Three repos, registered in this order:

| name | language | source path |
|------|----------|-------------|
| `convergio` | rust | the working tree itself |
| `convergio-edu` | typescript | `/Users/Roberdan/GitHub/convergio-edu` if present, else fixture proxy `tests/fixtures/fleet/plan-fsm-ts` |
| `ui-framework (external)` | typescript / python | `/Users/Roberdan/GitHub/ui-framework (external)` if present, else fixture proxy `tests/fixtures/fleet/plan-fsm-py` |

**Fixture-proxy mode** is the default in CI / on contributor machines:
the F2-11 mini-repos (plan-fsm-rs/ts/py) stand in for the missing
downstream repos. The numbers are then *structural only* — they prove
the pipeline runs end-to-end across three languages but they do not
exercise real cross-repo semantics. The gate assertion is **skipped**
in fixture-proxy mode (`assert!` only fires when both downstream repos
are real on disk). Re-running on a workstation that has the real
`convergio-edu` and `ui-framework (external)` checkouts produces the
authoritative go/no-go numbers.

## Classifier

`classify_pair_tp(name_a, kind_a, name_b, kind_b, score) -> bool` —
TP iff:

- normalised names are identical (lower-case, `[-_]`→space), OR
- same `node_kind` AND ≥50% shared significant tokens (len ≥ 4), OR
- score ≥ 0.98 *and* names are too short to tokenise.

Everything else is FP.

This is a **mechanical** classifier — it does not need a human.  It
deliberately under-counts TPs (e.g. cross-language renames like
`taskState` ↔ `task_state` ↔ `TaskFsm` are split unless they share a
4-char token), so the FP rate it reports is an **upper bound** on the
true FP rate. If the upper bound is <20% the gate passes; if it is
≥20% a human review of the pairs flagged FP is needed before NO-GO.

## Embedder

- **Default** (no feature flag): `DeterministicTestEmbedder(384)` — a
  hash-based embedder for fast, reproducible structural runs.
- **`--features fastembed`** + `CONVERGIO_BENCH_MODEL=multilingual-e5-small`:
  real ONNX model. Numbers from this configuration are the ones used
  for the F2 gate decision.

## Run book

```bash
# Structural (CI / contributors) — fixture proxies, no gate assertion
cargo test -p convergio-server --test e2e_f2_13_measure \
  -- --ignored --nocapture

# Real-model on workstation with all three repos checked out
cargo test -p convergio-server --features fastembed \
  --test e2e_f2_13_measure -- --ignored --nocapture
```

The test prints the build report, every cluster, every sampled pair
with TP/FP verdict, then a final 6-line summary block.  Capture stdout
to feed F2-15's retrospective ADR.

## Open items

- Authoritative real-model run with both downstream repos pending —
  sub-task of F2-13.  Numbers go into ADR-0038 §15.8.
- Sample size is hard-capped at 50; if `find_duplicates` returns fewer
  pairs the gate's `if sample_size >= 10` branch may also be skipped.
  Sample-size adequacy is reviewed in F2-15.
