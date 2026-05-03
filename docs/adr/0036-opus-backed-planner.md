---
id: 0036
status: accepted
date: 2026-05-03
topics: [planner, agents, opus, vendors]
related_adrs: [0027, 0032, 0033, 0034, 0035]
touches_crates: [convergio-planner, convergio-server]
last_validated: 2026-05-03
---

# 0036. Opus-backed planner replaces the line-split heuristic

- Status: accepted
- Date: 2026-05-03
- Tags: planner, agents, opus

## Context

Up to this PR the planner (`convergio-planner::Planner::solve`)
was a line-split heuristic — every non-blank line of the mission
became a wave-1 task with no `runner_kind`, no `profile`, no
evidence list. Useful for smoke tests, useless for real work.

Project owner direction: "il planner deve assolutamente essere
fatto sempre con opus e organizzare il piano, i task per
ottimizzare le PR, il contesto, l'uso dei vari modelli,
ottimizzare per qualità vs costo vs tempi, conoscere esattamente
cosa delegare a chi in sicurezza".

The planner is the routing brain: it has to know which tasks
deserve `claude:opus` and which can run on `copilot:gpt-5.2-mini`,
which tasks mutate vs. only inspect, and how to slice the work
into small reviewable PRs.

## Decision

Make `claude:opus` the default planner backend. The planner is
itself a vendor-CLI subprocess (ADR-0032 — no raw API calls):
`claude -p --model opus --output-format json --permission-mode plan
--input-format text`. The prompt is piped on stdin; the model
returns a JSON object matching `convergio_planner::PlanShape`; the
planner persists the plan + tasks via `convergio-durability`.

Three modes (selected via `$CONVERGIO_PLANNER_MODE`):

| Mode        | Behavior                                                     |
|-------------|--------------------------------------------------------------|
| `auto`      | Default. Use Opus when `claude` is on `PATH`, else heuristic.|
| `opus`      | Force Opus. Errors out when `claude` is missing.             |
| `heuristic` | Force the line-split fallback. Used by CI + unit tests.      |

The `--permission-mode plan` flag keeps the planner sub-agent
read-only — it produces JSON, it does not edit files. The
operator's existing Claude Max plan covers the cost (no API key in
Convergio's possession).

## Consequences

- The planner is now the first place where `runner_kind`,
  `profile`, and `max_budget_usd` (ADR-0034) are decided. Every
  task that flows through `convergio-server`'s `POST /v1/solve`
  carries those fields when the operator runs the daemon with
  `claude` on `PATH`.
- `claude` becomes a soft runtime dependency. Without it the
  daemon still works (heuristic fallback) but the per-task routing
  reverts to daemon-wide defaults.
- The JSON schema (`PlanShape`) is the new contract between the
  planner and the rest of Convergio. Schema changes require a
  prompt + parser update in the same PR.
- CI uses `CONVERGIO_PLANNER_MODE=heuristic` implicitly — no
  vendor login on the runner.
- Validation is shallow on purpose: the planner trusts the model
  for semantic decisions (which tasks belong in wave 2) and only
  rejects obvious schema drift (empty title, no tasks, wave < 1).

## Alternatives considered

- **Keep the heuristic, add hand-tuned routing rules.** A scaling
  trap — every new rule is a release.
- **Use Sonnet for planning.** Cheaper but the user's explicit
  ask was Opus. Opus also produces materially better task
  decompositions in this domain.
- **Stream tasks as they're emitted (stream-json output).** Would
  let the executor start dispatching wave 1 before the planner
  finishes wave 3. Out of scope for v1; revisit if planning
  latency becomes the bottleneck.
- **Planner as a daemon-managed agent (Supervisor::spawn).**
  Symmetrical with workers but adds a round-trip through the
  audit chain for what is essentially a synchronous JSON call.
  Deferred — current shape keeps `POST /v1/solve` simple.

## Validation

- `cargo fmt --all -- --check`
- `RUSTFLAGS="-Dwarnings" cargo clippy --workspace --all-targets -- -D warnings`
- `RUSTFLAGS="-Dwarnings" cargo test --workspace`
- Unit tests cover prompt shape, JSON envelope unwrapping, schema
  validation (rejects empty tasks, missing title), prose-prefix
  tolerance.
- The actual `claude -p` spawn is exercised via manual smoke,
  not in CI.
