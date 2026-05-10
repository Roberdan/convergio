---
id: 0047
status: proposed
date: 2026-05-10
topics: [layer-0, api-surface]
related_adrs: [0009, 0015]
touches_crates: [convergio-api, convergio-server, convergio-mcp]
last_validated: 2026-05-10
---

# 0047. Generate a discoverable action type registry (actions.json)

- Status: proposed
- Date: 2026-05-10
- Deciders: convergio maintainers
- Tags: layer-0, api-surface

## Context and Problem Statement

The MCP bridge (`convergio.help` / `convergio.act`) and external skills need a
machine-readable list of Convergio actions (name, capability bucket, summary)
without reading Rust code.

Today the daemon and bridge can each build catalogs in code, but that pushes
schema discovery into implementation details and makes it easy for clients to
get out of sync.

## Decision Drivers

- Provide a single canonical, versioned action registry that tools can ingest.
- Keep the registry stable and deterministic (byte-identical output).
- Avoid adding runtime IO or business logic to `convergio-api`.

## Considered Options

1. **Runtime registry only** — build the JSON at runtime from Rust.
2. **Build-time generated JSON file** — generate `crates/convergio-api/actions.json` from the Rust action surface.
3. **Hand-maintained JSON file** — edit `actions.json` directly.

## Decision Outcome

Chosen option: **Build-time generated JSON file**, because it gives a
discoverable artifact for skills and lets both the daemon and MCP bridge return
the exact same bytes.

### Positive consequences

- One canonical document (`actions.json`) for action discovery.
- `convergio.help` and `GET /v1/api/actions` can agree byte-for-byte.
- Changes to the `Action` enum automatically flow into the registry.

### Negative consequences

- `build.rs` writes a derived file into the crate directory, which is less pure
  than only writing into `OUT_DIR`.

## Links

- Related ADRs: [0009](0009-agent-client-protocol-adapter.md), [0015](0015-documentation-as-derived-state.md)
