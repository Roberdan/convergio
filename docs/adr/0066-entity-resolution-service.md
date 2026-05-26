---
id: 0066
status: proposed
date: 2026-05-25
topics: [ontology, entity-resolution, explainability]
related_adrs: [0022, 0051, 0052, 0054]
touches_crates: [convergio-api]
last_validated: 2026-05-25
---

# 0066. Entity Resolution Service with Explainability

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, entity-resolution, explainability

## Context

Once the ontology runtime (ADR-0062) exists, accelerators that
ingest from multiple sources — `convergio-edu` pulling from SIS
+ LMS + identity provider; future verticals pulling from CRM +
ERP + public registers — face the classic entity-resolution
problem: are "Maria Rossi (SIS:42)", "M. Rossi (LMS:user-991)",
and "rossi.m@uni.it (IdP:7c)" the same `Person` object?

Today each accelerator reinvents this, often as a one-off
script. The result: silent duplicates, silent merges, no audit
trail of why two records were unified, no path to *unmerge* when
a merge was wrong. All three failure modes are unacceptable in
regulated domains.

## Decision

Add an upstream **Entity Resolution (ER) service** as part of
the `ontology (future)` crate (or a sibling
`ontology-er (future)` if the surface grows large enough; the
crate split is deferred to implementation).

1. **Generic primitive — zero domain knowledge.**
   - Inputs: two candidate objects of the same `ObjectType`,
     plus a `MatchSpec` (set of property comparators with
     weights, plus optional blocking keys).
   - Output: a `MatchProposal`
     `{score, comparator_breakdown, recommended_action:
       merge|hold|split, rationale}`.
2. **Three resolution modes**, picked per `ObjectType`:
   - **Deterministic**: exact match on declared keys.
   - **Probabilistic**: Fellegi-Sunter-style weighted scoring,
     thresholds declared per type.
   - **Hybrid**: deterministic first, probabilistic for the
     residue.
3. **Explainability is non-negotiable.**
   - Every `MatchProposal` carries the per-comparator
     contribution, the threshold that was applied, and the
     blocking key used.
   - Stored alongside the resulting merge/unmerge action
     (ADR-0063) so lineage (ADR-0064) and provenance
     (ADR-0065) trace it end-to-end.
4. **Merge is a typed action with a guaranteed undo.**
   - `ontology.entity.merge` is a typed action with an
     explicit compensation `ontology.entity.unmerge`
     (ADR-0048 / ADR-0063), keyed by the merge proposal hash.
5. **Adversarial-review hook (ADR-0022).**
   - High-score-but-low-confidence merges (configurable band)
     are forwarded to the adversarial-review service before
     execution; refused proposals are audited.
6. **No built-in MatchSpecs in core.** Verticals supply them
   per `ObjectType`. Convergio ships only the engine + a
   reference YAML format + golden tests on a synthetic
   dataset.

## Decision Drivers

- ER is foundational for any multi-source ontology; making
  every vertical re-invent it is exactly the kind of drift
  ADR-0018 warns against.
- Explainability is a regulatory and trust requirement; the
  upstream primitive must enforce it.
- Reversibility composes with ADR-0063: an ER mistake is fixed
  by an unmerge action, never by manual SQL.

## Considered Options

1. **Leave ER to verticals.** Status quo; produces drift and
   silent duplicates; rejected.
2. **Ship a single canonical algorithm.** Rejected — domain
   variation in comparators (names, addresses, identifiers) is
   too large for a single algorithm.
3. **This proposal — engine + spec.** Accepted.

## Compliance Anchors

- P1 zero-debt.
- P2 local-first: ER runs in-process.
- ADR-0022 adversarial-review: borderline merges flow through
  the existing challenge surface.
- ADR-0065 purpose-binding: ER actions are bound to a declared
  purpose (e.g. "deduplication for billing", "deduplication for
  enrolment"), refused otherwise.

## Rollout

- W3 (plan *Ontology Platform W3: Bitemporal + Lineage + ER*):
  - `MatchSpec` schema + validator.
  - Deterministic + probabilistic engines + comparator library
    (string, date, id-with-checksum, geo-proximity).
  - Merge/unmerge typed actions + compensation linkage.
  - Adversarial-review hook + golden tests.

## Consequences

- `convergio-edu` (and future verticals) ship a much smaller
  ingest layer; ER becomes a config artefact, not code.
- New surface to evaluate for security (a malicious MatchSpec
  could collide unrelated entities); we enforce that
  MatchSpecs are part of a capability bundle (ADR-0008) and
  therefore signed.

## Alternatives left for verticals

- Domain comparators (e.g. CEDS name normalisation, fiscal-
  code parsing) live in the vertical's capability bundle.
- Public publication of "we use these MatchSpecs" — vertical.

## References

- ADR-0008 capability bundles
- ADR-0022 adversarial-review
- ADR-0048 compensating actions
- ADR-0062 ontology runtime core
- ADR-0063 typed actions framework
- ADR-0064 bitemporal + lineage
- ADR-0065 provenance bundle + purpose registry
- Fellegi, Sunter (1969). *A Theory for Record Linkage.*
