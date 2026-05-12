# AGENTS.md — convergio-api

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the compact agent action contract used by MCP and future
adapters. It is not the daemon and must not perform any **runtime** IO —
the library compiled into adapters touches no files, sockets, or
databases. Build-time codegen (e.g. `build.rs` emitting `actions.json`)
is allowed and explicitly read-only against `src/`.

## Invariants

- Keep `convergio.help` and `convergio.act` as the stable agent surface.
- Add actions deliberately; every action becomes prompt/API surface area.
- Keep request/response schemas serializable, versioned, and documented.
- No runtime IO in the library: no daemon HTTP calls, no database
  access, no filesystem reads/writes, no business logic. Build-time
  generation of the `actions.json` registry from `Action::ALL` is
  permitted (it is the canonical mechanical mirror of the in-memory
  catalog, not new behavior).
- Dynamic capability actions must be namespaced and schema-validated.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-api` stats:** 5 `*.rs` files / 30 public items / 750 lines (under `src/`).

Files approaching the 300-line cap:
- `src/lib.rs` (250 lines)
<!-- END AUTO -->
