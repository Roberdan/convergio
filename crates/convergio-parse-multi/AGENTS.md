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

- **`parse()` is the single entry point.** Callers do not instantiate
  `tree_sitter::Parser` directly; grammar selection is internal.
- **Top-level nodes only.** `parse()` returns only direct children of
  the root. Recursive traversal belongs to the fleet graph builder.
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
| `parse.rs`   | [`parse()`] — core bytes → nodes routine |
| `error.rs`   | [`ParseError`] |
| `migrate.rs` | Migration entry point (900-999, ADR-0003) |

## Tests

- Per-module `#[cfg(test)] mod tests` covers grammar loading, node
  extraction, and kind mapping.
- Integration tests live in `tests/` (cargo convention).

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-parse-multi` stats:** 8 `*.rs` files / 20 public items / 816 lines (under `src/`).

Files approaching the 300-line cap:
- `src/py.rs` (291 lines)
<!-- END AUTO -->
