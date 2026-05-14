# convergio-planner

Layer 4 reference planner.

`Planner::solve` turns a natural-language mission into one local
plan + tasks stored via Layer 1. Two backends ship in-tree and the
choice is controlled by `CONVERGIO_PLANNER_MODE`
(`auto` (default) | `opus` | `heuristic`):

- **Opus** — default when `claude` is on `PATH` (or when forced
  with `opus`). Spawns `claude -p --model opus --output-format json
  --permission-mode plan` (vendor CLI only, ADR-0032 / ADR-0036),
  pipes a structured prompt on stdin, parses the JSON response,
  and persists plan + tasks. The model picks `wave`, `sequence`,
  `runner_kind`, `profile` and `evidence_required` per task.
- **Heuristic** — deterministic line-split fallback. One task per
  non-blank line, all in wave 1, no `runner_kind` / `profile`
  override (durability falls back to daemon-wide defaults). Used
  automatically when `claude` is missing and explicitly when
  `CONVERGIO_PLANNER_MODE=heuristic` is set (CI, unit tests).

The crate is small on purpose: enough for a local quickstart and
easy to replace with your own client over the HTTP API.
