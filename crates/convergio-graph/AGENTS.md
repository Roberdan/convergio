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
- **Eager build, no lazy refresh (yet).** `cvg graph build` is the
  authoritative refresh path. Queries read whatever the last build
  wrote — they do **not** re-parse stale files inline. Lazy on read
  and a background refresh loop (e.g. a `CONVERGIO_GRAPH_REFRESH_SECS`
  knob) are future work; do not document them as shipped until the
  code lands.
- **No daemon dependency for parsing.** The parser runs in any
  process; persistence requires the SQLite pool from `convergio-db`.
- **No script glue.** Every operation surfaces as a `cvg graph ...`
  subcommand or a `/v1/graph/*` HTTP route. Bash wrappers are
  banned by AGENTS.md root rules.

## Module layout

| File | Owns |
|------|------|
| `parse.rs` | syn walker; produces `Vec<Node>` + `Vec<Edge>` from a single `*.rs` file |
| `meta.rs` | `cargo_metadata` wrapper; produces crate-level dependency edges |
| `doc_link.rs` | Markdown YAML frontmatter → ADR/doc `claims`/`mentions` edges |
| `build.rs` | Top-level orchestrator: walks the workspace, calls `meta` + `parse` + `doc_link`, persists via `store` |
| `store.rs` | SQLite read/write of nodes and edges, mtime-aware refresh |
| `model.rs` | `Node`, `Edge`, `NodeKind`, `EdgeKind`, `BuildReport` |
| `query.rs` | Read-side `for_task_text` + `ContextPack` |
| `drift.rs` | `cvg graph drift` + `DriftReport` |
| `cluster.rs` | Community detection + `ClusterReport` |

## Tests

Unit tests live alongside each module under `#[cfg(test)]`.
Integration tests live in `tests/store.rs` and `tests/query.rs`;
each boots a tempdir SQLite via `convergio_db::Pool` and seeds the
graph store inline. There is no `tests/fixtures/` directory — keep
inline fixtures small (one struct, one fn, two ADR nodes) so the
suite stays under a second.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-graph` stats:** 19 `*.rs` files / 53 public items / 2991 lines (under `src/`).

Files approaching the 300-line cap:
- `src/cluster.rs` (268 lines)
- `src/drift.rs` (254 lines)
<!-- END AUTO -->
