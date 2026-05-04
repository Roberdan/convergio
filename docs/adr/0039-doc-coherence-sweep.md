---
id: 0039
status: accepted
date: 2026-05-03
topics: [documentation, coherence, drift, gates, retrieval, semantic]
related_adrs: [0014, 0015, 0038, 0040]
touches_crates: [convergio-cli, convergio-coherence, convergio-server, convergio-durability, convergio-graph]
last_validated: 2026-05-04
implemented_in: []
authors: [Roberto D'Angelo]
---

# 0039. Doc-coherence sweep as a recurring three-layer plan

- Status: accepted (shipped — L1 `cvg docs regenerate --check` (ADR-0015) and L2 deterministic verifiers (`cvg coherence check` route-table + ADR status cross-check, ADR-0040 crate `convergio-coherence`); L3 semantic sweep + recurring-plan wiring tracked as future work, see below)
- Date: 2026-05-03
- Deciders: Roberto D'Angelo, claude-code-roberdan
- Tags: documentation, coherence, drift, gates, retrieval

## Context and Problem Statement

ADR-0015 made documentation a derived artefact: AUTO blocks in
`AGENTS.md`, `docs/adr/README.md`, and the workspace-members table
are rewritten by `cvg docs regenerate`. That gate catches **mechanical
drift** — a missing crate row, a stale ADR index, a renamed CLI verb.

It does not catch the two harder drift modes that have already shipped
on `main` since v3 began:

1. **Semantic drift in prose.** An ADR body claims behaviour X; the
   crate it `touches_crates` now does Y. Frontmatter is correct,
   AUTO blocks are clean, the body lies. ADR-0014's structural
   `drift.rs` cannot see this — it compares declared crates against
   `uses` edges, never against the *meaning* of the prose.
2. **Cross-document inconsistency.** The root `AGENTS.md` advertises
   an HTTP route surface that diverges from the actual `axum` router;
   `ARCHITECTURE.md` lists background loops that no crate spawns; an
   accepted ADR carries `implemented_in: []` long after the PR landed.

ADR-0038 adds the substrate that makes the semantic case tractable:
a multi-repo graph plus an embedding layer (BGE-M3-small + sqlite-vec)
behind a feature flag. The question this ADR answers is **how to
turn that substrate plus the existing `cvg docs regenerate` and
`cvg coherence check` surfaces into a single, recurring,
gate-respecting workflow** rather than three disjoint commands.

This is itself a recurring engineering plan (Convergio plan
`ee9ab55a-eb28-41ee-bd02-e22d002e8b3c` — *Doc-coherence sweep + wiki
sync*). The product premise is the same Convergio applies to agent
work: drift becomes evidence; evidence is gated; the sweep cannot be
silently skipped.

## Decision Drivers

| # | Driver | Source |
|---|---|---|
| D1 | Reliability is structural, not behavioural — drift must be impossible, not policed | ADR-0015 §Decision Drivers |
| D2 | Mechanical drift already has a working solution (AUTO blocks) — do not re-litigate | ADR-0015, implemented in PR #45 |
| D3 | Semantic drift needs the embedding substrate from ADR-0038 — but ADR-0038 is feature-flagged and probabilistic | ADR-0038 §3 Decision Drivers |
| D4 | Probabilistic outputs cannot block CI — they advise, humans decide | derived from ADR-0004 (zero tolerance on tech debt; probabilistic ≠ debt only when advisory) |
| D5 | Deterministic Rust verifiers already exist (`cvg coherence check` T1.17) — extend them, don't replace | ADR-0014 §Tier 2 |
| D6 | The sweep must itself be auditable — same plan/task/evidence model the leash uses on agents | ADR-0001 (audit chain), root AGENTS.md |
| D7 | i18n IT/EN day one — sweep output must be translatable | CONSTITUTION P5, ADR-0005 |

## Considered Options

1. **Status quo — keep `cvg docs regenerate` and `cvg coherence
   check` as separate one-shot commands.** Discipline asks reviewers
   to run both before merging. Empirically failed (ADR-0015 §Context).
2. **Single deterministic mega-verifier — extend `cvg coherence
   check` with all checks (route table, ADR status, semantic
   prose).** Conflates probabilistic and deterministic concerns;
   either the gate is too strict (semantic false positives block
   CI) or too loose (deterministic verifiers stop blocking).
3. **Three-layer sweep, single recurring plan (this ADR).**
   Mechanical / deterministic / semantic each run independently with
   their own block-vs-advisory contract; the recurring Convergio
   plan ties them together so the sweep cannot drop off the
   schedule. See §Decision Outcome.
4. **Outsource to an external LLM doc bot (Mintlify, etc.).**
   Violates D2 (local-first) and D6 (audit chain integrity);
   external tool cannot write evidence rows into the daemon.

## Decision Outcome

Chosen option: **Option 3 — three-layer doc-coherence sweep,
operated as a recurring Convergio plan**.

### Implementation status (2026-05-04)

The three-layer framework is **the accepted policy**, but the layers
ship on independent timelines:

- **L1 — mechanical AUTO blocks**: shipped. `cvg docs regenerate
  --check` is wired in CI per ADR-0015.
- **L2 — deterministic verifiers**: shipped this week. `cvg coherence
  check` (route-table verifier + ADR status / supersession
  cross-check, ADR-0040) is the umbrella verb; it is blocking in CI
  for `--strict` failures (`accepted_no_evidence`,
  `broken_supersession`).
- **L3 — semantic sweep + recurring-plan wiring**: tracked as future
  work. ADR-0038 F1/F2 already shipped the embedding substrate,
  but the "diff-time PR comment" and "nightly recurring plan with
  `doc_coherence_sweep` evidence rows" are not yet implemented.

The framework decision is final; L3 ships when the cost / token
budget under ADR-0038 F1 has been verified against real PRs.

`cvg coherence` is the umbrella verb. It operates in three layers
that share nothing except the plan id they attach evidence to:

### L1 — Mechanical (AUTO blocks, ADR-0015)

- **Already shipped.** `cvg docs regenerate` rewrites every AUTO
  block from workspace state. `cvg docs regenerate --check` returns
  non-zero on drift.
- **Block in CI.** A failing `--check` is a deterministic build
  break.
- **Owns**: workspace-members table, ADR index, CLI verb list,
  test count, crate `Cargo.toml` glue.
- **No change in this ADR** — listed for completeness so the three
  layers form one map.

### L2 — Deterministic verifiers (Rust, blocking)

Two new verifiers added under `cvg coherence check`, both
**deterministic** (no LLM, no embedding) and **blocking in CI**:

#### L2a — Route-table verifier

Cross-checks the `axum` router declared in
`crates/convergio-server/src/routes/` against the route lists in
the root `AGENTS.md` ("MCP tools available — useful HTTP routes")
and `ARCHITECTURE.md` ("request lifecycle"). Fails if any route
exists in code but not in docs, or vice versa. Implemented as a
small reflective walk over the `Router` build function, no
runtime daemon required.

#### L2b — ADR status vs implementation cross-check

For every ADR with `status: accepted`, asserts that
`implemented_in` is non-empty *or* the file has a `last_validated`
within the last 90 days. For `status: proposed` ADRs older than
60 days, surfaces a soft warning (still advisory). For
`status: superseded by NNNN`, asserts NNNN exists and itself does
not say `superseded by` to that same ADR (cycle detector).

Both verifiers live in the existing `convergio-cli coherence`
subcommand surface (no new crate). Output uses i18n bundles per
ADR-0005 / D7.

### L3 — Semantic sweep (LLM + embeddings, advisory)

- **Substrate**: ADR-0014 graph (per-repo Tier-3 retrieval) and
  ADR-0038 embedding layer (BGE-M3-small over `graph_vec_index`).
- **Diff path (PR-time)**: on every PR that touches `*.md` or
  `crates/*/src/**/*.rs`, the daemon runs a semantic sweep over the
  diff: for each changed ADR/README chunk, retrieve top-K nodes via
  ADR-0038 hybrid ranking, compose a small prompt
  ("does this prose still match the retrieved code?"), post the
  verdict as a **PR comment**, never as a status check. Advisory.
- **Full-repo path (nightly)**: a daemon plan (not a cron) walks the
  full ADR + README corpus, embeds each chunk, joins against
  `graph_node_embeddings` (ADR-0038 §5.2.3), and emits an
  `evidence` row of kind `doc_coherence_sweep` linked to the
  recurring plan id. Output is ranked drift candidates with
  cosine + provenance — never a green/red gate.
- **Why advisory**: D4. Probabilistic verdicts on prose are useful
  signal, not ground truth. A noisy gate teaches reviewers to ignore
  it, which is worse than no gate.

### Wiring: the sweep as a recurring plan

The sweep is itself a Convergio plan
(`ee9ab55a-eb28-41ee-bd02-e22d002e8b3c` — *Doc-coherence sweep +
wiki sync*). Each pass = one task transition with attached
evidence:

- L1 evidence: `kind = "auto_block_check"`, content = stdout of
  `cvg docs regenerate --check`.
- L2 evidence: `kind = "coherence_check"`, content =
  JSON verdict per verifier.
- L3 evidence: `kind = "doc_coherence_sweep"`, content =
  ranked drift candidates with cosine + provenance.

This means:

- **The sweep cannot silently skip.** A missed pass is a missing
  evidence row, which is itself drift the next run will flag.
- **The sweep is auditable.** Its own evidence flows through the
  same hash-chained audit log (ADR-0001) the leash applies to
  agent work.
- **The sweep is reversible.** Disabling L3 (semantic) is a
  feature-flag flip; L1 + L2 keep working unchanged.

### Positive consequences

- One verb (`cvg coherence`) with three explicit layers; nobody
  has to remember which command catches which drift mode.
- Probabilistic and deterministic concerns stay cleanly separated.
  CI breaks deterministically (L1 + L2). Semantic suggestions land
  as PR comments (L3). No flaky CI.
- The sweep is self-describing in its own audit trail. A new agent
  reading the recurring plan sees the last 30 sweeps' evidence rows
  and knows what is currently drifting.
- Reuses every substrate already accepted: ADR-0014 graph,
  ADR-0015 AUTO blocks, ADR-0038 embeddings. No new tier.

### Negative consequences

- L3 introduces an LLM call per ADR/README chunk on diff. Token
  cost is real; quantify under F1 of ADR-0038 before promoting L3
  beyond `proposed`.
- The recurring plan needs scheduler infrastructure that doesn't
  exist yet (today the daemon runs background loops, not scheduled
  plans — see root AGENTS.md "Background loops in the daemon").
  Cron via the daemon plan is a follow-up implementation task, not
  blocking this ADR.
- Three layers means three places where a doc claim can disagree
  with reality. Mitigated because each layer has a single owner and
  a clear contract; if a layer's signal is unreliable, it gets
  pulled, not patched.

## Pros and Cons of the Options (optional)

### Option 1 — Status quo
- 👍 Zero engineering cost.
- 👎 Does not catch semantic drift; relies on reviewer discipline that
  has empirically failed.

### Option 2 — Single mega-verifier
- 👍 One command, one mental model.
- 👎 Forces a binary block/pass on probabilistic output. Either CI
  becomes flaky or the gate becomes meaningless.

### Option 3 — Three-layer recurring sweep (chosen)
- 👍 Each layer has a clean block/advisory contract; reuses existing
  substrate; auditable through the same plan/evidence machinery the
  leash uses elsewhere.
- 👎 Three moving parts; semantic layer depends on ADR-0038 reaching
  F1 success.

### Option 4 — External LLM doc bot
- 👍 Zero engineering on Convergio side.
- 👎 Breaks local-first; cannot write into the audit chain; wrong
  shape of tool.

## Links (optional)

- ADR-0014 — Code-graph layer for Tier-3 context retrieval
- ADR-0015 — Documentation is derived state, not free text
- ADR-0038 — Fleet retrieval & cross-repo graph (semantic +
  multi-language)
- Convergio plan `ee9ab55a-eb28-41ee-bd02-e22d002e8b3c` —
  Doc-coherence sweep + wiki sync (the recurring plan this ADR
  describes)
- Root `AGENTS.md` § *Background loops in the daemon*
- `crates/convergio-cli/src/commands/coherence.rs` — current
  Tier-2 deterministic checks
