#!/usr/bin/env bash
# Post-merge / post-rebase smoke test.
#
# Run this immediately after resolving a merge or rebase BEFORE
# committing the resolution. It exits non-zero if the resolution
# produced a broken workspace, which is what `git merge -X theirs`
# or a wide `git checkout --theirs <path>` can silently do.
#
# Specifically catches the 2026-05-12 disaster on agent/5c9b25f
# where a broad conflict-resolution marked 457 files (including
# Cargo.toml) as deleted, then the merge was committed before
# anyone noticed.
#
# Usage:
#   ./scripts/post-rebase-smoke.sh
#   ./scripts/post-rebase-smoke.sh --crate convergio-server
#
# Wrap any merge-cascade recipe with this:
#
#   git checkout --theirs docs/INDEX.md
#   ./scripts/generate-docs-index.sh
#   ./scripts/post-rebase-smoke.sh || { echo "ABORT — do not commit"; exit 1; }
#   git add -A && git commit ...

set -euo pipefail

CRATE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --crate) CRATE="$2"; shift 2 ;;
        *) echo "unknown arg: $1"; exit 2 ;;
    esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

echo "→ workspace integrity"
./scripts/check-workspace-integrity.sh

echo
echo "→ cargo check ${CRATE:+-p $CRATE}${CRATE:-(workspace)}"
if [ -n "$CRATE" ]; then
    cargo check -p "$CRATE" --quiet 2>&1 | tail -3
else
    # Workspace-wide is slow; only do it when caller asked for it
    # via empty --crate (default). The integrity check already
    # confirmed `cargo metadata` works, so skip the heavy check
    # by default.
    cargo metadata --no-deps --format-version 1 >/dev/null
    echo "  metadata OK (skipped full check; pass --crate <name> for typecheck)"
fi

echo
echo "post-rebase smoke OK — safe to commit"
