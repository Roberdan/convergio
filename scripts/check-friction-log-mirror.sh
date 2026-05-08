#!/usr/bin/env bash
# Reject PRs that either:
#   (A) reference a friction id (F##) in commit messages without a
#       corresponding row in the friction log (closes F37 future fix), or
#   (B) add a new actionable F## row to the friction log without a
#       matching entry in the "Daemon task mirror" section
#       (closes F40 — keep one source of truth for outstanding work).
#
# Logic:
#   A. Scan `git log <base>..HEAD --pretty=%B` for `\bF[0-9]+\b`.
#      For each referenced F##, require a `| F## |` row in
#      docs/plans/v0.2-friction-log.md.
#   B. If the friction log file changes in this branch, find newly-added
#      actionable rows and require each to have a daemon mirror row that
#      includes both the F## label and a UUIDv4.
#
# Exit codes:
#   0  clean (or N/A)
#   1  one or more checks failed
#   2  malformed inputs (no friction log file present)

set -euo pipefail
export LC_ALL=C

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

LOG_PATH="docs/plans/v0.2-friction-log.md"
BASE_REF="${BASE_REF:-origin/main}"

if [ ! -f "$LOG_PATH" ]; then
  echo "friction log not found at $LOG_PATH" >&2
  exit 2
fi

# Resolve base ref. We *fail closed* if it's missing: otherwise a shallow
# checkout could silently scan only a suffix of commits and let older PR
# commits reference missing friction IDs.
if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "base ref not found: $BASE_REF" >&2
  echo "Fix: ensure the checkout fetched the base branch (e.g. actions/checkout fetch-depth: 0)," >&2
  echo "or rerun with BASE_REF=<ref> that exists locally." >&2
  exit 2
fi

extract_f_ids() {
  # Extract every `\bF[0-9]+\b` from stdin.
  # Implemented in awk for portability: BSD grep lacks `\b` and GNU-only
  # `\<`/`\>` word-boundary operators.
  #
  # Regex \b uses [A-Za-z0-9_] as "word" chars; treat `_` as a word char
  # so we don't match e.g. `FOO_F37`.
  awk '
    {
      s = $0
      while (match(s, /F[0-9]+/)) {
        pre = (RSTART == 1) ? "" : substr(s, RSTART - 1, 1)
        post = substr(s, RSTART + RLENGTH, 1)
        if ((pre == "" || pre !~ /[[:alnum:]_]/) && (post == "" || post !~ /[[:alnum:]_]/)) {
          print substr(s, RSTART, RLENGTH)
        }
        s = substr(s, RSTART + RLENGTH)
      }
    }
  '
}

# A) Commit messages must not reference missing friction-log rows.
log_text=$(git log "$BASE_REF"..HEAD --pretty=%B 2>/dev/null || true)
mentioned=$(printf "%s\n" "$log_text" | extract_f_ids | sort -u)
if [ -n "$mentioned" ]; then
  logged=$(awk '/^\| F[0-9]+ \|/ { print $2 }' "$LOG_PATH" | sort -u)
  missing_from_log=$(comm -23 <(printf "%s\n" "$mentioned") <(printf "%s\n" "$logged") 2>/dev/null || true)

  if [ -n "$missing_from_log" ]; then
    echo "FAIL: commit messages reference friction IDs missing from $LOG_PATH:" >&2
    while IFS= read -r m; do
      [ -z "$m" ] && continue
      echo "  - $m" >&2
    done <<< "$missing_from_log"
    echo >&2
    echo "Fix: add a row for each missing F## to the friction log (at minimum the Summary table)." >&2
    exit 1
  fi

  echo "OK: commit messages reference only logged F## ids"
fi

# B) New actionable friction-log rows must be mirrored in the daemon.
#    Only relevant when the file itself changed in this branch.
#
# 1) Lines added to the friction log file in this branch.
#    To distinguish a *new* F## row from a *status update* on an
#    existing one, we keep only labels present in `+` lines but
#    absent from `-` lines. Status updates show both signs for the
#    same F## label and are skipped — we only block actually-new
#    rows that lack a daemon mirror.
diff_lines=$(git diff "$BASE_REF"...HEAD -- "$LOG_PATH" 2>/dev/null || true)
if [ -z "$diff_lines" ]; then
  echo "no diff against $BASE_REF for $LOG_PATH — skipping mirror check"
  exit 0
fi

added_labels=$(echo "$diff_lines" \
  | awk '/^\+\| F[0-9A-Za-z-]+ \|/ {gsub(/^ +| +$/,"",$2); print $2}' \
  | sort -u)
removed_labels=$(echo "$diff_lines" \
  | awk '/^-\| F[0-9A-Za-z-]+ \|/ {gsub(/^ +| +$/,"",$2); print $2}' \
  | sort -u)

# `comm -23` keeps lines unique to the first input.
truly_new=$(comm -23 <(echo "$added_labels") <(echo "$removed_labels") 2>/dev/null || true)

if [ -z "$truly_new" ]; then
  echo "no new F## rows in $LOG_PATH — skipping mirror check"
  exit 0
fi

# Rebuild the full added rows for the truly-new labels so we can
# inspect their status column. Done as a per-label loop to stay
# portable across awk dialects (BSD awk on macOS rejects newlines
# in `-v` variable values).
added=""
while IFS= read -r lbl; do
  [ -z "$lbl" ] && continue
  row=$(echo "$diff_lines" \
    | awk -v want="$lbl" '
        $0 ~ "^\\+\\| " want " \\|" {
          print substr($0, 2)
          exit
        }
      ')
  [ -n "$row" ] && added="${added}${row}
"
done <<< "$truly_new"

# 2) Build the set of F## labels that appear in the daemon mirror
#    table together with a UUID-shaped token. The mirror header is
#    "Daemon task mirror"; every row has the form
#    `| F## | <plan> | \`<uuid>\` | ...`.
mirror_labels=$(awk '
  /^## Daemon task mirror/ { in_mirror = 1; next }
  /^## / && in_mirror      { in_mirror = 0 }
  in_mirror && /^\| F[0-9A-Za-z-]+ \|/ {
    label = $2
    # accept UUID v4-shape token in row, e.g. ``<8>-<4>-<4>-<4>-<12>``
    if (match($0, /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/)) {
      print label
    }
  }
' "$LOG_PATH" | sort -u)

# 3) For every newly added F##, require it to appear in the mirror set,
#    UNLESS the row's status column is exactly "accepted" (P3 by-design).
missing=""
while IFS= read -r row; do
  [ -z "$row" ] && continue
  label=$(echo "$row" | awk -F '|' '{gsub(/^ +| +$/,"",$2); print $2}')
  status=$(echo "$row" | awk -F '|' '{gsub(/^ +| +$/,"",$5); print $5}')
  case "$status" in
    accepted|"n/a (positive)") continue ;;
  esac
  if ! grep -qx "$label" <<< "$mirror_labels"; then
    missing="$missing $label"
  fi
done <<< "$added"

if [ -n "$missing" ]; then
  echo "FAIL: new actionable friction-log rows are missing a daemon mirror entry:" >&2
  for m in $missing; do echo "  - $m" >&2; done
  echo >&2
  echo "Fix: create a daemon task and add a row to the 'Daemon task mirror'" >&2
  echo "table in $LOG_PATH (see AGENTS.md § Friction log ↔ daemon mirror)." >&2
  exit 1
fi

echo "OK: every new F## row has a daemon mirror entry"
