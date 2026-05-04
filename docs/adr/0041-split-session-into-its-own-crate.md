---
id: 0041
status: accepted
date: 2026-05-03
topics: [legibility, refactor, cross-cutting]
related_adrs: [0013, 0014, 0015, 0040]
touches_crates: [convergio-cli, convergio-cli-session]
last_validated: 2026-05-03
---

# 0041. Split the session lifecycle suite into its own crate

- Status: accepted
- Date: 2026-05-03
- Deciders: Roberto, claude-code-roberdan
- Tags: legibility, refactor, cross-cutting

## Context and Problem Statement

After PR #164 (Check 1 plan-PR drift) merged, `convergio-cli`
shipped at 11033 lines — past the 11000-line per-crate hard cap
(CONSTITUTION § 13). PR #166 (SessionStart hook + telemetry)
needed roughly 500 additional lines, which would have pushed the
crate further over cap; it was closed pending this refactor.

Same failure mode that drove ADR-0040 (extract `convergio-coherence`).
The natural seam this time is the `cvg session` suite:

- 9 source files, ~1500 lines total, all colocated under
  `crates/convergio-cli/src/commands/session*.rs` and
  `session_checks/`.
- Two subcommands — `session resume` (cold-start brief) and
  `session pre-stop` (end-of-session safety net, PRD-001 §
  Artefact 4).
- Reusable from skills, runners, and the upcoming `convergio-mcp`
  action surface. Today they are reachable only via the `cvg`
  clap dispatcher; future SessionStart hook + bridge integrations
  benefit from being able to call the verifiers as a library.

ADR-0040 set the precedent. Bumping the cap is not the move.

## Decision Drivers

- **CONSTITUTION § 13 — 11000 lines per crate, hard cap.** The
  cap is the cap. PR #166 closed because of it; this refactor
  unblocks both #166 (re-applied as a follow-up) and any future
  session-lifecycle work.
- **Legibility (CONSTITUTION § 16).** A 9.1k-line CLI is still
  larger than the 5000-line soft cap, but pulling the session
  suite out shaves ~1.4k off and isolates an internally cohesive
  seam (cold-start brief + pre-stop checks).
- **Reusability.** Skills and the planned SessionStart hook
  should be able to call `pre_stop::run_pre_stop` as a library,
  not shell out to `cvg`.
- **No behavioural change.** The split must be observably a
  no-op: `cvg session resume --output json` must produce
  byte-identical output before and after, modulo the new crate's
  AGENTS.md / README.md and this ADR.
- **Test parity.** Every existing test moves with its module; no
  test is rewritten or skipped to make the move pass.

## Considered Options

1. **Bump the per-crate cap to 12k.** *Cheapest in PR diff, but
   gives up the only mechanism keeping the CLI legible. ADR-0040
   already rejected this when faced with the same squeeze.*
2. **Inline-split inside `convergio-cli`** (e.g. add a
   `commands::session::*` submodule tree without a new crate).
   *Saves one Cargo.toml but does not move the LOC out of the
   crate. Cap problem unsolved.*
3. **Extract `convergio-cli-session` (chosen).** Move the nine
   files to a new cross-cutting crate; keep a thin shim in the
   CLI so the dispatcher line in `main.rs` is unchanged.
4. **Fold into `convergio-coherence`.** Both are cross-cutting
   verifier-shaped crates extracted from the CLI. *Wrong on
   layering: coherence is doc-vs-code drift; session is
   daemon-shaped (cold-start brief, pre-stop checks against the
   live audit chain). Different concerns; do not couple them.*

## Decision Outcome

Chosen option: **Option 3 — extract `convergio-cli-session`**.

### Target topology

```
convergio-cli-session (~1500 LOC)
├─ session.rs            public entry: SessionCommand + run
├─ render.rs             cold-start brief renderers (human/json/plain)
├─ pre_stop.rs           Check trait + outcome types + registry
├─ pre_stop_run.rs       pre-stop dispatcher + report renderer
├─ session_tests.rs      session.rs unit tests (kept colocated)
└─ checks/
   ├─ mod.rs
   ├─ check_1_plan_pr_drift.rs
   ├─ friction_missing.rs
   └─ worktree_no_pr.rs
```

`convergio-cli` keeps `commands/session.rs` as a 35-line shim
that re-exports `SessionCommand`, builds the new crate's
`Client` from the CLI's `Client::base()`, converts the CLI's
`OutputMode` to the new crate's enum, and dispatches.

### Dependency direction

```
convergio-i18n ─< convergio-cli-session ─< convergio-cli
```

No back-edges. The new crate has no `convergio-cli` knowledge.
Mirrors the pattern from ADR-0040.

### Positive consequences

- `convergio-cli` drops from 11033 to ~9138 lines. Headroom for
  the SessionStart hook + telemetry work (PR #166 reapplied as a
  follow-up after this lands).
- The session suite is agent-callable from any binary that adds
  `convergio-cli-session` to its `Cargo.toml`. Skills and the
  upcoming SessionStart hook no longer need to shell out to
  `cvg`.
- Independent test surface — `cargo test -p convergio-cli-session`
  runs only the session tests, not the entire CLI.
- Mirrors ADR-0040's precedent for splitting cross-cutting
  concerns out of the host crate.

### Negative consequences

- One additional `Cargo.toml`, one additional `AGENTS.md`, one
  additional CI build target.
- The CLI's `OutputMode` and `Client` types now exist in two
  places (CLI's, the new crate's); the shim converts between
  them. Acceptable: the alternative is a `convergio-cli-session`
  dep on `convergio-cli` that re-exports the enum, which inverts
  the layering this ADR establishes (and ADR-0040 already
  rejected).

## Migration plan

One PR (this one). Squash-merge is disabled per CONSTITUTION § 15
so the file moves are visible in history.

1. Create `crates/convergio-cli-session/` with `Cargo.toml`,
   `AGENTS.md`, `CLAUDE.md` (symlink), `README.md`, `src/lib.rs`.
2. `git mv` the nine `session*.rs` files. Rename short:
   `session_pre_stop.rs` → `pre_stop.rs`, etc.
3. Rewrite `super::session_*` and `super::OutputMode` /
   `super::Client` references to the new short module names +
   `crate::OutputMode` / `crate::Client`.
4. Replace `crates/convergio-cli/src/commands/session.rs` with
   the shim.
5. Add `convergio-cli-session` to workspace members + CLI deps.
6. Run `cvg docs regenerate` and `scripts/generate-docs-index.sh`.
7. Verify `cvg session resume --output json` and
   `cvg session pre-stop --output json` are functionally
   identical (the doc index gains one new crate row, the new
   AGENTS.md / README.md add three new docs to the body scan
   — these are expected deltas, not regressions).

## Pros and Cons of the Options

### Option 3 (chosen)

- Good: respects the per-crate cap without raising it.
- Good: session suite becomes embeddable, not just callable from
  `cvg`.
- Good: matches ADR-0040's pattern.
- Bad: small `OutputMode` / `Client` duplication at the shim.

### Option 1 (rejected)

- Good: zero PR cost.
- Bad: caps exist for a reason (CONSTITUTION § 13). Bumping at
  the second squeeze is how soft caps die.

### Option 2 (rejected)

- Good: single-Cargo.toml move.
- Bad: does not solve the cap problem; just shuffles files.

### Option 4 (rejected)

- Good: one fewer crate.
- Bad: coherence is doc-vs-code drift; session is daemon-shaped.
  Different concerns; coupling them now would force an awkward
  re-split later.

## Links

- Convergio plan: `de413e30-f46c-435f-9b60-b81a8f9ffbab`.
- Convergio task: `07418af1-4aef-4cc8-8439-e3693c3d73bc`.
- Related ADRs: 0013 (durability split — pattern reused), 0014
  (Tier-3 code graph — sibling cross-cutting crate consumed by
  `session resume --task-id`), 0015 (docs as derived state —
  `cvg docs regenerate` regenerates the new workspace member
  row), 0040 (coherence split — direct precedent).
- Predecessor PR: #166 (closed pending this refactor;
  SessionStart hook + telemetry reapplied as a follow-up after
  this lands).
