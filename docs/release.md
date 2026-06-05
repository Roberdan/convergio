# Release, signing, and notarization

This repo can produce local release artifacts without requiring a hosted
service. macOS signing and notarization use Apple credentials from the
developer's machine or CI secrets; credentials must never be committed.

## Current local macOS state

On this development Mac, the one-time notarization setup has already
been completed:

| Item | Value |
|------|-------|
| Team ID | `93T3LG4NPG` |
| Signing identity | `Developer ID Application: Fight The Stroke Foundation (93T3LG4NPG)` |
| notarytool profile | `convergio-notary` |
| Last accepted submission | `b18cbba7-ce78-4a45-a8fa-278746070145` |
| Last notarized artifact | `dist/convergio-darwin-arm64-signed.zip` |
| Last artifact SHA-256 | `ea578f808e35918178477e94e894fa78c580d2e2de8e2adbd0dd64038425e79b` |
| Public repository | `https://github.com/Roberdan/convergio-local` |
| Public release | `https://github.com/Roberdan/convergio-local/releases/tag/v0.1.0` |

The temporary Desktop setup helper can be deleted after the profile is
created because `notarytool` stores the credential in the macOS Keychain.

## Normal local release flow

After code changes, build, package, sign, and notarize with:

```bash
sh scripts/package-local.sh
APPLE_NOTARY_PROFILE=convergio-notary sh scripts/sign-macos-local.sh
```

This produces:

| File | Purpose |
|------|---------|
| `dist/convergio-darwin-arm64.tar.gz` | unsigned local tarball |
| `dist/convergio-darwin-arm64-signed.zip` | signed and notarized macOS zip |
| `dist/convergio-darwin-arm64-signed.zip.sha256` | checksum |

Verify the result:

```bash
for bin in dist/convergio-darwin-arm64/bin/convergio \
  dist/convergio-darwin-arm64/bin/cvg \
  dist/convergio-darwin-arm64/bin/convergio-mcp; do
  codesign --verify --strict --verbose=2 "$bin"
done

xcrun notarytool log <submission-id> --keychain-profile convergio-notary
```

## One-time notarization setup

Only repeat this if the Keychain profile is missing, expired, or created
for the wrong Apple ID:

```bash
xcrun notarytool store-credentials convergio-notary \
  --apple-id "<apple-id-in-team>" \
  --team-id "93T3LG4NPG"
```

Use an Apple **app-specific password**, not the normal iCloud password
and not a 2FA code. The Apple ID must belong to the developer team.

## CI release workflow

`.github/workflows/release.yml` runs fmt, clippy, tests, `cargo deny`,
and `cargo audit` before publishing release artifacts on tags.

It produces:

- Linux + macOS tarballs
- SPDX JSON SBOMs (for binaries + image)
- SHA-256 checksums
- GitHub build-provenance attestations (OIDC)
- **Keyless cosign signatures** (`.sig` + `.crt`) for each tarball/SBOM/checksum
- A **daemon container image** pushed to GHCR (`ghcr.io/<owner>/<repo>`) and signed with cosign
- A signed **promotion bundle** consisting of:
  - `convergio-artifact-manifest.json` (immutable manifest)
  - `convergio-image.DIGEST` (pinned image digest)
  - `convergio-image.spdx.json` (image SBOM)
  - `convergio-artifact-manifest.SHA256SUMS` (checksums for the bundle)

The image SBOM is both uploaded as a signed Release asset and attached to the
image as a keyless cosign attestation (`--type spdxjson`).

## Promotion DAG (dev → stage → preprod → prod)

`.github/workflows/promotion.yml` is a manual `workflow_dispatch` pipeline that:

1. Downloads the signed promotion bundle from the GitHub Release
2. Verifies blob signatures + SHA256SUMS, verifies the image signature, and asserts the image has an SBOM attestation
3. Records a signed promotion step for each environment (GitHub Environments)

It supports an optional `prod-canary` hop and a configurable bake wait before
final promotion to `prod`.

To notarize in CI later, add GitHub secrets for either:

| Secret | Meaning |
|--------|---------|
| `APPLE_API_KEY_PATH` or `.p8` content secret | App Store Connect API key |
| `APPLE_API_KEY_ID` | API key ID |
| `APPLE_API_ISSUER_ID` | issuer UUID |
| `APPLE_SIGNING_CERTIFICATE_P12` | Developer ID Application certificate |
| `APPLE_SIGNING_CERTIFICATE_PASSWORD` | certificate password |

Do not fake signing or notarization in CI. If credentials are absent,
publish unsigned artifacts and label them as unsigned.

## Local supply-chain checks

Local development does not require supply-chain tools unless you want to
preflight CI. Optional commands:

```bash
cargo install cargo-deny --locked
cargo install cargo-audit --locked
cargo deny --locked check advisories bans licenses sources
cargo audit
```

`deny.toml` owns dependency source, license, ban, and RustSec advisory
policy. `.cargo/audit.toml` makes `cargo audit` fail on vulnerabilities,
unsound/unmaintained informational advisories, and yanked crates. SBOMs and
GitHub provenance are release workflow outputs; they are not a substitute
for future capability package signatures.
