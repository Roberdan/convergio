# Architecture

Convergio is a local-first runtime. One daemon runs on the user's
machine, persists state in one SQLite database, and exposes a localhost
HTTP API for agents and the `cvg` CLI.

## The layers

```text
┌──────────────────────────────────────────────────────────────┐
│  Layer 4 — Reference Local Flow                              │
│  planner · thor validator · executor · CLI: cvg              │
├──────────────────────────────────────────────────────────────┤
│  Layer 3 — Agent Lifecycle                                   │
│  spawn · heartbeat · process watcher · agent registry        │
├──────────────────────────────────────────────────────────────┤
│  Layer 2 — Agent Message Bus                                 │
│  publish · poll by cursor · ack · scoped per plan            │
├──────────────────────────────────────────────────────────────┤
│  Layer 1 — Durability Core + multi-actor coordination        │
│  plans · tasks · evidence · audit_log · gates · reaper       │
│  CRDT actor/op store · workspace leases · patch proposals    │
│  merge arbiter · capability registry (Ed25519-signed)        │
├──────────────────────────────────────────────────────────────┤
│  convergio-db                                                │
│  SQLite pool + per-crate migrations                          │
└──────────────────────────────────────────────────────────────┘
```

Lower layers do not depend on higher layers. A custom client can use
Layer 1-3 directly and ignore the reference Layer 4 crates.

## Crate map

| Crate | Layer | Public surface | Owns DB tables? |
|-------|-------|----------------|-----------------|
| `convergio-db` | 0 | `Pool`, `Backend` | no |
| `convergio-durability` | 1 | `Durability`, stores, audit, gates, reaper, CRDT, workspace, capabilities | yes (`plans`, `tasks`, `evidence`, `audit_log`, `crdt_*`, `workspace_*`, `agent_registry`, `capabilities`) |
| `convergio-bus` | 2 | `Bus::publish`, `Bus::poll`, `Bus::ack` | yes (`agent_messages`) |
| `convergio-lifecycle` | 3 | `Supervisor::spawn`, `heartbeat`, `mark_exited`, `get`, watcher | yes (`agent_processes`) |
| `convergio-server` | shell | `router(state)`, `AppState`, `convergio start` | no |
| `convergio-cli` | 4 | `cvg` binary | no |
| `convergio-tui` | 4 | `convergio_tui::run` — `cvg dash` TUI dashboard (read-only HTTP viewer, ADR-0029) | no |
| `convergio-planner` | 4 | `Planner::solve` | no |
| `convergio-thor` | 4 | `Thor::validate` -> `Verdict` (and on Pass, promotes `submitted` to `done` per ADR-0011) | no |
| `convergio-executor` | 4 | `Executor::tick`, `spawn_loop` | no |
| `convergio-graph` | 1 | Tier-3 code-graph (`cvg graph build / for-task / drift / cluster`, ADR-0014) | yes (`graph_*`) |
| `convergio-embed` | 1 | embeddings store + pluggable embedder trait (ADR-0038, F1) | yes (`embeddings_*`) |
| `convergio-runner` | 4 | vendor-CLI runners (shell / claude / copilot + custom TOML, ADR-0028, 0032, 0035) | no |
| `convergio-api` | cross-cutting | typed agent action contract (`Action`, `SCHEMA_VERSION`) | no |
| `convergio-mcp` | cross-cutting | stdio MCP bridge (`convergio.help`, `convergio.act`) | no |
| `convergio-i18n` | cross-cutting | Fluent bundles (`en`, `it`) + coverage gate | no |
| `convergio-coherence` | cross-cutting | doc/code coherence verifiers (`cvg coherence`) | no |
| `convergio-cli-session` | cross-cutting | session lifecycle (`cvg session`) | no |
| `convergio-brand` | cross-cutting | palette, claim, banner, boot animation (ADR-0037) | no |

## HTTP surface

All endpoints sit under `/v1`. Errors are:

```json
{ "error": { "code": "not_found", "message": "..." } }
```

| Status | Code | When |
|--------|------|------|
| 403 | `done_not_by_thor` | agent attempted `target=done` (ADR-0011) |
| 404 | `not_found` | missing plan / task / evidence / message / agent / capability |
| 409 | `gate_refused` | a gate refused a task transition |
| 409 | `not_submitted` | tried to promote a task that is not in `submitted` |
| 409 | `workspace_lease_conflict` | resource already leased by a live agent |
| 409 | `workspace_patch_refused` | patch proposal violates workspace policy |
| 409 | `workspace_merge_refused` | merge arbiter refused queued patch |
| 422 | `spawn_failed` / `spawn_timed_out` | Layer 3 could not execute or durably record the requested process |
| 422 | `invalid_workspace_lease` / `invalid_agent` / `invalid_capability` | malformed input |
| 500 | `audit_broken` / `invalid_timestamp` / `lifecycle_data_error` / `internal` | server-side fault or invalid persisted data |

### Endpoints

| Method | Path | Layer |
|--------|------|-------|
| GET | `/v1/health` | shell |
| GET | `/v1/status` | shell |
| POST · GET | `/v1/plans` | 1 |
| GET · PATCH | `/v1/plans/:id` | 1 |
| POST | `/v1/plans/:id/transition` (ADR-0026) | 1 |
| GET | `/v1/plans/:id/triage` | 1 |
| POST · GET | `/v1/plans/:plan_id/tasks` | 1 |
| GET | `/v1/tasks/:id` | 1 |
| POST | `/v1/tasks/:id/transition` | 1 |
| POST | `/v1/tasks/:id/retry` | 1 |
| POST | `/v1/tasks/:id/close-post-hoc` (ADR-0026) | 1 |
| POST | `/v1/tasks/:id/heartbeat` | 1 |
| POST · GET | `/v1/tasks/:id/evidence` | 1 |
| DELETE | `/v1/evidence/:id` | 1 |
| POST | `/v1/tasks/:id/context` | 1 |
| GET | `/v1/audit/verify` | 1 |
| GET | `/v1/audit/refusals/latest` | 1 |
| GET | `/v1/audit/events` | 1 |
| GET | `/v1/audit/stream` (SSE, P1.1) | 1 |
| POST | `/v1/audit/append` (P2-2, ADR-0002 § Custom kinds) | 1 |
| GET | `/v1/crdt/conflicts` | 1 |
| POST | `/v1/crdt/import` | 1 |
| GET · POST | `/v1/workspace/leases` | 1 |
| POST | `/v1/workspace/leases/:id/release` | 1 |
| POST | `/v1/workspace/patches` | 1 |
| POST | `/v1/workspace/patches/:id/enqueue` | 1 |
| POST | `/v1/workspace/merge/next` | 1 |
| GET | `/v1/workspace/merge-queue` | 1 |
| GET | `/v1/workspace/conflicts` | 1 |
| GET · POST | `/v1/agent-registry/agents` | 1 |
| GET | `/v1/agent-registry/agents/:id` | 1 |
| POST | `/v1/agent-registry/agents/:id/heartbeat` | 1 |
| POST | `/v1/agent-registry/agents/:id/retire` | 1 |
| GET | `/v1/capabilities` | 1 |
| POST | `/v1/capabilities/install-file` | 1 |
| POST | `/v1/capabilities/verify-signature` | 1 |
| GET · DELETE | `/v1/capabilities/:name` | 1 |
| POST | `/v1/capabilities/:name/disable` | 1 |
| POST · GET | `/v1/graph/build` · `/v1/graph/stats` (ADR-0014) | 1 |
| POST | `/v1/graph/refresh` | 1 |
| GET | `/v1/graph/for-task/:id` | 1 |
| GET | `/v1/graph/drift` | 1 |
| GET | `/v1/graph/cluster/:crate_name` | 1 |
| GET | `/v1/embed/stats` (ADR-0038, F1) | 1 |
| POST · GET | `/v1/plans/:plan_id/messages` | 2 |
| GET | `/v1/plans/:plan_id/messages/tail` | 2 |
| GET | `/v1/plans/:plan_id/topics` | 2 |
| POST | `/v1/messages/:id/ack` | 2 |
| GET · POST | `/v1/system-messages` (ADR-0025) | 2 |
| POST | `/v1/agents/spawn` | 3 |
| POST | `/v1/agents/spawn-runner` (ADR-0028) | 3 |
| GET | `/v1/agents/:id` | 3 |
| POST | `/v1/agents/:id/heartbeat` | 3 |
| POST | `/v1/solve` | 4 |
| POST | `/v1/capabilities/planner/solve` (ADR-0036) | 4 |
| POST | `/v1/dispatch` | 4 |
| POST | `/v1/plans/:id/validate` | 4 |

## Request lifecycle: task transition

```text
client POST /v1/tasks/:id/transition
   |
   v
convergio-server handler
   |
   v
convergio-durability::Durability::transition_task
   |
   |- if target = done: refused, returns 403 done_not_by_thor (ADR-0011)
   |- otherwise runs gate pipeline
   |- on gate refusal: HTTP 409 gate_refused + audit row task.refused
   |
   v
updates task state and appends one audit row
   |
   v
response: 200 Task
```

Promoting `submitted` to `done` runs through a different path:

```text
client POST /v1/plans/:id/validate
   |
   v
convergio-thor::Thor::validate
   |
   |- inspects every task in the plan
   |- verifies required evidence kinds are present
   |
   |- on Fail: returns Verdict::Fail with reasons (no state change)
   |- on Pass: calls Durability::complete_validated_tasks(submitted_ids)
   |             which writes one task.completed_by_thor audit row
   |             per promoted task, atomically
   |
   v
response: 200 Verdict
```

## Local runtime

The daemon defaults to:

```text
database: sqlite://$HOME/.convergio/v3/state.db?mode=rwc
bind:     127.0.0.1:8420
```

Configuration:

| Variable / flag | Default | Notes |
|-----------------|---------|-------|
| `CONVERGIO_DB` / `--db` | local SQLite file | must be `sqlite://...` |
| `CONVERGIO_BIND` / `--bind` | `127.0.0.1:8420` | keep localhost for the local security model |
| `CONVERGIO_LOG` | `info` | tracing filter |
| `CONVERGIO_THOR_PIPELINE_CMD` | unset | trusted-local only; Thor runs it through `sh -c` before `submitted -> done` promotion |

## Audit hash chain

Every audited row contains:

```text
seq, payload, prev_hash, hash
```

where:

```text
hash = sha256(prev_hash || canonical_json(payload))
```

`GET /v1/audit/verify` recomputes the chain. Run open-ended
verification for the strongest local tamper-evidence guarantee.

Audit kinds:

- `plan.created`
- `task.created` · `task.in_progress` · `task.submitted` · `task.failed` · `task.pending`
- `task.refused` (gate refusal OR ADR-0011 done refusal)
- `task.completed_by_thor` (Thor-driven submitted -> done; ADR-0011)
- `task.reaped` (reaper released a stale `in_progress`)
- `evidence.attached`
- workspace / CRDT / capability events

## Migration coexistence

Each crate owns its own migration files and shares the same SQLite
`_sqlx_migrations` bookkeeping table by version range:

| Crate | Range |
|-------|-------|
| `convergio-durability` | 1-100 |
| `convergio-bus` | 101-200 |
| `convergio-lifecycle` | 201-300 |

Every migrator uses `set_ignore_missing(true)` so independent crates can
coexist in the same local database file (ADR-0003).

## Background loops

- **Reaper** — `convergio_durability::reaper::spawn`. Releases stale
  `in_progress` tasks back to `pending` and writes `task.reaped` audit
  rows.
- **Watcher** — `convergio_lifecycle::watcher::spawn`. Polls tracked
  process rows and flips dead PIDs to `exited`. PID liveness probing is
  implemented with POSIX `kill -0`; on Windows the watcher intentionally
  treats rows as still running until a platform-specific probe is added.
- **Executor** — `convergio_executor::spawn_loop`. Picks wave-ready
  pending tasks and spawns the configured local agent template. The
  manual `POST /v1/dispatch` endpoint remains available as an operator
  override and test seam.

Knobs: `CONVERGIO_REAPER_TICK_SECS`,
`CONVERGIO_REAPER_TIMEOUT_SECS`, `CONVERGIO_WATCHER_TICK_SECS`, and
`CONVERGIO_EXECUTOR_TICK_SECS`.

## Where to look

- HTTP routes -> `crates/convergio-server/src/routes/`
- Schema -> `crates/<crate>/migrations/`
- Gates -> `crates/convergio-durability/src/gates/`
- Audit -> `crates/convergio-durability/src/audit/`
- Reaper -> `crates/convergio-durability/src/reaper.rs`
- Bus -> `crates/convergio-bus/src/bus.rs`
- Supervisor -> `crates/convergio-lifecycle/src/supervisor.rs`
- Thor (validator + done promotion) -> `crates/convergio-thor/src/thor.rs`
- CLI -> `crates/convergio-cli/src/commands/`
- E2E tests -> `crates/convergio-server/tests/e2e_*.rs`
- Agent contract (MCP / future ACP) -> `crates/convergio-api/src/`
