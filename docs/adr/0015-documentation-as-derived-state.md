---
id: 0015
status: accepted
date: 2026-05-01
topics: [documentation, reliability, drift, gates]
related_adrs: [0001, 0002, 0011, 0014]
touches_crates: [convergio-cli]
last_validated: 2026-05-01
implemented_in: [f52b52e, 1c82421]
---

# 0015. Documentation is derived state, not free text

- Status: accepted (implemented in main; see PR #45 — `f52b52e` ADR-0015 + `cvg docs regenerate` workspace_members)
- Date: 2026-05-01
- Deciders: Roberto, claude-code-roberdan
- Tags: documentation, reliability, drift, gates

## Context and Problem Statement

On 2026-05-01 a fresh agent reading the root `AGENTS.md` flagged
that the test count it claimed (`171 tests`) was off by ~100, and
that the workspace-layout block did not list `convergio-graph`
(shipped in PR #41 hours earlier). Both lies had been on `main`
for varying amounts of time. The CI gates noticed neither.

This is **not** a discipline failure. It is a **design** failure.
Convergio's whole product premise is that the agent's work fails
the gate when its evidence does not match its claim — but the
documentation an agent reads to *form* a claim is not subject to
any gate. The pipeline catches the agent's **output**; it does not
catch the agent's **input**.

`AGENTS.md` is the agent's compiler. If the compiler lies, every
downstream decision compounds.

## Decision Drivers

- **Reliability is structural, not behavioural.** Asking humans (or
  agents) to "remember to update the docs" is the same anti-pattern
  Convergio rejects elsewhere. Reliability comes from making the
  drift impossible, not from policing it.
- **Tier-2 already covers frontmatter.** `cvg coherence check`
  (T1.17) verifies ADR frontmatter against `workspace.members` and
  the README index. We have a precedent for declarative drift
  detection. The body needs the same treatment.
- **Tier-3 sees the code.** ADR-0014's graph engine knows the
  workspace's real shape — crates, modules, items. It is the
  natural source of truth for any doc claim about *what code
  exists*.
- **Composable layers, not new doc systems.** We do not want a
  documentation generator (Sphinx, mdBook, Antora). We want the
  existing markdown to stay honest. Solution must work in-place.

## Considered Options

1. **Status quo + discipline.** Reviewers catch drift in PR review.
   Has empirically failed: PR #41 added a crate, the doc table was
   not updated, no reviewer caught it.
2. **Auto-regen on every commit (lefthook).** A pre-commit hook
   re-renders all markdown derived sections. Always-current, but
   commits silently mutate files the human did not edit — surprising
   and noisy.
3. **Auto-regen on demand + CI gate (chosen).** Markdown declares
   derived sections via HTML comment markers. A `cvg docs regenerate`
   command rewrites them. CI runs `cvg docs regenerate --check` and
   fails if the committed content differs from the regenerated form.
   Same shape as `docs/INDEX.md` today (T1.16): file is committed,
   CI verifies it matches `./scripts/generate-docs-index.sh` output.
4. **Strip all derived state from markdown, render only on demand.**
   The agent calls `cvg docs render <file>` to read. No drift
   possible. But: human reviewers and `git blame` lose the rendered
   text, and IDE/GitHub previews stop being useful.

## Decision Outcome

Chosen option: **3 — auto-regen on demand + CI gate**, because it
preserves the existing markdown reading experience (`cat AGENTS.md`
still works, GitHub renders normally, `git blame` is meaningful)
while making derived state impossible to drift past CI.

The mechanism is HTML comment markers, the same syntax Markdown
already silently strips on render:

```markdown
<!-- BEGIN AUTO:workspace_members -->
- convergio-db (Layer 0 — sqlx pool + migrations)
- convergio-durability (Layer 1 — plans/tasks/evidence/audit/gates)
- ...
<!-- END AUTO -->
```

A new subcommand `cvg docs regenerate` walks the repo's `*.md`
files, finds every `<!-- BEGIN AUTO:<name> -->` ... `<!-- END AUTO -->`
block, looks up `<name>` in a registry of generators, and rewrites
the block contents. `--check` runs without writing and exits
non-zero if any block would change. CI wires this as advisory at
v0 (matches `cvg coherence check`) and as a hard gate after we have
data on false positives.

### Generator registry (v0)

| name | source | output shape |
|------|--------|--------------|
| `workspace_members` | `cargo metadata --no-deps` | bullet list with crate name + description from each `Cargo.toml` |

PR 14.3 (sibling to this ADR's implementation) extends the registry:

| name | source | output shape |
|------|--------|--------------|
| `test_count` | `cargo test --workspace -- --list` | one-line summary `{n} tests across {m} crates` |
| `cvg_subcommands` | `cvg --help` (parsing clap output) | bullet list of top-level verbs |
| `adr_index` | scan of `docs/adr/[0-9]*.md` frontmatter | the README index table |

Subsequent PRs add generators per discovered need; the registry is
a Rust trait object table inside `convergio-cli/src/commands/docs.rs`.

### Positive consequences

- **Drift becomes a compile error.** A PR that adds a crate without
  refreshing the workspace_members marker fails CI. Same UX as
  forgetting to commit a generated file.
- **The doc tells you what is auto.** A reviewer sees the marker
  and knows not to edit between them. The principle is visible in
  the file itself, no out-of-band convention.
- **Generators reuse the graph.** Most generators query
  `convergio-graph` (already shipped) — no new parser zoo, no
  duplicated traversal logic.
- **Layered with existing tiers.** Tier-1 (`docs/INDEX.md`) still
  manages the file map, Tier-2 (`cvg coherence check`) still
  verifies frontmatter, this is the body-level layer in between.

### Negative consequences

- **Marker hygiene.** Humans must not write inside the markers; the
  CI gate enforces but the friction is real.
- **Generator coverage debt.** Every new derived claim requires a
  generator entry. Manageable while the registry stays small (under
  ~20); needs review if it grows past that.
- **Bootstrapping cost.** Existing markdown must be retrofitted with
  markers around the sections that should be auto. v0 ships only
  one (`workspace_members` in `AGENTS.md` root) as proof; the rest
  follow incrementally as we touch each file.

## Pros and Cons of the Options

### Option 3 (chosen)

- 👍 Reuses the `docs/INDEX.md` pattern reviewers already understand.
- 👍 Same advisory → gate evolution path used elsewhere in the project.
- 👎 Markers are visual noise in the source markdown.

### Option 2 (rejected)

- 👍 Simpler: never out of date.
- 👎 Surprising: commits silently mutate files the author did not touch.

### Option 4 (rejected)

- 👍 Theoretically pure.
- 👎 Breaks the GitHub render → review experience that humans rely on.

## Migration plan

- **PR W4c (this ADR + first generator)**: ship ADR-0015 as
  `proposed`, ship `cvg docs regenerate` with the
  `workspace_members` generator only, add the marker pair around
  the workspace layout block in `AGENTS.md`, wire the CI advisory.
- **PR 14.3 (graph wave)**: add the `test_count`, `cvg_subcommands`,
  `adr_index` generators. Promote ADR-0015 to `accepted` once two
  generators have shipped without false positives.
- **Subsequent PRs**: marker-up `STATUS.md`, the per-crate
  `AGENTS.md` files, `ROADMAP.md` (where it lists shipped vs queued).
  Each is one trivial commit because the generator does the work.
- **v0.4** candidate: promote the CI step from advisory to hard
  gate. Same evolution as `cvg coherence check`.

## Drift policy

Derived state (AUTO blocks + `docs/INDEX.md`) is regenerated **on
demand** (developer or agent invokes `cvg docs regenerate` /
`./scripts/generate-docs-index.sh` intentionally) and via a **nightly
cron** on `main` (`.github/workflows/auto-blocks-drift.yml`). It is
**not** regenerated per-PR.

The original v0.4 plan promoted the CI gate from advisory to hard
fail (PRs #57 / #60 / #63). That model produced an O(N²) cascade on
serial merges: every PR carried freshly-regenerated AUTO blocks, and
the second of any pair to merge hit stale auto-blocks against the
post-merge `main`, failing CI. Retro 544e78cc P2-9 captured this:
P0.5 added a lefthook `pre-push` band-aid (#178) for the same pain.
Both the lefthook gate and the per-PR CI hard-fail are removed in
favour of the cron-reconciled model:

- Per-PR CI surfaces drift via the advisory `Docs drift (advisory)`
  job (`continue-on-error: true`). Drift is visible without blocking.
- The nightly cron regenerates and (eventually) opens a single
  drift-PR. Reconciliation cost is O(1) per day, not O(N²) per merge.
- This is the FIRST time Convergio explicitly accepts that committed
  derived state may be momentarily stale at merge time. The gain
  (eliminating the cascade) outweighs the cost (≤24h staleness).

## Links

- ADR-0014 (Tier-3 code graph) — supplies the data for most generators.
- T1.17 / `cvg coherence check` — the precedent for advisory-then-gate.
- T1.16 / `docs/INDEX.md` — the precedent for committed-but-derived
  artefacts under a CI freshness gate.
- PR W4a — the manual fix that motivated this ADR.
- Retro 544e78cc P2-9 — the cascade analysis that motivated the
  drift-policy update.
