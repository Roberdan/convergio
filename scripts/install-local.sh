#!/usr/bin/env sh
set -eu
export LC_ALL=C   # locale-stable sort/awk/grep across macOS / Linux CI (T1.19 / F27)

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"

fail() {
  echo "error: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

need cargo
need git

cargo install --force --locked --path crates/convergio-server
cargo install --force --locked --path crates/convergio-cli
cargo install --force --locked --path crates/convergio-mcp

warn_shadowed() {
  name="$1"
  expected="$HOME/.cargo/bin/$name"
  actual_path=$(command -v "$name" 2>/dev/null || true)
  if [ -n "$actual_path" ] && [ "$actual_path" != "$expected" ]; then
    cat >&2 <<WARN
WARN: '$name' on PATH is '$actual_path', but Cargo installed '$expected'.
      Fix by putting '$HOME/.cargo/bin' earlier in PATH.
WARN
  fi
}

warn_shadowed convergio
warn_shadowed cvg
warn_shadowed convergio-mcp

# Install Git hooks so the file-size guard, fmt/clippy gates, and
# commitlint run on every commit. Without this every fresh clone
# silently bypasses CONSTITUTION § 13. Closes F31.
#
# F39: clean up any absolute core.hooksPath leftover from a previous
# install or from an ancestor folder rename — lefthook expects the
# default relative .git/hooks/ path, and an absolute override breaks
# the moment the repo is moved or renamed.
hooks_path=$(git config --get core.hooksPath 2>/dev/null || true)
case "$hooks_path" in
  /*)
    echo "info: clearing absolute core.hooksPath ($hooks_path)" >&2
    git config --unset core.hooksPath || true
    ;;
esac

if command -v lefthook >/dev/null 2>&1; then
  lefthook install
else
  cat <<'HINT' >&2

WARN: lefthook not on PATH — Git hooks NOT installed.
      Without them every commit skips fmt/clippy/file-size/commitlint
      gates locally (CI still catches them, but slow feedback).
      Install one of:
        brew install lefthook && lefthook install
        go install github.com/evilmartians/lefthook@latest && lefthook install
        npm install -g lefthook && lefthook install

HINT
fi

case ":$PATH:" in
  *":$HOME/.cargo/bin:"*) ;;
  *)
    cat <<'PATHHINT' >&2

WARN: ~/.cargo/bin is not on PATH — shells won't find `cvg` / `convergio`.
      Add this to your shell config (bash/zsh):
        export PATH="$HOME/.cargo/bin:$PATH"

PATHHINT
    ;;
esac

cat <<'MSG'

Installed:
  convergio  local daemon
  cvg        local CLI
  convergio-mcp  MCP bridge for agents

Start:
  cvg setup
  convergio start

In another terminal:
  cvg doctor
  cvg health
  cvg demo
MSG
