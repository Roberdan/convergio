# AGENTS.md — convergio-tui

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0029-tui-dashboard-crate-separation.md](../../docs/adr/0029-tui-dashboard-crate-separation.md).

This crate is the **terminal dashboard** behind `cvg dash`. It is a
human-facing console: plans, active tasks, agents, and PRs in one
4-pane view, refreshing on a tick. It is consumed only by
`convergio-cli` (the `cvg` binary) and never by the daemon, the MCP
bridge, or another agent-facing surface.

## Invariants

- **Read-only on the daemon.** The TUI may issue `GET` against the
  HTTP API and shell out to `gh pr list`. It must never `POST`,
  `PUT`, or `DELETE`. State-changing commands belong in `cvg`
  subcommands, not in the dashboard. (Future: a vim-mode interactive
  evolution may add `:validate <plan>` / `:claim <task>`; that lives
  in a follow-up ADR.)
- **No business logic.** Everything rendered must be a 1:1 view of
  what the daemon returns. No client-side merging, no derived
  validation, no summary statistics that the daemon does not already
  expose. If a number is needed, expose it server-side first.
- **No `convergio-cli`/`convergio-server`/SQLite imports.** The
  client is a small reqwest wrapper. Dashboard code must compile
  in isolation against the HTTP contract documented in
  [../../ARCHITECTURE.md § HTTP surface](../../ARCHITECTURE.md).
- **Accessibility (CONSTITUTION P3).** The TUI must work on a
  basic 80×24 terminal without true-colour support. Information
  conveyed by colour must also be conveyed by symbol or label.
  Status text takes priority over decorations.
- **Respect the 300-line cap.** Each pane lives in its own file;
  rendering helpers, state, keymap, and tick loop are split by
  concern. The crate is designed to grow horizontally (one file
  per pane) rather than vertically (one mega-file).
- **No background loops, no spawned threads.** A single
  `tokio::time::interval` drives the refresh inside `run`. The TUI
  exits cleanly when `q` is pressed or the parent terminal is
  resized to a width below the layout's minimum.
- **i18n where it costs nothing (CONSTITUTION P5).** Localised
  strings flow through `convergio-i18n` if a translation key already
  exists for the same concept (e.g. status names). Pure layout
  labels (`Plans`, `Tasks`, `Agents`, `PRs`) start English-only and
  are migrated to Fluent in a follow-up if a non-English user
  reports it as a barrier.

## Module layout

| File | Owns |
|------|------|
| `src/lib.rs` | `pub fn run(daemon_url, tick_secs)` — entry point. Sets up the terminal, runs the event loop, restores the terminal on exit. |
| `src/state.rs` | `AppState` aggregating `Plans`, `Tasks`, `Agents`, `Prs`, plus selected pane / row offsets. |
| `src/client.rs` | `reqwest`-based fetcher: `GET /v1/plans`, `GET /v1/agents`, `GET /v1/audit/verify`, `gh pr list` shell-out. Read-only. |
| `src/tick.rs` | `tokio::time::interval` refresh loop with a graceful debounce. |
| `src/keymap.rs` | Keybinding dispatcher: `q` quit, `r` refresh-now, `Tab` change pane, `j/k` move row. |
| `src/render.rs` | Top-level layout (4-pane split + footer + header). Each pane delegates to `panes::*`. |
| `src/panes/plans.rs` | Plans pane: title, progress bar, breakdown counts, current selection highlight. |
| `src/panes/tasks.rs` | Active tasks pane: top N tasks with status colour + age + agent owner. |
| `src/panes/agents.rs` | Agents pane: id, kind, status (idle/working/terminated), last heartbeat. |
| `src/panes/prs.rs` | PRs pane: number, title, branch, CI conclusion. |

## Tests

Use `ratatui::backend::TestBackend` for snapshot-style buffer
assertions. No tokio runtime is needed for renderer tests; an
`AppState` fixture drives the render output. Tick-loop tests stay in
`tick.rs` with a paused tokio runtime.

E2E tests against a live daemon belong under
`crates/convergio-server/tests/` (cross-crate convention) when the
TUI gains action surfaces; today the TUI is read-only and the
`reqwest` client is exercised by integration smoke in
`tests/client.rs`.

## Where to put new behaviour

| You want to... | Put it in... |
|----------------|--------------|
| Add a new pane | New file under `src/panes/` + register it in `state.rs` (the `Pane` enum) and `render.rs` (the layout split). |
| Add a new keystroke | `src/keymap.rs` only — never inside a pane renderer. |
| Add a new fetched resource | `src/client.rs` (HTTP method) + `src/state.rs` (storage) — keep panes ignorant of HTTP. |
| Add a new action (state-changing) | This is out of scope for the read-only MVP. Open an ADR before adding write paths. |

## What this crate is NOT

- Not the cold-start brief — that is `cvg session resume`.
- Not the human snapshot — that is `cvg status --output human`.
- Not the agent JSON — that is `cvg status --output json` and the
  MCP bridge.
- Not a service or a daemon. The TUI is a one-shot client process.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-tui` stats:** 17 `*.rs` files / 75 public items / 2743 lines (under `src/`).

Files approaching the 300-line cap:
- `src/state.rs` (300 lines)
- `src/header_banner.rs` (266 lines)
<!-- END AUTO -->
