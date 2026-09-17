#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "error: $*" >&2; exit 1; }

output="grove-skill.tar.gz"
src="docs/skills/grove"
ver=""
while [ $# -gt 0 ]; do
  case $1 in
    --output) output=$2; shift 2 ;;
    --src) src=$2; shift 2 ;;
    --version) ver=$2; shift 2 ;;
    *) echo "usage: skill-bundle.sh [--output grove-skill.tar.gz] [--src docs/skills/grove] [--version X.Y.Z]" >&2; exit 2 ;;
  esac
done

[ -f "$src/SKILL.md" ] || die "skill root not found: $src/SKILL.md"
[ -d "$src/references" ] || die "references not found: $src/references"
[ -d "$src/diagrams" ] || die "diagrams not found: $src/diagrams"
tar --help 2>/dev/null | grep -q -- --sort || die "GNU tar is required (for --sort)"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -R "$src" "$stage/grove"

if [ -n "$ver" ]; then
  head -1 "$stage/grove/SKILL.md" | grep -q '^---$' || die "SKILL.md frontmatter not found"
  awk -v v="$ver" 'NR == 1 { print; print "version: " v; next } { print }' \
    "$stage/grove/SKILL.md" > "$stage/grove/SKILL.md.new"
  mv "$stage/grove/SKILL.md.new" "$stage/grove/SKILL.md"
fi

tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
    -C "$stage" -cf - grove | gzip -n > "$output"
echo "bundle written to $output"
