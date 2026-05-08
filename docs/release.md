---
topic: release
status: living
---

# Release artifacts

Convergio publishes prebuilt local binaries as GitHub Release assets. These are meant for **single-user local** installs.

## CI release workflow

On release tags, `.github/workflows/release.yml` runs policy checks and then builds release artifacts for:

- `linux-x86_64`
- `macos-arm64`

For each platform it uploads to the GitHub Release:

- `convergio-<platform>.tar.gz`
- `convergio-<platform>.spdx.json` (SBOM)
- `convergio-<platform>.SHA256SUMS`

It also emits GitHub build-provenance attestations.

## Local packaging (same layout)

To produce a local tarball with the same directory layout as CI:

```bash
sh scripts/package-local.sh
```

This writes:

- `dist/convergio-<platform>.tar.gz`
- `dist/convergio-<platform>.SHA256SUMS` (best-effort; CI publishes a fuller checksum set)

## macOS signing and notarization (optional)

macOS signing/notarization require real Apple credentials and must not be faked in the repo.

On a Mac with a **Developer ID Application** certificate installed:

```bash
sh scripts/package-local.sh
sh scripts/sign-macos-local.sh
```

To notarize, provide either a notarytool keychain profile:

```bash
APPLE_NOTARY_PROFILE=convergio-notary sh scripts/sign-macos-local.sh
```

or App Store Connect API key variables:

```bash
APPLE_API_KEY_PATH=/path/AuthKey_XXXX.p8 \
APPLE_API_KEY_ID=XXXX \
APPLE_API_ISSUER_ID=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx \
sh scripts/sign-macos-local.sh
```

CI can be extended later to notarize by adding the corresponding GitHub secrets. If credentials are absent, publish **unsigned** artifacts and label them as such.

## Local supply-chain checks (optional)

Optional preflight for CI parity:

```bash
cargo install cargo-deny --locked
cargo install cargo-audit --locked
cargo deny --locked check advisories bans licenses sources
cargo audit
```

`deny.toml` owns dependency source, license, ban, and RustSec advisory policy. `.cargo/audit.toml` configures failure policy for `cargo audit`.
