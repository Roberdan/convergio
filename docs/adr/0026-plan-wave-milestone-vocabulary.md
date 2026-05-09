---
id: 0026
status: accepted
date: 2026-05-01
topics: [planning, plans, vocabulary, lifecycle]
related_adrs: [0011, 0012, 0021]
touches_crates: [convergio-durability, convergio-server, convergio-cli]
last_validated: 2026-05-01
---

# 0026. Plan / wave / milestone — one vocabulary, one source of truth

- Status: accepted
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: planning, vocabulary, lifecycle

## Context and Problem Statement

A 2026-05-01 audit of the daemon's open task list (5 plans, ~50
open tasks) surfaced four failure modes. Each one is small on
its own; together they explain why `cvg status` is no longer a
trustworthy "what to do next" surface — exactly the friction
F40 was supposed to prevent.

1. **"Wave" means two things.** Top-level plans named
   `Wave 0 — Convergio Vision`, `Wave 0b — Claude adapter`,
   `Wave 0b.2 — session pre-stop` collide with the integer
   `wave` field that lives on every task inside other plans
   (`v0.2 wave 1`, `v0.3 wave 2`). Same word, two surfaces, no
   relationship. A fresh agent reading "Wave 0b is at 9/11
   submitted" and "v0.2 wave 1 has 12 pending" cannot tell
   whether they are talking about the same level of granularity.

2. **`wave` integers have lost their meaning.** In `v0.2`, wave
   1 has tasks at sequence 1 (8 entries from 2026-04), sequence
   100-105 (6 entries added 2026-05 to mirror friction log F35-F40),
   and sequence 200-204 (5 entries from this same audit). Wave 1
   is no longer "the first release ondata"; it is "the bucket
   that exists by default". Wave 2 holds the durability split
   PRs (PR 13.x) plus a since-shipped bus filter task. Wave 3
   does not exist on `v0.2`. The numbering is decorative.

3. **Tasks that already shipped in `main` stay `pending` in the
   daemon.** Concrete: `596c6601-…` "Bus poll_messages: filter
   own published messages" is `pending` in v0.2 wave 2, but the
   feature merged in PR #71 (F53 / ADR-0024). The daemon has no
   notion of "feature already exists somewhere outside this
   task's own evidence" — only the task author knows, and they
   forget. F35 was supposed to fix this with `cvg pr sync`, but
   that command only operates on the *plan* it is invoked on
   and only matches PRs whose body declares `Tracks: <task-uuid>`.
   Cross-plan or pre-convention work is invisible.

4. **`F##` tags are riding on free-text titles, so they drift.**
   v0.2 contains two distinct tasks both titled `F49 — …`
   (graph estimated_tokens, gh pr update-branch), neither of
   which is the friction-log F49 (`cvg task retry`, mirrored as
   `d5e35aaa-…`). Likewise the v0.2 daemon `F38` and `F39` were
   the friction log's F44 and F45 until this commit retired
   them. The tag is not a key; it is a mood ring.

These four pieces compound: `wave` does not mean a release
sequence, plans named "Wave" overlap with wave-the-field, the
`F##` tag is unreliable, and there is no automation for "this
task is already done somewhere else". The daemon's view of
"open work" includes ghosts.

## Decision Drivers

- **Single vocabulary.** A word should have one meaning per
  layer.
- **Source-of-truth for "what is open".** `cvg status` must
  reflect reality, not a trailing wishlist.
- **Reversibility for the daemon's own dogfood plans.** Every
  rename / retire must leave audit footprints; no silent SQL
  edits.
- **No gold-plating.** We are renaming and adding lifecycle
  semantics, not redesigning the planner.

## Considered Options

### Option A — Rename plans, keep `wave` integer free-form, add `task.closed_post_hoc`

Smallest patch. Drop `Wave` prefix from the three top-level
plans (`Wave 0` → `W0 — Convergio Vision`; `Wave 0b` →
`W0b — Claude Code adapter`; `Wave 0b.2` → `W0b.2 — Session
pre-stop`). Document `wave` as "operator-chosen priority
bucket inside a plan; no enforced ordering". Add a new audit
kind `task.closed_post_hoc` for tasks the operator retires
because the work shipped outside the daemon. Costs: low. Doesn't
fix the `F##`-tag drift.

### Option B — Promote `wave` to a first-class lifecycle stage

Make `wave 1 / 2 / 3` mean "this release / next release / later".
Migrate every task forward into a real release-ondata mapping.
Costs: high one-shot migration; touches every existing task and
every gate that references waves; pretends we have a release
discipline we do not yet have.

### Option C — Drop `wave` entirely, keep `sequence` only

Replace `wave` with a single `priority bucket` enum
(`now / next / later`). Costs: schema migration; clear semantic
win, but throws away historical `wave` field on 200+ shipped
tasks for marginal gain. Disruptive.

## Decision Outcome

Chosen: **Option A**, with two additions to make the post-hoc
closure ergonomic:

1. New audit kind **`task.closed_post_hoc`**, written by a new
   facade method `Durability::close_task_post_hoc(task_id,
   reason, agent_id)`. Transitions any pending/failed task to
   `done` (yes, *to `done`* — this is the one exception to
   ADR-0011, recorded as a deliberate audit row with `reason`
   filled in and the operator's identity attached). Reason is
   mandatory and printed in `cvg status`.

2. New CLI surface **`cvg task close-post-hoc <id> --reason "..."`**
   so triage passes do not require curl. Future `cvg plan triage`
   (friction log F26 — daemon task `ce528dd3-…`) will batch this.

`wave` stays as it is — an integer field with operator-assigned
meaning, no enforcement. Documented explicitly: "wave is a
free-form priority bucket, not a release sequence; do not rely
on `wave 1 < wave 2` for any gate logic". The field is kept
because removing it is more disruptive than redocumenting it.

`F##` tags stay free-text in titles. The mirror table in
`docs/plans/v0.2-friction-log.md` (added 2026-05-01) is the
authoritative `F## → UUID` map; `cvg task get <uuid>` is the
authoritative status. Anyone reading a task title with `F##`
should treat the number as a label, not a key.

### Plan rename moves

- `Wave 0 — Convergio Vision: Long-Tail + Urbanism` →
  `W0 — Convergio Vision: Long-Tail + Urbanism`
- `Wave 0b — Claude Code adapter (PRD-001 implementation)` →
  `W0b — Claude Code adapter (PRD-001 implementation)`
- `Wave 0b.2 — cvg session pre-stop (PRD-001 Artefact 4 deferred slice)` →
  `W0b.2 — cvg session pre-stop (PRD-001 Artefact 4 deferred slice)`

The `v0.x` plans are unchanged — their names already do not
collide.

### What this decision does not do

- It does not introduce milestones, release-trains, or any new
  schema column. The schema is fine; the words around it were
  not.
- It does not retroactively triage the open task list. ADR-0026
  ships the lifecycle primitive (`task.closed_post_hoc`); the
  triage pass (`A` in the operator's plan above) is a separate
  human review.
- It does not bind `wave` to anything. The wishful "wave 1 is
  the first release" reading remains a convention people may
  follow inside a plan; the daemon does not enforce it.

## Consequences

### Positive

- "Wave" no longer collides with itself. `cvg status` listing
  `W0b — Claude adapter [draft] tasks: 9/11 submitted` next to
  `v0.2 wave 1 has 12 pending` is now obviously two different
  layers.
- `task.closed_post_hoc` lets us drain the ghost tasks (Bus
  poll_messages filter, F38/F39 daemon-side numbering, etc.)
  via auditable transitions, not raw SQL.
- The friction log mirror remains the canonical `F## → UUID`
  map, locking down the tag drift.

### Negative

- One more transition kind for downstream audit consumers to
  recognise. Acceptable: the existing chain is content-agnostic.
- `task.closed_post_hoc` is the second escape valve from
  ADR-0011 alongside Thor's `complete_validated_tasks`. We
  must guard against operators using it as a generic "I'm
  bored, close it" button. Mitigation: the `reason` field is
  mandatory, surfaces in the audit log, and the daemon enforces
  that it references a shipping artifact (e.g. PR number, commit
  SHA, or release tag/link) before allowing the close.

### Neutral

- `wave` integers continue to drift over time inside individual
  plans. We accept this; the alternative is a planner the user
  did not ask for.

## Validation

- Once shipped, `cvg status --project convergio-local` shows no
  duplicate-named entities at any level.
- `cvg task close-post-hoc <uuid> --reason "shipped in PR #71"`
  flips the task to `done` and writes one audit row. The audit
  chain remains valid.
- Triage pass (operator's "A") closes the surveyed ghost tasks
  with `task.closed_post_hoc` rows that name a concrete commit
  / PR.
- This ADR's decision drivers are revisited if a fifth distinct
  meaning of "wave" appears anywhere in the repo.
