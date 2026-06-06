---
adr: "0080"
title: "LLM-assisted ontology authoring (ontology-author)"
status: proposed
date: 2026-06-06
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0051", "0075", "0036", "0038", "0078", "0079"]
---

# ADR-0080 — LLM-assisted ontology authoring (ontology-author)

- **Status**: proposed
- **Date**: 2026-06-06
- **Deciders**: Roberto D'Angelo
- **Related**: ADR-0051 (Ontology Runtime Core), ADR-0075 (W3C-PROV
  provenance), ADR-0036 (planner via vendor CLI), ADR-0038 (embeddings),
  ADR-0078 (Postgres backend), ADR-0079 (Azure deployment)

## Context and Problem Statement

The Ontology Runtime Core (ADR-0051) stores typed ObjectTypes, links,
and properties, and exports SHACL and JSON-Schema. Today an ontology is
written **by hand**. For a vertical such as a university Student
Information System, the ontology is large (Student, Enrollment, Course,
CourseOffering, Assessment, Grade, Transcript, Credential, Issuer …) and
must align with domain standards (OneRoster, CEDS, EDCI/Europass).
Hand-authoring this is slow and error-prone, and it is the **root**
artifact from which the storage schema, the UI, and credential issuance
all derive.

We want to **automate the first draft** of a domain ontology from source
material, while staying true to the Convergio thesis: the machine must
**prove** its output, not just assert it. A bare "ask an LLM to draw an
ontology" wrapper is explicitly *not* what we are building.

## Decision Drivers

- The ontology is the foundation of every downstream artifact — getting
  it right cheaply has the highest leverage.
- LLM output is non-deterministic and must be constrained and validated.
- Provenance and auditability are the differentiator (ADR-0075, ADR-0002):
  every generated type must say where it came from.
- Reuse existing primitives (ontology runtime, embeddings, LLM gateway,
  provenance) rather than new infrastructure.
- Human-in-the-loop: an ontology is governance; it cannot auto-commit.

## Considered Options

1. **Manual authoring only** — status quo.
2. **Unconstrained LLM → free-text ontology** — fast, untrustworthy,
   off-thesis.
3. **Constrained, validated, provenance-tracked, human-gated authoring
   pipeline** (`convergio-ontology-author`). *(chosen)*

## Decision Outcome

Chosen option: **Option 3.** Add a leaf crate `convergio-ontology-author`
and a `cvg ontology author` command implementing this pipeline:

1. **Ingest** domain sources (standards: OneRoster, CEDS, EDCI/Europass;
   plus operator-supplied regulations). PDFs/DOCX are converted with
   **markitdown** (never LibreOffice), then chunked.
2. **Ground** the generation with `convergio-embed` retrieval over the
   chunks (ADR-0038), using the deterministic-test embedder in CI.
3. **Propose** via the vendor-agnostic **LLM gateway** (ADR-0036 spirit),
   with the model's output **constrained to the ontology runtime's own
   JSON-Schema** (ObjectType/property/link), not free text.
4. **Validate**: run the draft through SHACL + JSON-Schema export and a
   type/link-closure check; a bounded repair loop fixes violations or the
   run fails closed.
5. **Review**: present a human-readable diff + graph render (existing
   `convergio-ontology` capabilities); the operator approves explicitly.
6. **Commit** as a new ontology **version** (branching operator), with a
   **W3C-PROV bundle** (ADR-0075) on every generated type recording the
   source document(s) and the model used.

Determinism contract for CI: golden tests with a pinned embedder and a
recorded/stubbed model response, asserting the produced ObjectTypes,
links, and SHACL shapes for a fixed OneRoster+EDCI+CEDS seed.

Scope boundary: `ontology-author` produces a **draft for review**. It
never auto-commits, never bypasses gates, and is leaf-only (depends on
ontology + embed + llm-gateway + provenance; nothing depends on it).

### Positive consequences

- Drafting a domain ontology drops from days to minutes, then human-gated.
- Output is typed, SHACL-valid, provenance-tracked, versioned, auditable —
  the thesis applied to ontology design itself.
- Becomes the on-ramp for every future vertical (reusable ontology packs).

### Negative consequences

- LLM cost/latency per authoring run.
- Golden tests must be maintained as the runtime schema evolves.
- Quality of the draft depends on the quality of the seed standards.

## Pros and Cons of the Options

### Option 1 — manual only
- 👍 Full control, deterministic.
- 👎 Slow; the bottleneck for every vertical.

### Option 2 — unconstrained LLM
- 👍 Fastest to demo.
- 👎 No validation, no provenance, off-thesis; produces untrustworthy
  schemas.

### Option 3 — constrained pipeline (chosen)
- 👍 Fast *and* trustworthy; reuses existing primitives; human-gated.
- 👎 More moving parts; CI determinism work.

## Links

- Related ADRs: ADR-0051, ADR-0075, ADR-0036, ADR-0038, ADR-0078, ADR-0079
- Seed standards (PoC): OneRoster, EDCI/Europass, CEDS.
- v1 vertical scope: student records/grading + transcript/credential
  issuance (Verifiable Credentials).
