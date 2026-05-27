# crates/convergio-provenance — AGENTS.md

## Responsibility

Emit W3C-PROV-JSON provenance bundles for ontology and audit mutations.

## Status

Working serialization crate: `emit_bundle()` validates identifiers and emits `wasGeneratedBy` and `wasAssociatedWith` relations; `to_prov_json()` serializes the bundle. Persistence, signing, and HTTP lookup live outside this leaf crate.

## Boundaries

- Leaf crate. **No** `convergio-db`, `convergio-durability`, or `convergio-server` deps.
- Schema follows W3C PROV-JSON.

## Invariants

- `#![forbid(unsafe_code)]`
- `#![warn(missing_docs)]` on every `pub` item.
- File-size cap 300 lines.

## Tests

`cargo test -p convergio-provenance`.
