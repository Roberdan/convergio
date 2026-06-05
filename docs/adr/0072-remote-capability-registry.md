---
id: 0072
status: proposed
date: 2026-05-27
topics: [capabilities, registry, supply-chain, signing]
related_adrs: [0008]
touches_crates: [convergio-cli, convergio-server, convergio-durability]
last_validated: 2026-05-27
---

# 0072. Remote capability registry (W9)

- Status: proposed
- Date: 2026-05-27
- Tags: capabilities, supply-chain, signing, registry

## Context

ADR-0008 introduced the `.cap` capability bundle as
`first-party-local`: an operator installs from a file on disk
(`cvg capability install-file <path>`) and the daemon stores
manifests in SQLite. This works for one machine but does not
scale to:

- A vertical accelerator demo (W12 — `convergio-edu`) that
  composes 5+ capabilities a fresh user has never seen.
- `A11yGate` phase 2 (W11) which depends on the first
  capability bundle that carries a Node runtime dependency
  (`a11y.axe`). Without remote install, every operator has
  to manually mirror `axe-core` releases.
- A11y-, security-, compliance-vetted capability sets that the
  community can publish without each operator having to vet
  the producer's GPG key by hand.

The v1.0 master plan (`docs/plans/v1.0-production-ready.md`
§ W9) flagged this as the gating piece for W11 and W12. This
ADR defines the contract; the implementation lands in
follow-up PRs.

## Decision

We add **read-only remote install** on top of the existing
local store. There is no centralized "store" we operate
on behalf of users: the daemon resolves capability names
through a small set of operator-configured **registry
endpoints**, fetches signed manifests over HTTPS, verifies
the Ed25519 signature against a versioned local trust store,
then hands the bundle to the existing local installer.

### 1. URL scheme

A registry endpoint is an HTTPS origin that serves two paths:

```
GET /v1/index.json
GET /v1/<name>/<version>.cap
GET /v1/<name>/<version>.cap.sig
GET /v1/<name>/manifest.json
```

- `index.json` is a flat list of `{name, latest_version,
  description, keywords}` for client-side search. It is small
  (< 1 MB at v1.0 scale).
- `<version>.cap` is the bundle (same format as ADR-0008).
- `<version>.cap.sig` is the **detached** Ed25519 signature
  over the raw bundle bytes.
- `manifest.json` is the per-capability metadata
  (`{name, versions: [...], authors: [...], homepage, license,
  signing_key_id, ...}`).

A registry **is** a static site. The reference registry is
GitHub Pages backed by a repo (`convergio-registry`) — there
is no Convergio-operated service to keep alive.

### 2. Trust store

The daemon ships with a baked-in trust store at
`crates/convergio-server/assets/trust-store/v1.json` listing
the public keys that are valid **at compile time**. Operators
add or override keys via
`~/.convergio/v3/trust-store.d/*.json` (lexicographic merge).
Trust store entries are:

```json
{
  "key_id": "convergio-root-2026",
  "algorithm": "ed25519",
  "public_key_b64": "...",
  "valid_from": "2026-01-01T00:00:00Z",
  "valid_until": "2027-01-01T00:00:00Z",
  "owner": "Convergio core team",
  "revoked": false
}
```

Key rotation is **explicit**: a new key requires either a new
daemon release or an operator-side trust-store entry.
Revocation flips `revoked: true` in a follow-up release. There
is no CRL/OCSP — small surface, no online dependency.

### 3. CLI surface

```
cvg capability search <query>
cvg capability install <name>[@<version>] [--registry URL]
cvg capability info <name>
cvg capability registry add <URL>
cvg capability registry list
cvg capability registry remove <URL>
cvg capability trust list
cvg capability trust add <path-to-key.json>
```

`install` is idempotent: re-running with the same
`name@version` is a no-op once the bundle is on disk.

### 4. Verification flow

```
1. resolve <name>[@<version>] across configured registries
   (first hit wins; warn if multiple matches across regs)
2. fetch .cap + .cap.sig over HTTPS
3. find key_id from manifest.json or the .sig metadata
4. look up key_id in trust store; refuse if missing/revoked/
   outside valid window
5. verify Ed25519(.cap, sig, pubkey); refuse on mismatch
6. hand .cap bytes to existing `install-file` codepath
7. record install in audit log as
   `capability_installed_remote` with {name, version, key_id,
   source_url, sha256}
```

Tampering → `CapabilityInstallError::SignatureMismatch` →
HTTP 409 from `POST /v1/capabilities/install`. Same shape as
existing gate refusals.

### 5. Network discipline

- One configurable HTTP client (`reqwest::Client` with
  `rustls-tls`), 10s connect + 30s read timeout, max 50 MB
  bundle, no redirects across origins.
- All fetches go through a tiny `RegistryFetcher` trait so
  tests inject `MockFetcher`. **No live network in CI.**
- `--offline` flag on `install` bypasses fetch and reads from
  `~/.convergio/v3/cache/registry/<url-hash>/`. This is also
  how mirrors work.

### 6. Mirror discipline

A mirror is just another registry endpoint that happens to be
inside an operator's VPC. Documentation
(`docs/capability-registry.md`, follow-up PR) shows the
two-step recipe:

```
# Mirror operator
gh release download v1.2.3 --repo convergio/convergio-registry \
  --pattern '*.cap' --pattern '*.sig'
# serve them behind nginx/Caddy on https://internal.example.com

# Convergio operator
cvg capability registry add https://internal.example.com
cvg capability registry remove https://capabilities.convergio.dev
```

### 7. Reproducibility

For every published `.cap`, a sibling `.cap.build-attestation.json`
exists with `{git_sha, cargo_lock_sha256, builder, timestamp}`.
CI for `convergio-registry` rebuilds and refuses to publish on
hash mismatch. Operators can replay the same check locally with
`cvg capability verify <name>@<version>` (follow-up).

## Consequences

### Positive

- W11 (`a11y.axe`) and W12 (`convergio-edu`) become installable
  with one command per capability instead of an out-of-band
  download dance.
- Supply-chain story for v1.0 is honest: signing is the
  precondition for install, not an afterthought; trust roots
  are versioned, scoped, and operator-overridable.
- No Convergio-operated infrastructure beyond a static site —
  consistent with the local-first stance of CONSTITUTION § 1.
- ADR-0008's storage layer is reused unchanged; remote install
  is a thin shim on top.

### Negative / Trade-offs

- Operators who never want network egress must use
  `--offline` consistently or set
  `CONVERGIO_CAPABILITY_OFFLINE=1`. We accept the extra knob.
- No discovery of unsigned/community capabilities at v1.0 —
  every entry must be signed by a trusted key. Encourages
  good practice; deferred sketch of an "unverified" tier to
  post-v1.0.
- Ed25519-only keeps cryptography small; adding more
  algorithms is a future ADR.

### Out of scope (deferred)

- Centralized publishing API. Publishing is
  "push to `convergio-registry` repo, CI rebuilds and signs."
- Capability uninstall over the network (it's a local op).
- Per-capability auto-update. Operators run `install` again.
- W11 capability bundle contents themselves (separate ADR).

## Implementation plan

This ADR is design-only. The implementation lands as:

1. **F1 — Fetcher + trust store** (this slice's follow-up PR):
   `RegistryFetcher` trait + `HttpsRegistryFetcher` impl +
   trust-store loader + unit tests with `MockFetcher`. No
   CLI wiring yet. ~300 LOC across 2-3 files.
2. **F2 — Verifier**: Ed25519 verify path, audit row, refusal
   shape. Hook into existing `install-file` codepath behind
   a feature flag. ~200 LOC.
3. **F3 — CLI subcommands**: `search`, `install`, `info`,
   `registry add|list|remove`, `trust add|list`. ~400 LOC
   across `crates/convergio-cli/src/commands/capability/`.
4. **F4 — Reference registry**: separate `convergio-registry`
   repo with the static-site + CI rebuild attestation. Out of
   scope for this workspace but tracked as issue.
5. **F5 — Docs**: `docs/capability-registry.md` for authors
   and mirror operators.

Each slice is one PR with its own tests. F1 unblocks F2; F3
depends on F2; F4 and F5 are parallelizable after F3.

## References

- ADR-0008 — capability bundle format and local registry.
- `docs/plans/v1.0-production-ready.md` § W9.
- Issue #429 — W11 phase 2 (blocked on this ADR's F2).
- Issue #396 — v1.0 tracking.
