---
status: accepted
date: 2026-05-25
deciders: Roberdan, Copilot
---

# ADR-0055 — Plan objectives table and PlanCoherenceGate

## Context

W4 in the production-ready plan asks for OKR-style objectives on every
plan and gates that refuse work which is not aligned with a stated
outcome. Today plans carry a title and description but no
machine-readable "what does this plan exist to achieve" field. Without
that, Smart Thor (W3) cannot judge whether evidence actually moves the
plan toward its goal, and operators cannot triage stale plans by
"does anyone still care about this objective?".

## Decision

Introduce `plan_objectives`, a 1:1 sidecar table keyed by `plan_id`,
storing a single free-form objective string plus timestamps. The table
is separate from `plans` so we do not touch the `PLAN_SELECT` constant
or migrate every existing `Plan` consumer — additive, reversible.

Add `PlanCoherenceGate` to `default_pipeline()` (immediately after
`PlanStatusGate`, before `EvidenceGate`). The gate fires only on
`task.submitted` transitions and refuses with
`plan_coherence/plan_missing_objective` when no objective row exists.

Enforcement is opt-in via the env var
`CONVERGIO_REQUIRE_PLAN_OBJECTIVE=1`. With the flag unset (default),
the gate is a no-op — this preserves backward compatibility for every
existing plan and every existing test fixture. A follow-up PR will
flip the default to "on" once a one-shot backfill migration has
written objectives onto every shipped plan.

The DAO (`PlanObjectiveStore`) exposes `get` / `set` (UPSERT) /
`exists`. CLI verbs (`cvg plan objective set/get`), MCP actions,
HTTP routes, key-results table, and `PlanOutcomeGate` are deferred to
the W4 follow-up PR — this slice ships the durable storage plus the
gate scaffold so subsequent work can build on stable primitives.

## Consequences

**Positive**

- Stable schema for objectives lands now; no downstream consumer is
  blocked on schema bikeshedding.
- New gate is wired but inert by default — zero regression risk on
  existing deployments.
- Production deployments can flip the flag immediately after
  backfilling objectives, without a redeploy.

**Negative**

- The opt-in flag is technical debt until flipped. Tracked as a
  follow-up; ADR-0055 follow-up PR must remove the flag and make
  enforcement unconditional.
- Operators who do not set the flag get a false sense of "plan
  coherence enforced" — mitigated by surfacing the flag state in
  `cvg doctor` (also deferred to the follow-up).

## Links

- Plan: `docs/plans/v1.0-production-ready.md` W4
- Sibling gates: ADR-0050 (PromptInjectionGate), ADR-0051 (A11yGate)
- Migration: `crates/convergio-durability/migrations/0015_plan_objectives.sql`

