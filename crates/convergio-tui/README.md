# convergio-tui (`cvg dash`)

Terminal dashboard for the local Convergio daemon. A single 5-pane
console — Plans, Active Tasks, Agents, PRs, Bus — that refreshes on a
tick and gives you the state of the system at a glance.

```bash
cvg dash                            # default refresh 5s
cvg dash --tick-secs 2              # faster refresh
CONVERGIO_URL=http://host:8420 cvg dash  # remote daemon
```

## Layout

```
┌─ Plans (4 active) ────────────┬─ Active tasks (8) ───────────────────┐
│▶ W0b.2 cvg session pre-stop  │ T 5298055b in_progress  bus.inbound  │
│  1/8  [█·······]              │ T 564926dc in_progress  plan_pr     │
│  v0.3 Smart Thor              │ T abc12345 submitted    F32          │
│  0/29 [········]              │                                       │
├─ Agents (5) ─────────────────┼─ PRs (5 open) ────────────────────────┤
│ ◉ claude-code-roberdan idle  │ #92 hardening/mcp-e2e      CI:✗      │
│ ◉ claude-opus-overnight idle │ #93 hardening/lifecycle    CI:✓      │
│ ○ claude-code-demo-alpha idle│ #94 hardening/docs         CI:✓      │
└──────────────────────────────┴───────────────────────────────────────┘
 connected · refresh 5s · audit ✓ · q quit  r refresh  tab pane  j/k row
```

## Keys

| Key | Action |
|-----|--------|
| `q`, `Ctrl+C` | quit |
| `Enter` | drill-down into the focused pane (scope filter, or detail panel for Bus) |
| `Esc` | step back: leave detail / clear scope / quit when already at top |
| `r` | refresh now (skip tick wait) |
| `Tab` / `Shift+Tab` | next / previous pane |
| `j` / `k`, `↓` / `↑` | scroll within pane |
| `e` | toggle exited agents in the Agents pane |
| `t` | toggle terminal-status tasks (`done`/`failed`) in the Tasks pane |

## Configuration

| Variable | Default | Notes |
|----------|---------|-------|
| `CONVERGIO_URL` / `--url` | `http://127.0.0.1:8420` | daemon base URL |
| `CONVERGIO_DASH_TICK_SECS` / `--tick-secs` | `5` | refresh interval, clamped to `[1, 300]` |
| `CONVERGIO_DASH_NO_GH` | unset | when `1`, skip the `gh pr list` shell-out (PRs pane shows `gh disabled`) |

## What this is and what it is not

- **Is**: a read-only console for humans. Useful when running the
  daemon locally and wanting one window that summarises everything.
- **Is not**: an action surface. State-changing commands belong in
  `cvg` subcommands. The dashboard never `POST`s. (See
  [ADR-0029](../../docs/adr/0029-tui-dashboard-crate-separation.md)
  for why this is intentional.)
- **Is not**: a service. `cvg dash` is a one-shot foreground process.
  Quit with `q` and the terminal is restored.
- **Is not**: a replacement for `cvg status` (snapshot text/json),
  `cvg session resume` (cold-start brief), or the MCP bridge.

## Architecture

Split deliberately so each file stays under the 300-line cap and
each pane owns its own renderer:

| File | Responsibility |
|------|----------------|
| `src/lib.rs` | terminal setup/teardown + main event loop |
| `src/state.rs` | aggregate state (plans, tasks, agents, prs) + focus |
| `src/client.rs` | `reqwest` fetcher (read-only) + `gh` shell-out |
| `src/tick.rs` | refresh interval driver |
| `src/keymap.rs` | key dispatch |
| `src/render.rs` | top-level multi-pane layout + footer |
| `src/panes/plans.rs`  | Plans pane renderer |
| `src/panes/tasks.rs`  | Active tasks pane renderer |
| `src/panes/agents.rs` | Agents pane renderer |
| `src/panes/prs.rs`    | PRs pane renderer |
| `src/panes/bus.rs`    | Bus pane renderer |

Read [`AGENTS.md`](./AGENTS.md) before editing.

## Distribution

`convergio-tui` is a library crate consumed by `convergio-cli`. It
ships inside the `cvg` binary — there is no separate `cvg-dash`
executable. Install via:

```bash
sh scripts/install-local.sh
```

The same script that installs `convergio` (daemon), `cvg` (client),
and `convergio-mcp` (MCP bridge) also installs the TUI as part of
`cvg`.

## Status

- **Shipped**: 5 panes (Plans, Tasks, Agents, PRs, Bus), tick refresh,
  quit, scroll, `Enter`/`Esc` drill-down + scope filter, `e`/`t`
  visibility toggles, live SSE Bus tail with polling fallback.
- **Next** (separate ADR/PR if/when needed): `:command` interactive
  mode for state-changing actions.
