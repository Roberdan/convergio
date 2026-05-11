#!/usr/bin/env bash
# Post-merge fleet cleanup.
#
# Run by lefthook on every `git merge` / `git pull` into the operator's
# main checkout. Mops up the residue that the autonomous PR sessions
# tend to leave behind:
#
#   1. `git worktree prune` — drop stale `.git/worktrees/<id>` admin
#      directories whose worktree dir is gone (typical after running
#      `git worktree remove --force` from outside the worktree).
#   2. Remove worktrees under `.claude/worktrees/agent-*` whose branch
#      no longer exists on `origin` (PR merged with --delete-branch or
#      closed and the operator nuked the remote ref).
#   3. Delete local `agent/<id>` branches whose remote ref is gone.
#   4. Sync `agent_processes`: rows marked `running` whose `pid` no
#      longer answers `kill -0` get flipped to `exited` (exit_code=-1).
#      Closes the loop with `~/.claude/hooks/pre-completion-gate.sh`
#      check 5 — both sides see the same truth.
#
# Idempotent. Best-effort. Never blocks the post-merge flow.
#
# Driven by the 2026-05 insights audit (13 stale remote branches, 6
# zombie processes left after the F2 fleet sessions).

set +e

repo_root=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$repo_root" ]; then
  exit 0
fi
cd "$repo_root" || exit 0

# 1. Drop stale admin dirs under .git/worktrees/.
git worktree prune 2>/dev/null

# 2 + 3. Walk the agent worktrees. For each, check if its branch
#        still exists on origin. If not, remove the worktree + branch.
if [ -d ".claude/worktrees" ]; then
  for wt in .claude/worktrees/agent-*; do
    [ -d "$wt" ] || continue
    name=$(basename "$wt")
    branch="agent/${name#agent-}"
    # Skip if the remote ref is still alive (PR open or merged but
    # branch retained).
    if git ls-remote --exit-code --heads origin "$branch" >/dev/null 2>&1; then
      continue
    fi
    # Skip if the worktree has uncommitted or unpushed work — that
    # signals a live agent run that hasn't reached its push step yet.
    # Killing it here would erase audit reports / WIP commits before
    # they can be salvaged (2026-05-11 incident: cleanup hook fired
    # mid-flight during the per-crate audit pass and wiped two active
    # runners' reports).
    if [ -n "$(git -C "$wt" status --porcelain 2>/dev/null)" ]; then
      continue
    fi
    if [ "$(git -C "$wt" rev-parse HEAD 2>/dev/null)" != "$(git rev-parse HEAD 2>/dev/null)" ]; then
      # Worktree has commits past the operator's main HEAD — likely a
      # local commit that hasn't been pushed yet.
      continue
    fi
    git worktree remove --force "$wt" 2>/dev/null
    git branch -D "$branch" 2>/dev/null
  done
fi

# Also nuke local agent/* branches whose remote ref is gone, even
# when no worktree was attached to them. Skip branches that point to
# commits ahead of main — they belong to a live runner that just
# committed before the push step.
main_head=$(git rev-parse HEAD 2>/dev/null)
for branch in $(git for-each-ref --format='%(refname:short)' refs/heads/agent/ 2>/dev/null); do
  if git ls-remote --exit-code --heads origin "$branch" >/dev/null 2>&1; then
    continue
  fi
  branch_head=$(git rev-parse "$branch" 2>/dev/null)
  if [ -n "$branch_head" ] && [ "$branch_head" != "$main_head" ]; then
    continue
  fi
  git branch -D "$branch" 2>/dev/null
done

# 4. Sync `agent_processes` with live PIDs in the convergio DB.
db=$HOME/.convergio/v3/state.db
if [ -f "$db" ] && command -v sqlite3 >/dev/null 2>&1; then
  running=$(sqlite3 "$db" \
    "SELECT id, pid FROM agent_processes WHERE status='running' AND pid IS NOT NULL;" 2>/dev/null)
  if [ -n "$running" ]; then
    while IFS='|' read -r row_id pid; do
      [ -z "$pid" ] && continue
      if ! kill -0 "$pid" 2>/dev/null; then
        sqlite3 "$db" \
          "UPDATE agent_processes
              SET status='exited',
                  ended_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                  exit_code=-1
            WHERE id='$row_id' AND status='running';" 2>/dev/null
      fi
    done <<< "$running"
  fi
fi

exit 0
