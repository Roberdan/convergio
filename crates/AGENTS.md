# AGENTS.md — crates

For repo-wide rules see [../AGENTS.md](../AGENTS.md).

Each crate has one responsibility. Do not solve cross-cutting problems by
turning one crate into a god module.

## Layer boundaries

| Crate | Boundary |
|-------|----------|
| `convergio-db` | database connection pool primitives (per-crate migrations live in their owning crate; see ADR-0003) |
| `convergio-durability` | plans, tasks, evidence, audit, gates |
| `convergio-bus` | persisted plan-scoped agent messages |
| `convergio-lifecycle` | spawning, heartbeat, process watching |
| `convergio-server` | HTTP routing only |
| `convergio-cli` | human/admin HTTP client |
| `convergio-tui` | terminal dashboard used by `cvg dash` |
| `convergio-api` | stable agent action schema |
| `convergio-mcp` | MCP bridge over the API schema |
| `convergio-graph` | task-scoped code graph retrieval |
| Layer 4 crates | reference planner/executor/validator behavior |

## Context rules

- Read the crate-local `AGENTS.md` before editing a crate.
- Keep files under the 300-line Rust cap.
- Add a new crate or module when a concept has a separate boundary.
- Do not add direct SQLite access to agent-facing crates.
- Do not let CLI/MCP bypass daemon gates.
