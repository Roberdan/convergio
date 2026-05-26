# AGENTS.md — convergio-ops

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the **Workflow & Operations Engine** (Ontology Platform W8):

- Workflow definitions and workflow instances persisted with **bitemporal** history.
- A small BPMN-2.0 subset interpreter (sequence, parallel fork/join, exclusive gateway, timer).
- Ops-level semantics: escalation (human tasks) and compensation (undo actions).

## Invariants

- Tables are append-only bitemporal: close the current system-time row and insert a new one.
- Every state-changing method must write an audit row (via convergio-durability audit chain).
- No HTTP routing here — routes live in `convergio-server`.
- Keep Rust files under the 300-line cap.
