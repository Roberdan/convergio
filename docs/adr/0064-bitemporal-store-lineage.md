---
id: 0064
status: proposed
date: 2026-05-25
topics: [ontology, bitemporal, lineage, audit]
related_adrs: [0002, 0006, 0062, 0063]
touches_crates: [convergio-db, convergio-durability]
last_validated: 2026-05-25
---

# 0064. Bitemporal Store + Lineage over Ontology Objects

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, bitemporal, lineage

## Context

Once typed actions (ADR-0063) project effects onto ontology
objects, accelerators need to answer two questions that flat
state cannot:

1. **"What did we believe at time T?"** (system_time)
2. **"What was true in the world at time T?"** (valid_time)

These are the canonical bitemporal axes. Verticals that touch
regulated domains — academic records, clinical decisions, public
budget allocations — cannot ship without them, because audit and
correction-of-record workflows require reconstructing past
states *as known then*.

Lineage is the second half: given an ontology object's current
value, which actions, which inputs, which agents produced it?
The hash-chained `audit_log` (ADR-0002) already encodes most of
the raw evidence; this ADR makes it queryable as a graph.

## Decision

1. **Bitemporal columns on every ontology object.**
   - `valid_from`, `valid_to`, `system_from`, `system_to`
     (UTC, RFC 3339).
   - Mutations are append-only: an update closes the previous
     row (sets `system_to`) and inserts a new row.
   - Corrections-of-record are explicit: a `correction_of` link
     to the row being corrected, with a typed reason.
2. **Bitemporal query API.**
   - `GET /v1/ontology/objects/{id}?as_of=<system_time>&valid_at=<valid_time>`
   - CLI: `cvg ontology object show <id> --as-of ... --valid-at ...`
   - MCP: `ontology.object.snapshot` action with the same params.
3. **Lineage graph.**
   - Derived view over `audit_log` + `ontology_events`
     (ADR-0063) joined on action and effect hashes.
   - Query API: "for object X at system_time T, return the DAG
     of actions, agents, evidence rows, plans that produced
     it".
   - `cvg ontology lineage <object_id>` returns
     human-readable / JSON / dot-graph forms.
4. **Performance posture.**
   - Default index: `(object_id, system_to)` for "current"
     reads; secondary `(object_id, valid_to)` for "valid-at"
     reads.
   - Snapshot materialisation is opt-in per `ObjectType` via a
     `snapshot_cache: true` flag (extends ADR-0031 materialised
     timing cache posture).
5. **No retention policy in core.** Verticals decide what to
   prune (e.g. `convergio-edu` will not prune at all because of
   higher-ed retention rules; other verticals may).

## Decision Drivers

- Auditability beyond hash-chain: questions about *past
  beliefs* are not answerable from `audit_log` alone.
- Reversibility partner to ADR-0063: undo only makes sense
  against a known prior state.
- Local-first (P2): pure SQLite, no temporal extension required.

## Considered Options

1. **Single timestamp + snapshot on demand.** Rejected: cannot
   distinguish "we believed wrong" from "the world changed".
2. **Adopt an external bitemporal database (XTDB, Datomic).**
   Rejected: violates P2; introduces operational dependency.
3. **This proposal — bitemporal columns inside SQLite.**
   Accepted.

## Compliance Anchors

- P1 zero-debt: any append-only mutation must close the prior
  row in the same transaction; a gate refuses partial mutations.
- P2 local-first.
- P5 audit invariants: lineage rows reference but never
  duplicate `audit_log` rows; the hash chain stays canonical.

## Rollout

- W3 (plan *Ontology Platform W3: Bitemporal + Lineage + ER*):
  - Schema migration adding the four time columns + `correction_of`
    link.
  - Query API + CLI + MCP surfaces.
  - Lineage view + golden tests on the demo ontology.
  - Performance budget: a `valid_at` lookup on a 1M-row
    `ObjectType` returns p95 < 50 ms on the reference machine.

## Consequences

- Storage grows roughly linearly with mutation count.
  Verticals must size accordingly; a CLI command
  `cvg ontology size <type>` exposes the picture.
- New CI gate (later ADR): no action may mutate ontology rows
  without writing bitemporal closures.
- The lineage graph becomes a public artefact that verticals
  like `convergio-edu` can publish (with privacy redaction
  applied at the vertical layer).

## Alternatives left for verticals

- Privacy redaction over lineage (k-anonymity, DP) — vertical.
- Retention policies — vertical.

## References

- ADR-0002 hash-chained audit
- ADR-0006 CRDT actor/op store
- ADR-0031 materialised timing cache
- ADR-0062 ontology runtime core
- ADR-0063 typed actions framework
