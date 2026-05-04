---
id: 0042
status: accepted
date: 2026-05-04
topics: [gates, durability, parallelism, multi-agent]
related_adrs: [0007, 0026]
touches_crates: [convergio-durability, convergio-server, convergio-api, convergio-cli]
last_validated: 2026-05-04
---

# 0042. Wave-sequence gate refactor — opt-in parallel waves via per-task `parallel_safe`

- Status: accepted
- Date: 2026-05-04
- Deciders: Roberto, claude-code-Roberdan (P0-5 of the 2026-05-04 retrospective fix plan)
- Tags: gates, durability, parallelism, multi-agent

## Context and Problem Statement

The 2026-05-04 retrospective (`~/Desktop/convergio-retrospective-2026-05-04.md`)
catalogued findings A7 / C7 / E1: every sub-agent in the F2 session reported
"gate refused, 7 task(s) in earlier waves still open" the moment it tried to
move its task to `in_progress`. The workaround that shipped was to skip
`in_progress` and transition straight to `submitted`, which both:

1. Defeats the gate's audit purpose (no `task.in_progress` row → progress is invisible to the dashboard, the reaper has nothing to time-out, the bus has no signal that work is underway).
2. Forces the gate-writer's intent to be expressed in the *bypass*, not the design.

The gate that produces the refusal lives at
`crates/convergio-durability/src/gates/wave_sequence_gate.rs` and is exactly
14 lines of SQL: refuse `in_progress` for any task whose wave > 1 if any
earlier-wave task is not in (`done`, `failed`).

The gate's intent is sound — it makes "Phase 1 ships before Phase 2 starts"
machine-checkable. But the unit "wave" is too coarse for a real engineering
plan, where wave-1 routinely contains a mix of:

- truly serial primitives (e.g. a migration that other tasks depend on),
- thin docs-only ADR drafts that touch nothing, and
- independent CLI subcommands that share no files.

Holding all three to the slowest task forces the parallel-PR workflow into
a serial pipeline, which is the *exact* failure mode this product exists
to prevent.

The retrospective MD lists three candidate fixes (§2 P0-5):
plan-level `--allow-parallel-waves`, per-task `parallel_safe`, or removing
the gate entirely in favour of dependency edges.

This ADR is the **deliverable**. Implementation is a follow-up under the
same task.

## Session failure rate (evidence)

From the 2026-05-04 retrospective MD, §0 finding C7 + §2 P0-5:

| Sub-agent | wave | in_progress refused? | bypass used |
|---|---|---|---|
| F2-04 .. F2-15 (12 sub-agents) | 1+ | yes (every one) | direct `pending → submitted` |
| Total wave-2+ sub-agents | 12 | 12 | 12 |
| Refusal rate | — | **100 %** | **100 %** |

A gate that fires on 100 % of legitimate work and is bypassed by 100 % of
agents is not a gate — it is friction with no signal value. The retrospective
also surfaced the second-order cost: the bypass produces tasks with no
`task.in_progress` audit row, which breaks `cvg coherence agents` heartbeat
matching (finding A11 / B3) and degrades the dashboard.

## Decision Drivers

- **CONSTITUTION P1 — Zero tolerance for technical debt.** A gate that exists
  only to be bypassed *is* technical debt; it lies about the agent's intent.
- **CONSTITUTION P4 — No scaffolding only.** The current gate is fully wired
  but its real-world signal is zero.
- **ADR-0007 § Multi-agent coordination.** Lease + patch coordination is what
  prevents parallel agents from clobbering each other; the wave gate is a
  weaker sibling that does not even consider file overlap.
- **ADR-0026 § Plan + wave + milestone vocabulary.** Defines waves as
  *coordinated promotion checkpoints*, not as serialisation primitives.
  Reading the gate against that vocabulary, the gate over-reaches.
- **Backwards compatibility.** Plans authored against the current rule must
  keep their behaviour unless the author opts in. No silent semantics flip.
- **Audit clarity.** Every transition must still emit one audit row. The fix
  must not introduce a "ghost" promotion path.

## Considered Options

1. **Plan-level `--allow-parallel-waves` flag.** Add one boolean column to
   `plans`; the gate passes when the flag is set. *Cheapest. But the
   granularity is wrong: a real plan rarely has the same parallel-safety
   property for every task. Today's retrospective plan, for example, has
   P0-7 (plan number — runs first, others depend on it for human reference),
   P0-1 (cvg pr merge — independent), P0-3 (reaper extension — independent),
   P0-4 (close-post-hoc verifier — independent). One flag forces an
   all-or-nothing decision the plan author cannot make honestly.*

2. **Per-task `parallel_safe` field (chosen).** Add a `parallel_safe` boolean
   column to `tasks` (default `false`, preserving today's semantics). The
   `WaveSequenceGate` permits the transition when `parallel_safe = true`.
   The plan author opts in per task at creation time; subsequent edits go
   through the existing `task.metadata_updated` audit row to keep the change
   visible. *Granular, audited, opt-in, no semantics flip for old plans.*

3. **Remove the gate entirely; rely on dependency edges.** Add a
   `dependencies` table; the gate reads the edges and refuses if any direct
   predecessor is not terminal. *Architecturally cleanest, but requires a
   new domain entity, a new migration, edge-management CLI/HTTP surface,
   topological-sort logic, and replacement of every existing wave-gate test.
   Order of magnitude more work than option 2 for a retrospective fix that
   needs to land this week.*

4. **Bump every retrospective task to wave 1.** *Sidesteps the gate but
   destroys the staging-by-wave grouping the operator uses to read the
   plan. Equivalent to surrender; rejected.*

5. **Loosen the gate to "at least one earlier-wave task terminal" instead of
   "all".** *Half-measure that still penalises the parallel case (a wave-2
   task with two wave-1 predecessors must wait for one) and creates a new
   class of subtle ordering bugs. Rejected.*

## Decision Outcome

Chosen option: **Option 2 — per-task `parallel_safe` field**.

### Schema change

New nullable column on `tasks`:

```sql
ALTER TABLE tasks ADD COLUMN parallel_safe INTEGER NOT NULL DEFAULT 0;
```

`INTEGER NOT NULL DEFAULT 0` keeps SQLite happy and matches the existing
`tasks.failed` boolean style; new migration in the durability crate's
range (next free version per ADR-0003).

### Gate change

`WaveSequenceGate::check` becomes (pseudocode):

```rust
if !matches!(target, InProgress) || task.wave <= 1 {
    return Ok(());
}
if task.parallel_safe {
    return Ok(());                  // ← new path
}
// existing predecessor count query, unchanged
```

The bypass is **only** for `target = in_progress`. The submitted/done
gates still run end-to-end, including wave-strict Thor validation. We
trade *start-time ordering* for *finish-time ordering* — exactly what
the parallel-PR workflow needs.

### API change

- `POST /v1/plans/:id/tasks` accepts an optional `parallel_safe: bool`
  field (default `false`).
- `cvg task create --parallel-safe` flag (already in idiom for `--wave`,
  `--sequence`).
- `GET /v1/tasks/:id` and `GET /v1/plans/:id/tasks` return the new field.
- No change to transition/evidence/audit shapes.

### Audit

A `parallel_safe = true` task that transitions to `in_progress` writes
the existing `task.in_progress` row. The payload gains `parallel_safe:
true` so audit-stream consumers see *which* tasks went around the gate
and *why*. No new audit kind.

### Coherence verifier

`cvg coherence` gains a sub-verifier (`coherence::wave_parallelism`)
that reports tasks where `parallel_safe = true` is set but the task has
no predecessor lineage that justifies it (e.g. wave > 1 without any
wave-1 task in scope). Advisory only at first; promote to `--strict`
once the field has a release in the wild.

### Why not option 3 *yet*

Dependency edges remain the right long-term direction (P2-?); they
subsume `parallel_safe`. This ADR explicitly defers that work and notes
the migration path: when edges land, `parallel_safe` becomes a derived
property (`true` ⟺ no incoming edges in earlier waves), and the column
can be dropped via a follow-up migration.

## Consequences

### Positive

- **Unblocks P0-1 today.** With `parallel_safe = true` on the six
  remaining wave-1 tasks of the retrospective plan (all docs/CLI/coherence
  changes that share no files), the wave-1 batch can submit + Thor-pass
  in parallel instead of in series.
- **Restores the gate's signal.** Tasks that legitimately need ordering
  keep the default `parallel_safe = false`; the gate fires only when the
  plan author chose serial. Bypasses go away because the legitimate path
  exists.
- **Audit-visible decision.** The `parallel_safe` flag rides on every
  audit row that touches the task — the choice is durable and queryable.
- **No breaking change for existing plans.** Default `false` preserves
  today's behaviour byte-for-byte.

### Negative / risks

- **Plan-author burden.** Every parallel-safe task now needs an explicit
  flag. We mitigate with the new `cvg coherence wave_parallelism`
  verifier and with plan-template defaults (a follow-up of P2-10
  templates with pre-populated `parallel_safe` per task category).
- **Field creep.** A second per-task boolean (after `parallel_safe`,
  what next?) is the start of a slippery slope toward dependency edges
  done the wrong way. Mitigation: this ADR is explicit that edges are
  the long-term direction; further per-task booleans require a fresh ADR.
- **Migration coordination.** The new column and the gate change must
  ship in the same release; partial deployments (gate sees the column,
  CLI does not yet send it) are safe because the default is the
  conservative one.

### Neutral

- Affects `convergio-durability` (gate + migration + store), `convergio-api`
  (action schema), `convergio-server` (route accepts the field),
  `convergio-cli` (`cvg task create --parallel-safe`). No agent-facing API
  ADR change beyond the new optional field.

## Implementation plan (follow-up tasks)

Tracked under a separate task on the retrospective plan; not part of this
ADR's deliverable.

1. **Migration**: new file under
   `crates/convergio-durability/migrations/` (next free version) adding
   the column with default `0`.
2. **Store**: extend `TaskRow` + `Task` model with `parallel_safe: bool`;
   thread through `TaskStore::create` and `TaskStore::list_*`.
3. **Gate**: update `WaveSequenceGate::check` per the snippet above; add
   tests covering both the default-blocking and `parallel_safe = true`
   permissive cases.
4. **API**: extend `convergio-api::actions::TaskCreate` with the new
   field; update MCP bridge.
5. **HTTP**: extend `convergio-server::routes::tasks::create_task` body.
6. **CLI**: add `--parallel-safe` to `cvg task create`.
7. **Coherence verifier**: new `wave_parallelism` sub-verifier under
   `convergio-coherence`.
8. **Dogfood**: mark the six remaining wave-1 tasks of the retrospective
   plan (`P0-2..P0-7` minus the one already submitted) with
   `parallel_safe = true`; verify Thor wave-1 validates incrementally.

Acceptance for the impl PR: a fresh `cvg task create --parallel-safe`
emits a `task.created` audit row with `parallel_safe: true` in payload,
the gate accepts an `in_progress` transition, and the existing
`tests/gates.rs` plus a new `wave_parallelism_gate_permits_when_flagged`
test all pass.

## Audit log evidence required for this ADR

- `evidence.added kind=context_pack` for task `cf35d6da-f4c7-4122-9fcd-02ddcc5e67d2`
- `evidence.added kind=semantic_query`
- `evidence.added kind=adr` (this file)
- `task.transitioned in_progress`, `task.transitioned submitted` for the same task
- `thor.validated` once wave-1 is collectively submitted (gated on this ADR's own implementation, classic chicken-and-egg the retrospective MD calls out)
