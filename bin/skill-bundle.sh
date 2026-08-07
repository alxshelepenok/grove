#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "error: $*" >&2; exit 1; }

output="grove-skill.md"
src="docs/skills"
while [ $# -gt 0 ]; do
  case $1 in
    --output) output=$2; shift 2 ;;
    --src) src=$2; shift 2 ;;
    *) echo "usage: skill-bundle.sh [--output grove-skill.md] [--src docs/skills]" >&2; exit 2 ;;
  esac
done

sections="model.md protocol.md cli.md evidence.md rules.md lockfile.md typography.md checklist.md"
diagrams="dual-track.md graph-template.md workflow.md"

[ -f "$src/index.md" ] || die "index not found: $src/index.md"
for f in $sections; do
  [ -f "$src/$f" ] || die "section not found: $src/$f"
done
for f in $diagrams; do
  [ -f "$src/diagrams/$f" ] || die "diagram not found: $src/diagrams/$f"
done

slugify() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | sed -e 's/[^a-z0-9 _-]//g' -e 's/ /-/g'
}

heading_slug() {
  h=$(sed -n 's/^# \(.*\)$/\1/p' "$1" | head -1)
  [ -n "$h" ] || die "no H1 heading in $1"
  slugify "$h"
}

work=$(mktemp)
trap 'rm -f "$work"' EXIT

awk '/^---$/{n++; print; if (n == 2) exit; next} {print}' "$src/index.md" > "$work"
awk '/^---$/{n++; next} n >= 2 {print}' "$src/index.md" >> "$work"

for f in $sections; do
  printf '\n\n---\n\n' >> "$work"
  cat "$src/$f" >> "$work"
done

printf '\n\n---\n\n# 9. Diagrams\n' >> "$work"
for f in $diagrams; do
  printf '\n\n' >> "$work"
  sed 's/^\(#\{1,5\}\) /\1# /' "$src/diagrams/$f" >> "$work"
done

for f in index.md $sections; do
  anchor=$(heading_slug "$src/$f")
  sed -i -e "s|](${f})|](#${anchor})|g" -e "s|](${f}#|](#|g" "$work"
done
for f in $diagrams; do
  anchor=$(heading_slug "$src/diagrams/$f")
  sed -i -e "s|](diagrams/${f})|](#${anchor})|g" -e "s|](diagrams/${f}#|](#|g" "$work"
done
sed -i "s|](diagrams/)|](#9-diagrams)|g" "$work"

mv "$work" "$output"
echo "bundle written to $output"
