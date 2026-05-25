---
status: accepted
date: 2026-05-25
deciders: Roberdan, Copilot
---

# ADR-0056 — Parametric plan templates and `cvg plan-templates`

## Context

W6 in the production-ready plan asks for re-usable plan scaffolds so
operators can spin up "the same kind of plan" without retyping titles,
descriptions, evidence kinds and wave ordering by hand. Today every
plan is bespoke: `cvg plan create` + N × `cvg task create`, with no
shared shape. That is fine for one-off experiments but it leaks into
demos and onboarding flows where we want to show off a repeatable
recipe (e.g. "spin up a vertical accelerator for $domain targeting
$audience").

## Decision

Add **first-party, in-Rust templates** as Rust `static` constants in
`convergio-planner::templates`, plus a top-level CLI verb
`cvg plan-templates` exposing them as `list`, `show`, and `render`.

A `Template` carries:

- a kebab-case `name`,
- a `summary`, optional `description`,
- a single `objective` (Fluent-friendly free-form string, will land
  in `plan_objectives` once W4's POST route is wired),
- a list of `parameters` (each with `name` + `help`),
- a `title` for the resulting plan,
- a list of `TemplateTask` records with `wave`, `sequence`, `title`,
  optional `description`, and a static `evidence_required` list.

`Template::render(&HashMap<String,String>)` substitutes `{{var}}`
placeholders in `objective`, `title`, task titles and task
descriptions, and returns a `RenderedTemplate { objective, plan:
PlanShape }` that maps 1:1 onto the existing planner schema. Unknown
or missing parameters raise `PlannerError::Template`.

`cvg plan-templates` is intentionally **render-only**. The operator
pipes the JSON into existing routes (or into a future
`cvg plan create --template` shortcut) — we do not yet POST the plan,
nor do we POST the objective; those live in W6-follow-up alongside
W4's `POST /v1/plans/:id/objective`.

Templates ship as `static` constants instead of YAML files because:

- the workspace has no YAML dependency,
- bin-embedded scaffolds cannot drift from compiled code,
- a future capability-supplied YAML loader can land as ADR-0057+
  without re-shaping the trait surface.

## Consequences

- `convergio-cli` now depends on `convergio-planner` for offline
  render. Acceptable: rendering is a pure function with no daemon
  round-trip and no I/O.
- One built-in scaffold ships in this PR
  (`vertical-accelerator-v1`, 5 tasks, 4 params). Additional
  built-ins land as follow-ups; YAML-supplied templates land via a
  later capability hook.
- Until `POST /v1/plans/:id/objective` exists (W4-follow-up),
  operators must manually copy the rendered `objective` into the
  daemon — we surface it in JSON so this is a one-line `jq`.
- The render output is the `RenderedTemplate` shape, not bare
  `PlanShape`, so callers can `jq .plan` or `jq .objective` without
  re-merging.

## Alternatives considered

- **YAML on disk under `templates/`**: rejected — pulls a new dep,
  introduces drift between code and templates, and we have no
  capability hook yet to load them at runtime.
- **`cvg plan create --template <name> --param k=v`**: rejected for
  this slice — would require touching `crates/convergio-cli/src/commands/plan.rs`
  (already 283 LOC, cap 300) and would conflate render with the
  follow-up create flow. Easier to ship render first, layer create
  on top later.

## Status

Implemented in PR (W6). Built-in: `vertical-accelerator-v1`.
Follow-ups tracked separately:

- POST a rendered plan + objective from a single CLI verb.
- Capability-supplied YAML templates.
- More built-ins as the production-ready plan demands.
