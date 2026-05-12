# AGENTS.md — convergio-db

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns SQLite connection primitives only ([`Pool`], backend
detection, sqlite-vec extension registration). It deliberately does
**not** own a shared migration runner: per ADR-0003, each owning
domain crate ships and runs its own migrations against the shared
pool, using `sqlx::migrate!` directly. Keep it that way unless a
future ADR explicitly introduces a shared migration primitive here.

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
**`convergio-db` stats:** 3 `*.rs` files / 8 public items / 242 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
