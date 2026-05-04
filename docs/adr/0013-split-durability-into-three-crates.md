---
id: 0013
status: proposed
date: 2026-04-30
topics: [layer-1, refactor, legibility]
related_adrs: [0001, 0002, 0006, 0007, 0008]
touches_crates: [convergio-durability, convergio-server, convergio-api]
last_validated: 2026-04-30
---

# 0013. Split convergio-durability along three seams

- Status: proposed
- Date: 2026-04-30
- Deciders: Roberto, claude-code-roberdan (overnight wave)
- Tags: layer-1, refactor, legibility

## Context and Problem Statement

`convergio-durability` shipped at v0.1.x with one crate covering five
distinct concerns: plan/task/evidence persistence, the hash-chained
audit log, the gate pipeline, the workspace coordination layer
(leases + CRDT + patches), and the local capability registry. As of
2026-04-30 the crate is **8296 LOC** across 33 files; the per-crate
soft-warn cap is 5000 LOC. The legibility audit flagged it as the
dominant headroom risk and `T2.05` queues a split.

The problem is not just LOC: it is conceptual. An agent reading the
crate cold cannot tell which file owns which invariant, because the
crate's `lib.rs` is a flat `pub mod` list and the public facade
(`Durability`) collapses everything into one struct.

## Decision Drivers

- **Legibility first** (CONSTITUTION § 16). Each crate should fit in
  the agent context budget (≤ 5000 LOC) without hand-waving.
- **Boundary clarity.** ADR-0002 (audit) and ADR-0007 (workspace) are
  already separate decisions; the crate layout should mirror them.
- **No regression.** The HTTP routing in `convergio-server` and the
  agent action contract in `convergio-api` already speak `Durability`;
  the split must keep the surface stable.
- **Independent test surfaces.** Today every audit-tamper test
  rebuilds the whole crate, including workspace + capability code it
  does not touch.
- **No premature abstraction.** Resist the urge to introduce a trait
  per seam if a plain Rust module dependency suffices.

## Considered Options

1. **Status quo + file-level discipline.** Keep one crate; rely on
   the per-file 300-line cap to drive splits within it. *Cheapest,
   but does not address the agent context cost.*
2. **Three-crate split (recommended).**
   - `convergio-audit` — hash chain, gates, evidence canonicalization.
   - `convergio-state` — plans, tasks, evidence, agents (rename of the
     residual crate; keeps the name `convergio-durability` to avoid
     a public-API rename, with `convergio-state` as an aspirational
     v0.4 rename).
   - `convergio-coordination` — workspace leases, CRDT merges,
     capability registry.
3. **Two-crate split.** Audit + state vs coordination. *Less
   disruptive, but leaves the audit/gates pair tangled with the rest
   of state — defeats the legibility win we are after.*
4. **Workspace-level extraction with lib re-exports.** Split into
   three crates but re-export everything from `convergio-durability`
   so callers see no change. *Adds a layer of indirection that buys
   nothing past the migration.*

## Decision Outcome

Chosen option: **Option 2 — three-crate split**, because the audit /
state / coordination boundaries are real (each owns its own
migrations, its own ADR, and its own test surface) and the
five-figure LOC concentration is the single largest legibility risk
on the v0.2 → v0.3 path.

The split lands in three PRs over one wave to keep each diff
reviewable.

### Implementation status (2026-05-04)

Stays `proposed`. Internal facade modules
(`workspace_facade.rs`, `crdt_facade.rs`, `capability_facade.rs`,
`agent_facade.rs`, `facade.rs`, `facade_admin.rs`,
`facade_transitions.rs`, `facade_plan_transitions.rs`,
`facade_retry.rs`) carve out the three intended seams *inside the
single `convergio-durability` crate*, but the three-crate split
(`convergio-audit` / `convergio-state` / `convergio-coordination`)
described in PR 13.1–13.3 has not been performed. The decision still
holds; the file-level boundaries are now legible enough that the
crate-level split can land later without further design churn.

### Target topology

```
convergio-audit       (~1800 LOC)   audit::{log, model, hash, canonical} + gates
convergio-durability  (~3200 LOC)   stores: plans, tasks, evidence, agents, reaper, model
convergio-coordination(~2400 LOC)   workspace + crdt + capability stores + signature
```

`convergio-server` depends on all three; `convergio-cli` already
depends on none directly. Migration files move with their owner
crate, keeping the per-crate version-range convention from ADR-0003
intact.

### Dependency direction

```
convergio-audit ─┬─< convergio-durability ─< convergio-server
                  └─< convergio-coordination ─^
```

Audit has no dependents inside the trio. Durability and Coordination
both depend on it (gates need to write audit rows). Coordination
does NOT depend on Durability, and vice versa — they share types
through `convergio-api` already.

### Positive consequences

- Each crate fits the 5000-LOC soft cap with headroom.
- An agent investigating the audit chain (ADR-0002) reads ~1800 LOC
  end-to-end; an agent fixing a CRDT merge bug reads ~2400 LOC.
- Independent compile units → faster incremental builds for changes
  scoped to one seam.
- Test isolation: `convergio-audit` tests no longer pull in workspace
  schema migrations.

### Negative consequences

- Three new `Cargo.toml` files, three new `AGENTS.md`, three new
  migration ranges to manage (we will pick `400-499` for audit,
  `500-599` for coordination; durability keeps `100-399`).
- One transient PR diff is large — tracked with a wave PR per crate.
- Public re-exports from `convergio-durability` retained for one
  release cycle to avoid breaking the agent action contract; the
  re-exports get removed in v0.4.

## Migration plan

Three PRs, each with its own scope and acceptance:

### PR 13.1 — extract `convergio-audit`

- Move `src/audit/`, `src/gates/`, evidence canonicalization helpers.
- New crate `convergio-audit` with `migrations/0400_*` (`audit_log`,
  evidence-debt markers).
- `convergio-durability` re-exports `pub use convergio_audit::*` for
  backwards compat.
- Tests moving with the crate: `audit_tamper`, `gates`, `no_debt_gate`,
  `no_debt_gate_multilang`, `zero_warnings_gate`, `no_secrets_gate`,
  `no_stub_gate`. ~46 tests.
- Acceptance: `cargo test -p convergio-audit` runs the moved suite;
  `cargo test --workspace` count is unchanged.

### PR 13.2 — extract `convergio-coordination`

- Move `src/store/{workspace,workspace_rows,workspace_patch,workspace_merge,crdt,crdt_merge,crdt_merge_types,capabilities}.rs`,
  `capability_facade.rs`, `capability_signature.rs`, `crdt_facade.rs`,
  `workspace_facade.rs`.
- New crate `convergio-coordination` with `migrations/0500_*`.
- Re-exports kept in `convergio-durability`.
- Tests moving: `crdt_merge`, `workspace_*` tests.
- Acceptance: same as 13.1; existing E2E tests in `convergio-server`
  still pass.

### PR 13.3 — drop the re-exports

- After v0.3 ships and external callers (CLI, MCP) update imports,
  remove `pub use` re-exports in `convergio-durability/src/lib.rs`.
- Update `AGENTS.md` and the AGENT.md cross-references.

## Pros and Cons of the Options

### Option 2 (chosen)

- 👍 Three crates each fit in the agent context budget.
- 👍 Mirrors the existing ADR boundaries (0002 audit, 0007
  coordination) — the code now matches the decisions.
- 👍 Independent migration ranges enforce no schema cross-talk.
- 👎 Three sequential PRs are coordination overhead; mitigated by
  keeping re-exports during the transition.

### Option 1 (rejected)

- 👍 Zero migration cost.
- 👎 Does nothing for the per-crate 8000-LOC concentration.

### Option 3 (rejected)

- 👍 One PR instead of three.
- 👎 The audit + gates pair stays glued to plan/task/evidence state,
  exactly the entanglement we are trying to undo.

## Links

- Plan task: T2.05 (`cba9d4d1` in plan
  `8cb75264-8c89-4bf7-b98d-44408b30a8ae`).
- Spec: this ADR + the legibility-audit baseline of 81/100.
- Related ADRs: 0001 (four-layer architecture), 0002 (audit chain),
  0003 (per-crate migration ranges), 0006 (CRDT storage), 0007
  (workspace coordination), 0008 (downloadable capabilities).
- Open question: do we move the `reaper` loop into `convergio-audit`
  (it writes `task.reaped` audit rows) or keep it with `state`? Lean
  toward keeping it with state — the reaper owns task lifecycle,
  audit is a write target. To resolve in PR 13.1.
