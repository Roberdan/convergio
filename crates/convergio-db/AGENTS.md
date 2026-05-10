# AGENTS.md — convergio-db

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns SQLite connection and migration primitives only.

## Invariants

- SQLite is the only supported database for the local product.
- Keep this crate free of domain logic.
- Do not introduce Postgres/team/tenant abstractions.
- Migrations belong to the crate that owns the table semantics.
- Connection helpers must be safe for concurrent local agents.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-db` stats:** 3 `*.rs` files / 8 public items / 240 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
