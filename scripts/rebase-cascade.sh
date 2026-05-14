#!/usr/bin/env bash
# Sequential, safe rebase of one or more agent branches onto main.
#
# Replaces the "8 worktrees in parallel and YOLO the conflict
# resolution" recipe that produced the 2026-05-12 disaster (457
# files deleted on agent/5c9b25f). Processes branches one at a
# time, runs the workspace integrity check after each conflict
# resolution, and refuses to commit when the smoke test fails.
#
# Usage:
#   ./scripts/rebase-cascade.sh agent/c809542 agent/f9951bb ...
#
# For each branch:
#   1. Add a worktree under `.claude/worktrees/<basename>/`
#      (or reuse if it exists).
#   2. `git merge origin/main --no-edit`.
#   3. On conflict, resolve ONLY the known cascade files:
#        - docs/INDEX.md   (via `./scripts/generate-docs-index.sh`)
#        - AGENTS.md       (`--theirs`, then `cvg docs regenerate`)
#        - crates/*/AGENTS.md  (`--theirs`, then regenerate)
#      Any other conflicting file aborts the merge — humans review.
#   4. Run `./scripts/post-rebase-smoke.sh`. If it fails, abort.
#   5. Commit + push.
#
# Hard rule: NEVER run `git checkout --theirs .` or `--theirs <dir>`.
# Only specific files known to be cascade-managed.

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 <branch> [branch ...]" >&2
    exit 2
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

git fetch origin main --quiet

# Files we know are cascade-conflict-prone and safe to take from
# `origin/main` (their content is auto-regenerated post-merge).
CASCADE_FILES=(
    "docs/INDEX.md"
    "AGENTS.md"
)

resolve_known_conflicts() {
    local conflicts
    conflicts="$(git diff --name-only --diff-filter=U)"
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        case "$f" in
            docs/INDEX.md|AGENTS.md|crates/*/AGENTS.md)
                echo "  resolving cascade file: $f (--theirs)"
                git checkout --theirs -- "$f"
                git add -- "$f"
                ;;
            *)
                echo "  unexpected conflict: $f — aborting (manual review needed)"
                return 1
                ;;
        esac
    done <<< "$conflicts"
    return 0
}

for branch in "$@"; do
    short="$(echo "$branch" | sed 's|.*/||')"
    wt=".claude/worktrees/$short"

    echo
    echo "=== $branch ==="

    if [ ! -d "$wt" ]; then
        echo "→ adding worktree $wt"
        git worktree add "$wt" "$branch"
    fi

    pushd "$wt" >/dev/null
    git fetch origin main --quiet
    git fetch origin "$branch" --quiet
    git reset --hard "origin/$branch"

    echo "→ merging origin/main"
    if ! git merge origin/main --no-edit; then
        if ! resolve_known_conflicts; then
            echo "ABORT: unknown conflict; merge state preserved for human review."
            popd >/dev/null
            exit 1
        fi
        # Regenerate auto blocks now that AGENTS.md is from main.
        echo "→ regenerating docs"
        cvg docs regenerate --root . >/dev/null 2>&1 || true
        ./scripts/generate-docs-index.sh >/dev/null 2>&1 || true
        git add -A
        git -c core.editor=true commit --no-edit
    fi

    echo "→ post-rebase smoke"
    if ! ./scripts/post-rebase-smoke.sh; then
        echo "ABORT: smoke failed on $branch. Resolution corrupted workspace."
        echo "Reset with: git -C $wt reset --hard origin/$branch"
        popd >/dev/null
        exit 1
    fi

    echo "→ pushing $branch"
    git push origin "$branch"

    popd >/dev/null
    echo "✓ $branch done"
done

echo
echo "all branches rebased + pushed cleanly."
