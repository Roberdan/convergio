#!/usr/bin/env bash
# Mirror a curated subset of repo docs into the GitHub wiki working copy.
# SOURCE_DIR (default $PWD), WIKI_DIR (required). Idempotent; caller
# commits + pushes (see .github/workflows/wiki-sync.yml).
set -euo pipefail
SOURCE_DIR="${SOURCE_DIR:-$PWD}"
WIKI_DIR="${WIKI_DIR:?WIKI_DIR is required}"
[[ -d "$SOURCE_DIR" ]] || { echo "sync-wiki: SOURCE_DIR '$SOURCE_DIR' missing" >&2; exit 1; }
[[ -d "$WIKI_DIR" ]]   || { echo "sync-wiki: WIKI_DIR '$WIKI_DIR' missing"   >&2; exit 1; }

# wiki page name | source path
PAGES=(
  "Home.md|README.md"
  "Architecture.md|ARCHITECTURE.md"
  "Constitution.md|CONSTITUTION.md"
  "Roadmap.md|ROADMAP.md"
  "Vision.md|docs/vision.md"
  "Multi-Agent.md|docs/multi-agent-operating-model.md"
  "Setup.md|docs/setup.md"
  "Release.md|docs/release.md"
  "Agent-Protocol.md|docs/agent-protocol.md"
  "ADRs.md|docs/adr/README.md"
)

# Home.md: drop the leading shields/badges block, then rewrite links
# in the mirrored set to [[Wiki-Page]] form.
write_home() {
  awk 'BEGIN{b=0;d=0} !d&&/^\[!\[/{b=1;next} b&&/^[[:space:]]*$/{b=0;d=1;next} b{next} {print}' "$1" \
    | python3 -c '
import re,sys
m={"README.md":"Home","ARCHITECTURE.md":"Architecture","CONSTITUTION.md":"Constitution",
   "ROADMAP.md":"Roadmap","docs/vision.md":"Vision","docs/multi-agent-operating-model.md":"Multi-Agent",
   "docs/setup.md":"Setup","docs/release.md":"Release","docs/agent-protocol.md":"Agent-Protocol",
   "docs/adr/README.md":"ADRs"}
def r(x):
    p=m.get(x.group(2).lstrip("./"))
    return f"[[{p}]]" if p else x.group(0)
sys.stdout.write(re.sub(r"\[([^\]]+)\]\(\.?/?([^)]+)\)", r, sys.stdin.read()))' \
    > "$2"
}

# ADRs.md: ./NNNN-foo.md → absolute main blob URL.
write_adrs() {
  python3 -c '
import re,sys
b="https://github.com/Roberdan/convergio/blob/main/docs/adr/"
t=open(sys.argv[1],encoding="utf-8").read()
t=re.sub(r"\[([^\]]+)\]\(\.\/([0-9]{4}-[^)]+\.md)\)", lambda m:f"[{m.group(1)}]({b}{m.group(2)})", t)
open(sys.argv[2],"w",encoding="utf-8").write(t)' "$1" "$2"
}

for entry in "${PAGES[@]}"; do
  page="${entry%%|*}"
  rel="${entry##*|}"
  src="$SOURCE_DIR/$rel"
  dst="$WIKI_DIR/$page"
  if [[ ! -f "$src" ]]; then
    echo "sync-wiki: missing '$rel', skipping $page" >&2
    continue
  fi
  case "$page" in
    Home.md) write_home "$src" "$dst" ;;
    ADRs.md) write_adrs "$src" "$dst" ;;
    *)       cp "$src" "$dst" ;;
  esac
done

echo "sync-wiki: mirrored ${#PAGES[@]} pages into $WIKI_DIR"
