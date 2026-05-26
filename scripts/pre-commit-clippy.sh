#!/usr/bin/env bash
set -euo pipefail

# Pre-commit clippy runner.
#
# Goal: keep commits snappy by clippy-checking only the crates touched in the
# staged diff. Full workspace clippy remains a CI concern.

crates=$(git diff --cached --name-only --diff-filter=ACMR \
  | awk -F/ '$1=="crates" && NF>=2 {print $2}' \
  | sort -u)

if [ -z "$crates" ]; then
  echo "clippy (skip) no staged crate files"
  exit 0
fi

for crate in $crates; do
  echo "clippy: $crate"
  RUSTFLAGS="-Dwarnings" cargo clippy -p "$crate" --all-targets -- -D warnings
done
