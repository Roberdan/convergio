# AGENTS.md — convergio-thor

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

Thor is the reference validator over plans/tasks/evidence.

## Invariants

- Validation reports must be deterministic and explainable.
- Do not duplicate gate logic; call the owning layer or consume its
  results.
- A plan is not valid if tasks are completed without accepted evidence.
- Keep this crate provider-neutral.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-thor` stats:** 4 `*.rs` files / 11 public items / 582 lines (under `src/`).

Files approaching the 300-line cap:
- `src/pipeline.rs` (279 lines)
<!-- END AUTO -->
