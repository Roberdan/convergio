---
type: Plan
status: F1-α in flight (storage + selective + source hash + embedder trait + brute-force KNN)
owner: Convergio
updated: 2026-05-03
source_of_truth: repo
related_adrs: [0038, 0034, 0014, 0003, 0004, 0005, 0015]
related_specs: [docs/spec/fleet-retrieval-cross-repo-graph.md, docs/spec/fleet-retrieval-golden-methodology.md]
---

# Fleet retrieval & cross-repo graph — execution plan

## Objective

Land the fleet brain proposed by ADR-0038 in three product-gated phases
(F1 → F2 → F3) without violating the Convergio Constitution. The plan
below decomposes the work into concrete tasks with IDs, dependencies,
acceptance criteria, and validation commands an agent can run.

This file is the engineering source of truth. The PRD lives at
`docs/spec/fleet-retrieval-cross-repo-graph.md`; the ADR at
`docs/adr/0038-fleet-retrieval-cross-repo-graph.md`; the recall
methodology at `docs/spec/fleet-retrieval-golden-methodology.md`.

## Gate map

| Phase | Status | Blocker |
|-------|--------|---------|
| **F0 — Foundation** | in flight (this PR) | none — closes when this PR merges |
| **F1 — Single-repo embedding prototype** | blocked | F0 merged + Roberto's answers on D-1, D-2, D-7, D-8, D-9 |
| **F2 — Multi-repo opening** | blocked | F1 go/no-go gate (recall@10 ≥ 0.85, p95 < 1s, storage < 50MB, incremental rebuild < 30s) |
| **F3 — Fleet-grade orchestration** | blocked | F2 go/no-go gate (≥3 cross-repo patterns surfaced, duplicate FP rate < 20%, all 524 tests green + ≥60 new) |

Each gate is **reversible**: if a phase's go/no-go fails, the work
revert path is documented in ADR-0038 § 8.2.

## Open product decisions (Roberto)

These ten decisions block F1 design lock. Defaults from ADR-0038 § 12
are recommended; PR #129 collision already resolved (ADR is 0038, not
0034).

| #    | Decision | Default | Status |
|------|----------|---------|--------|
| D-1  | Embedding model | BGE-M3-small (multilingual, 384-dim, int8) | open |
| D-2  | Quantisation | int8 default | open |
| D-3  | Single daemon vs federated | single | defer to F2 lock |
| D-4  | Fleet config location | `~/.convergio/v3/fleet.toml` | defer to F2 lock |
| D-5  | Cluster naming | centroid keyword extraction | defer to F3 |
| D-6  | Doc-drift snapshot trigger | on daemon start (lazy) | defer to F3 |
| D-7  | First fleet repos | convergio + convergio-edu + convergio-ui-framework | open |
| D-8  | Re-embed strategy | source_hash change | open |
| D-9  | API versioning | F1+F2 in v3.x; F3 in v4.0 | open |
| D-10 | Telemetry on retrieval recall | opt-in, local-only | defer to F2 |

## F0 — Foundation tasks (this PR)

| ID | Subject | Status | Acceptance | Validation |
|----|---------|--------|-----------|-----------|
| F0-1 | Move PRD into `docs/spec/` | done | file exists at `docs/spec/fleet-retrieval-cross-repo-graph.md`, references point to ADR-0038 | `grep -n 'ADR-0034' docs/spec/fleet-retrieval-cross-repo-graph.md` returns nothing |
| F0-2 | Renumber ADR 0034→0038, move into `docs/adr/` | done | file exists at `docs/adr/0038-fleet-retrieval-cross-repo-graph.md`, frontmatter `id: 0038` | `head -3 docs/adr/0038-*.md \| grep 'id: 0038'` |
| F0-3 | Reserve migration ranges in ADR-0003 (700/800/900) | done | ranges visible in ADR-0003 table, marked *proposed* | `grep 'convergio-embed.*700' docs/adr/0003-migration-coexistence.md` |
| F0-4 | Update `docs/adr/README.md` index | done | row 0038 present between AUTO markers | `grep '0038-fleet-retrieval' docs/adr/README.md` |
| F0-5 | Write golden-set methodology spec | done | file exists at `docs/spec/fleet-retrieval-golden-methodology.md`, defines recall@K + fixture format + CI cost control | `test -f docs/spec/fleet-retrieval-golden-methodology.md` |
| F0-6 | Write durable plan (this file) | done | file exists, references PRD + ADR + methodology | `test -f docs/plans/fleet-retrieval-cross-repo-graph.md` |
| F0-7 | Register plan + tasks via Convergio MCP (dogfood) | in progress | plan + F1 tasks exist in `~/.convergio/v3/state.db`; evidence linked to F0 file paths | `cvg plan ls \| grep fleet` |
| F0-8 | Open PR with 5 H2 sections | in progress | PR open against `main`, body has Problem/Why/What changed/Validation/Impact | `gh pr view --json title,body` |

## F1 — Single-repo embedding prototype

**Scope**: ADR-0038 § 6 F1. Validate on Convergio's own repo only.
Recall@10 lift over substring baseline is the single decision metric.

| ID | Subject | Blocked by | Acceptance | Validation |
|----|---------|-----------|-----------|-----------|
| F1-1 | Bootstrap `crates/convergio-embed/` (lib + AGENTS.md + CLAUDE.md symlink + lib.rs `//!`) | F0-2, D-1, D-2 | `cargo check -p convergio-embed` clean; crate has `///` docs on every `pub`; ≤300 LOC/file | **landed** (PR #135 `feat/fleet-f1-embed-foundation`): `cargo check -p convergio-embed` clean; ≤300 LOC enforced (store split into `store.rs` + `codec.rs`); cross-layer `/v1/embed/stats` route + e2e wired |
| F1-2 | Migration `0700_embeddings.sql` (schema in ADR-0038 § 5.2.3) | F1-1 | applies on fresh `~/.convergio/v3/state.db`; idempotent | **landed**: F1-α ships only the storage table; `sqlite-vec` virtual table deferred to F1-β |
| F1-3 | `fastembed-rs` integration with BGE-M3-small (or model from D-1) loaded lazily on first query | F1-1 | model downloads to `~/.convergio/v3/models/`; query returns float32[384] (or vector dim from D-1); fallback path on download failure | **partial — trait shipped, model deferred**: `Embedder` trait + `DeterministicTestEmbedder` (NOT a model) shipped in F1-α; real `fastembed-rs`/BGE-M3 lands in F1-β |
| F1-4 | `sqlite-vec` extension load behind `embed` feature flag | F1-1, F1-2 | `vec0` virtual table created when feature on; daemon boots fine when feature off | **deferred to F1-β**: pure-Rust brute-force KNN ships in F1-α with stable signature so the swap is local |
| F1-5 | Selective embedding pipeline (crates, modules, documented items, ADRs, docs) | F1-3 | embeds only the categories listed in ADR-0038 § 5.4; `source_hash` re-embed trigger | **landed**: `EmbedPolicy`/`EmbedTarget` + `SourceText`/`needs_reembed` shipped with unit + e2e coverage |
| F1-6 | Hybrid retrieval ranking (RRF default, linear with `--alpha` opt-in) | F1-3, F1-4 | `for-task` returns nodes with `match_source ∈ {structural, semantic, both}` and `score_components` | unit test `rrf_fusion_preserves_provenance` |
| F1-7 | `cvg graph for-task --semantic` and `--gap-check` flags | F1-6 | flags wired through CLI → HTTP → server route; default off; behaves identically to current command when off (D6 back-compat) | E2E test `for_task_semantic_off_matches_legacy_output` |
| F1-8 | Golden set: 30 historical Convergio tasks with hand-curated expected files | F0-5 | `tests/fixtures/retrieval-golden/` has 30 task fixtures, schema validated against `docs/spec/fleet-retrieval-golden-methodology.md` | `cargo test -p convergio-embed golden_set_loads` |
| F1-9 | Bench harness `crates/convergio-embed/benches/recall.rs` measuring substring vs hybrid recall@10 | F1-8 | bench runs in CI on a 50-task subset; full set on nightly main only | `cargo bench -p convergio-embed --bench recall -- --quick` |
| F1-10 | F1 go/no-go report → ADR-0038 decision log | F1-9 | report shows recall@10 lift ≥15% absolute, p95 < 1s, storage < 50MB, incremental rebuild < 30s, OR documents the no-go finding | F1 retrospective ADR appended to `docs/adr/0038-...` decision log |

**F1 go/no-go criteria** (any ONE failed criterion = no-go):

- Hybrid recall@10 over substring-only baseline: **≥ +0.15 absolute**
- p95 query latency on 3K-node graph (warm): **< 1 s**
- Embedding storage on Convergio repo: **< 50 MB**
- Incremental rebuild after 5-file change: **< 30 s**
- All existing 524 tests still green; new tests added are deterministic
  per ADR-0038 § 7.1/7.2

**No-go path**: revert F1 migrations and crate, keep F0 docs as a
record of why we chose not to proceed; write retrospective ADR.

## F2 — Multi-repo opening

**Scope**: ADR-0038 § 6 F2. Open the graph to TS + Python parsers and
backfill convergio + convergio-edu + convergio-ui-framework.

| ID | Subject | Blocked by | Acceptance | Validation |
|----|---------|-----------|-----------|-----------|
| F2-1 | Bootstrap `crates/convergio-parse-multi/` (TS + Python tree-sitter) | F1 go | `cargo check` clean; per-language module ≤300 LOC; pinned grammar versions | unit test per language emits `(Vec<Node>, Vec<Edge>)` |
| F2-2 | Bootstrap `crates/convergio-fleet/` with `fleet.toml`, `cvg fleet add/ls/build` | F1 go, D-4 | crate ≤300 LOC/file; CLI commands flow through `convergio-server` HTTP | E2E `fleet_add_then_ls` |
| F2-3 | Migration `0601_repo_dimension.sql` + `0602_repo_not_null.sql` (backfill `repo='convergio'`) | F2-2 | idempotent on fresh DB; existing graph rows updated; never block daemon boot | E2E `existing_graph_rows_get_repo_field` |
| F2-4 | Cross-repo similarity batch job (`cvg fleet build --refresh-similarity`) producing `similar_to` and `duplicates` edges | F2-1, F2-2 | edges created only above configured cosine; `match_source` recorded | unit test `similarity_batch_respects_threshold` |
| F2-5 | `cvg fleet patterns` and `cvg fleet duplicates` | F2-4 | output JSON schema documented; `--explain <id>` evidence trail | E2E `fleet_patterns_finds_seeded_cluster` |
| F2-6 | Cross-language fixture set under `tests/fixtures/fleet/` | F2-1 | three mini-repos (Rust + TS + Python) with ≥3 deliberate cross-language pattern duplicates | `cargo test fleet_fixtures_parse_all_languages` |
| F2-7 | F2 go/no-go report | F2-5 | ≥3 real cross-repo patterns surfaced on Roberto's fleet; FP rate < 20% on 50 sampled pairs; +60 tests; 524 baseline still green | F2 retrospective appended to ADR-0038 decision log |

**F2 go/no-go criteria**: at least 3 real cross-repo patterns
discovered AND duplicate FP rate < 20% (manual review of 50 sample
pairs by Roberto) AND test count green.

## F3 — Fleet-grade orchestration

**Scope**: ADR-0038 § 6 F3. Cross-repo plans, fleet audit verifier,
fleet-aware MCP actions.

| ID | Subject | Blocked by | Acceptance | Validation |
|----|---------|-----------|-----------|-----------|
| F3-1 | Migration `0801_fleet_plans.sql` | F2 go | `fleet_plans` + `fleet_plan_repos` tables; idempotent | E2E `fleet_plan_create_inserts_rows` |
| F3-2 | `cvg fleet plan create / show / ls / add-task` | F3-1 | per-repo plan rows linked under one fleet_plan_id; rollup status JSON | E2E `fleet_plan_fanout_to_three_repos` |
| F3-3 | `cvg fleet validate <plan-id>` runs gate pipeline per touched repo | F3-2 | green only when all touched repos pass; per-repo verdicts surfaced | E2E `fleet_validate_returns_409_on_one_repo_fail` |
| F3-4 | `cvg audit verify --fleet <plan-id>` walks all touched chains | F3-1 | derived view, never mutates per-repo chain (D11) | E2E `audit_verify_fleet_detects_tampering` |
| F3-5 | `cvg fleet rot` (semantic dead-code with role-aware thresholds) | F2 go, D-5 | output respects `--threshold` + `--explain <id>`; advisory only | E2E `fleet_rot_ranks_unreachable_with_low_cosine` |
| F3-6 | `cvg fleet doc-drift` with snapshot embeddings | F2 go, D-6 | ADR/README candidates surfaced with semantic delta summary | E2E `doc_drift_finds_seeded_drift` |
| F3-7 | MCP fleet actions (`convergio.fleet.for_task`, `convergio.fleet.validate`) | F3-2, F3-3 | typed action additions in `convergio-api`; `convergio-mcp` exposes them | E2E `mcp_fleet_action_round_trip` |
| F3-8 | F3 go/no-go report → ADR-0038 decision log | F3-7 | one real cross-repo task executed end-to-end on Roberto's fleet, fleet audit green, daily incremental rebuild < 5 min for 5 repos | F3 retrospective ADR |

## Validation commands (any phase)

The agent that picks up a task from this plan must run, before
declaring `submitted`:

```bash
cargo fmt --all -- --check
RUSTFLAGS="-Dwarnings" cargo clippy --workspace --all-targets -- -D warnings
RUSTFLAGS="-Dwarnings" cargo test --workspace
```

Plus the per-task validation column above. P5 (i18n IT/EN) parity:
every new user-facing string must flow through `convergio-i18n` Fluent
bundles (CONSTITUTION § P5).

## Risks (cross-phase)

See ADR-0038 § 9 for the full table. Top three the executing agent
must keep front-of-mind:

- **R1 — embedding model produces bad similarities for code** →
  blocked by golden-set gate at F1
- **R7 — audit chain federation introduces non-determinism** → fleet
  chain stays *derived view*; per-repo chain remains canonical
  (ADR-0001 invariant)
- **R9 — probabilistic tests flaky** → fixed model + fixed seed +
  deterministic int8 quantisation; recall@10 has hard floor not exact
  match (ADR-0038 § 7.2)

## Links

- PRD: [`docs/spec/fleet-retrieval-cross-repo-graph.md`](../spec/fleet-retrieval-cross-repo-graph.md)
- ADR: [`docs/adr/0038-fleet-retrieval-cross-repo-graph.md`](../adr/0038-fleet-retrieval-cross-repo-graph.md)
- Recall methodology: [`docs/spec/fleet-retrieval-golden-methodology.md`](../spec/fleet-retrieval-golden-methodology.md)
- ADR-0014 — Tier-3 single-repo retrieval (extended by 0038)
- ADR-0034 — Per-task runner fields (PR #129, parallel work; no overlap)
- CONSTITUTION § P1, P2, P4, P5
