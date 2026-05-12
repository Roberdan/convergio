# AGENTS.md — convergio-graph

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0014-code-graph-tier3-retrieval.md](../../docs/adr/0014-code-graph-tier3-retrieval.md).

This crate is the Tier-3 retrieval layer: a syn-based parser of the
workspace, persisted in SQLite, queryable for context-pack
generation, cluster detection, and ADR/code drift.

## Invariants

- **syn parse-only.** No name resolution, no type resolution, no
  macro expansion. Records what is written, not what it means. Users
  needing deeper semantics layer rustdoc JSON on top in v1.
- **SQLite-only persistence.** Schema in `migrations/0600_*.sql`.
  Migration range 600-699 (ADR-0003).
- **Lazy on read.** Queries compare file mtime to the parsed-at
  timestamp and re-parse stale nodes inline. Background loops are
  opt-in (`CONVERGIO_GRAPH_REFRESH_SECS`).
- **No daemon dependency for parsing.** The parser runs in any
  process; persistence requires the SQLite pool from `convergio-db`.
- **No script glue.** Every operation surfaces as a `cvg graph ...`
  subcommand or a `/v1/graph/*` HTTP route. Bash wrappers are
  banned by AGENTS.md root rules.

## Module layout

| File | Owns |
|------|------|
| `parse.rs` | syn walker; produces `Vec<Node>` + `Vec<Edge>` from a single `*.rs` file or a crate root |
| `meta.rs` | `cargo_metadata` wrapper; produces crate-level dependency edges |
| `doc_link.rs` | Markdown frontmatter + grep-based ADR↔crate edges |
| `store.rs` | SQLite read/write of nodes and edges, mtime-aware refresh |
| `refresh.rs` | Optional daemon loop; lazy-on-read API used by `for_task` |
| `model.rs` | `Node`, `Edge`, `ContextPack`, `DriftReport`, `ClusterReport` |

## Tests

E2E tests live under `tests/`. Each test boots a tempdir SQLite via
`convergio-db::Pool` and runs the parser against a fixture crate at
`tests/fixtures/`. Keep fixtures small (one struct, one fn) so the
test suite stays under a second.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-graph` stats:** 16 `*.rs` files / 51 public items / 2868 lines (under `src/`).

Files approaching the 300-line cap:
- `src/parse.rs` (284 lines)
- `src/drift.rs` (277 lines)
- `src/cluster.rs` (268 lines)
<!-- END AUTO -->
