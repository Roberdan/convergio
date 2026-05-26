---
id: 0064
status: accepted
date: 2026-05-26
topics: [vision, product-strategy, eu-sovereignty, ontology, compliance]
related_adrs: [0016, 0017, 0018, 0051, 0054]
touches_crates: []
last_validated: 2026-05-26
---

# 0064. Pivot to an EU-sovereign, AI-native, local-first open ontology platform

- Status: accepted
- Date: 2026-05-26
- Deciders: Roberdan
- Tags: vision, product-strategy, eu-sovereignty, ontology, compliance

## Context and Problem Statement

ADR-0016 reframed Convergio from “leash” to “shovel”: the runtime that makes the long tail of
vertical AI accelerators shippable. That framing remains true, but it is insufficient for the
procurement and compliance reality of the EU public sector and regulated markets.

In those markets, the product expectation is increasingly **Palantir-shaped** (ontology-backed,
provenance-rich, purpose-bound, audit-grade) *and* simultaneously blocked from Palantir itself
by sovereignty, vendor-lock-in, and geopolitical procurement constraints. The opportunity is
therefore not “Palantir, but cheaper”. It is **“Palantir-good primitives, but open, local-first,
and EU-sovereign by construction”**.

Convergio already ships the enforcement spine: gates that refuse work (HTTP 409), a tamper-evident
hash-chained audit log (ADR-0002), and a locality posture (single-user, SQLite-only). The missing
piece is the explicit product decision: Convergio becomes a **platform** for ontology-native
verticals while keeping the leash as the safety belt.

## Decision Drivers

- **EU AI Act enforcement (2026)**: auditability, transparency obligations, and documented
  risk-management processes become table stakes for AI-adjacent systems.
- **GDPR Art. 5(1)(b) purpose limitation**: regulated data handling needs a first-class
  “why” dimension, not only “who/what/when”. (Purpose registry is a platform primitive; see ADR-0054.)
- **NIS2 / DORA expectations**: tamper-evident logs, traceability, and operational resilience are not
  optional for critical and public-sector systems.
- **EU public-sector procurement reality**: local-first and sovereignty requirements frequently
  disqualify opaque hosted platforms and foreign-controlled control planes.
- **Long-tail throughput requires shared semantics**: without an ontology core, every vertical
  re-invents a schema registry, provenance story, and compliance narrative, fragmenting the city
  (ADR-0018 urbanism).
- **No marketing-first drift**: the pivot must preserve Convergio’s “mechanical enforcement” posture
  (gates + audit), not become a slide deck.

## Considered Options

1. **Option A — Stay leash-only**: keep Convergio as a gate/audit leash and let ontology/platform
   concerns live entirely in vertical accelerators.
2. **Option B — Go “full ontology platform” immediately**: treat Convergio as a platform first,
   invest into ontology/provenance/purpose surfaces as the primary product, and downplay the leash.
3. **Option C — Layered product: leash as safety belt + ontology as platform (chosen)**: keep the
   leash framing as the runtime enforcer, and explicitly pivot the product to a sovereign, open
   ontology platform that verticals build on.

## Decision Outcome

Chosen option: **Option C**, because it preserves what Convergio already proves in practice
(runtime refusal + audit) while making the platform primitives explicit and governable via ADRs.
This is the only option that is both credible to EU regulated procurement and consistent with the
urbanism posture of ADR-0018.

### Positive consequences

- Convergio’s product is now legible to EU markets: *local-first, sovereign, audit-grade ontology
  platform*, not “yet another guardrail tool”.
- Platform primitives (ontology runtime core, provenance bundles, purpose registry) become shared
  municipal services rather than per-vertical forks (ADR-0051, ADR-0054).
- Compliance requirements (purpose limitation, provenance export, tamper-evidence) are expressed as
  enforceable building codes, not “best effort” guidelines.

### Negative consequences

- Scope pressure increases: “platform” implies a larger surface area (CLI, HTTP routes, evidence
  kinds, gates) and raises expectations.
- Requires strict documentation honesty: “pivot” must not be written as shipped features. Anything
  not implemented remains explicitly future work (docs/AGENTS.md rule).
- Increases the burden of coherence work: README, vision, compliance docs, and licensing language
  must be kept consistent.

### Neutral consequences

- The five sacred principles do not change; the pivot narrows *who we serve* and *how we frame the
  product*, not the enforcement posture.
- Local-first remains the differentiator: sovereignty is achieved by locality + auditable exports,
  not by “EU cloud”.

## What changes (committed by this decision)

These are *product-direction* changes. Where an item is not yet implemented, it is tracked as
follow-up work (not silently implied).

- **`docs/vision.md`** references this ADR and names the EU-sovereign pivot as load-bearing.
- **`README.md`** is updated to reflect the platform framing (leash + ontology) and the target
  market constraints (follow-up).
- **`COMPLIANCE.md`** is added/updated to anchor AI Act, GDPR purpose limitation, NIS2/DORA posture
  to concrete mechanisms (audit chain, provenance bundles) (follow-up).
- **License posture** is reviewed for EU public-sector adoption constraints (follow-up; do not
  change license text without an explicit licensing ADR).

## What does NOT change

- **Local-first, single-user, SQLite-only** architecture.
- The **five sacred principles** (P1–P5) as non-negotiable building codes.
- The enforcement spine: **evidence → gates → HTTP 409 refusals → audit chain**.
- “No scaffolding only”: platform primitives must be fully wired when shipped.

## Links

- Related ADRs:
  - ADR-0016 long-tail shovel framing: `docs/adr/0016-long-tail-vertical-accelerators.md`
  - ADR-0017 ISE/hve alignment: `docs/adr/0017-ise-hve-alignment.md`
  - ADR-0018 urbanism frame: `docs/adr/0018-urbanism-over-architecture.md`
  - ADR-0051 ontology runtime core: `docs/adr/0051-ontology-runtime-core.md`
  - ADR-0054 provenance bundle + purpose registry: `docs/adr/0054-provenance-bundle-purpose-registry.md`
- Task: Tb1f00b74-e6ee-4134-87df-4c8fafdae08c
