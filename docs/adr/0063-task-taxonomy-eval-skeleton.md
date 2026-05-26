---
status: accepted
date: 2026-05-25
deciders: Roberdan, Copilot
---

# ADR-0063 — Task taxonomy + eval outcome ledger skeleton (W10 slice)

## Context

W10 (Cost-of-Pass) asks for a full evaluation framework: closed task
taxonomy, `model_evaluations` aggregation view, Thor-side recorder
that fires on every `pipeline_passed`/`pipeline_refused` audit row,
MCP `eval.recommend` + `eval.report` + `eval.calibrate` actions, and
a `cost_of_pass` KR baked into vertical-accelerator templates.

That full slice is 7-10 days and depends on real Smart Thor data
(W3) and a routing consumer (W8). Today we have neither at scale,
but the **shape** of the data we will eventually store is well
understood. Locking it in now prevents the inevitable "we wrote
Thor against ad-hoc fields and now have to migrate" scenario.

## Decision

Ship the W10 storage skeleton:

1. **Migration 0016** introduces two tables:
   * `task_taxonomy` — single-column closed enum, seeded with the
     seven canonical kinds from the production-ready plan
     (`generate-test`, `review-code`, `write-docs`, `refactor`,
     `plan`, `summarise`, `generic`). New kinds = new migration.
   * `eval_outcomes` — one row per pipeline verdict, keyed by
     `(runner_kind, taxonomy_kind)`. `cost_usd` and `latency_ms`
     are nullable so Thor can start emitting verdicts before it
     learns to extract usage numbers.
2. **`TaxonomyStore`** — read-only DAO (`list`, `contains`).
3. **`EvalOutcomeStore`** — `record(NewEvalOutcome)` + a thin
   `count_for(runner_kind, taxonomy_kind)` placeholder for the
   `model_evaluations` view that W10-full will replace with a real
   pass-rate / mean-cost / p95-latency aggregation.

No HTTP surface, no MCP action, no CLI command yet — those depend
on data nobody is yet producing. Adding them at this stage would
be P4 "scaffolding only".

## Consequences

* The data shape is committed; W10-full (Thor recorder, view,
  `eval.recommend`) becomes purely additive on top of the existing
  tables.
* Adding a new taxonomy kind is a migration, not a runtime call —
  matches the production-ready plan's explicit "closed list"
  requirement and avoids the long-tail problem where every team
  invents their own kind.
* The W8 `dispatch.choice` audit row (ADR-0062) and the W10
  `eval_outcomes.runner_kind` use the same `vendor:model` wire
  format, so the eventual router can join them without a
  translation layer.

## Out of scope (W10-full)

* `model_evaluations` aggregation view.
* Thor-side recorder hook on `pipeline_passed` / `pipeline_refused`.
* `eval.recommend` / `eval.report` / `eval.calibrate` MCP actions
  and matching `cvg eval` subcommands.
* Cold-start handling logic in `recommend`.
* `cost_of_pass` KR integration with vertical-accelerator
  templates (depends on W4 + W6).
