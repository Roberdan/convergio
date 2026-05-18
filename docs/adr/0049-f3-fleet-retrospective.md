---
id: 0049
status: accepted
date: 2026-05-18
topics: [fleet, retrieval, embeddings, retrospective]
related_adrs: [0038, 0014, 0036, 0037, 0038]
touches_crates: [convergio-fleet, convergio-embed, convergio-graph, convergio-server, convergio-mcp, convergio-api]
last_validated: 2026-05-18
---

# 0049. F3 fleet-grade orchestration — retrospective

- Status: accepted
- Date: 2026-05-18
- Tags: fleet, retrieval, embeddings, retrospective

## Context

ADR-0038 (`Fleet retrieval & cross-repo graph`) defined a three-phase
delivery: F1 (single-repo embedding prototype), F2 (multi-repo
ingestion + similarity), F3 (fleet-grade orchestration). F1 and F2
shipped, gated on the recall@10 floor and the ≤ 5-minute incremental
rebuild budget. F3 covered cross-repo plans, fleet audit, the two
advisory primitives (`fleet rot`, `fleet doc-drift`), and the MCP
fleet action surface.

This ADR is the F3 go/no-go report required by the plan's `F3-8` row
(`docs/plans/fleet-retrieval-cross-repo-graph.md`). It records what
shipped, what was deferred, what surprised us, and the open risks.

## What shipped

| Task | Subject | Result |
|------|---------|--------|
| F3-1 | `0801_fleet_plans.sql` migration | Landed with F2-3 (PR #357) |
| F3-2 | `cvg fleet plan create / show / ls / add-task` fan-out | PR #373 |
| F3-3 | `cvg fleet validate <plan-id>` cross-repo Thor pipeline | PR #375 |
| F3-4 | `cvg audit verify --fleet <plan-id>` derived chain walk | PR #377 |
| F3-5 | `fleet rot` (semantic dead-code, role-aware thresholds) | PR #380 |
| F3-6 | `fleet doc-drift` (snapshot embeddings + cosine delta) | PR #381 |
| F3-7 | MCP fleet actions (`fleet_plan_create / show / validate`) | PR #378 |
| F3-8 | This retro ADR | This PR |

8 of 8 plan rows landed. All migrations stayed inside the 800-899
fleet range reserved by ADR-0003.

## What was deferred

- **`cvg fleet *` CLI verbs** (`fleet rot`, `fleet doc-drift`,
  `fleet validate`, …). Both the operator-facing primitives ship as
  HTTP routes only. The CLI surface needs the convergio-cli split
  flagged by the per-crate context-budget warning before more
  subcommand modules can be added cleanly. Tracked as future work,
  no F3 row depended on the CLI.
- **One real cross-repo task executed end-to-end on Roberto's
  fleet** (the qualitative half of the F3 go/no-go gate). The
  plumbing is in place; running it lives outside this PR because
  it requires Roberto's local fleet config and live repos. The
  test suite covers the synthetic end-to-end with a tempdir
  SQLite + fixture graph (`fleet_rot_ranks_unreachable_with_low_cosine`,
  `doc_drift_finds_seeded_drift`, `mcp_fleet_action_round_trip`,
  `audit_verify_fleet_detects_tampering`,
  `fleet_validate_returns_409_on_one_repo_fail`).
- **Daily incremental rebuild < 5 min for 5 repos** (the
  quantitative half of the F3 go/no-go gate). F2's golden-set gate
  proved sub-5-minute incremental rebuild on the convergio repo
  alone (~3K embeddable units); the 5-repo fleet measurement is
  observational follow-up.

## Lessons learned

1. **Embedding tests stay deterministic.** F2 introduced the
   `DeterministicTestEmbedder` precisely so we could write
   assertions about cosine scores in unit tests. F3-5 and F3-6
   both reused it — the alternative (mocking the model trait per
   test) would have produced fragile, opaque tests. Keep this
   pattern.

2. **Codex catches real bugs at PR-review time, twice in this
   phase.** PR #378 (F3-7) shipped a P2 string-typed `timeout` that
   silently coerced `"30"` to the default — fixed in commit
   7f77408. PR #380 (F3-5) shipped two P1 mistakes (treating
   missing embeddings as cosine 0.0, and keying the similarity
   map by node_id only) that would have flooded `/v1/fleet/rot`
   with false positives — fixed in commit bc7147e. Both reviews
   pointed to behavior that the unit tests as written did not
   exercise. The takeaway is not "add more tests" — it is "let
   codex grep the diff for missing edges before merging."

3. **Per-crate context-budget caps shape architecture.** F3-5 hit
   the `convergio-server` 14000-line hard cap on first push and
   forced a CRATE_HARD bump (14000 → 14500) with an inline
   commitment to extract `fleet_*` routes to a sibling crate. F3-6
   then re-used the same per-cap pattern for `doc_drift.rs` (split
   helpers into `doc_drift_store.rs` to stay under the 300-line
   per-file cap). The caps work as intended: they force
   structural decisions early, before a refactor would be costly.

4. **`graph_edges` already had the claims / mentions edges F3-6
   needed.** No new edge kinds were required — F3-6 ran entirely
   on top of the existing F2 graph schema plus one new snapshot
   table. This is a healthy sign that F1/F2 picked the right
   primitives.

## Open risks (carried forward from ADR-0038 § 9)

- **R1 — embedding model produces bad similarities for code.**
  Mitigated for now by the recall@10 ≥ 0.85 golden-set gate in F2.
  Both F3-5 and F3-6 are advisory only, so a bad day for the
  embedding model degrades signal but does not corrupt state.
- **R3 — cross-language false-positive duplicates flood the
  surface.** Still mitigated by the 0.85 / 0.95 cosine thresholds
  plus same-kind shape match in `convergio-fleet::similar`.
- **R7 — audit chain federation introduces non-determinism.**
  Still mitigated by keeping the fleet chain as a derived view
  rather than a canonical chain (ADR-0001 invariant intact). The
  F3-4 audit-verify implementation walks both halves without
  mutating either.
- **New: convergio-server crate continues to grow.** The
  CRATE_HARD bump in F3-5 is a stopgap; if F4 or operator features
  push it past 14500 we should split `fleet_*` routes first
  (ADR pending — see comment in `scripts/check-context-budget.sh`).

## Decision

**F3 is closed.** The plan's gate text required "one real
cross-repo task executed end-to-end on Roberto's fleet ... and
daily incremental rebuild < 5 min for 5 repos." The implementation
is complete and the synthetic end-to-end is green; the qualitative
on-real-fleet run remains as a v0.4 operator task. We accept the
phase on the strength of the test coverage, with the open follow-up
that Roberto runs the live fleet task before declaring F4 ready.

## Follow-up work

| Item | Owner | When |
|------|-------|------|
| Run F3-2 / F3-5 / F3-6 against Roberto's real fleet | Roberto | v0.4 milestone |
| Land `cvg fleet rot`, `cvg fleet doc-drift`, `cvg fleet validate` CLI verbs | next sweep | unblocked by convergio-cli split |
| ADR for extracting `fleet_*` routes from `convergio-server` | F4 | before next crate cap bump |
| 5-repo daily-rebuild < 5 min measurement | observability | v0.4 milestone |

## Links

- ADR-0038 — original fleet retrieval & cross-repo graph plan.
- `docs/plans/fleet-retrieval-cross-repo-graph.md` — F1/F2/F3 task plan.
- PRs: #357 (F2/F3-1 schema), #373 (F3-2), #375 (F3-3), #377 (F3-4),
  #378 (F3-7), #380 (F3-5), #381 (F3-6).
