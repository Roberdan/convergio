# convergio-reports (local instructions)

## Responsibility

This crate owns the **report engine**:

- Persistent `ReportTemplate` definitions
- Renderers: HTML, PDF (lopdf), DOCX
- Embedded provenance: QR + canonical JSON manifest

## Boundaries / invariants

- Owns migrations **501–599** (ADR-0003).
- Report parameters must be validated against an ontology `ObjectType` JSON Schema.

## Validation

- `cargo test -p convergio-reports`
- `cargo clippy -p convergio-reports --all-targets -- -D warnings`
