# AGENTS.md — convergio-planner

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the reference planner. It proves the loop; it is not the
final AI planning brain.

## Invariants

- Keep output deterministic and testable.
- Plans/tasks should be small enough for workers to understand.
- Do not embed provider-specific prompts or hosted assumptions.
- Future advanced planners should be capabilities unless they are core
  coordination logic.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-planner` stats:** 6 `*.rs` files / 16 public items / 603 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
