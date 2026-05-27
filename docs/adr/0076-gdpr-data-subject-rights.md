---
adr: "0076"
title: "GDPR data-subject-rights handlers"
status: accepted
date: 2026-05-27
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0073", "0074", "0075", "0002", "0051"]
---

# ADR-0076 — GDPR data-subject-rights handlers

- **Status**: accepted
- **Date**: 2026-05-27
- **Supersedes**: —
- **Related**: ADR-0073 (EU-sovereign pivot), ADR-0074 (AGPL-3.0
  relicense), ADR-0075 (W3C-PROV provenance), ADR-0002 (audit chain),
  ADR-0051 (Ontology Runtime Core)

## Context

The EU-sovereign pivot (ADR-0073) and the regulatory matrix in
COMPLIANCE.md commit Convergio to supporting GDPR data-subject
rights. The v1.0 blocking scope is Article 15 access, Article 17
erasure, Article 20 portability, plus an audited HTTP intake surface.

## Decision

We model GDPR data-subject-rights in the leaf crate `convergio-gdpr`. Public surface:

- `DataSubjectId` — opaque stable subject identifier.
- `GdprRight` — enum of the seven rights (Access, Rectification,
  Erasure, Restriction, Portability, Objection,
  AutomatedDecisionSafeguards).
- `DataSubjectRequest` — incoming request shape.
- `DataSubjectResponse` — outgoing response shape, anchored by an
  `audit_seq: Option<u64>` so the audit chain (ADR-0002) is the
  authoritative ledger of compliance actions.
- `DataSubjectRecord` — caller-supplied subject-scoped record.
- `GdprError::UnsupportedRight` — explicit response for rights outside
  the implemented v1.0 scope.
- `handle_request()` / `handle_request_with_records()` — Article 15,
  17, and 20 handlers.

This ADR's scope: leaf-crate handlers, `POST /v1/gdpr/requests`, and
audit/provenance anchoring for fulfilled requests. Durable subject
indexes and CLI sugar remain follow-up work.

## Status of the implementation

| Area | Status |
|------|--------|
| Type contract (request/response/right/error/records) | **shipped** |
| Article 15 access handler | **shipped** |
| Article 17 erasure tombstone handler | **shipped** |
| Article 20 portability handler | **shipped** |
| HTTP surface `POST /v1/gdpr/requests` | **shipped** |
| Audit + provenance anchoring | **shipped** |
| DB schema (subject map, request log) | **planned** — follow-up |
| CLI surface `cvg gdpr request` | **planned** — follow-up |
| Operator runbook in COMPLIANCE.md | **planned** — follow-up |

## Alternatives considered

1. **Skip the leaf crate, put types in `convergio-server` routes.**
   Rejected: forces a heavy dep on `axum` for anything that wants to
   typecheck against the GDPR shape (durability, ontology, CLI).
2. **Ship HTTP endpoints returning 501 immediately.** Rejected:
   too easy for downstream COMPLIANCE.md claims to read "endpoint
   exists" as "right is implemented". A leaf crate with explicit
   an explicit unsupported-right error is harder to mis-cite.
3. **Defer until after wave 3.** Rejected: ontology, durability and
   provenance call sites need to start importing the types now to
   avoid a single megacommit when the impl lands.

## Consequences

- **Implemented v1.0 core**: Article 15/17/20 requests produce
  structured responses and are anchored in the audit chain via the
  HTTP route.
- **AGPL alignment** (ADR-0074): once endpoints exist, the AGPL
  ensures any downstream SaaS hosting `/v1/gdpr/*` publishes
  modifications — non-trivial because regulators *will* ask to inspect
  custom handling code.
- **Audit-chain anchoring**: `DataSubjectResponse.audit_seq` is the
  single source of truth for fulfilled HTTP requests.

## References

- Regulation (EU) 2016/679 (GDPR), Art. 15, 16, 17, 18, 20, 21, 22
- ADR-0073 — EU-sovereign pivot
- ADR-0074 — AGPL-3.0-or-later relicense
- ADR-0075 — W3C-PROV-JSON provenance bundles
- ADR-0002 — Audit hash chain
- COMPLIANCE.md § GDPR data-subject rights
