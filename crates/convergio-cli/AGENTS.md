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
**`convergio-cli` stats:** 77 `*.rs` files / 78 public items / 11908 lines (under `src/`).

Files approaching the 300-line cap:
- `src/commands/graph.rs` (300 lines)
- `src/commands/task.rs` (299 lines)
- `src/commands/discover.rs` (297 lines)
- `src/commands/fleet.rs` (297 lines)
- `src/commands/agent_list.rs` (293 lines)
- `src/commands/agent_spawn.rs` (293 lines)
- `src/commands/service.rs` (288 lines)
- `src/commands/setup_self_check.rs` (280 lines)
- `src/commands/plan.rs` (278 lines)
- `src/commands/setup.rs` (277 lines)
- `src/commands/status_render.rs` (272 lines)
- `src/commands/update_run.rs` (272 lines)
- `src/commands/doctor.rs` (269 lines)
- `src/commands/update_repo_root.rs` (268 lines)
- `src/commands/capability.rs` (256 lines)
- `src/commands/bus.rs` (255 lines)
- `src/commands/fleet_cleanup.rs` (252 lines)
<!-- END AUTO -->
