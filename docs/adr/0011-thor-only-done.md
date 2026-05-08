---
id: 0011
status: accepted
date: 2026-04-30
topics: [layer-1, layer-4, gate-pipeline, breaking-change]
related_adrs: []
touches_crates: []
last_validated: 2026-04-30
---

# 0011. Done is set only by Thor (the validator)

- Status: accepted
- Date: 2026-04-30
- Deciders: Roberdan, office-hours dogfood session
- Tags: layer-1, layer-4, gate-pipeline, breaking-change

## Context and Problem Statement

Convergio's product claim is *"a local daemon that refuses agent work
whose evidence does not match the claim of done — and writes every
refusal to a hash-chained audit log."* The leash holds only if the
agent cannot mark its own work `done`.

In v0.1.0 the durability layer accepted any `target` on
`POST /v1/tasks/:id/transition`, including `target=done`. An agent
could therefore execute, in order:

```
cvg task transition <id> in-progress --agent-id rogue
# attach evidence the agent itself crafted
cvg task transition <id> submitted   --agent-id rogue
cvg task transition <id> done        --agent-id rogue
```

and reach `done` without the validator ever running. The audit chain
recorded each transition honestly, but **the chain alone does not
enforce who is allowed to flip status** — that is a contract problem,
not an audit problem.

CONSTITUTION §6 (technical non-negotiable, *Server-enforced gates
only*) states verbatim: *"A task cannot honestly be marked complete
by the client alone. The daemon verifies evidence and transitions
state. **Clients propose; the daemon disposes.**"* The pre-ADR-0011
behaviour violated that rule by letting the client both propose and
dispose `done`.

## Decision Drivers

- **Match the claim to the mechanism.** The README and `docs/vision.md`
  promise auditable refusal of self-incriminating evidence. That
  promise dies the moment the agent self-promotes.
- **Constitution §6** is the most explicit principle in the project,
  and the existing implementation contradicts it.
- **Layer boundaries.** `done` is the verdict produced by the
  validator (Thor / `cvg validate`). It is not state the worker
  computes; it is state the validator concludes.
- **Future composability** (the OODA loop work, smart Thor that runs
  the project's pipeline, agent↔Thor negotiation, 3-strike
  escalation, wave-as-PR validation) all assume Thor is the *single
  chokepoint* between submitted and done. Without that chokepoint,
  none of the smart-validator features have a place to attach.

## Considered Options

1. **Refuse `target=done` at the transition endpoint, set `done`
   only via `Thor::validate` → `Durability::complete_validated_tasks`.**
   Breaking: `cvg task transition X done` returns 403; clients call
   `cvg validate <plan_id>` instead. The `Done` variant disappears
   from the agent-facing CLI value enum and from the MCP action
   surface (`Action::CompleteTask` removed; SCHEMA_VERSION bumped).
2. Validate the `agent_id` on `target=done` against a reserved
   identifier (e.g. `"thor"`). No transition endpoint change —
   instead the durability layer trusts a magic string. Hacky:
   `agent_id` is metadata, not authentication.
3. Leave `transition_task` accepting `done` but make `cvg validate`
   the *recommended* path. Keeps the bug as the default behaviour
   and fails the constitution test in spirit.

## Decision Outcome

Chosen option: **Option 1**, because it is the only option that
puts the rule into the type system (the agent-facing `TaskTarget`
clap enum no longer offers `done`) and the durability layer
(`transition_task` rejects `Done` with a dedicated error variant
and writes a `task.refused` audit row), at the cost of one stable,
well-explained breaking change.

### Implementation

- `convergio-durability::transition_task` rejects
  `target=TaskStatus::Done` with `DurabilityError::DoneNotByThor`,
  writing one audit row of kind `task.refused` so the refusal is
  itself non-falsifiable.
- New method `Durability::complete_validated_tasks(task_ids: &[String])`
  flips a slice of `submitted` tasks to `done` atomically (single
  transaction), one `task.completed_by_thor` audit row per task.
  Reserved for the validator. Skips the gate pipeline because the
  gates already ran on the prior `submitted` transition.
- `convergio-thor::Thor::validate` now promotes every task currently
  in `submitted` (with all required evidence kinds present) to
  `done` as part of the Pass branch. The verdict is idempotent: a
  plan whose tasks are already all `done` re-validates as Pass with
  zero promotions.
- HTTP layer maps `DoneNotByThor` to **403 Forbidden** with stable
  code `done_not_by_thor`, and `NotSubmitted` to 409 with
  `not_submitted`.
- `convergio-cli::TaskTarget` value enum drops the `Done` variant —
  `cvg task transition X done` errors at clap parse time with a
  helpful message.
- `convergio-cli::demo` no longer issues `transition done` for the
  clean path; it submits and then calls `validate`. The demo now
  *teaches* the correct flow.
- `convergio-api::Action::CompleteTask` removed; `SCHEMA_VERSION`
  bumped from `"1"` to `"2"`. Agents previously calling
  `convergio.act complete_task` must now call
  `convergio.act validate_plan` after submitting.
- New audit kind: `task.completed_by_thor` (Thor promotions) joins
  the existing `task.refused`, `task.submitted`, `task.in_progress`
  family. Future cleanup subscribers (T12) listen on this event.

### Positive consequences

- The leash claim becomes mechanically enforceable, not a convention.
- Thor is now load-bearing in a way the architecture has been
  promising since ADR-0001.
- The demo now models the correct workflow.
- Future smart-Thor work (T14 — runs cargo fmt/clippy/test/doc-checks
  before Pass) plugs in cleanly: it lives inside `Thor::validate`
  before the call to `complete_validated_tasks`, and every agent
  must traverse it to reach `done`.

### Negative consequences

- **Breaking change for any caller of `cvg task transition X done`**
  or `convergio.act complete_task`. Documented in CHANGELOG; error
  messages name the replacement command/action.
- Additional surface for the validator: it now mutates state, not
  just inspects it. Mitigated by the dedicated audit kind and the
  single-transaction atomicity.

## Out of scope (tracked separately)

- **Cleanup pipeline on completion** (lease release, agent process
  shutdown, patch-proposal merge, capability child shutdown) is
  *not* in this ADR. Cleanup subscribers should listen on
  `task.completed_by_thor` rather than have the durability layer
  call across layer boundaries. Tracked as plan task T12.
- **Smart Thor** (running the project's actual pipeline before Pass)
  is tracked as plan task T14.
- **Agent ↔ Thor negotiation** (plan amendments) is tracked as
  plan task T15.
- **3-strike escalation to human** is tracked as plan task T16.
- **OODA-aware plan revision philosophy ADR** is tracked as plan
  task T17.
- **Wave-scoped validation** (`cvg validate <plan> --wave N`) is
  tracked as plan task T18 — the API
  `complete_validated_tasks(&[String])` is already wave-friendly.

## Pros and Cons of the Options

### Option 1 (chosen) — refuse at transition layer

- 👍 Encodes the rule in the type system and at the gate boundary.
- 👍 Demo, tests, and CLI now teach the correct workflow.
- 👍 Future smart-Thor features have an obvious home.
- 👎 One breaking change to the agent surface (mitigated by clear
  error messages and a SCHEMA_VERSION bump).

### Option 2 — gate the `agent_id`

- 👍 Smaller diff.
- 👎 Uses metadata as authentication; agent_id is documented as a
  free-form annotation, not a privilege grant.
- 👎 Future smart-Thor still has to add the same logic; this option
  postpones the right design rather than implementing it.

### Option 3 — leave it, document the recommended path

- 👍 Zero breaking change.
- 👎 The constitution claim and the runtime behaviour stay
  contradictory. Every external user of v0.1 can demonstrate the
  contradiction in three CLI commands.

## Links

- CONSTITUTION §6 (`Server-enforced gates only`).
- Office-hours plan task T11 on plan
  `8cb75264-8c89-4bf7-b98d-44408b30a8ae`.
- Friction log: `docs/plans/v0.1.x-friction-log.md` finding F13.
- Related: [ADR-0001](0001-four-layer-architecture.md),
  [ADR-0002](0002-audit-hash-chain.md),
  [ADR-0004](0004-three-sacred-principles.md),
  [ADR-0026](0026-plan-wave-milestone-vocabulary.md) (the one
  narrow operator exception, `task.closed_post_hoc`).
