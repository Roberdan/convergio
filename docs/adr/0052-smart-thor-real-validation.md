---
status: accepted
date: 2026-05-25
deciders: roberdan
---

# ADR-0052 — Smart Thor: real validation, audited pipeline runs, and skip-when-trusted

- Status: accepted
- Date: 2026-05-25
- Deciders: Roberdan, Copilot CLI agent (autonomous v1.0 push)
- Workstream: W3 from `convergio-production-ready-plan.md`
- Supersedes: extends ADR-0012 (Thor as validator)

## Context

T3.02 ("Smart Thor") in the v1.0 plan asks for a Thor that runs the
project's *real* validation pipeline before promoting tasks
`submitted -> done`, not just verifies evidence shape. The pre-W3
implementation already had the seed: a single shell command in
`CONVERGIO_THOR_PIPELINE_CMD` is executed with timeout and tail
truncation. That covers *bring your own pipeline* — `make ci`,
`scripts/release-check.sh`, etc. — but the v1.0 plan listed three
gaps that operators kept hitting:

1. The default Rust workspace check (fmt + clippy + test) had to be
   re-typed in every `~/.zshrc`/`launchctl setenv` invocation. A
   missing step (no `-D warnings`) silently passed.
2. Thor wrote nothing to the audit log about what it actually ran.
   When validation passed, there was no after-the-fact way to prove
   "yes, the workspace really compiled at this seq". When it failed,
   the only evidence was the verdict's freeform reason string.
3. Fleet flows (one operator, ~5 git checkouts in parallel) ran the
   whole pipeline once per repo. The slowest step (cargo test on a
   large workspace) dominated wall-clock. There was no seam to say
   "this worktree's HEAD has already been validated end-to-end —
   trust the evidence and skip".

## Decision

Extend Thor (still single-crate, no new public traits exposed) with
three additive changes; preserve the existing shell-cmd seam.

1. **Built-in `cargo:auto` recipe.** Setting
   `CONVERGIO_THOR_PIPELINE_CMD=cargo:auto` runs a hardcoded
   three-step pipeline:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   Each step has the shared timeout. The first failing step short-
   circuits the rest and its name appears in the refusal reason
   (`pipeline_refused: clippy: ...`).
2. **`pipeline.run` audit row on every invocation.** Pass and fail
   both append one `EntityKind::Plan` audit row with transition
   `pipeline.run` and a canonical `RunReport` payload (recipe label,
   ok flag, optional failing-step name, duration_ms,
   steps_attempted). Reuses the existing hash chain — no schema
   change.
3. **Skip-when-trusted via `pipeline_run` evidence kind.** When
   `CONVERGIO_THOR_WORKTREE_REV` is set and every task being
   promoted carries an evidence row of kind `pipeline_run` whose
   `worktree_rev` field equals that env var, Thor skips the
   pipeline entirely and the verdict is `Pass`. This is the seam
   fleet workflows use to validate once per worktree instead of
   per plan.
4. **Configurable timeout.** `CONVERGIO_THOR_PIPELINE_TIMEOUT_SECS`
   overrides the 600s default for both `sh -c` and `cargo:auto`
   recipes; the public `Thor::with_pipeline_timeout` constructor is
   unchanged.

## Why not

- **A full `ProjectPipeline` trait abstraction.** The v1.0 plan
   listed a trait. With exactly two recipe shapes today (shell-cmd
   and cargo:auto) a Rust enum is simpler, equally testable, and
   keeps the public surface of `convergio-thor` empty of trait
   gymnastics. We can add the trait the day we ship a third recipe
   that needs runtime polymorphism — not before.
- **Storing the pipeline command in the daemon DB.** Operator-only,
   trusted-local config stays in the environment per the existing
   security boundary (see ADR-0012 § Trust). DB-stored commands
   would invert that boundary because any plan creator could then
   prompt-inject through them.
- **Auto-detecting the worktree revision via `git rev-parse`.**
   Considered but rejected: Thor runs inside the daemon, the
   worktree being validated may not be `cwd`, and shelling out to
   git introduces a new failure mode for an optimisation. Operator
   sets the env explicitly; absent env means skip is off.

## Consequences

- Operators who configured `make ci` keep working unchanged.
- New operators get a one-env-var path to a sane Rust pipeline.
- `pipeline.run` audit rows show up in `cvg audit events` and in
   the verifier output. The hash chain extends per the existing
   invariant.
- Fleet workflows can shave repeated full-workspace runs by
   attaching one `pipeline_run` evidence row per worktree HEAD.

## Tests

`crates/convergio-thor/tests/smart_pipeline.rs` (4 tests):

- audit row on pass,
- audit row on fail,
- skip honored on matching `worktree_rev`,
- skip refused on stale `worktree_rev`.

Existing `pipeline_hardening.rs` (timeout + truncation) and
`validate*.rs` (evidence-shape + wave) tests remain green — no
behaviour regression on the shell-cmd path.
