---
id: 0044
status: accepted
date: 2026-05-05
topics: [agents, contract, compliance, coherence, registry, bus, graph, embed, thor, audit]
related_adrs: [0009, 0014, 0023, 0024, 0038, 0039, 0040, 0042]
touches_crates: [convergio-cli, convergio-coherence]
last_validated: 2026-05-05
---

# 0044. Plan execution contract — required mechanism utilization per task

- Status: accepted
- Date: 2026-05-05
- Deciders: Roberto D'Angelo, claude-sonnet-p0-0 (NORTH STAR P0-0 of the 2026-05-04 retrospective fix plan)
- Tags: agents, contract, compliance, coherence

## Context and Problem Statement

The 2026-05-04 retrospective (plan `ed6ceb9b`) found that agents are instrumenting
Convergio's mechanisms inconsistently: some tasks used the code graph, others did not;
some agents skipped bus announcements; semantic search was never used after the vector
store was bootstrapped. There is no machine-readable contract that says which mechanisms
**must** be exercised and which are optional for a given task type.

Without a contract, compliance is invisible and `cvg coherence` cannot score a plan.

The three concrete outcomes of this ADR:

1. **Install correctness** — `cvg setup self-check` exits 0 on a correctly configured
   system and non-zero on missing/mismatched components.
2. **Contract table** — a machine-readable definition of which evidence kinds each
   task type requires.
3. **Plan verifier** — `cvg coherence plan-execution <plan-id>` scores compliance
   for every closed task in a plan against this table.

## Decision Drivers

| # | Driver |
|---|--------|
| D1 | NORTH STAR invariant: every evidence kind required by contract must be present on every closed task |
| D2 | The contract must be machine-readable so `cvg coherence plan-execution` can score it |
| D3 | Requirements must vary by task type — code tasks differ from ADR-only tasks |
| D4 | The self-check must be runnable before a task starts (`cvg setup self-check` exits 0) |

## Mechanism table

Eight mechanisms are defined. Each row gives:
- **Mechanism** — stable identifier used in `cvg coherence plan-execution` scoring
- **How checked** — what evidence or daemon state the verifier inspects
- **Evidence kind** — task-evidence kind attached by the agent (`—` = daemon-state check only)

| # | Mechanism | How checked | Evidence kind |
|---|-----------|-------------|---------------|
| M1 | `registry` | `GET /v1/agent-registry/agents` — at least one agent with a recent heartbeat | — |
| M2 | `bus` | bus has messages from a non-system agent for this plan | — |
| M3 | `sub_agent_spawn` | evidence kind `spawn_record` present on task | `spawn_record` |
| M4 | `graph_context` | evidence kind `context_pack` present on task | `context_pack` |
| M5 | `semantic_query` | evidence kind `semantic_query` present on task | `semantic_query` |
| M6 | `thor` | task transitioned through `submitted` state | — |
| M7 | `loops` | `GET /v1/health` returns `ok: true` — loops are daemon-internal | — |
| M8 | `audit` | any evidence present implies audit events were written | — |

## Task-type contract table

Task types are inferred from evidence present on the task. The required/optional
classification determines whether `cvg coherence plan-execution --strict` exits non-zero.

| Task type | Inferred when | M4 graph | M5 embed | M6 thor | evidence: ci_run | evidence: merge_record |
|-----------|---------------|----------|----------|---------|------------------|------------------------|
| `code` | `code` or `merge_record` evidence present | REQUIRED | optional | implied by submitted | REQUIRED | REQUIRED |
| `doc_only` | `adr` evidence present, no `code`/`merge_record` | optional | optional | implied by submitted | REQUIRED | REQUIRED |
| `analysis` | no `code`, `merge_record`, or `adr` evidence | optional | optional | implied by submitted | optional | optional |

Notes:
- M3 `sub_agent_spawn` is always optional — not every task needs sub-agents.
- M5 `semantic_query` is optional for all task types until the vector store is bootstrapped
  (`GET /v1/embed/stats` returns `count > 0`).
- M7 `loops` is a daemon-level invariant: if the daemon is healthy, loops are running.
- M1 `registry` and M2 `bus` are plan-level checks, not per-task.
- M6 `thor` is implied by the task having been in `submitted` state, which is how
  all done tasks got there; the verifier does not re-check this per-task.

## Install correctness (`cvg setup self-check`)

The following table defines what `cvg setup self-check` verifies and the exit code
semantics. FAIL checks must all pass for exit 0; WARN checks produce advisory output
but do not fail the command.

| Check | Source | Severity |
|-------|--------|----------|
| `daemon_up` | `GET /v1/health` returns 200 | FAIL |
| `version_match` | CLI `CARGO_PKG_VERSION` == `health.version` | FAIL |
| `loops_running` | `GET /v1/health` returns `ok: true` | FAIL |
| `mcp_registered` | `convergio-mcp` in `.mcp.json` or `~/.claude/settings.json` | WARN |
| `fleet_bootstrap` | `~/.convergio/v3/fleet.toml` exists and has ≥ 1 `[[repo]]` entry | WARN |
| `embed_nonempty` | `GET /v1/embed/stats` returns `count > 0` | WARN |
| `registry_active` | `GET /v1/agent-registry/agents` returns ≥ 1 agent | WARN |

## Consequences

- `cvg coherence plan-execution <plan-id>` can score any plan by checking evidence kinds
  against this contract table.
- `cvg coherence plan-execution --strict` fails when any required mechanism is missing
  on a closed task or when registry/bus plan-level checks fail.
- `cvg setup self-check` is the canonical gate before starting a task.
- Future task types (e.g. `infra`, `test_only`) can be added by extending the contract table.
- Evidence kind `spawn_record` is defined here; agents that spawn sub-agents should attach it.
- The verifier is backwards-compatible: tasks that pre-date this ADR score as `analysis`
  (no required evidence), so they never cause spurious failures.
