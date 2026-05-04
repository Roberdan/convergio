---
id: 0008
status: proposed
date: 2026-04-29
topics: [extensibility, security, supply-chain, mcp]
related_adrs: []
touches_crates: []
last_validated: 2026-04-30
---

# 0008. Install new behavior as signed isolated capabilities

- Status: proposed
- Date: 2026-04-29
- Deciders: Roberto, Copilot, Opus 4.7 review
- Tags: extensibility, security, supply-chain, mcp

## Context and Problem Statement

Convergio should evolve without becoming another monolith. Future
features such as planner, GitHub integration, organization support, or
cloud sync should be installable on demand instead of always compiled
into one binary.

Downloading executable code from the internet creates a supply-chain
risk. The extension model must therefore be explicit, signed, isolated,
auditable, and reversible from the first implementation.

## Decision Drivers

- Core should stay small and reliable.
- Features should be installable with a stable user experience.
- Remote install must not execute unsigned code.
- Capability crashes must not crash the daemon.
- Capabilities must not mutate core audit/task tables directly.
- MCP should remain a two-tool surface for agents.

## Considered Options

1. **Compile every feature into the core** — simple but trends toward a
   monolith.
2. **Rust dynamic library plugins** — flexible but unsafe because Rust ABI
   is not stable and plugin crashes can take down the daemon.
3. **Signed capability packages running as separate processes** — more
   operational complexity, but safer and extensible.

## Decision Outcome

Chosen option: **Option 3**, because it separates failure domains and
allows Convergio to grow without giving arbitrary code access to the core
process.

### Implementation status (2026-05-04)

**Partially shipped — status remains `proposed` until the remote
distribution model lands.**

Shipped today:

- Ed25519 signature verification primitive
  (`convergio_durability::capability_signature::capability_signature_payload`).
- `POST /v1/capabilities/install-file` and
  `POST /v1/capabilities/verify-signature` routes; matching
  `cvg capability install-file` / `cvg capability verify-signature`
  CLI verbs.

Not yet shipped (and load-bearing for graduating this ADR):

- Capability `disable` / `remove` / `sync` lifecycle verbs.
- Daemon-to-capability JSON-RPC stdio protocol with per-capability
  tokens and action allow-listing.
- Capability-local `state.db` migrations + registry-recorded migration
  hashes.
- `doctor.json` declarative checks consumed by `cvg doctor`.
- Supply-chain CI (cargo-deny / cargo-audit / SBOM / build provenance)
  and the first-party remote registry.

### Positive consequences

- New features can ship independently.
- Core daemon remains small.
- Capability processes can be supervised, disabled, and audited.
- Remote installs can be blocked until signature verification succeeds.

### Negative consequences

- Requires package format, registry, signing, uninstall, and rollback.
- Requires compatibility metadata and lifecycle management.
- First implementation must be careful to avoid supply-chain shortcuts.

## Package format

Capability archive:

```text
manifest.toml
bin/convergio-cap-<name>
migrations/
docs/
schemas/
doctor.json
locales/en/
locales/it/
```

The manifest declares:

- name and version;
- compatible Convergio core version range;
- platform triplets such as `aarch64-apple-darwin`,
  `x86_64-unknown-linux-gnu`, and `x86_64-pc-windows-msvc`;
- binary path;
- SHA-256 hashes;
- signature metadata;
- MCP action schemas;
- migrations;
- doctor checks;
- requested permissions;
- localization coverage.

## Install location

Capabilities install under:

```text
~/.convergio/capabilities/<name>/
```

Installation uses a staging directory and an atomic move into place after
validation.

## Trust and signing

Remote install is not allowed before signature verification exists.

Minimum trust model:

- package signatures use minisign-compatible Ed25519 signatures;
- Convergio core embeds first-party root keys and root metadata;
- root metadata supports multiple keys and a signature threshold;
- target metadata delegates package signing to release keys;
- optional `~/.convergio/trust/keys.toml` supports self-hosted keys;
- revocation metadata is checked before install and during capability
  startup;
- signature covers the manifest hash and artifact hashes;
- manifest includes validity bounds to reduce replay of old vulnerable
  packages;
- macOS binaries in first-party packages are codesigned/notarized.

No `--allow-unsigned` flag is allowed in the first public release.

The current core exposes a local verification primitive and a signed local
install-file path:

```bash
cvg capability verify-signature \
  --name planner \
  --version 0.1.0 \
  --checksum sha256:<artifact-sha256> \
  --manifest manifest.json \
  --signature <ed25519-signature-hex> \
  --trusted-key first-party:<ed25519-public-key-hex>

cvg capability install-file ./planner.tar.gz \
  --signature <ed25519-signature-hex> \
  --trusted-key first-party:<ed25519-public-key-hex>

cvg capability disable planner
cvg capability remove planner
```

The signature payload schema is `convergio.capability.signature.v1` and
covers the capability name, version, package checksum, and manifest
SHA-256. Remote install code must load trusted keys from core trust
metadata or the local trust override; it must not accept a key fetched
from the same remote package as a trust root.

Root rotation requires new root metadata signed by both the old threshold
and the new threshold. If a daemon is too old to understand the current
root metadata, remote capability install is refused until Convergio core
is upgraded.

Clock behavior is conservative. If local time is outside metadata
validity bounds and no recent trusted registry timestamp is cached,
remote install is refused. Already-installed capabilities may keep
running during an offline grace period unless they are locally marked
revoked.

Installed capabilities are rechecked for revocation on daemon startup and
at least daily when registry access is available. A revoked installed
capability is disabled, its actions are removed from MCP help, and doctor
reports a `capability_revoked` finding.

## Supply-chain release requirements

Remote capability distribution requires supply-chain CI before it is
enabled:

- dependency policy checks with `cargo deny`;
- vulnerability checks with `cargo audit`;
- SBOM attached to each release artifact;
- GitHub build provenance/attestation for produced archives;
- detached capability signatures produced by CI or a documented release
  signing machine, never ad hoc inside application code;
- release metadata containing artifact digest, source revision, builder
  identity, and signing key ID.

The first public remote registry is first-party only. Third-party
publishing, transparency logs, and marketplace trust require a later ADR.

## Process isolation

Capabilities are separate processes, never Rust dylibs loaded into the
daemon.

Rules:

- run with a restricted working directory;
- communicate with the daemon through a narrow local protocol;
- use per-capability tokens and action allow-lists;
- do not access core SQLite tables directly;
- use a capability-local SQLite database unless a future ADR allows
  namespaced shared tables;
- cannot read audit log or other capabilities' state unless explicitly
  granted by daemon APIs.

Separate process does not mean full sandbox. v0.1 capability isolation is
process isolation plus least-privilege daemon APIs:

- no direct core database path in the capability environment;
- no inherited sensitive environment variables except explicit allow-list;
- per-capability log files;
- restart rate limits;
- configurable working directory under the capability install root.

OS sandboxing is platform-specific and requires a later implementation
decision before untrusted third-party capabilities are allowed. Until
then, remote capabilities are first-party only and should be described as
isolated processes, not as fully sandboxed code.

## Daemon protocol

The daemon-to-capability boundary is a local RPC protocol owned by
Convergio, not raw database access.

Initial transport: stdio for supervised child processes. The daemon
spawns the capability, passes a short-lived capability token over the
child environment, and speaks newline-delimited JSON-RPC. Future UDS
transport may be added for long-running capabilities.

Authorization rules:

- tokens are scoped to one capability name, version, and process ID;
- tokens rotate on process restart;
- every capability request includes an action name;
- daemon validates action name against the installed manifest allow-list;
- namespace ownership is exclusive, so only capability `planner` can
  expose `planner.*`;
- requests outside the allow-list are refused and audited.

## Migrations

Each capability owns:

```text
~/.convergio/capabilities/<name>/state.db
```

Shared core SQLite migrations are out of scope for v0.1. Core tables such
as `plans`, `tasks`, `evidence`, `audit_log`, and CRDT tables are
off-limits.

Migrations must be transactional. The registry records applied migration
hashes. Remove/upgrade must have explicit semantics; data may be kept
disabled rather than destructively dropped.

## MCP action routing

The visible MCP surface remains:

```text
convergio.help
convergio.act
```

Capability actions are dynamic and namespaced:

```text
planner.solve
github.open_pr
```

`convergio.help(topic="actions")` includes installed capability action
schemas. Capability action additions do not bump the core schema version;
core protocol changes do.

Action schemas use JSON Schema 2020-12. The daemon validates request
parameters before forwarding to the capability and validates capability
responses before returning them to MCP clients. A capability version may
add actions, but changing or removing an existing action requires a new
capability major version.

Schema conflicts are install-time errors. Namespace conflicts are refused
before extraction is committed.

## Registry

Remote registry is deferred until signing is implemented.

Registry entries include:

- capability name;
- latest version;
- platform URL;
- checksum;
- signature URL;
- compatibility range;
- revoked versions.

For the first public release, remote registry support is first-party
only. Third-party marketplace and multi-publisher trust are out of scope.

## Doctor checks

Capability `doctor.json` is declarative only. It cannot run arbitrary
shell commands.

Allowed checks may include:

- file exists;
- manifest version;
- daemon endpoint reachable;
- capability process status;
- schema version match.

The core validates doctor check schemas before persisting them.

Example:

```json
{
  "checks": [
    {"kind": "file_exists", "path": "bin/convergio-cap-planner"},
    {"kind": "manifest_version", "version": "0.1.0"},
    {"kind": "schema_version", "action": "planner.solve", "version": 1}
  ]
}
```

## Permissions

Capability manifests declare permissions from a closed enum:

| Permission | Meaning |
|------------|---------|
| `plan:read` | read plans/tasks/evidence through daemon APIs |
| `plan:write` | create/update plans/tasks through daemon APIs |
| `workspace:read` | inspect workspace resource metadata |
| `workspace:propose_patch` | submit patch proposals |
| `network:none` | no daemon-granted network need |
| `network:https` | requires outbound HTTPS |
| `capability_db:readwrite` | use its own capability-local DB |

The daemon enforces only daemon-mediated permissions. OS-level network
and filesystem enforcement requires the future sandboxing work described
above.

## Supervision

Capabilities run under daemon supervision:

- max restart rate is enforced per capability;
- repeated crashes disable the capability until manual restart;
- stdout/stderr are captured to capability-local logs;
- health status is surfaced in `cvg doctor`;
- process exit, restart, disable, and crash-loop events are audited.

## Uninstall, disable, and rollback

Required operations:

- disable: stop process and hide actions, keep data;
- remove: delete package files only when no core references require it;
- upgrade: install new version into staging, run migrations, switch
  atomically;
- rollback: revert to previous package if upgrade fails before cutover.

Refusal to remove is a first-class error with a reason.

## Required tests

- install local signed package;
- reject unsigned or bad-signature package;
- reject unsafe archive paths/symlinks/device files;
- reject incompatible core version;
- disable removes actions from MCP catalog;
- failed migration rolls back;
- capability process crash does not crash daemon.

## Links

- Related ADRs: [0006](0006-crdt-storage.md),
  [0007](0007-workspace-coordination.md)
