---
id: 0034
status: accepted
date: 2026-05-02
topics: [runners, executor, planner, schema, agents]
related_adrs: [0028, 0032, 0033]
touches_crates: [convergio-durability, convergio-executor, convergio-runner, convergio-cli, convergio-planner, convergio-server]
last_validated: 2026-05-02
---

# 0034. Per-task runner selection (kind / profile / budget)

- Status: accepted
- Date: 2026-05-02
- Tags: runners, executor, schema

## Context

ADR-0028 introduced runner kinds (`shell`, `claude`, `copilot`).
ADR-0032 narrowed the daemon to vendor CLIs only (no raw API).
ADR-0033 replaced nuke flags with three permission profiles
(`Standard`, `ReadOnly`, `Sandbox`).

What was still missing: the **per-task** mapping between a planned
unit of work and the vendor CLI / model / permission profile that
should execute it. Up to this point the executor used a single
daemon-wide `SpawnTemplate` (ADR-0027) — fine for the MVP shell
runner, useless for heterogeneous swarms where some tasks should
run with `claude:opus` (planning, complex reasoning) and others
with `claude:sonnet` or `copilot:gpt-5.2-mini` (mechanical edits).

The user constraint is explicit: the planner — itself an
`opus`-backed subagent (separate ADR forthcoming) — must be able
to assign a `runner_kind` and `profile` per task, and the executor
must honor that assignment without recompiling the daemon.

## Decision

Add three nullable columns to `tasks`, threaded end-to-end:

| Column | Type | Meaning |
|--------|------|---------|
| `runner_kind` | `TEXT` | Wire format `<vendor>:<model>` (e.g. `claude:opus`, `copilot:gpt-5.2`). `NULL` = use daemon default. |
| `profile` | `TEXT` | One of `standard`, `read_only`, `sandbox` (ADR-0033). `NULL` = use daemon default. |
| `max_budget_usd` | `REAL` | Soft cap. Surfaced to the runner for self-policing; not an enforcement mechanism. `NULL` = unbounded. |

The executor's `dispatch_one` branches:

- `runner_kind` **None** + no `CONVERGIO_EXECUTOR_USE_RUNNER` env →
  legacy `SpawnTemplate` path (kept for shell smoke tests).
- otherwise → `spawn_via_runner`: parse `RunnerKind`, parse
  `PermissionProfile`, fetch tier-3 graph context-pack, build
  `SpawnContext`, call `for_kind(&kind).prepare(&ctx)?`, hand the
  prepared argv + cwd + stdin prompt to the supervisor.

`SpawnSpec` grew two fields (`cwd`, `stdin_payload`) so the
supervisor can run the agent in the right working directory and
pipe the prepared prompt via stdin (vendor CLIs read prompts from
stdin in non-interactive mode).

## Consequences

- The schema migration is `0011_task_runner_fields.sql` in the
  durability crate's range (1–100, ADR-0003).
- `cvg task create` exposes `--runner`, `--profile`,
  `--max-budget-usd` so a human can drive the new path without
  touching the planner. Defaults remain `None`.
- Future PR B will make the runner registry config-driven via
  `~/.convergio/runners.toml` so adding `qwen`, `codex`, `gemini`
  does not require a recompile.
- Future PR C will move the planner from template-heuristics to a
  `claude:opus` subagent that emits JSON plans with a chosen
  `runner_kind` per task.

## Alternatives considered

- **Single daemon-wide kind/profile.** Forces every task through
  the same model — rejected: defeats the cost/quality routing
  goal stated by the project owner.
- **In-process model registry.** Easy to wire but means a
  redeploy to add a new vendor CLI — rejected: violates the
  "configurable without recompile" requirement.
- **Persist runner choice in a side table.** Would normalize the
  cardinality but doubles the join cost on every dispatch tick
  for no functional gain.

## Validation

- `cargo fmt --all -- --check`
- `RUSTFLAGS="-Dwarnings" cargo clippy --workspace --all-targets -- -D warnings`
- `RUSTFLAGS="-Dwarnings" cargo test --workspace`
- Migration applies cleanly on a fresh `~/.convergio/v3/state.db`.
- `cvg task create ... --runner claude:opus --profile standard
  --max-budget-usd 0.5` round-trips through the API.
