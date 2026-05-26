---
status: accepted
date: 2026-05-25
deciders: Roberdan, Copilot
---

# ADR-0061 — `cvg capability search` (local-only slice of W9)

## Context

W9 in the production-ready plan asks to graduate the capability
registry from `first-party-local` (ADR-0008) to `first-party-remote`
with a signed HTTPS manifest endpoint, `cvg capability install
<name>@<version>` pulling from that endpoint, Ed25519 signature
verification with a versioned trust store, mirror discipline, and
a reproducibility test in CI. That is an 8-12 day workstream and
includes operator decisions (Pages vs custom service, trust-store
location, key rotation policy) that cannot be made unilaterally
mid-session.

The user-visible verb `cvg capability search <query>` is, however,
**already useful today against the local registry** — operators
already accumulate dozens of installed capabilities and have no way
to grep them without piping `cvg capability list --output json`
through `jq`. Shipping this verb early also pins the CLI shape for
the eventual remote slice (so when remote lands, the only delta is
"now searches across local AND a cached remote index" rather than
"a brand-new subcommand").

## Decision

Ship `cvg capability search <query>` as a **local-only** subcommand.
It calls the existing `GET /v1/capabilities` route and does
case-insensitive substring matching on `name`, `version`, and
`status`. Output respects `--output human|json|plain`.

Remote registry, signature verification of remote downloads, install
from URL, mirror discipline, key rotation policy, and CI bundle
reproducibility test all remain explicit W9 follow-ups (see below).

## Consequences

- Operators get a useful filter today without any new server route,
  schema change, or trust decision.
- The CLI surface is forward-compatible: once a cached remote index
  exists, this verb expands to search across both.
- No new dependency, no new evidence kind, no new audit row — the
  call is read-only and the existing `list` route already audits
  reads where applicable.

## Alternatives considered

- **Wait for the full remote slice**: rejected. Blocks operator
  ergonomics for weeks and bundles a tiny client change with a
  large infrastructure decision.
- **Server-side `GET /v1/capabilities?q=…`**: rejected for now. The
  filter is cheap client-side over typical local registry sizes
  (≤ low hundreds). When remote lands the server-side variant is
  trivial to add.

## Follow-ups (the rest of W9)

- HTTPS manifest endpoint (`https://capabilities.convergio.dev/...`)
  — likely GitHub Pages.
- `cvg capability install <name>@<version>` pulling from remote.
- Ed25519 trust store + rotation policy ADR.
- Mirror discipline doc.
- CI bundle reproducibility test.
- `docs/adr/0008-downloadable-capabilities.md` for capability authors.

## Status

Implemented in the same PR as this ADR. Verb available immediately:
`cvg capability search <query>`.
