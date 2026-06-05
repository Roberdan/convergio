# AGENTS.md — convergio-capability-registry

Responsibility: remote capability registry — HTTPS resolution of
capability bundles + versioned Ed25519 trust store. **F1 slice only**
of [ADR-0072](../../docs/adr/0072-remote-capability-registry.md).
F2 (signature verification + daemon audit row + 409 path) and F3 (CLI
surface) land in follow-up PRs.

## Boundaries

- Pure library. No DB access, no audit writes, no axum routing.
- All HTTP I/O goes through the [`RegistryFetcher`] trait so unit
  tests use [`MockFetcher`] (no live network in CI).
- HTTPS-only (ADR-0072 § 5): `HttpsRegistryFetcher::new` rejects any
  non-`https://` URL at construction time.
- 10 s connect / 30 s read timeout, 50 MB default bundle cap, no
  cross-origin redirects.

## Invariants

- `TrustStore::lookup` filters out unknown / revoked / out-of-window
  entries — never expose any of those three to a verifier.
- `TrustStoreEntry::verifying_key` validates algorithm tag, key
  length (32 bytes), revocation status, and base64 shape **before**
  handing the key to `ed25519-dalek`.
- Overlay merge is lexicographic, last-write-wins by `key_id` — this
  is how operators rotate or revoke baked-in roots without rebuilding
  the daemon.

## Tests

- Unit tests live next to the code they cover, integration tests
  under `tests/`. Files stay under the 300-line cap.
- No live network. Add new behaviour by extending `MockFetcher`,
  never by talking to a real registry.
