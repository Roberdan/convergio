# AGENTS.md — convergio-cli

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

`cvg` is a human/admin HTTP client. The daemon remains the source of
truth.

## Invariants

- Do not import server crates or write SQLite directly.
- Keep output accessible and useful without color.
- User-facing strings must go through i18n where the command is localized.
- `--output human|json|plain` should be extended consistently.
- CLI convenience must not bypass server-side gates or audit.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-cli` stats:** 55 `*.rs` files / 40 public items / 8418 lines (under `src/`).

Files approaching the 300-line cap:
- `src/commands/graph.rs` (300 lines)
- `src/main.rs` (300 lines)
- `src/commands/update_repo_root.rs` (292 lines)
- `src/commands/fleet.rs` (289 lines)
- `src/commands/service.rs` (288 lines)
- `src/commands/setup.rs` (273 lines)
- `src/commands/status_render.rs` (272 lines)
- `src/commands/update_run.rs` (272 lines)
- `src/commands/doctor.rs` (259 lines)
- `src/commands/bus.rs` (257 lines)
- `src/commands/capability.rs` (256 lines)
<!-- END AUTO -->
