---
id: 0059
status: proposed
date: 2026-05-25
topics: [ontology, tui, observability, dashboard]
related_adrs: [0023, 0029, 0051, 0052, 0053, 0055, 0056, 0058]
touches_crates: [convergio-tui, convergio-ontology, convergio-api]
last_validated: 2026-05-25
---

# 0059. TUI Ontology Inspector (read-only)

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, tui, observability

## Context

ADRs 0051–0058 add an ontology runtime, typed actions,
bitemporal store + lineage, ER, branching, connectors, and an
LLM gateway. All of these emit state that operators need to
inspect at runtime, not just at audit time. The CLI is fine
for one-shot queries; nothing exists for live navigation.

Convergio already ships a TUI (`convergio-tui`, ADR-0029) with
its dashboard separation pattern. The natural place for an
ontology observability surface is **inside that TUI**, not in a
new graphical frontend (which would violate the local-first
posture and the urbanism split — see ADR-0018 and the
`convergio-workbench` accelerator stub).

This ADR is deliberately **read-only**. Authoring of schemas
remains YAML inside signed capability bundles (ADR-0008);
visual editing would risk becoming the de facto authoring
channel and pull schema design out of the signed-bundle world.

## Decision

Add an **Ontology Inspector** panel-set to `convergio-tui`,
following the dashboard separation pattern of ADR-0029. Six
panels, all read-only, all sourced from existing crate APIs:

1. **Types** — browse `ObjectType` / `LinkType` / `PropertyType`
   from the schema registry; show `schema_version`,
   `content_hash`, `breaking` flag, registering plan. Drilldown
   into property definitions and JSON-Schema preview.
   _Source: ADR-0051._
2. **Live events** — rolling tail of `ontology_events`,
   filterable by `object_type`, `action`, `agent`, `purpose`.
   Subscribes to the existing Layer 2 bus topic.
   _Source: ADR-0052._
3. **Lineage** — ASCII DAG renderer for the lineage of a
   selected `object_id`, with `--as-of` and `--valid-at`
   navigation (bitemporal). Hash references link back to
   `audit_log` rows.
   _Source: ADR-0053._
4. **Branches** — active scenario branches, author, expiry,
   mutation count, diff size; quick-jump to branch diff.
   _Source: ADR-0056._
5. **ER queue** — pending `MatchProposal` rows in `hold`
   state, with comparator breakdown and recommended action;
   no merge button (refers the user to the CLI / capability
   surface).
   _Source: ADR-0055._
6. **Gateway calls** — rolling tail of LLM gateway calls
   (prompt hash, model id, schema id, purpose, refuse/accept
   outcome); never shows raw prompt content (it might carry
   redactable PII). A separate "detail" view requires an
   active `inspect` purpose to reveal payloads — refused
   otherwise.
   _Source: ADR-0058._

### Non-goals

- **No mutations.** The Inspector never writes. Every action
  the operator wants to take (validate YAML, register type,
  approve merge, register purpose) is surfaced as a
  CLI-command hint with the exact `cvg` invocation copied to
  clipboard. This keeps the typed-action surface (ADR-0052)
  authoritative.
- **No graphical rendering** (no Tauri, no embedded
  webserver). ASCII / box-drawing characters only.
- **No new dependency outside the existing TUI stack.**

### Compliance posture

- P1 zero-debt: panels return structured errors with stable
  reasons; no swallowed failures.
- P2 local-first: TUI reads via the same `127.0.0.1` HTTP API
  as the CLI; no extra surface area.
- P3 accessibility: every panel must be usable without colour
  (the `--no-color` mode already supported by `convergio-tui`
  is the contract); no animations required to comprehend
  state.
- Payload-redaction in the Gateway panel honours the LLM
  gateway's redactor chain (ADR-0058) by default.

## Decision Drivers

- Operators need a live picture of an ontology under load;
  one-shot CLI queries scale poorly for triage.
- The existing TUI is the right home — same auth surface,
  same accessibility contract, same deployment shape.
- Read-only keeps the surface cheap and the security model
  simple.

## Considered Options

1. **Web UI inside the daemon.** Rejected — violates P2 (new
   bind / new attack surface) and pulls toward authoring.
2. **External desktop app (Tauri).** Rejected — introduces
   build-time dependency on a UI runtime; better as a
   separate vertical accelerator (`convergio-workbench`).
3. **Read-only TUI panels (this proposal).** Accepted.

## Rollout

- W6 of the ontology runtime plan family — plan
  *[core] Ontology Runtime W6: TUI Inspector*:
  - One task per panel + a "home" panel + golden snapshot
    tests for each.

## Consequences

- The TUI gains six panels but no new state ownership.
- Operators of any vertical (edu, future verticals) get the
  same observability surface for free.
- Authoring continues to flow through YAML + capability
  bundles; the visual workbench is explicitly punted to a
  vertical accelerator.

## Alternatives left for verticals

- Graphical / web-based ontology workbench (see
  `convergio-workbench` plan stub) — vertical accelerator
  built on the core HTTP + MCP API.

## References

- ADR-0008 capability bundles
- ADR-0018 long-tail vertical accelerators
- ADR-0023 observability tier
- ADR-0029 TUI dashboard crate separation
- ADR-0051 ontology runtime core
- ADR-0052 typed actions framework
- ADR-0053 bitemporal store + lineage
- ADR-0055 entity resolution service
- ADR-0056 scenario branching
- ADR-0058 LLM gateway primitive
