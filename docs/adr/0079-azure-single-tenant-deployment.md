---
adr: "0079"
title: "Deploy verticals as customer-owned single-tenant on Azure EU"
status: proposed
date: 2026-06-06
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0073", "0074", "0038", "0078", "0080"]
---

# ADR-0079 — Deploy verticals as customer-owned single-tenant on Azure EU

- **Status**: proposed
- **Date**: 2026-06-06
- **Deciders**: Roberto D'Angelo
- **Related**: ADR-0073 (EU-sovereign pivot), ADR-0074 (AGPL-3.0),
  ADR-0038 (fleet cross-repo), ADR-0078 (Postgres backend), ADR-0080
  (ontology-author)

## Context and Problem Statement

To build real verticals (e.g. a university Student Information System,
ADR-0080) the runtime must be deployable on managed cloud, not only on
a laptop. The obvious target operators ask for is **Azure**. This
collides head-on with the constitution: P6 says local-first, **no remote
control plane, no telemetry, sovereignty by construction** — and Azure is
a US hyperscaler. We must decide how a cloud deployment can exist without
turning Convergio into the thing it refuses to be.

The resolution hinges on what "sovereignty" actually means. It is a
property of **control and ownership**, not a prohibition on capable
infrastructure: an EU institution that runs Convergio in **its own Azure
subscription, in an EU region, with its own keys and its own database**,
under the EU Data Boundary, retains full control. What sovereignty forbids
is a *Convergio-operated* control plane that can see or steer customer
data.

## Decision Drivers

- Operators (universities, public sector) require managed, backed-up,
  HA infrastructure — not a laptop.
- P6 sovereignty: no Convergio-run control plane, no phone-home, no
  shared multi-tenant store.
- Data residency: EU region + EU Data Boundary mandatory for the target
  market (EU AI Act, GDPR, NIS2).
- Key custody: credential-signing keys (ADR-0080 issuance) must live in
  the customer's own Key Vault, never ours.
- Repeatability: one-command, auditable deployment per institution.

## Considered Options

1. **Convergio-hosted multi-tenant SaaS** — we run it for everyone.
2. **Customer-owned single-tenant on Azure EU** — each institution runs
   one isolated deployment in its own subscription. *(chosen)*
3. **On-prem / Azure Stack / EU sovereign cloud only** — no hyperscaler.

## Decision Outcome

Chosen option: **Option 2 — customer-owned single-tenant on Azure EU.**

Deployment contract:

- **One deployment per institution** (single-tenant). No shared database,
  no cross-tenant code paths. Data isolation is physical, not logical.
- Runs in the **customer's own Azure subscription**, **EU region**, under
  the **EU Data Boundary**. Convergio (the project) operates nothing and
  receives no telemetry.
- Components: Azure Container Apps (or AKS) for the daemon, Azure Database
  for PostgreSQL Flexible Server (ADR-0078), Azure Blob for artifacts,
  **Azure Key Vault** for signing keys, **Microsoft Entra ID** for
  operator/admin identity (ADR-TBD identity), all in-region.
- **Fleet, not control plane**: an operator may use `cvg fleet`
  (ADR-0038) to *operate* many single-tenant deployments, but fleet never
  co-mingles tenant data — it is an operator-side orchestration tool, run
  by the customer or their chosen integrator, not a Convergio service.
- Shipped as **Infrastructure-as-Code** (Bicep or Terraform) + container
  image, so a deployment is reproducible and auditable. The IaC and the
  AGPL source are the only artifacts; there is no hosted binary.
- A `COMPLIANCE.md` deployment annex maps each Azure component to the
  residency/sovereignty claim with a verification step.

### Positive consequences

- Sovereignty holds: the customer owns control, keys, data, and region.
- Real managed infra (HA, backup, PITR, scale) becomes available.
- Matches public-sector procurement (each institution = its own contract
  and instance).
- Smaller security model: no cross-tenant isolation to get wrong.

### Negative consequences

- Per-deployment operational cost; mitigated by IaC + fleet tooling.
- "Sovereign on a US hyperscaler" is a nuanced claim we must defend
  honestly in COMPLIANCE.md (EU Data Boundary ≠ immunity from US legal
  reach; on-prem remains the maximal-sovereignty option).
- We must support a non-Azure path (Option 3) for buyers who reject
  hyperscalers — kept as a documented variant of the same IaC.

## Pros and Cons of the Options

### Option 1 — hosted multi-tenant SaaS
- 👍 Easiest for small buyers; one ops surface.
- 👎 Directly violates P6; we become a control plane over customer data.

### Option 2 — customer-owned single-tenant Azure EU (chosen)
- 👍 Sovereignty by ownership; managed infra; clean isolation.
- 👎 Per-deployment cost; nuanced "sovereign-on-Azure" messaging.

### Option 3 — on-prem / sovereign cloud only
- 👍 Maximal sovereignty.
- 👎 High friction; excludes Azure-standardised institutions. Kept as a
  documented IaC variant, not the default.

## Links

- Related ADRs: ADR-0073, ADR-0074, ADR-0038, ADR-0078, ADR-0080
- Follow-up: identity/access ADR (Entra ID OIDC + RBAC/ABAC) — to be
  filed when the SIS vertical work starts.
- Compliance: COMPLIANCE.md deployment annex (to be added).
