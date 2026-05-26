---
id: 0067
status: proposed
date: 2026-05-25
topics: [ontology, branching, what-if, crdt, workshop]
related_adrs: [0006, 0007, 0051, 0052, 0053]
touches_crates: [convergio-durability]
last_validated: 2026-05-25
---

# 0067. Scenario Branching (Workshop primitives)

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, branching, what-if

## Context

The "Workshop" experience that vertical accelerators want to
offer — exemplified by `convergio-edu` ADR-0020 layer 5 — is
fundamentally: *take a slice of the ontology, fork it, let an
agent or a human apply hypothetical typed actions, compare to
mainline, merge selectively or discard*.

Convergio already has the right substrate (CRDT actor/op store
ADR-0006 + workspace leases ADR-0007). What it lacks is the
ontology-aware façade: branches whose unit is `ObjectType` /
`ObjectId`, not file paths.

## Decision

Introduce **scenario branches** as a first-class ontology
primitive:

1. **Branch = named CRDT overlay** scoped to a subset of the
   ontology declared as `{object_ids[], object_types[]}`.
   Branches are created via `cvg ontology branch create` and
   linked to a `plan` for audit.
2. **Branch isolation.**
   - Reads inside the branch see branch-local mutations on top
     of the mainline snapshot at branch-creation time.
   - Reads outside the branch are unaffected.
   - Bitemporal queries (ADR-0064) continue to work inside the
     branch with branch-local `system_time`.
3. **Typed actions inside a branch.**
   - All typed actions (ADR-0063) run identically inside a
     branch; effects land on the overlay.
   - Side-effects flagged `irreversible: true` are refused
     inside branches by default (a branch may opt-in for a
     specific irreversible action with a capability whose
     purpose explicitly covers "scenario simulation").
4. **Diff and merge.**
   - `cvg ontology branch diff <name>` returns a structured
     delta (created/modified/linked/unlinked objects).
   - `cvg ontology branch merge <name> --strategy ...` produces
     a merge plan as a `plan` in the daemon, which goes
     through gates + Thor (ADR-0011) like any other plan.
   - Conflicts are reported per-object with the CRDT
     mechanism's existing arbiter.
5. **Lifecycle.**
   - Branches expire by default after N days (configurable per
     plan) and the reaper closes stale branches; mutations are
     never silently discarded — closure converts the branch
     into an archived diff for later inspection.

## Decision Drivers

- Workshop without branching is a footgun: agents propose
  hypothetical edits that accidentally land on production
  state. Branching is the safety belt.
- The CRDT substrate already exists; this is a typed façade,
  not new storage.
- Plans / gates / Thor remain the gatekeepers of merges to
  mainline.

## Considered Options

1. **Shadow tables per scenario.** Rejected — duplicates
   storage, breaks bitemporal queries.
2. **Read-only what-if (no mutations stored).** Rejected — too
   weak: agents need to chain hypothetical actions.
3. **This proposal — typed CRDT overlays.** Accepted.

## Compliance Anchors

- P1 zero-debt.
- P2 local-first.
- ADR-0007 workspace lease semantics extended to branches.
- ADR-0011 Thor-only `done`: a branch merge is a plan; Thor
  validates.

## Rollout

- W4 (plan *Ontology Platform W4: Workshop + Connector SDK*):
  - Branch creation / list / drop CLI + MCP.
  - Overlay read path with bitemporal compatibility tests.
  - Diff + merge plan generator.
  - Default expiry + reaper hook.
  - Golden tests: create branch, apply 10 actions, diff,
    merge subset, verify mainline state.

## Consequences

- Vertical Workshop UX (e.g. `convergio-edu`) becomes a thin
  client over generic branch primitives.
- Storage profile gains a configurable "active branches" cap
  exposed via the existing capacity surface.
- New gate (later): refuse a branch merge whose typed actions
  carry preconditions that no longer hold on current mainline.

## Alternatives left for verticals

- Branch UX (cards, side-by-side diff visualisers) lives in
  the vertical frontend; core ships data + CLI only.

## References

- ADR-0006 CRDT actor/op store
- ADR-0007 workspace leases
- ADR-0011 Thor sole authority for `done`
- ADR-0062 ontology runtime core
- ADR-0063 typed actions framework
- ADR-0064 bitemporal + lineage
