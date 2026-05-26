# AGENTS.md — convergio-ontology

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the **ontology DSL** and the **versioned schema registry**
primitives: `ObjectType`, `LinkType`, `PropertyType`, semver policy,
content hashing, and deterministic export.

## Invariants

- **No runtime IO in the library.** No filesystem reads/writes, no network,
  no daemon HTTP. Persistence and routes belong to other crates.
- **Deterministic output.** JSON-Schema export must be byte-stable for the
  same logical schema.
- **Semver enforced.** Registering a new schema version must validate the
  version bump against the computed change class (patch/minor/major).
- **Breaking changes require an explicit migration reference.**
- **English-only** user-facing text in this crate (P7).
- Keep files under **250 lines** (task constraint).

## Module layout

| File | Owns |
|------|------|
| `ids.rs` | Stable identifiers (`TypeId`), validation |
| `version.rs` | Strict semver (`SchemaVersion`) |
| `model.rs` | `ObjectType`, `LinkType`, `PropertyType` |
| `diff.rs` | Change classification between schema versions |
| `registry/` | In-memory schema registry + migration policy |
| `json_schema.rs` | Deterministic JSON-Schema export |
| `iri_mapping.rs` | CEDS/ELMO/ESCO IRI mapping table primitives |

## Tests

- Unit tests for semver parsing + ordering.
- Golden-ish tests for JSON-Schema determinism.
- Registry tests covering allowed/refused version bumps.
