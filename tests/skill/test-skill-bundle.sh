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

bin/skill-bundle.sh --output "$work/bundle-a.md" > /dev/null
bin/skill-bundle.sh --output "$work/bundle-b.md" > /dev/null

cmp -s "$work/bundle-a.md" "$work/bundle-b.md"
report $? "bundle is byte-stable for unchanged sources"

head -2 "$work/bundle-a.md" | grep -q '^name: grove$'
report $? "skill frontmatter stays at the top"

prev_line=0
order_ok=1
for heading in "# 1. Formal model" "# 2. Workflow protocol" "# 3. CLI reference" "# 4. Evidence (Definition of Done)" "# 5. Rules" "# 6. Lockfile specification" "# 7. Typography" "# 8. Quality checklist" "# 9. Diagrams"; do
  count=$(grep -cF "$heading" "$work/bundle-a.md")
  line=$(grep -nF "$heading" "$work/bundle-a.md" | head -1 | cut -d: -f1)
  if [ "$count" -ne 1 ] || [ -z "$line" ] || [ "$line" -le "$prev_line" ]; then
    order_ok=0
    echo "  heading problem: '$heading' count=$count line=$line prev=$prev_line"
  fi
  prev_line=$line
done
report $((1 - order_ok)) "every numbered section appears exactly once in the recorded order"

if grep -qE '\]\([^)#][^)]*\.md(#[^)]*)?\)' "$work/bundle-a.md"; then
  grep -nE '\]\([^)#][^)]*\.md(#[^)]*)?\)' "$work/bundle-a.md" | head -5
  r=1
else
  r=0
fi
report $r "no relative .md links remain"

grep -qF '](#1-formal-model)' "$work/bundle-a.md"
report $? "file links rewritten to section anchors"

grep -qF '](#dual-track-loops)' "$work/bundle-a.md"
report $? "diagram links rewritten to appendix anchors"

if grep -q '^# Dual-track loops' "$work/bundle-a.md"; then r=1; else r=0; fi
report $r "diagram headings demoted under the appendix"

grep -q '^## Dual-track loops' "$work/bundle-a.md"
report $? "appendix contains the diagram content"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
