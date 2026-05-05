---
id: 0043
status: accepted
date: 2026-05-05
topics: [api, consistency, breaking-change, mcp]
related_adrs: [0009, 0024, 0025]
touches_crates: [convergio-server, convergio-api, convergio-mcp, convergio-cli]
last_validated: 2026-05-05
---

# 0043. API consistency — `id` and `payload` field naming

- Status: accepted
- Date: 2026-05-05
- Deciders: Roberto, claude-code-Roberdan (P1-4 of the 2026-05-04 retrospective fix plan)
- Tags: api, consistency, breaking-change

## Context and Problem Statement

The 2026-05-04 retrospective catalogued findings C2 and C3:

- **C2** — `register_agent` accepts `{ "id": "...", ... }` while
  `heartbeat_agent` accepts `{ "agent_id": "...", ... }`. Same
  entity, two field names. Every caller needs a lookup.
- **C3** — `publish_message` uses `{ "topic": ..., "payload": {...}}`
  but other routes in the daemon use `body` for the same role
  (notably planner and validator pass-throughs use `body`; bus uses
  `payload`).

Concrete evidence in code today:

| route | path | id field | content field |
|---|---|---|---|
| `POST /v1/agent-registry/agents` | register | `id` | (no body field) |
| `POST /v1/agent-registry/agents/:id/heartbeat` | heartbeat | path | `current_task_id` etc. |
| `POST /v1/plans/:plan_id/messages` | bus publish | path | `payload` |
| `POST /v1/system-messages` | system bus | — | `payload` |
| `POST /v1/audit/events` (proposed P2-2) | append | — | `payload` |

The MD also flags that some bus consumers use `body` in their JSON
bodies (legacy from earlier ADR-0023 drafts). Today's actual on-the-
wire shape is consistent with `payload`; the inconsistency is
documented intent vs. shipped reality.

## Decision Drivers

- **CONSTITUTION P1 — Zero tolerance.** A field name divergence is
  technical debt; every caller pays.
- **MCP bridge (`convergio-mcp`).** Each new field name doubles as a
  new edge in the action schema. The schema should not need to
  encode "this entity uses `id`, this uses `agent_id`".
- **Breaking change tolerance.** v0.3.x is pre-1.0; we are explicitly
  free to break wire-format consistency to fix it once.

## Considered Options

1. **Status quo.** *Reject. Every caller pays for the divergence.
   The MD called it out exactly because the cost is real.*

2. **Standardise on `<entity>_id` in path params and bodies (e.g.
   `agent_id`, `plan_id`).** *Verbose but unambiguous. Requires
   renaming the `id` field of register_agent — breaking change for
   one caller.*

3. **Standardise on `id` in entity bodies; keep `<entity>_id` only
   in cross-references** (chosen). The entity's own primary key is
   `id` whenever the route already names the entity in its path or
   surrounding type (e.g. `POST /v1/agent-registry/agents` body is
   `{"id": "...", ...}`). Foreign-key references in payloads use
   `<entity>_id` (e.g. an audit row's payload references `task_id`).
   This matches the existing `register_agent` shape; the breaking
   change is on `heartbeat`.

4. **Standardise on `payload` for free-form JSON content** (chosen).
   Every route that carries an opaque blob (bus, audit append, MCP
   action passthrough) uses `payload`. `body` becomes a layer-2
   concept (axum `Json<Body>` parameter binding) and never appears
   in the on-the-wire JSON.

5. **Use OpenAPI to enforce naming.** *Out of scope; tracked
   separately.*

## Decision Outcome

Chosen: **Options 3 + 4 together** — `id` for entity-self,
`<entity>_id` for foreign references, `payload` for opaque JSON.

### Concrete changes

| route | before | after |
|---|---|---|
| `POST /v1/agent-registry/agents/:id/heartbeat` body | `{"agent_id": "...", "current_task_id": "...", "status": "..."}` | `{"current_task_id": "...", "status": "..."}` (id is in path) |
| (any future route accepting both an entity primary id and FK refs) | mixed | `id` for self, `<entity>_id` for FK |
| Bus `payload` | already `payload` | unchanged |
| Validator/planner pass-throughs | `body` | `payload` |

### Migration

- **CLI**: `cvg agent` shim drops the `agent_id` redundancy in the
  heartbeat body (the path already carries it).
- **MCP** (`convergio-mcp`): the action schema is regenerated; the
  `heartbeat_agent` action signature loses the `agent_id` field on
  the body.
- **Server**: deserializer accepts the old `agent_id` body field for
  one minor release (0.3.x → 0.4.0) and emits a `tracing::warn` when
  it sees it. 0.4.0 removes the legacy path.

### Audit

The renaming itself does not change any audit row shape — payloads
already use `agent_id` for foreign-key fields. The route input shape
is the only thing that changes.

## Consequences

### Positive

- Every "what's the field name" lookup goes away. The convention is
  one rule (`id` for self, `<entity>_id` for FK, `payload` for opaque
  JSON) and applies everywhere.
- The MCP schema gets simpler: the `agents/:id/heartbeat` action no
  longer has a body with the same id as the path.
- Future routes onboard against a written contract.

### Negative

- One breaking change for `heartbeat_agent` callers. We mitigate
  with a one-release deprecation window + warning.
- Existing CLI/skills that send `{"agent_id": "..."}` as part of the
  heartbeat body must drop the field; the path already carries it.

### Neutral

- No DB schema change.
- No audit kind change.
- Touches `convergio-server` (route extractor), `convergio-api`
  (action schema), `convergio-mcp` (regenerate bridge), and
  `convergio-cli` (heartbeat CLI shim).

## Implementation plan (follow-up tasks)

Tracked as a separate task. Not part of this ADR's deliverable.

1. Tag the existing `agent_id` body field on heartbeat as
   `#[serde(alias = "agent_id")]` with a deprecation warning when
   the alias is used (0.3.x window).
2. Update `convergio-cli` heartbeat command to omit the field.
3. Update the MCP bridge action schema generator to match.
4. Update README + ARCHITECTURE.md examples.
5. Bump the major-minor version on removal (0.4.0).

Acceptance for the impl PR: a heartbeat call with the legacy field
emits a `tracing::warn`, while a call without it succeeds silently;
all existing tests pass.
