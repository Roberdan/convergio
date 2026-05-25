# ADR-0062 — Dispatch-choice audit row (W8 slice)

* Status: Accepted
* Date: 2026-05-25
* Tracks: production-ready-plan W8 (multi-vendor routing)

## Context

W8 in the production-ready plan asks for a routing algorithm that
picks the cheapest runner that still meets a target `pass_rate` and
latency budget, using historical signals from Smart Thor (W3) and the
model-evaluation framework (W10). That algorithm is a 5-7 day piece
of work that depends on data we do not yet collect at scale.

The smaller observable problem today is "for every spawn the
executor performs, the audit log should record **what** was chosen
and **why**, in a stable shape". Without that row, even the manual
audit story is opaque: there is no single place where an operator
can answer "why did Convergio pick `copilot:gpt-5.2` for task X?".

## Decision

Emit one `dispatch.choice` audit row per `dispatch_one` call, keyed
to the task. Payload shape:

```json
{
  "runner_kind": "copilot:gpt-5.2",   // "legacy-shell" for shell smoke tasks
  "profile": "balanced",              // null when the task carried none
  "rationale": "task_override",       // task_override | default | legacy
  "plan_id": "<uuid>"
}
```

The rationale enum is intentionally narrow. W8-full will add
variants (`pareto_winner`, `cost_floor`, `latency_cap`,
`cold_start`) — adding variants is forward-compatible because the
payload is freeform JSON.

Implementation is a single module `convergio_executor::dispatch_choice`
with a `record_for_task(durability, task, plan_id, kind, legacy)`
helper invoked from `dispatch_one` after the atomic claim and
before the spawn. Audit emission failures are logged and swallowed:
a failed audit row must never block a spawn.

## Consequences

* Operators can already query "what runner did this task get?" via
  `cvg audit events | rg dispatch.choice` with no further work.
* The W8-full router can replace the rationale labels without
  changing the row shape — downstream tooling (TUI, future
  evaluators) doesn't need to be re-deployed in lock-step.
* The payload deliberately omits cost/latency telemetry — that
  belongs to a future `dispatch.outcome` row emitted by the
  evaluator (W10), keeping the choice/outcome split clean.

## Out of scope (W8-full)

* Routing decision algorithm itself (currently still "task override
  beats default beats legacy").
* `dispatch.outcome` row capturing pass_rate / cost / latency
  observed post-run — that depends on W10's eval framework.
* CLI surface (`cvg dispatch why <task-id>`) — easy to add once the
  rows accumulate.
