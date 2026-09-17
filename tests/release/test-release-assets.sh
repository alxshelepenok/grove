#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

cmd=$(sed -n '/gh release create/,/install\.ps1 install\.ps1\.sig/p' .github/workflows/release.yml)
occurs=$(printf '%s\n' "$cmd" | grep -oE 'grove-skill\.tar\.gz(\.sig)?' || true)

printf '%s\n' "$cmd" | grep -q 'out/\*\.tar\.gz'
report $? "the out/*.tar.gz glob attaches the tar.gz archives (skill included)"

plain=$(printf '%s\n' "$occurs" | grep -cx 'grove-skill\.tar\.gz' || true)
[ "$plain" -eq 0 ]
report $? "grove-skill.tar.gz is never an explicit asset (the glob covers it)"

sigs=$(printf '%s\n' "$occurs" | grep -cx 'grove-skill\.tar\.gz\.sig' || true)
[ "$sigs" -eq 1 ]
report $? "grove-skill.tar.gz.sig is listed exactly once"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
