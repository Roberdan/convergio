---
id: 0021
status: proposed
date: 2026-05-01
topics: [vision, planning, plans, gates, smart-thor]
related_adrs: [0001, 0011, 0012, 0014, 0016, 0018]
touches_crates: [convergio-durability, convergio-cli, convergio-thor]
last_validated: 2026-05-01
---

# 0021. Plans are Objectives + Key Results — strategic programming for the municipality

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, planning, smart-thor

## Context and Problem Statement

A `plan` in Convergio today is a *list of tasks* with a free-text
title and description. Nothing in the data model answers the
question:

> "What is this plan trying to achieve, and how will we know if it
> achieved it?"

Without that answer, three failures recur in our own dogfood
plans:

1. **Plans grow forever.** Friction log F26 (v0.2.x) recorded the
   "v0.1.x — Close honesty gaps" plan growing from 14 to 38 tasks
   without coordination. New tasks accreted because nobody could
   say "this task does not contribute to the plan's purpose" —
   the purpose was never written down measurably.
2. **Plans never close.** Tasks reach `done` one by one, but the
   plan itself stays in `draft` indefinitely. There is no
   structural notion of *plan-level done*, because there is no
   structural notion of *plan-level success*.
3. **Demos look successful when the underlying business outcome
   isn't.** `convergio-edu` Wave 3 might ship a working demo
   while completely missing its actual purpose (e.g. "make
   education accessible to dyslexic kids in EN+IT"). The demo
   passes; the goal is silent.

The urban-planning analogue (`docs/vision.md` § 5) is **strategic
programming**: every municipality publishes multi-year goals
("reduce traffic by 20%") with measurable indicators ("+50 km
of bike lanes", "average crossing time -15%", "PM10
below 30 µg/m³"). The goals are public, the indicators are
public, the audit is public. The procurement, the construction,
the inspections all chain back to the indicators.

OKR (Objective + Key Results, Andy Grove → John Doerr → most of
modern tech) is the same pattern: a textual objective that
describes the desired outcome, plus 3-5 measurable Key Results
that prove the objective was met. We adopt it as the structural
spine of `plan`.

## Decision Drivers

- **Outcome > Output (ADR-0012).** OKR is *literally* "outcome,
  measured". Smart Thor (T3.02) can verify task evidence; only
  KR can verify plan-level outcome.
- **Modulor compositionality (ADR-0018).** Tasks compose into
  plans. Without an objective, the composition is mathematical
  but not semantic. Adding `objective + key_results` to plans
  closes the semantic loop without breaking the Modulor (tasks
  remain the atomic unit).
- **Friction log evidence.** F26 plan-growth pathology has a
  named cure: a plan with explicit KR refuses tasks that do not
  trace back to a KR. The architecture forces the discipline.
- **Long-tail accelerator authoring.** A vertical accelerator
  template (`education-accelerator-v1`, ROADMAP Wave 1) has to
  declare its objective and the KR a builder must hit to claim
  a successful instantiation. Without OKR in the data model,
  templates carry the goal in prose only, which rots.

## Considered Options

### Option A — OKR as a Markdown convention only

Document a convention: every plan should have an "Objective"
and "Key Results" section in its description. No schema change,
no enforcement. Costs: it is what we have today
(`docs/plans/*.md`). The convention is unenforced, friction-log-
proven inadequate.

### Option B — OKR as a sibling table, optional

Add `plan_key_results` table; treat the OKR as advisory metadata
that does not gate anything. Costs: same fate as option A —
optional metadata is metadata that nobody fills in. The schema
churn buys nothing if no gate consumes it.

### Option C — OKR as first-class plan structure with gate
enforcement (chosen)

Three coupled changes:

1. **Schema**: `plans.objective` becomes NOT NULL; new
   `plan_key_results` table required.
2. **Gate**: a new `PlanCoherenceGate` refuses to transition a
   task to `submitted` if the task does not declare which KR
   it contributes to (`tasks.contributes_to_kr_id` nullable but
   audited as a warning if NULL).
3. **Validation**: smart Thor (T3.02) cannot promote a plan to
   `done` until every KR has a `current_value` that meets its
   `target_numeric`, or the human explicitly overrides via
   3-strike escalation.

## Decision Outcome

Chosen option: **Option C**, because OKR as advisory metadata is
indistinguishable from no OKR at all (option A and B); only
gate-and-validation enforcement creates the discipline F26 was
asking for.

### Implementation status (2026-05-04)

Stays `proposed`. None of the schema or gate machinery has shipped:
`plans.objective` does not exist, the `plan_key_results` table does
not exist, `tasks.contributes_to_kr_id` does not exist, and neither
`PlanCoherenceGate` nor `PlanOutcomeGate` is implemented. `cvg plan
triage` exists for stale-task surfacing but is unrelated to OKR. The
ADR is gated on Wave 1 (smart Thor T3.02) and remains the intended
schema shape until then.

### Schema (Wave 1)

```sql
ALTER TABLE plans
  ADD COLUMN objective TEXT NOT NULL DEFAULT '';
-- The DEFAULT '' is a migration concession; new plans created
-- via cvg plan create require a non-empty objective at the API
-- layer. Old plans may carry empty objective until edited.

CREATE TABLE plan_key_results (
  id BLOB PRIMARY KEY,
  plan_id BLOB NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL,
  statement TEXT NOT NULL,           -- "ship 5 capability blocks"
  target_numeric REAL,               -- 5.0 (nullable for binary KR)
  target_unit TEXT,                  -- "blocks", "%", "ms", null
  measurement_method TEXT NOT NULL,  -- "count of capability install-files in registry"
  current_value REAL,                -- nullable until first measurement
  current_value_evidence_id BLOB,    -- nullable; pointer to evidence row
  last_measured_at TEXT,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'on_track', 'at_risk', 'achieved', 'missed'))
);

CREATE INDEX plan_key_results_plan ON plan_key_results(plan_id, sequence);

ALTER TABLE tasks
  ADD COLUMN contributes_to_kr_id BLOB
  REFERENCES plan_key_results(id);
-- Nullable: not every task contributes to a measured KR
-- (e.g. infrastructure / refactor tasks). PlanCoherenceGate
-- emits a warning, not a refusal, on NULL — see below.
```

### Gates (Wave 1)

#### `PlanCoherenceGate` (new, mandatory)

Refuses a task transition to `submitted` if the plan has no
`objective` and at least one `plan_key_result`. This forces the
plan author to declare an objective and at least one KR before
work can be marked complete.

Emits a *warning* (not a refusal) when a task has
`contributes_to_kr_id IS NULL`. The warning is recorded in the
audit row as `task.coherence_warning` so the operator can see
"these tasks did not declare which KR they advance" without the
gate becoming hostile to refactor / infrastructure work.

#### `PlanOutcomeGate` (new, blocks plan-level done)

Smart Thor (T3.02) integration. When validating an entire plan,
the gate enforces: for plan to reach `done`, every
`plan_key_result` must be `status IN ('achieved', 'missed_with_override')`.
Missed KRs require an explicit human override row in the audit
log (3-strike escalation, ADR-0012) — the plan can close as
"missed", but it cannot silently close as "done".

### CLI surface (Wave 1)

```bash
# Create a plan with objective inline
cvg plan create "Wave 0 docs" \
  --objective "Articulate the long-tail urbanism framing so contributors can place new work without reading scattered context"

# Add KR to existing plan
cvg plan kr add <plan_id> \
  --statement "every Wave 0 deliverable references VISION + ADR + ROADMAP"
  --target 100 --unit "%" \
  --method "ratio of files cross-referenced / total Wave 0 files"

# Update KR measurement (called by an agent or manually)
cvg plan kr measure <kr_id> --value 87 --evidence-id <evidence_id>

# Show OKR progress
cvg status --plan <plan_id> --okr
```

### MCP surface (Wave 1)

New actions: `set_plan_objective`, `add_key_result`,
`update_key_result_value`, `list_plan_okrs`. Closed schema,
audited.

### Drift detection (Wave 2 — code-graph integration)

ADR-0014 code graph already detects "tasks declared they touched
crate X but actually touched crate Y". The same engine extends:
"tasks declared they advance KR X but their evidence does not
move KR X's `current_value`". Surfaces as a `kr.drift` audit
row, advisory in v1, gating in v2.

### Worked example — this very Wave 0 plan

The Wave 0 plan currently materialised in convergio (plan
`543c0d38-…`) gets retroactively annotated:

```
Objective: Articulate the long-tail urbanism framing of Convergio
so contributors can place new work without reading scattered
context, and dogfood the gate pipeline on the resulting
documentation.

Key Results:
  KR1 — every Wave 0 deliverable file cross-references at least
        one of (VISION, ADR, ROADMAP). Target: 100%.
  KR2 — at least one independent reader, given only the new
        VISION + README, can answer the four-marginal-cost
        question correctly. Target: yes/no, validated by external
        review.
  KR3 — the audit chain over the Wave 0 commit verifies clean
        end-to-end. Target: 0 broken hashes.
```

This is what the architecture forces: a plan that lives in
convergio without an objective + KR is structurally incomplete
after this ADR ships.

## What this decision does not do

- It does not adopt OKR as a project management methodology in
  the human-facing sense. We do not run "OKR check-in meetings".
  The data model is the discipline; the human practice is
  whatever the operator chooses.
- It does not impose KR on existing plans. Migration default
  empties `objective` and ships zero KR; the gate refuses *new
  task submission* on plans without objective + KR, not retro
  on done plans.
- It does not require KR for every task. The
  `contributes_to_kr_id` column is nullable; the
  PlanCoherenceGate warns rather than refuses, so refactor /
  infra tasks can ship without lying.

## Consequences

### Positive

- F26 plan-growth pathology gets a structural cure. Tasks that
  do not advance a KR are visible in the audit log and create
  pressure (advisory in v1, gate in v2) to close them.
- The "strategic programming" service of the municipality becomes
  operational. Plans become legible at a single line.
- Vertical accelerator templates (Wave 1) ship with their KR
  pre-declared. A builder instantiating `education-accelerator-v1`
  inherits the OKR contract and cannot pretend the
  accelerator is done without measuring it.
- Smart Thor (T3.02) gets a new outcome dimension: not just "do
  the tasks pass?" but "did the plan achieve its KR?". This is
  the missing piece for *outcome* validation per ADR-0012.

### Negative

- Schema churn on a heavily used table (`plans` + new
  `plan_key_results`). Migration must be careful; ADR-0003
  (per-crate migration coexistence) keeps blast radius bounded.
- The "advisory warning vs hard refusal" distinction is a
  judgement call. PlanCoherenceGate warns on NULL
  `contributes_to_kr_id` to avoid blocking refactor work; some
  operators will want it to refuse instead. Mitigation:
  configurable per-plan via metadata flag.
- Operators who don't want OKR have to put up with a
  one-line empty objective field. They get the discipline
  whether they wanted it or not. This is intentional;
  CONSTITUTION § Sacred principles is not negotiable, and OKR
  becomes an extension of the same posture.

### Neutral

- This ADR depends on smart Thor (T3.02, Wave 1). The
  `PlanOutcomeGate` cannot land before T3.02 promotes Thor
  beyond evidence-shape checks.
- Drift detection ties into ADR-0014 code graph (Wave 1
  proposed status) which is currently advisory. Hardening to
  gate is Wave 2.

## Validation

This ADR is validated **after Wave 1 ships** (not before — the CLI
and gate it depends on do not exist in v0.2.x). At that point:

1. `cvg plan create` without `--objective` returns a clear
   error pointing at this ADR.
2. A new plan with at least one KR can be created, a task can
   declare `contributes_to_kr_id`, and `cvg status --plan
   <id> --okr` shows the relationship.
3. Smart Thor refuses to promote a plan to `done` while one of
   its KR is in `pending` or `at_risk` status.
4. The Wave 0 plan currently in convergio gets retroactively
   annotated with the worked-example OKR above, and `cvg
   status --plan 543c0d38-… --okr` returns a useful report.

Until Wave 1 ships, this ADR is `proposed` status and the
worked example above is illustrative only — the schema and CLI
do not exist yet.
