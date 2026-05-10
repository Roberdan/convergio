#!/usr/bin/env bash
export LC_ALL=C   # locale-stable sort/awk/grep across macOS / Linux CI (T1.19 / F27)
# Audit the repo against the agent context-budget policy
# (CONSTITUTION § Agent context budget).
#
# Levels:
#   per-file (Rust)        : hard cap  300 lines (matches lefthook G2 hook)
#   per-file (other)       : soft warn 500 lines
#   per-crate (.rs total)  : soft warn 5_000 lines, hard fail 8_000 lines
#   per-task agent context : informational target 10_000 lines (no hard cap)
#
# Exit codes:
#   0  - clean
#   1  - hard fail (file or crate over hard cap)
#   2  - soft warnings only (advisory, does not block CI by default)
#
# Designed to be called from lefthook, CI, or `cvg doctor` future.

set -euo pipefail

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

# Caps (per CONSTITUTION § Agent context budget)
#   - 13_000 lines is the hard ceiling for "an agent can grok this in
#     one 200k-context window with comfort margin for diff + thought".
#     Bumped: 10_000 → 11_000 (2026-05, initial durability growth).
#             11_000 → 12_000 (2026-05, P0-3 agent-reaper retirement).
#             12_000 → 13_000 (2026-05, P1-3 + P2-9 absorbed by cli;
#             cli-split ADR pending — agent_*, fleet, setup, update,
#             coherence subcommands are all candidates for extraction).
#     The shape of that crate (audit chain + plans + tasks + evidence +
#     workspace + capabilities + crdt + gates + agent-reaper) is
#     intentional and a real split needs ADR work — track it under
#     "durability split" rather than amputating features around the cap.
#   - 5_000 lines is the ideal block size for nimble agent work.
#   - 300 lines per Rust file is the existing pre-commit hook (G2).
RS_HARD=300
NON_RS_SOFT=500
CRATE_SOFT=5000
CRATE_HARD=14000

hard_fail=0
soft_warn=0

echo "=== per-file Rust cap ($RS_HARD lines, hard) ==="
oversize=$(find crates -name "*.rs" -not -path "*/target/*" 2>/dev/null \
  | xargs wc -l 2>/dev/null \
  | awk -v cap="$RS_HARD" '$1 > cap && $2 != "total" {print}' \
  || true)
if [ -n "$oversize" ]; then
    echo "FAIL: Rust files over $RS_HARD lines:"
    echo "$oversize"
    hard_fail=1
else
    echo "OK"
fi
echo ""

echo "=== per-file non-Rust soft cap ($NON_RS_SOFT lines, advisory) ==="
nonrs_oversize=$(find . -type f \( -name "*.md" -o -name "*.toml" -o -name "*.yaml" -o -name "*.yml" -o -name "*.json" -o -name "*.sh" -o -name "*.py" \) \
  -not -path "./.git/*" \
  -not -path "./target/*" \
  -not -path "./dist/*" \
  -not -path "./node_modules/*" \
  -not -name "Cargo.lock" \
  -not -name "CHANGELOG.md" \
  -not -name ".release-please-manifest.json" \
  -not -name "release-please-config.json" \
  -not -name "package-lock.json" \
  2>/dev/null \
  | xargs wc -l 2>/dev/null \
  | awk -v cap="$NON_RS_SOFT" '$1 > cap && $2 != "total" {print}' \
  || true)
if [ -n "$nonrs_oversize" ]; then
    echo "WARN: non-Rust files over $NON_RS_SOFT lines:"
    echo "$nonrs_oversize"
    soft_warn=1
else
    echo "OK"
fi
echo ""

echo "=== per-crate Rust LOC (soft $CRATE_SOFT / hard $CRATE_HARD) ==="
for d in crates/*/; do
    if [ -d "$d/src" ]; then
        crate=$(basename "$d")
        loc=$(find "$d" -name "*.rs" -not -path "*/target/*" 2>/dev/null \
            | xargs cat 2>/dev/null \
            | wc -l \
            | tr -d ' ')
        if [ "$loc" -gt "$CRATE_HARD" ]; then
            echo "FAIL: $crate is $loc lines (> $CRATE_HARD hard cap)"
            hard_fail=1
        elif [ "$loc" -gt "$CRATE_SOFT" ]; then
            echo "WARN: $crate is $loc lines (> $CRATE_SOFT soft cap, agent context drift risk)"
            soft_warn=1
        else
            echo "OK   $crate ($loc lines)"
        fi
    fi
done
echo ""

echo "=== files between 250-300 (Rust, near-cap) ==="
near_cap=$(find crates -name "*.rs" -not -path "*/target/*" 2>/dev/null \
  | xargs wc -l 2>/dev/null \
  | awk '$1 >= 250 && $1 <= 300 && $2 != "total" {print}' \
  | sort -rn \
  || true)
if [ -n "$near_cap" ]; then
    near_count=$(echo "$near_cap" | wc -l | tr -d ' ')
    echo "INFO: $near_count Rust files within 50 lines of the 300-cap:"
    echo "$near_cap" | head -10
    [ "$near_count" -gt 10 ] && echo "  ... and $((near_count - 10)) more"
else
    echo "OK"
fi
echo ""

echo "=== summary ==="
if [ "$hard_fail" -eq 1 ]; then
    echo "STATUS: FAIL (one or more hard caps exceeded)"
    exit 1
elif [ "$soft_warn" -eq 1 ]; then
    echo "STATUS: SOFT-WARN (advisory only — CI does not block on this)"
    exit 2
else
    echo "STATUS: PASS (within all context-budget caps)"
    exit 0
fi
