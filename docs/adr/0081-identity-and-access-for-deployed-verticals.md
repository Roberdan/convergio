---
adr: "0081"
title: "Identity & access for deployed verticals — Entra ID OIDC + RBAC/ABAC"
status: proposed
date: 2026-06-06
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0073", "0078", "0079", "0080", "0076"]
---

# ADR-0081 — Identity & access for deployed verticals (Entra ID OIDC + RBAC/ABAC)

- **Status**: proposed
- **Date**: 2026-06-06
- **Deciders**: Roberto D'Angelo
- **Related**: ADR-0073 (EU-sovereign pivot), ADR-0078 (Postgres),
  ADR-0079 (Azure single-tenant), ADR-0080 (ontology-author),
  ADR-0076 (GDPR DSR)

## Context and Problem Statement

The local-first daemon today has **no human identity model**: it trusts
whoever can reach the loopback socket (plus purpose-binding headers).
That is correct for a laptop. It is unacceptable for a deployed vertical
(ADR-0079) such as a university Student Information System, where
registrar staff, faculty, and students must have **different, audited,
least-privilege access** to the same data — and where regulators expect
authentication, authorization, and an audit trail tied to a real person.

We need an identity and access model for *deployed* Convergio that does
not compromise the sovereignty posture: Convergio must not become an
identity provider or hold a credential store it could be compelled to
disclose.

## Decision Drivers

- Deployed verticals have multiple human roles over shared data.
- P6 sovereignty: Convergio runs no IdP, holds no passwords, is not a
  control plane over the customer's users.
- The operator (institution) already runs an IdP — for the Azure target
  (ADR-0079) that is **Microsoft Entra ID**; many also federate an
  academic IdP (SAML/eduGAIN) behind it.
- Authorization for an ontology platform must be **data-shaped**: access
  rules refer to ObjectTypes and instances, not just static role names.
- Every privileged action must already be auditable (ADR-0076 DSR,
  ADR-0075 provenance) — identity must feed that trail, not bypass it.

## Considered Options

1. **Keep loopback-trust only** — no human identity. *(insufficient for
   deployment)*
2. **Convergio-owned identity store** (local users + passwords).
3. **Federated OIDC to the operator's IdP (Entra ID) + RBAC, with an
   ABAC layer for instance-level rules.** *(chosen)*

## Decision Outcome

Chosen option: **Option 3.**

- **Authentication: OIDC to the operator's IdP.** For Azure deployments
  this is **Entra ID**; the daemon validates the IdP's signed JWT
  (issuer, audience, `exp`, signature against the published JWKS) on
  every request. Convergio stores **no passwords** and runs **no IdP**.
  The local-first laptop mode keeps loopback-trust; OIDC is required only
  when a deployment is configured with an issuer.
- **Authorization, layer 1 — RBAC.** A small set of platform roles
  (`reader`, `author`, `operator`, `admin`) is mapped from IdP group /
  app-role claims via deployment configuration. Roles gate *capabilities*
  (read schema, author ontology, import to registry, run DSR, manage
  deployment).
- **Authorization, layer 2 — ABAC over the ontology.** Because the data
  model is the ontology itself, fine-grained rules are expressed against
  ObjectTypes and instance attributes (e.g. "a `Student` may read only
  the `Enrollment` instances linked to their own `Student` node";
  "faculty may read `Grade` only for `CourseOffering`s they teach").
  ABAC rules are versioned ontology artifacts, validated and
  provenance-tracked like the schema (ADR-0080 posture).
- **Audit binding.** The authenticated subject (a stable IdP `sub`) is
  recorded as the PROV `Agent` (ADR-0075) on every mutation and as the
  actor on every DSR (ADR-0076). No privileged action is anonymous.
- **Separation from the registry-import gate.** The `ontology import`
  capability (the ADR-0080 loop close) becomes a role-gated, audited
  daemon action; the CLI remains a thin client (it cannot bypass the
  gate).

### Positive consequences

- Sovereignty preserved: the customer owns identities; Convergio holds no
  credentials and cannot act as a control plane over users.
- Authorization is expressed in the same ontology language the platform
  already validates and versions — one mental model, fully auditable.
- Drops cleanly onto the Azure single-tenant target (Entra ID is native).

### Negative consequences

- ABAC over instances is non-trivial to implement and to keep performant;
  it must be staged (RBAC first, ABAC after) behind golden tests.
- A JWKS/issuer misconfiguration is a lockout risk; deployment tooling
  must validate identity config before cutover, and keep a documented
  break-glass admin path bound to the operator's subscription.

## Pros and Cons of the Options

### Option 1 — loopback-trust only
- 👍 Zero complexity; perfect for the laptop.
- 👎 No multi-user access control; unusable for a deployed vertical.

### Option 2 — Convergio-owned identity store
- 👍 Self-contained; no IdP dependency.
- 👎 Violates P6 (we become a credential holder / control plane);
  duplicates an IdP every institution already runs; security liability.

### Option 3 — federated OIDC + RBAC + ABAC (chosen)
- 👍 Sovereign, native to Azure, data-shaped authorization, audit-bound.
- 👎 ABAC complexity; must be staged and tested.

## Links

- Related ADRs: ADR-0073, ADR-0078, ADR-0079, ADR-0080, ADR-0076,
  ADR-0075.
- Follow-up: identity config schema + JWKS validation crate; ABAC rule
  format as a versioned ontology artifact; deployment-time identity
  pre-flight check (ADR-0079 IaC).
