# convergio-ontology

Ontology Runtime Core for Convergio (ADR-0053).

This crate owns the platform-side schema registry for typed domain objects, links, and properties, deterministic JSON-Schema/SHACL exporters, bitemporal/object storage primitives, and typed action registry building blocks.

Convergio ships **no built-in ontology**. Verticals (`convergio-edu`, `convergio-research`, ...) register their domain YAML against this registry at plan-create time. The crate is a registrar, not a librarian.

See [`docs/adr/0053-ontology-runtime-core.md`](../../docs/adr/0053-ontology-runtime-core.md) for the core decision record.
