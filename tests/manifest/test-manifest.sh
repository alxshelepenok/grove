#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."

golden=tests/manifest/golden/manifest.json
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

sha256_of() { sha256sum "$1" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }

mkdir -p "$work/artifacts"
(
  cd "$work/artifacts"
  printf 'fixture grove linux x64\n' > grove-v0.2.0-linux-x64.tar.gz
  printf 'fixture grove macos arm64\n' > grove-v0.2.0-macos-arm64.tar.gz
  printf 'fixture grove macos x64\n' > grove-v0.2.0-macos-x64.tar.gz
  printf 'fixture grove windows x64\n' > grove-v0.2.0-windows-x64.tar.gz
  printf 'fixture mcp linux x64\n' > grove-mcp-v0.2.0-linux-x64.tar.gz
  printf 'fixture mcp macos arm64\n' > grove-mcp-v0.2.0-macos-arm64.tar.gz
  printf 'fixture mcp macos x64\n' > grove-mcp-v0.2.0-macos-x64.tar.gz
  printf 'fixture mcp windows x64\n' > grove-mcp-v0.2.0-windows-x64.tar.gz
  printf 'fixture desktop linux x64\n' > grove-desktop-v0.2.0-linux-x64.tar.gz
  printf 'fixture desktop macos arm64\n' > grove-desktop-v0.2.0-macos-arm64.tar.gz
  printf 'fixture desktop macos x64\n' > grove-desktop-v0.2.0-macos-x64.tar.gz
  printf 'fixture desktop windows x64\n' > grove-desktop-v0.2.0-windows-x64.tar.gz
  for f in grove-v0.2.0-* grove-mcp-v0.2.0-* grove-desktop-v0.2.0-*; do
    printf '%s  %s\n' "$(sha256_of "$f")" "$f"
  done > "$work/SHA256SUMS"
  printf '%s  %s\n' "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" "sbom.cdx.json" >> "$work/SHA256SUMS"
)

bin/manifest.sh --version 0.2.0 --sums "$work/SHA256SUMS" --artifacts-dir "$work/artifacts" --now 1754000000 --previous /nonexistent --output "$work/manifest.json"
r=$?
report $r "manifest.sh runs on fixture input"

if [ $r -eq 0 ]; then
  diff -u "$golden" "$work/manifest.json" > "$work/diff.out" 2>&1
  r=$?
  [ $r -ne 0 ] && cat "$work/diff.out"
  report $r "output is byte-identical to golden file"
fi

field_version=$(sed -n 's/^  "version": "\([^"]*\)",$/\1/p' "$work/manifest.json")
[ "$field_version" = "0.2.0" ]
report $? "sed extraction: version"

field_sequence=$(sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$work/manifest.json")
[ "$field_sequence" = "1" ]
report $? "sed extraction: sequence starts at 1 without previous manifest"

block=$(sed -n '/^        "grove_mcp_macos_arm64": {$/,/^        }/p' "$work/manifest.json")
field_url=$(printf '%s\n' "$block" | sed -n 's/^          "url": "\([^"]*\)",$/\1/p')
[ "$field_url" = "https://github.com/alxshelepenok/grove/releases/download/v0.2.0/grove-mcp-v0.2.0-macos-arm64.tar.gz" ]
report $? "sed extraction: artifact url"

field_sha=$(printf '%s\n' "$block" | sed -n 's/^          "sha256": "\([0-9a-f]*\)",$/\1/p')
expected_sha=$(sha256_of "$work/artifacts/grove-mcp-v0.2.0-macos-arm64.tar.gz")
[ "$field_sha" = "$expected_sha" ]
report $? "sed extraction: artifact sha256"

field_size=$(printf '%s\n' "$block" | sed -n 's/^          "size": \([0-9]*\)$/\1/p')
[ "$field_size" = "24" ]
report $? "sed extraction: artifact size"

bin/manifest.sh --version 0.2.0 --sums "$work/SHA256SUMS" --artifacts-dir "$work/artifacts" --now 1754000000 --previous "$work/manifest.json" --output "$work/manifest2.json"
bumped=$(sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$work/manifest2.json")
[ "$bumped" = "2" ]
report $? "sequence increments from previous manifest"

if bin/manifest.sh --version 9.9.9 --sums "$work/SHA256SUMS" --artifacts-dir "$work/artifacts" --now 1754000000 --output "$work/none.json" 2>/dev/null; then r=1; else r=0; fi
report $r "fails closed when no artifacts match the version"

if bin/manifest.sh --version '0.2.0$(touch /tmp/pwned)' --sums "$work/SHA256SUMS" --artifacts-dir "$work/artifacts" --output "$work/inj.json" 2>/dev/null; then r=1; else r=0; fi
[ ! -e /tmp/pwned ] && [ $r -eq 0 ]
report $? "injection-shaped version rejected without evaluation"

expires=$(sed -n 's/^  "expires_at": "\([^"]*\)",$/\1/p' "$work/manifest.json")
[ "$expires" = "2026-01-27T22:13:20Z" ]
report $? "expires_at is created_at + 180 days"

bin/manifest.sh --refresh --previous "$work/manifest.json" --now 1760000000 --output "$work/refreshed.json"
ref_seq=$(sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$work/refreshed.json")
ref_created=$(sed -n 's/^  "created_at": "\([^"]*\)",$/\1/p' "$work/refreshed.json")
ref_expires=$(sed -n 's/^  "expires_at": "\([^"]*\)",$/\1/p' "$work/refreshed.json")
[ "$ref_seq" = "2" ] && [ "$ref_created" = "2025-10-09T08:53:20Z" ] && [ "$ref_expires" = "2026-04-07T08:53:20Z" ]
report $? "refresh bumps sequence and rewrites timestamps"

diff <(sed -e '/"created_at"/d' -e '/"expires_at"/d' -e '/"sequence"/d' "$work/manifest.json") \
     <(sed -e '/"created_at"/d' -e '/"expires_at"/d' -e '/"sequence"/d' "$work/refreshed.json") >/dev/null
report $? "refresh keeps the artifacts block byte-identical"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
