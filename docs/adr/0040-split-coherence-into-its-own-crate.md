---
id: 0040
status: accepted
date: 2026-05-03
topics: [legibility, refactor, cross-cutting]
related_adrs: [0013, 0014, 0015, 0039]
touches_crates: [convergio-cli, convergio-coherence]
last_validated: 2026-05-03
---

# 0040. Split the coherence verifiers into their own crate

- Status: accepted
- Date: 2026-05-03
- Deciders: Roberto, claude-code-roberdan
- Tags: legibility, refactor, cross-cutting

## Context and Problem Statement

`convergio-cli` shipped at 10978 lines — eleven lines below the
11000-line per-crate hard cap (CONSTITUTION § 13). The deferred
ADR-status verifier work (PR #144) needed roughly 525 additional
lines, which would have pushed the crate to 11583 — over cap.

The cap is the cap. The right move is to split, not to bump the
ceiling. The natural seam is the `cvg coherence` suite:

- 7 source files, 1422 lines total, all colocated under
  `crates/convergio-cli/src/commands/coherence*.rs`.
- Pure local verifiers — they walk the repo, read files, and emit
  a JSON / human / plain report. No HTTP, no SQLite, no daemon
  dependency. The only crate they reach for is `convergio-i18n`
  (for localized messages).
- Reusable from other entry points: skills, runners, future MCP
  bridges. Today they are reachable only by going through the
  `cvg` clap dispatcher, which is wasteful for non-CLI callers.

ADR-0013 sets the precedent for this kind of cross-cutting split
(durability into three seams). ADR-0039 made it policy that
`cvg coherence check` runs before any docs-touching PR merges, so
keeping the verifiers cheap to embed in any agent loop is now a
load-bearing concern.

## Decision Drivers

- **CONSTITUTION § 13 — 11000 lines per crate, hard cap.**
  Refactoring is cheaper than living forever in cap-watch mode.
- **Legibility (CONSTITUTION § 16).** A 9.5k-line CLI is already
  larger than the 5000-line soft cap; pulling the coherence suite
  out shaves ~1.4k off and isolates an internally cohesive seam.
- **Reusability.** Skills and the upcoming `convergio-mcp` action
  surface should be able to call the verifiers as a library, not
  shell out to `cvg`.
- **No behavioural change.** The split must be observably a
  no-op: `cvg coherence check --output json` must produce
  byte-identical output before and after, modulo the new crate's
  own AGENTS.md / README.md and the new ADR file.
- **Test parity.** Every existing test moves with its module; no
  test is rewritten or skipped to make the move pass.

## Considered Options

1. **Bump the per-crate cap to 12k.** *Cheapest in PR diff, but
   gives up the only mechanism keeping the CLI legible.*
2. **Inline-split inside `convergio-cli`** (e.g. add a
   `commands::coherence::*` submodule tree without a new crate).
   *Saves one Cargo.toml but does not move the LOC out of the
   crate. Cap problem unsolved.*
3. **Extract `convergio-coherence` (chosen).** Move the seven
   files to a new cross-cutting crate; keep a thin shim in the
   CLI so the dispatcher line in `main.rs` is unchanged.
4. **Move into `convergio-graph`.** Tier-3 retrieval already lives
   there. *Coupling a doc-coherence verifier to the code-graph
   crate is wrong on layering: the graph is for code, the
   verifiers are for docs.*

## Decision Outcome

Chosen option: **Option 3 — extract `convergio-coherence`**.

### Target topology

```
convergio-coherence (~1500 LOC)
├─ coherence.rs          public entry: CoherenceCommand + run
├─ body.rs / body_scan.rs body drift detector + line scanners
├─ parse.rs              ADR frontmatter + index + workspace parsers
├─ routes.rs             cvg coherence routes orchestrator
├─ routes_parse.rs       axum / ARCHITECTURE.md / AGENTS.md parsers
└─ routes_diff.rs        three-bucket drift diff
```

`convergio-cli` keeps `commands/coherence.rs` as a 30-line shim
that re-exports `CoherenceCommand`, converts the CLI's
`OutputMode` into the new crate's enum, and dispatches.

### Dependency direction

```
convergio-i18n ─< convergio-coherence ─< convergio-cli
```

No back-edges. The new crate has no `convergio-cli` knowledge.

### Positive consequences

- `convergio-cli` drops from 11000 to ~9550 lines. Headroom for
  the deferred ADR-status verifier (PR #144 reapplied as a
  follow-up).
- The verifiers are agent-callable from any binary that adds
  `convergio-coherence` to its `Cargo.toml`. Skills no longer
  need to shell out to `cvg`.
- Independent test surface — `cargo test -p convergio-coherence`
  runs only the coherence tests, not the entire CLI.
- Mirrors ADR-0013's precedent for splitting cross-cutting
  concerns out of the host crate.

### Negative consequences

- One additional `Cargo.toml`, one additional `AGENTS.md`, one
  additional CI build target.
- The CLI's `OutputMode` enum now exists in two places (CLI's
  one, the new crate's one); the shim converts between them.
  Acceptable: the alternative is a `convergio-cli` dep on
  `convergio-coherence` that re-exports the enum, which inverts
  the layering we just established.

## Migration plan

One PR (this one). Squash-merge is disabled per CONSTITUTION § 15
so the file moves are visible in history.

1. Create `crates/convergio-coherence/` with `Cargo.toml`,
   `AGENTS.md`, `CLAUDE.md` (symlink), `README.md`, `src/lib.rs`.
2. `git mv` the seven `coherence*.rs` files. Rename short:
   `coherence_body.rs` → `body.rs`, etc.
3. Rewrite `super::coherence_*` and `super::OutputMode`
   references to the new short module names + `crate::OutputMode`.
4. Replace `crates/convergio-cli/src/commands/coherence.rs` with
   the shim.
5. Add `convergio-coherence` to workspace members + CLI deps.
6. Run `cvg docs regenerate` and `scripts/generate-docs-index.sh`.
7. Verify `cvg coherence check --output json` and
   `cvg coherence routes --output json` are functionally
   identical (the doc index gains one new crate row, the new
   AGENTS.md / README.md add three new docs to the body scan
   — these are expected deltas, not regressions).

## Pros and Cons of the Options

### Option 3 (chosen)

- Good: respects the per-crate cap without raising it.
- Good: verifiers become embeddable, not just callable from
  `cvg`.
- Good: matches ADR-0013's pattern.
- Bad: small `OutputMode` duplication at the shim.

### Option 1 (rejected)

- Good: zero PR cost.
- Bad: caps exist for a reason (CONSTITUTION § 13). Bumping at
  the first squeeze is how soft caps die.

### Option 2 (rejected)

- Good: single-Cargo.toml move.
- Bad: does not solve the cap problem; just shuffles files.

### Option 4 (rejected)

- Good: one fewer crate.
- Bad: `convergio-graph` is Tier-3 code retrieval; doc coherence
  is a different layer. Wrong home.

## Links

- Convergio plan: `ee9ab55a-eb28-41ee-bd02-e22d002e8b3c`.
- Convergio task: `76fa5d30-2e98-471a-b468-18a72f942db4`.
- Related ADRs: 0013 (durability split — pattern reused), 0014
  (Tier-3 code graph — sibling cross-cutting crate), 0015 (docs
  as derived state — `cvg docs regenerate` regenerates the new
  workspace member row), 0039 (doc coherence sweep policy —
  consumer of the verifiers).
- Predecessor PR: #144 (closed pending this refactor; ADR-status
  verifier reapplied as a follow-up after this lands).
