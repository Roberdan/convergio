# AGENTS.md — convergio-api

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the compact agent action contract used by MCP and future
adapters. It is not the daemon and must not perform IO.

## Invariants

- Keep `convergio.help` and `convergio.act` as the stable agent surface.
- Add actions deliberately; every action becomes prompt/API surface area.
- Keep request/response schemas serializable, versioned, and documented.
- Do not add daemon HTTP calls, database access, or business logic here.
- Dynamic capability actions must be namespaced and schema-validated.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-api` stats:** 3 `*.rs` files / 24 public items / 663 lines (under `src/`).

Files approaching the 300-line cap:
- `src/action.rs` (289 lines)
<!-- END AUTO -->
