---
id: 0007
status: accepted
date: 2026-04-29
topics: [layer-4, workspace, git, multi-agent, merge]
related_adrs: []
touches_crates: [convergio-durability, convergio-server, convergio-api]
last_validated: 2026-05-04
---

# 0007. Coordinate multi-agent workspace changes with leases and patch proposals

- Status: accepted (shipped — `workspace_resources`/`workspace_leases`/`patch_proposals`/`merge_queue`/`workspace_conflicts` in migration `0005_workspace_coordination.sql`, full `/v1/workspace/leases`, `/v1/workspace/patches`, `/v1/workspace/merge/next`, `/v1/workspace/merge-queue`, `/v1/workspace/conflicts` surface, action enum `submit_patch_proposal`)
- Date: 2026-04-29
- Deciders: Roberto, Copilot
- Tags: layer-4, workspace, git, multi-agent, merge

## Context and Problem Statement

CRDT-aware database state does not solve the largest practical failure
mode of coding agents: multiple agents editing the same files, rebasing
worktrees, opening noisy pull requests, and triggering CI from
incompatible assumptions.

Convergio must coordinate filesystem and Git changes as part of the core
local runtime. Worktrees are useful execution sandboxes, but they are not
a coordination model. The canonical workspace must be updated only by a
Convergio-controlled merge arbiter.

## Decision Drivers

- Multiple local agents must be able to work in parallel.
- Same-file and stale-base edits must be detected before acceptance.
- Agents must not push, merge, or mark work done based only on isolated
  worktree state.
- CI/gate results must be serialized into durable evidence and audit.
- The model must work locally first and remain compatible with future
  multi-machine operation.

## Considered Options

1. **Let agents use Git directly** — each agent manages branches,
   worktrees, merges, PRs, and CI.
2. **One worktree per agent, humans merge later** — isolate writes but
   defer coordination to humans.
3. **Convergio resource leases + patch proposals + merge arbiter** —
   agents work in sandboxes, then submit patches for Convergio to check,
   queue, apply, test, and audit.

## Decision Outcome

Chosen option: **Option 3**, because it makes concurrency explicit and
keeps "done" tied to accepted, auditable workspace state.

### Positive consequences

- Agents can work in parallel without writing directly to the canonical
  branch.
- Convergio can refuse stale or conflicting patches before they pollute
  Git history.
- CI and gate results become part of the same task/evidence/audit flow.
- The model extends naturally to remote runners and future PR
  integrations.

### Negative consequences

- More coordination state and merge logic in the core.
- Some agent workflows must change from "commit/push" to "submit patch".
- Symbol-level coordination requires future code intelligence to be
  reliable; v0.1 can start with file/directory resources.

## Core model

The workspace layer will model:

| Concept | Purpose |
|---------|---------|
| `workspace_resources` | repo, directory, file, symbol, generated artifact, CI lane |
| `workspace_leases` | scoped time-bound claims on resources |
| `agent_sessions` | actor, task, base revision, sandbox/worktree path |
| `patch_proposals` | submitted diff with base commit and file hashes |
| `merge_queue` | serialized accepted proposals awaiting apply/test |
| `workspace_conflicts` | stale base, same-file edit, semantic conflict, CI failure |

## Lease rules

Leases are mandatory for agents and advisory for humans.

- An agent must hold a lease for every resource it intends to modify.
- Evidence for workspace-changing tasks must reference lease IDs.
- Leases expire and are reaped like stale tasks.
- Directory leases exclude child file/directory leases while active.
- A file lease request inside an active directory lease is refused unless
  the current holder explicitly releases or downgrades the parent lease.
- A directory lease request is refused when any child lease is active.
- Lease upgrades/downgrades are new audited lease operations, not in-place
  mutation.
- Future symbol leases can refine file-level locking once code
  intelligence exists.

## Patch proposal flow

1. Agent claims a task.
2. Agent opens an isolated worktree or sandbox.
3. Agent obtains leases for intended resources.
4. Agent edits locally.
5. Agent submits a patch proposal with:
   - task ID;
   - lease IDs;
   - base commit;
   - affected paths;
   - preimage hashes;
   - patch content;
   - evidence references.
6. Convergio checks lease coverage, stale bases, file hashes, and policy.
7. Accepted proposals enter the merge queue.
8. Merge arbiter applies/rebases/tests one canonical update at a time or
   in safe independent batches.
9. Accept/refuse/rebase/CI results are appended to audit.
10. Task completion remains blocked until gates accept the final evidence.

## Patch format

v0.1 patch proposals use Git patch semantics:

- text changes use `git diff --binary` compatible patches;
- binary file changes are allowed only when the patch contains a Git
  binary patch and stays below the configured size limit;
- file modes and symlink changes are represented explicitly;
- submodule changes are refused in v0.1;
- absolute paths and paths escaping the repository are refused;
- line endings are validated against the preimage hash rather than
  normalized silently;
- generated artifacts require a resource lease on the artifact and, when
  known, on the generator input.

## Git strategy

Worktrees are execution sandboxes, not the source of truth.

- Agents do not push directly.
- Agents do not open PRs directly in v0.1.
- Agents do not update the canonical branch directly.
- The canonical branch is changed only by the merge arbiter.
- PR/CI platform integration can later be a capability, but the core
  already models merge queue and CI lane state.

## Conflict strategy

| Situation | Behavior |
|-----------|----------|
| different files, compatible bases | can merge in parallel or queue safely |
| same file, non-overlapping hunks | may auto-merge after hash checks |
| same file, overlapping hunks | conflict |
| stale base hash | refusal |
| generated artifact touched by multiple agents | conflict unless generator policy allows |
| CI failure | proposal refused or held, task not done |

Source code is not treated as generic text CRDT in v0.1. Code changes
are audited patch operations with deterministic merge checks.

## CRDT relationship

Convergio metadata uses CRDT operations. Source files use leases, patch
proposals, and merge arbitration.

Future semantic CRDTs for structured code may be capabilities, but the
core does not rely on byte-level CRDT merge for correctness.

## CI boundary

The core owns merge queue state and generic CI lane status only:
`queued`, `running`, `passed`, `failed`, `canceled`, and the evidence
references for those results.

Provider-specific CI and pull-request integrations are capabilities.
They report results back to the core through daemon APIs; they do not own
the merge queue tables.

## Required tests

- two agents edit different files and merge cleanly;
- two agents edit the same file and conflict is surfaced;
- stale base hash is refused;
- expired lease is reaped;
- CI/gate refusal blocks `done`;
- audit verifies after merge queue operations.

## Links

- Related ADRs: [0002](0002-audit-hash-chain.md),
  [0006](0006-crdt-storage.md)
