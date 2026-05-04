---
id: 0006
status: accepted
date: 2026-04-29
topics: [layer-0, layer-1, storage, crdt, sync]
related_adrs: []
touches_crates: [convergio-durability, convergio-server, convergio-api]
last_validated: 2026-05-04
---

# 0006. Model state with row and column CRDT metadata from day zero

- Status: accepted (shipped — `crdt_actors`/`crdt_ops`/`crdt_cells`/`crdt_row_clocks` in migration `0004_crdt_core.sql`, `/v1/crdt/import` + `/v1/crdt/conflicts` routes, action enum `import_crdt_ops`/`list_crdt_conflicts`)
- Date: 2026-04-29
- Deciders: Roberto, Copilot
- Tags: layer-0, layer-1, storage, crdt, sync

## Context and Problem Statement

Convergio is local-first today, but it must coordinate many agents even
on one machine and must not block future multi-machine synchronization.
If the core schema is classic last-writer-wins SQLite only, future sync
or organization support will require breaking migrations or lossy merge
behavior.

The goal is not to ship peer-to-peer sync in v0.1. The goal is to make
the storage model multi-actor from day zero: every field that can be
edited by multiple actors has an explicit merge policy, conflicts are
surfaced, and CRDT operations are auditable.

## Decision Drivers

- Parallel agents may update plans, tasks, evidence, and coordination
  metadata concurrently.
- Future multi-device or organization sync must not require replacing
  the v0.1 database model.
- Merge behavior must be deterministic and testable.
- Conflicts must not be hidden by generic last-writer-wins.
- CRDT merges that affect visible state must remain inside the audit
  boundary.

## Considered Options

1. **Classic SQLite rows only** — keep current tables and rely on
   transactions/updated_at.
2. **Make every field a generic CRDT blob** — store all state as CRDT
   documents.
3. **Declared CRDT fields with materialized SQL state** — keep queryable
   tables, but route mergeable writes through CRDT-aware helpers and an
   operation log.

## Decision Outcome

Chosen option: **Option 3**, because it preserves SQLite simplicity and
queryability while making multi-actor merge behavior explicit.

Every mergeable field must declare a CRDT type. Non-mergeable fields
remain ordinary SQL fields and are protected by domain transactions.

### Positive consequences

- Core state can later sync across machines without throwing away the
  schema.
- Agents can work concurrently on different fields/rows.
- Conflicts are first-class state and can block unsafe task completion.
- The audit log can prove which CRDT operations were accepted and merged.

### Negative consequences

- More schema and store complexity from day zero.
- All writers must use CRDT-aware helpers for declared fields.
- Tests must cover multi-actor merge cases, not only single-writer CRUD.

## Field policies

Convergio does not treat every column as the same CRDT. Field policy is
domain-specific:

| Field kind | Policy |
|------------|--------|
| operational scalar where overwrite is safe | LWW register |
| user-authored scalar text | MV-register |
| unordered collections | OR-set |
| evidence and audit | append-only event/set |
| task status | domain state machine |

Task status is not a generic LWW register. A stale actor cannot overwrite
`done` back to `in_progress`, and `submitted`/`done` transitions still
run gates.

## Actor and operation model

Each local installation has a stable `actor_id`. Every write gets a
per-actor monotonic counter. The operation identity is:

```text
(actor_id, counter)
```

Operations also record a hybrid logical clock for human-readable ordering
and debugging, but merge correctness must not depend on wall-clock time.

The local actor record is stored in the Convergio config directory and
mirrored in `crdt_actors`. It is generated once during setup or first
daemon start. If `~/.convergio` is deleted, the next setup creates a new
actor identity. Restored backups therefore keep their original actor ID;
running two restored copies concurrently is treated as a clone conflict
unless the operator explicitly rotates one actor ID and imports the old
ops as historical data.

## Schema foundation

The core schema will add tables equivalent to:

| Table | Purpose |
|-------|---------|
| `crdt_actors` | stable local and imported actor identities |
| `crdt_ops` | append-only CRDT operation log |
| `crdt_cells` | materialized per entity/field CRDT state |
| `crdt_row_clocks` | row-level summary clocks for efficient checks |

Existing tables such as `plans`, `tasks`, and `evidence` remain
materialized, query-friendly state. They are not the only source of
history for declared CRDT fields.

## Audit integration

Any imported or local CRDT operation that changes visible state must be
inside the audit boundary.

Required audit events:

- local CRDT op accepted;
- imported CRDT op accepted;
- CRDT merge produced visible materialized change;
- CRDT conflict created;
- CRDT conflict resolved.

The audit payload must include enough operation IDs and field metadata to
reconstruct why the materialized value changed.

Audit rows are local facts, not a distributed CRDT. Imported CRDT ops are
deduplicated by operation ID before audit append. A batch import produces
one `crdt.imported` audit row containing the sorted operation IDs and
batch digest, followed by deterministic `crdt.materialized` rows for
visible changes. Re-importing the same batch produces no new audit rows.

Future multi-machine sync must exchange CRDT ops and local audit proofs;
it must not attempt to merge independent audit hash chains into one
global chain without a new ADR.

## Conflict surfacing

MV-register conflicts and domain conflicts are persisted. They are not
silently resolved by timestamp.

Conflict UX requirements:

- `cvg doctor` reports unresolved CRDT conflicts;
- API/MCP responses can return a stable conflict code;
- unresolved conflicts can block `submitted`/`done` when they affect the
  task or evidence being completed;
- agents must receive a next-step hint instead of overwriting.

Concurrent task status transitions are handled by the task state machine,
not by a register merge. If two actors submit the same task concurrently,
each transition is validated against the same pre-state and evidence set.
At most one materialized transition may win automatically. Any transition
that depends on different evidence, different gates, or a stale pre-state
creates a domain conflict and must be retried against the current
materialized task.

## Retention and compaction

`crdt_ops` is append-only for correctness in v0.1. To avoid silent data
loss, v0.1 does not garbage-collect operations automatically.

The planned compaction model is snapshot-based:

- write a materialized snapshot with row clocks;
- audit the snapshot digest;
- retain all operations newer than the snapshot frontier;
- allow manual export before pruning older operations.

Automatic pruning requires a later ADR.

## v0.1 scope

v0.1 does not need network synchronization. It must include:

- local actor identity;
- CRDT operation persistence;
- deterministic import/merge of an operation batch;
- materialized state updates for declared fields;
- conflict persistence and reporting;
- tests for two actors.

## Required tests

- two actors edit different columns of the same row and merge cleanly;
- two actors edit the same MV-register field and create a conflict;
- task status merge obeys the domain state machine;
- imported operation batch is idempotent;
- audit verification passes after CRDT merge events.

## Links

- Related ADRs: [0002](0002-audit-hash-chain.md),
  [0003](0003-migration-coexistence.md)
