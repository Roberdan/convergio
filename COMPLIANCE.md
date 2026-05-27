# Compliance posture — Convergio

> **Honest status.** This document maps EU regulatory obligations to
> the Convergio primitive that satisfies them, with a per-primitive
> status: **enforced** (gate or audit kind exists today),
> **partial** (primitive ships but the obligation is not fully
> wired), **planned** (ADR exists, code does not yet).
>
> If you find a claim here that the code does not back, that is a
> bug — open an issue. We will move it to `planned` or close the
> gap, not soften the claim.

See [ADR-0073 — EU-sovereign pivot](./docs/adr/0073-eu-sovereign-pivot.md)
for the framing. See `CONSTITUTION.md` for the non-negotiable rules
that constrain *how* compliance is implemented.

---

## 1. Regulatory matrix

| Regulation | Article / requirement | Convergio primitive | Status |
|---|---|---|---|
| **GDPR** | Art 5(1)(b) — purpose limitation | Purpose Registry (ADR-0065) + `PurposeGate` | **planned** |
| **GDPR** | Art 5(1)(f) — integrity, confidentiality | Local-first + SQLite-on-disk + hash-chained audit (ADR-0002) | **partial** |
| **GDPR** | Art 12 / Art 15 — right of access (DSAR) | `GET /v1/gdpr/subjects/:id/access` + provenance bundle (ADR-0065) | **planned** |
| **GDPR** | Art 17 — right to erasure | `POST /v1/gdpr/subjects/:id/erase` + immutable erasure audit row | **planned** |
| **GDPR** | Art 25 — data protection by design | Local-first default, 5 sacred principles + § 6 sovereignty | **enforced** |
| **GDPR** | Art 30 — records of processing | Purpose Registry export + capability registry | **planned** |
| **GDPR** | Art 32 — security of processing | Local-only bind, no remote control plane | **partial** |
| **AI Act** | Art 9 — risk management system | Plan/wave/gate pipeline + ADR record | **partial** |
| **AI Act** | Art 12 — record-keeping / logging | Hash-chained audit log (ADR-0002), append-only | **enforced** |
| **AI Act** | Art 13 — transparency to deployers | `GET /v1/api/actions`, `cvg gates show`, ADR repository | **enforced** |
| **AI Act** | Art 14 — human oversight | `cvg validate` / Thor validator (ADR-0050+) + manual `cvg task transition` | **partial** |
| **AI Act** | Art 15 — accuracy, robustness, cybersecurity | `NoDebtGate`, `ZeroWarningsGate`, `NoSecretsGate` | **enforced** (for evidence) |
| **NIS2** | Art 21 — risk management measures | Local-only deployment + audit chain + capability allowlist | **partial** |
| **NIS2** | Art 23 — incident notification (24h/72h) | `GET /v1/audit/stream` SSE + `cvg audit refusals` | **partial** |
| **DORA** | Art 17 — ICT-related incident management | Hash-chained audit chain reconstruction + compensating actions (ADR-0048) | **partial** |
| **EU Data Act** | Art 5 — data portability for users | `GET /v1/audit/events`, `GET /v1/objects/:id/provenance`, JSON export | **planned** |
| **eIDAS 2** | Qualified electronic signatures (future) | Signed capability install (ADR-0008) | **partial** |

**Legend.** *Enforced*: gate or audit kind exists today, refusing
non-compliant transitions or recording mandatory events.
*Partial*: primitive ships but obligation is not yet end-to-end wired
(missing endpoint, missing gate, missing export format).
*Planned*: ADR accepted, code not yet written. See the EU-sovereign
pivot plan in the daemon for sequencing.

---

## 2. Sovereignty posture

| Property | Status | Evidence |
|---|---|---|
| Single-tenant, single-user runtime | **enforced** | `127.0.0.1` bind default; no auth/RBAC code path |
| SQLite on local disk (no remote DB) | **enforced** | `convergio-db` crate; no cloud-DB driver in workspace |
| No telemetry, no phone-home | **enforced** | grep the source: no analytics SDK, no `reqwest::post` to any vendor endpoint outside the user-configured runner |
| Model choice is the operator's | **enforced** | vendor-CLI runners (`convergio-runner`); no API key stored or required by Convergio itself |
| OSI-approved license | **planned** | tracked under W13 in the EU-sovereign pivot plan (relicense to AGPL-3.0-or-later + CLA) |
| Reproducible local build | **enforced** | `cargo build --workspace`, Rust toolchain pinned via `rust-toolchain.toml`, `Cargo.lock` committed |
| Tamper-evident audit | **enforced** | hash-chained audit log (ADR-0002); `cvg audit verify` |
| Compensating actions for accidental writes | **partial** | ADR-0048; subset of mutating actions has registered inverses |

---

## 3. Threats explicitly out of scope

Convergio is a **localhost daemon**. The following are *not* in the
trust boundary:

- the OS user account running the daemon (full filesystem access);
- the agents the operator chooses to spawn (they run with the
  daemon-user's privileges);
- the network the operator's machine is on (the daemon does not
  bind to `0.0.0.0`, but the OS can be reconfigured to do so);
- the LLM endpoints the spawned vendor CLIs talk to (they are
  external services chosen by the operator).

For multi-user deployments or shared infrastructure, additional
controls outside Convergio are required.

---

## 4. How to verify

- **`cvg audit verify`** — re-hash the audit chain end-to-end; exit
  code non-zero if any row was rewritten.
- **`cvg gates show`** — list every gate, its required evidence
  kinds, and its stable refusal reasons (ADR-0047).
- **`cvg actions list`** — list every typed action the daemon
  exposes, including capability and human-readable summary.
- **`cvg audit refusals latest`** — show the most recent gate
  refusals (one-line each).
- **`cvg session resume`** — daemon health + audit chain status +
  open PRs + next tasks; one-shot operator dashboard.

If a claim in section 1 cannot be reproduced by one of these
commands, that is a bug. Open an issue tagged `compliance-drift`.

---

## 5. Reporting a compliance gap

- **Security-sensitive issues** — see [`SECURITY.md`](./SECURITY.md).
- **Regulatory-mapping mistakes** (this document) — open a GitHub
  issue with label `compliance`. We will either fix the code or
  downgrade the claim; we will not soften the claim and leave the
  code as-is.
- **Sovereignty regressions** (any change that introduces a remote
  call path the operator did not opt into) — treat as a § 6
  violation in `CONSTITUTION.md` and block on review.
