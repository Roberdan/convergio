# convergio-ontology

Ontology Runtime Core for Convergio (ADR-0053).

This crate is the platform-side primitive for typed domain objects.
It owns the schema registry — `ObjectType`, `LinkType`,
`PropertyType` — and the deterministic exporters that turn registered
schemas into JSON-Schema and SHACL.

Convergio ships **no built-in ontology**. Verticals
(`convergio-edu`, `convergio-research`, ...) register their domain
YAML against this registry at plan-create time. The crate is a
registrar, not a librarian.

See [`docs/adr/0053-ontology-runtime-core.md`](../../docs/adr/0053-ontology-runtime-core.md)
for the full decision record.

## Status

W1 — scaffold. No public API yet; follow the plan
*[core] Ontology Runtime W1: Runtime Core* for the rollout tasks.
