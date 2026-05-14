#!/usr/bin/env bash
# Refuse to proceed when the workspace skeleton is broken.
#
# Driven by the 2026-05-12 incident where a botched merge-cascade
# resolution on agent/5c9b25f marked 457 files as deleted —
# including the workspace `Cargo.toml`. Local CI failed (good)
# but no local check stopped the push before the round-trip.
#
# This script is the missing pre-push tripwire. Run it from any
# checkout / worktree; it exits non-zero with a concrete message
# the moment any of these holds:
#
#   - a sentinel file disappeared from the working tree;
#   - the working tree differs from origin/main by more than
#     `CONVERGIO_MAX_DELETIONS_VS_MAIN` deletions
#     (default 50, override via env);
#   - `cargo metadata --no-deps` cannot find a workspace manifest.
#
# Soft-warn (exit 2) is reserved for advisory cases; the lefthook
# pre-push entry treats only exit 1 as blocking.

set -euo pipefail

# Sentinels = files whose loss means the workspace is structurally
# broken. Keep this list small; we want false-negatives never,
# false-positives rare.
SENTINELS=(
    "Cargo.toml"
    "Cargo.lock"
    "AGENTS.md"
    "ARCHITECTURE.md"
    "CONSTITUTION.md"
    "rust-toolchain.toml"
    "lefthook.yml"
)

MAX_DEL="${CONVERGIO_MAX_DELETIONS_VS_MAIN:-50}"

missing=()
for f in "${SENTINELS[@]}"; do
    if [ ! -f "$f" ]; then
        missing+=("$f")
    fi
done

if [ "${#missing[@]}" -gt 0 ]; then
    echo "workspace integrity FAIL: sentinel file(s) missing from working tree:"
    for f in "${missing[@]}"; do
        echo "  - $f"
    done
    echo
    echo "This means a botched merge/rebase deleted workspace skeleton."
    echo "Recovery: restore from origin/main, e.g."
    echo "  git checkout origin/main -- ${missing[*]}"
    exit 1
fi

# Cross-check against origin/main for surprise mass deletions.
# Only fires when origin/main is fetched; if not, skip silently.
if git rev-parse --verify --quiet origin/main >/dev/null; then
    del_count="$(git diff origin/main --name-only --diff-filter=D 2>/dev/null | wc -l | tr -d ' ')"
    if [ "$del_count" -gt "$MAX_DEL" ]; then
        echo "workspace integrity FAIL: $del_count files deleted vs origin/main (cap $MAX_DEL)."
        echo
        echo "First 10 deletions:"
        git diff origin/main --name-only --diff-filter=D | head -10 | sed 's/^/  - /'
        echo
        echo "If this is intentional (e.g. legitimate large refactor) re-run with:"
        echo "  CONVERGIO_MAX_DELETIONS_VS_MAIN=<N> git push ..."
        exit 1
    fi
fi

# Final structural check: cargo must see the workspace.
if ! cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
    echo "workspace integrity FAIL: \`cargo metadata --no-deps\` cannot find Cargo.toml."
    echo
    echo "This is what CI saw on PR #359 before the 2026-05-12 recovery."
    echo "Recovery: \`git checkout origin/main -- Cargo.toml\` and re-check."
    exit 1
fi

echo "workspace integrity OK"
