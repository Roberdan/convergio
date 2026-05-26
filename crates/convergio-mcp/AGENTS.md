# AGENTS.md — convergio-mcp

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the stdio MCP bridge. It adapts agent tools to the daemon;
it is not an orchestrator and not a raw HTTP proxy.

## Invariants

- Expose only `convergio.help` and `convergio.act`.
- Keep prompts compact; put durable context in Convergio, not in tool
  descriptions.
- All state-changing actions go through the daemon HTTP API.
- Log diagnostics without leaking secrets.
- Capability actions must remain namespaced behind `convergio.act`.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-mcp` stats:** 16 `*.rs` files / 0 public items / 2154 lines (under `src/`).

Files approaching the 300-line cap:
- `src/e2e_tests.rs` (291 lines)
- `src/help_actions.rs` (288 lines)
- `src/actions.rs` (280 lines)
<!-- END AUTO -->
