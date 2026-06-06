# Ontology Platform — reference connectors (capability payloads)

This folder contains **two reference connector capability payloads** used by the Ontology Platform W6 workstream:

- `connector-sis-ethos` — SIS via Banner Ethos REST (reference)
- `connector-canvas-rest-lti13` — Canvas LMS via REST + LTI 1.3 (reference)

Each connector includes:

- an **ontology mapping YAML** with **per-field** `lawful_basis` and `dpa_reference`
- a minimal **DPA reference** doc for auditors/reviewers

These payloads are **installable** in Convergio’s local capability registry as signed `.tar.gz` packages.
They are *not yet runnable as live connectors* (Connector SDK runtime is still proposed; see ADR-0057).
