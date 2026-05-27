---
adr: "0074"
title: "Relicense Convergio to AGPL-3.0-or-later"
status: accepted
date: 2026-05-27
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0073", "0004"]
---

# ADR-0074 — Relicense Convergio to AGPL-3.0-or-later

## Context

ADR-0073 reframed Convergio as **"the open, EU-sovereign platform
where AI agents and humans converge on data both can trust"** and
added a 6th sacred principle: *Sovereignty by construction.*

The prior license — **Convergio Community License v1.3** — was a
custom text. Two facts make a custom license incompatible with the
new framing:

1. **It is not OSI-approved.** The 6th principle requires an
   OSI-approved license. A bespoke text fails this requirement by
   construction, regardless of how generous the actual terms are.
2. **It is silent on network use.** A SaaS operator could fork
   Convergio, run it as a hosted service, and never publish their
   modifications. For a sovereignty-positioned project this is the
   exact loophole the license must close.

The EU public sector — the audience the pivot targets — increasingly
mandates OSI-approved copyleft (EUPL, AGPL, GPL) for procurements
where modifications must remain in the commons. AGPL-3.0 is the
strongest copyleft answer for network-deployed software and is
widely recognised in EU procurement contexts.

## Decision

**License Convergio under `AGPL-3.0-or-later`, with a Contributor
License Agreement (CLA) to preserve relicensing optionality.**

Specifically:

1. Replace `LICENSE` with the verbatim text of GNU AGPL-3.0
   (`https://www.gnu.org/licenses/agpl-3.0.txt`).
2. Add `NOTICE` enumerating contributors and the relicensing
   provenance.
3. Add `CONTRIBUTING-CLA.md` (Apache ICLA model) that grants the
   maintainer copyright + patent licenses to redistribute future
   contributions under AGPL-3.0-or-later, including any later
   FSF-published version the project chooses to adopt.
4. Update `deny.toml` license allowlist:
   - Remove `LicenseRef-Convergio-Community-1.3`.
   - Add `AGPL-3.0`, `AGPL-3.0-or-later`.
5. Update `README.md` license badge.
6. Update root `AGENTS.md` to drop the "immutable" qualifier on
   `LICENSE` and replace it with a pointer to this ADR.
7. Per-file SPDX headers (`// SPDX-License-Identifier:
   AGPL-3.0-or-later`) are a **follow-up PR**, not a blocker for
   this one. The repository-level `LICENSE` file is authoritative
   for AGPL compliance even without per-file headers; SPDX headers
   are a hygiene improvement.

## Consequences

### Positive

- **OSI-approved**: principle 6 satisfied for the license axis.
- **Network-use copyleft**: any SaaS operator deploying a modified
  Convergio to third parties must publish their source. This is
  the load-bearing property of AGPL vs GPL.
- **EU procurement compatibility**: AGPL is on the standard
  allow-list for public-sector tenders requiring strong copyleft.
- **CLA preserves optionality**: the project can adopt
  AGPL-4.0 (if/when published) without re-collecting consent from
  every historical contributor.

### Negative / risks

- **Enterprise hesitation**: some corporate legal teams forbid
  internal use of AGPL-licensed software because of the SaaS
  trigger. This is *intentional* for Convergio's positioning — we
  do not want our code embedded in closed proprietary SaaS — but
  it does shrink the addressable enterprise audience for purely
  internal evaluations.
- **Compatibility with permissive code we depend on**: AGPL is
  one-way compatible with Apache-2.0 / MIT / BSD (we can consume,
  they can't consume us under their own terms). No action needed;
  the existing `deny.toml` allowlist already reflects this.
- **No grandfathering of v1.3 users**: anyone who took the code
  under v1.3 keeps those rights for that snapshot. Going forward,
  all new releases are AGPL-3.0-or-later.
- **CHANGELOG entry must be marked `BREAKING`** because downstream
  re-distributors need to know.

### Neutral

- Single-user local-first deployments are unaffected in practice —
  AGPL's network clause does not trigger unless the operator
  distributes the service to third parties.
- The audit-chain, gate pipeline, and durability guarantees are
  **license-agnostic** and continue to enforce principles 1–5.

## Validation

This PR ships:

- `LICENSE` (AGPL-3.0 verbatim, 661 lines).
- `NOTICE` (contributors + relicensing provenance).
- `CONTRIBUTING-CLA.md` (Apache ICLA model adapted).
- `deny.toml` allowlist updated.
- `.github/workflows/license-check.yml` (dedicated visibility job
  that runs `cargo deny check licenses` on every PR; supplements
  the existing `cargo deny check ...` invocation in `ci.yml`).
- `README.md` badge.
- Root `AGENTS.md` updated.
- `CHANGELOG.md` `BREAKING:` entry.

Acceptance criteria:

1. `cargo deny check licenses` exits zero on this branch.
2. `grep -r "Convergio Community License" --include="*.md"
   --include="*.toml" --include="*.rs" --include="LICENSE" .`
   returns matches only inside ADRs and CHANGELOG history (i.e.
   historical references), never inside live license-bearing files.
3. The README badge renders an SPDX `AGPL-3.0-or-later` shield.

## References

- [GNU AGPL-3.0 full text](https://www.gnu.org/licenses/agpl-3.0.txt)
- [OSI: AGPL-3.0 listing](https://opensource.org/license/agpl-v3)
- [SPDX: AGPL-3.0-or-later](https://spdx.org/licenses/AGPL-3.0-or-later.html)
- ADR-0073 — EU-sovereign pivot
- ADR-0004 — Three (now six) sacred principles
