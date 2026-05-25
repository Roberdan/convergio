---
id: 0053
status: proposed
date: 2026-05-25
topics: [ontology, modulor, platform-vs-vertical, p4]
related_adrs: [0001, 0006, 0014, 0015, 0018]
touches_crates: [convergio-ontology, convergio-db, convergio-graph, convergio-api]
last_validated: 2026-05-25
---

# 0051. Ontology Runtime Core (`convergio-ontology` crate)

- Status: proposed
- Date: 2026-05-25
- Deciders: convergio maintainers
- Tags: ontology, modulor, platform-vs-vertical

## Context

Vertical accelerators built on Convergio (e.g. `convergio-edu`,
research, healthcare-compliance, public-sector workflow) each end
up re-implementing the same primitive: a **schema registry of
typed domain objects, links, and properties** with evolution rules
and machine-readable export. `convergio-edu` ADR-0020 made this
explicit for higher-ed; without an upstream primitive each
accelerator forks its own copy, which violates the urbanism
posture of ADR-0018 (city of buildings on shared codes).

The municipality already knows how to govern *task-shaped* state.
It does not know how to govern *domain-shaped* state. The Modulor
`(task, evidence, gate, audit_row)` needs a peer:
`(object, link, property, schema_version)`.

## Decision

Introduce a new core crate **`convergio-ontology`** that owns:

1. **Schema registry**
   - `ObjectType`, `LinkType`, `PropertyType` records persisted in
     SQLite (under the daemon, single-user, local-first per P2).
   - Stable string identifiers, semver-tracked `schema_version`,
     content-hash of the type spec recorded in `audit_log`.
   - Type evolution rules: additive changes are minor; renames /
     removals / type-narrowing require a `breaking` flag and a
     migration plan reference (a `plan` in the daemon).
2. **Canonical export**
   - Deterministic JSON-Schema export per `ObjectType`.
   - Deterministic SHACL shape export for linked-data interop.
   - Byte-identical re-export (same posture as ADR-0047
     `actions.json`).
3. **CLI surface** — `cvg ontology` subcommands:
   `register-type`, `list-types`, `describe`, `export`,
   `diff <from> <to>`, `validate <yaml>`.
4. **MCP surface** — typed action `ontology.describe` /
   `ontology.list` via `convergio.help` / `convergio.act`.
5. **No domain content in core.** Convergio ships **zero** built-in
   `ObjectType` instances. The crate is a registrar, not a
   librarian. Verticals (`convergio-edu`, ...) provide the schema
   YAML and register it at plan-create time.

## Decision Drivers

- Modulor extension: keep one atomic shape, applied to a new axis.
- Local-first (P2): SQLite, no remote schema service.
- Zero-debt (P1): build-time export, no runtime IO leaks.
- Urbanism (ADR-0018): platform owns the cadastre of types, not
  the buildings on top.
- Drift-aware (ADR-0014): the existing `convergio-graph` crate
  already knows how to track structural drift; we reuse its
  posture for ontology drift between schema versions.

## Considered Options

1. **Embed schema registry inside `convergio-db`.** Rejected:
   conflates persistence with semantics; couples migrations to
   schema evolution.
2. **External schema service (Postgres / dedicated daemon).**
   Rejected: violates P2 local-first and forces vertical
   accelerators into networked infrastructure.
3. **Per-accelerator schema crate.** This is the status quo and
   produces drift between accelerators; rejected.

## Compliance Anchors

- P1 zero-debt: registry export must be byte-identical and clean
  on `cargo build -- -D warnings`.
- P2 local-first: SQLite only, `127.0.0.1` only.
- P4 no scaffolding: a registered type must be reachable from the
  daemon API, the CLI, and the MCP surface.

## Rollout

- W1 (Wave 1 of the Ontology Platform plan family, see plan
  *Ontology Platform W1: Foundations*):
  - Crate scaffold + schema tables + audit-log hook.
  - JSON-Schema export + golden tests.
  - `cvg ontology` CLI subcommands + `--output json|human`.
  - MCP `ontology.*` actions registered in `actions.json`
    (ADR-0047).
  - Demo schema in `docs/examples/mini-ontology.yaml` used by
    integration tests; not shipped as production data.

## Consequences

- Verticals stop forking their own type registries; `convergio-edu`
  becomes a consumer of `convergio-ontology` and provides only
  CEDS/ELMO/EMREX/ESCO-aligned YAML.
- A new crate appears in the workspace; the docs-as-derived-state
  pipeline (ADR-0015) regenerates the members table.
- A new gate may later refuse evidence that references unknown
  `ObjectType` identifiers (deferred to a follow-up ADR).
- The daemon gains one more module on its API surface; we
  preserve the ADR-0043 ID/payload consistency rules.

## Alternatives left for verticals

- Domain-specific validation rules (e.g. CEDS conformance for
  edu) live in the vertical, not in `convergio-ontology`.
- Public ontology publication / CC-BY-SA licensing is a vertical
  decision (`convergio-edu` ADR territory).

## References

- ADR-0001 daemon + plans/tasks/evidence
- ADR-0006 CRDT actor/op store
- ADR-0014 code-graph crate
- ADR-0015 documentation as derived state
- ADR-0018 long-tail vertical accelerators
- ADR-0047 action type registry
- `convergio-edu` ADR-0020 (consumer)
