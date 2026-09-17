#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

bin/skill-bundle.sh --output "$work/bundle-a.tar.gz" > /dev/null
bin/skill-bundle.sh --output "$work/bundle-b.tar.gz" > /dev/null

cmp -s "$work/bundle-a.tar.gz" "$work/bundle-b.tar.gz"
report $? "bundle is byte-stable for unchanged sources"

tar -tzf "$work/bundle-a.tar.gz" > "$work/listing.txt"
for entry in "grove/" "grove/SKILL.md" "grove/references/rules.md" "grove/diagrams/workflow.md"; do
  grep -qx "$entry" "$work/listing.txt"
  report $? "archive contains $entry"
done

head -2 docs/skills/SKILL.md | grep -q '^name: grove$'
report $? "skill frontmatter stays at the top of SKILL.md"

find docs/skills -name "*.md" -print0 | while IFS= read -r -d '' f; do
  dir=$(dirname "$f")
  grep -o "]([^)]*)" "$f" | sed 's/^](//; s/)$//' | while IFS= read -r t; do
    case "$t" in http*|\#*|mailto:*) continue ;; esac
    path="${t%%#*}"
    [ -z "$path" ] && continue
    [ -e "$dir/$path" ] || echo "BROKEN: $f -> $t"
  done
done > "$work/broken.txt"
if [ -s "$work/broken.txt" ]; then
  head -5 "$work/broken.txt"
  r=1
else
  r=0
fi
report $r "every relative link resolves inside the skill directory"

bin/skill-bundle.sh --output "$work/bundle-v.tar.gz" --version 9.9.9 > /dev/null
rm -rf "$work/unpack"
mkdir -p "$work/unpack"
tar -xzf "$work/bundle-v.tar.gz" -C "$work/unpack"
sed -n '2p' "$work/unpack/grove/SKILL.md" | grep -qx 'version: 9.9.9'
report $? "--version stamps the frontmatter second line"

if grep -q '^version:' docs/skills/SKILL.md; then r=1; else r=0; fi
report $r "source SKILL.md carries no version line"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
