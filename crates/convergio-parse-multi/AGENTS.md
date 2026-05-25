# AGENTS.md — convergio-parse-multi

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md](../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md).

This crate owns multi-language AST parsing for the fleet retrieval layer
(ADR-0038, F2). It wraps `tree-sitter` grammars for TypeScript and
Python behind a uniform `Lang`/`parse` interface, so the fleet graph
builder can extract nodes from heterogeneous repositories without
handling grammar internals.

## Invariants

- **Three entry points: `parse()`, `parse_ts()`, `parse_py()`.**
  `parse()` is the lightweight Rust path returning top-level
  [`ParsedNode`]s; `parse_ts()` and `parse_py()` are graph-shaped
  paths emitting `(Vec<Node>, Vec<Edge>)` directly for the fleet
  graph builder. Callers never instantiate `tree_sitter::Parser`;
  grammar selection is internal.
- **Top-level nodes only (lightweight path).** `parse()` returns only
  direct children of the root. Recursive traversal belongs to the
  fleet graph builder.
- **Partial-parse on syntax errors.** Every entry point logs `warn!`
  and continues when tree-sitter reports `root.has_error()`; there
  is no `SyntaxError` variant. Callers receive the best-effort node
  set so a single malformed file cannot blank an entire build.
- **No DB access.** This crate is pure parsing; persistence lives in
  `convergio-graph` and `convergio-fleet`.
- **Migration range 900-999** reserved by ADR-0003 for this crate.
  No tables exist in F2-1 (bootstrap); the first real migration lands
  in a later F2 task.
- **`#![forbid(unsafe_code)]`** — tree-sitter FFI is behind the
  grammar crates; this crate never crosses an unsafe boundary.

## Module layout

| File | Owns |
|------|------|
| `lang.rs`    | [`Lang`] discriminant + grammar resolution |
| `node.rs`    | [`ParsedNode`] + [`NodeKind`] |
| `parse.rs`   | [`parse()`] — lightweight Rust bytes → nodes |
| `ts.rs`      | [`parse_ts()`] — TypeScript → graph nodes/edges |
| `py.rs`      | [`parse_py()`] — Python → graph nodes/edges |
| `error.rs`   | [`ParseError`] |
| `migrate.rs` | Migration entry point (900-999, ADR-0003) |

## Tests

- Per-module `#[cfg(test)] mod tests` covers grammar loading and
  kind mapping for `parse.rs` and `lang.rs`. `ts.rs` and `py.rs`
  rely on integration tests under `tests/` (real source fixtures).

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-parse-multi` stats:** 9 `*.rs` files / 21 public items / 928 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
