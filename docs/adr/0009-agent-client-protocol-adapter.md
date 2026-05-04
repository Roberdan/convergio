---
id: 0009
status: proposed
date: 2026-04-29
topics: [protocol, editor, acp, mcp]
related_adrs: []
touches_crates: []
last_validated: 2026-04-30
---

# 0009. Treat Agent Client Protocol as a future northbound editor adapter

- Status: proposed
- Date: 2026-04-29
- Deciders: Roberto, Copilot
- Tags: protocol, editor, acp, mcp

## Context and Problem Statement

Convergio already exposes HTTP for the daemon boundary, `cvg` for humans,
and MCP for agent tools. Agent Client Protocol (ACP) is useful for
editor/IDE clients that want to talk to an agent through a standard
session protocol.

ACP should not replace MCP or become a way around Convergio's durable
task, evidence, gate, and audit model.

## Decision Drivers

- MCP and ACP solve different integration boundaries.
- Editor integrations should not bypass server-side gates.
- v0.1 must stay focused on the local core.
- A future ACP bridge should reuse existing Convergio daemon APIs.

## Considered Options

1. **Replace MCP with ACP** — make ACP the only agent integration
   protocol.
2. **Ignore ACP** — keep HTTP/CLI/MCP only.
3. **Add a future ACP adapter as a northbound editor interface** — keep
   MCP for tool use and let ACP clients interact with Convergio as an
   agent/proxy.

## Decision Outcome

Chosen option: **Option 3**, because it keeps the current MCP tool
contract stable while leaving a clean path for editor adoption.

### Implementation status (2026-05-04)

Stays `proposed`: the ADR is explicitly future-facing — it reserves the
`convergio-acp` adapter slot rather than shipping it. No `convergio-acp`
crate, no ACP transport, no editor session mapping exist today, by
design (§ "v0.1 scope" leaves ACP out of the public v0.1 cut). The
frame is real; the building inside it is tracked elsewhere.

### Positive consequences

- Editors that support ACP can eventually use Convergio directly.
- Existing agent hosts can keep using `convergio-mcp`.
- The daemon remains the source of truth.

### Negative consequences

- Another binary/protocol surface must be maintained later.
- ACP session semantics must be carefully mapped to Convergio plans,
  tasks, evidence, and workspace proposals.

## Protocol boundaries

| Surface | Role |
|---------|------|
| HTTP | daemon API and source-of-truth boundary |
| CLI (`cvg`) | human/admin interface |
| MCP | agents call Convergio as a tool |
| ACP | editors/IDEs talk to Convergio as an agent/proxy |

The possible future binary is `convergio-acp`.

## Rules

- ACP must not bypass `submitted`/`done` gates.
- ACP-driven work must still create plans/tasks/evidence/audit records.
- ACP patch or diff output must route through workspace patch proposals
  once ADR-0007 is implemented.
- ACP may stream progress to editors, but durable state lives in the
  daemon.
- ACP is not a prerequisite for public v0.1.

## Initial scope when implemented

1. Read-only status/session proof of concept.
2. Map editor sessions to Convergio plans/tasks.
3. Stream progress and references to task/evidence IDs.
4. Only then allow ACP-driven task completion, still gated by the server.

## Links

- Related ADRs: [0007](0007-workspace-coordination.md),
  [0008](0008-downloadable-capabilities.md)
