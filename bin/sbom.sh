#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "error: $*" >&2; exit 1; }

usage() {
  echo "usage: sbom.sh [--output sbom.cdx.json] [--cyclonedx-bin cargo-cyclonedx] [--trivy-bin trivy]" >&2
  exit 2
}

output="sbom.cdx.json"
cyclonedx="cargo-cyclonedx"
trivy="trivy"

while [ $# -gt 0 ]; do
  case $1 in
    --output) output=$2; shift 2 ;;
    --cyclonedx-bin) cyclonedx=$2; shift 2 ;;
    --trivy-bin) trivy=$2; shift 2 ;;
    *) usage ;;
  esac
done

command -v "$cyclonedx" >/dev/null || die "cargo-cyclonedx not found: $cyclonedx"
command -v "$trivy" >/dev/null || die "trivy not found: $trivy"

work=$(mktemp -d)
trap 'rm -rf "$work"; rm -f packages/core/grove-core.cdx.json packages/mcp/grove-mcp.cdx.json packages/desktop/src-tauri/grove-desktop.cdx.json' EXIT

"$cyclonedx" cyclonedx --manifest-path packages/core/Cargo.toml --format json --spec-version 1.5 --all --quiet
mv packages/core/grove-core.cdx.json "$work/cargo-core.json"

"$cyclonedx" cyclonedx --manifest-path packages/mcp/Cargo.toml --format json --spec-version 1.5 --all --quiet
mv packages/mcp/grove-mcp.cdx.json "$work/cargo-mcp.json"

"$cyclonedx" cyclonedx --manifest-path packages/desktop/src-tauri/Cargo.toml --format json --spec-version 1.5 --all --quiet
mv packages/desktop/src-tauri/grove-desktop.cdx.json "$work/cargo-desktop.json"

"$trivy" fs --format cyclonedx --scanners vuln --list-all-pkgs --quiet --output "$work/julia.json" packages/grove

prov=packages/desktop/ui/js/vendor/PROVENANCE.md
prov_rows=$(sed -n 's/^| `\([a-z0-9.-]*\)` | \[\([a-z0-9-]*\)\]([^)]*) ([^)]*) | \([0-9][0-9.]*\) | `[^`]*`[^|]* | `\([0-9a-f]\{64\}\)` |$/\1 \2 \3 \4/p' "$prov")
[ -n "$prov_rows" ] || die "no vendored entries parsed from $prov - provenance table format drifted"
{
  printf '{"components": [\n'
  first=1
  printf '%s\n' "$prov_rows" | \
  while read -r file name version sha; do
    [ "$first" -eq 1 ] || printf ',\n'
    first=0
    printf '{"type": "library", "name": "%s", "version": "%s", "purl": "pkg:npm/%s@%s", "bom-ref": "pkg:npm/%s@%s", "hashes": [{"alg": "SHA-256", "content": "%s"}], "properties": [{"name": "grove:vendored_file", "value": "js/vendor/%s"}]}' \
      "$name" "$version" "$name" "$version" "$name" "$version" "$sha" "$file"
  done
  printf '\n]}\n'
} > "$work/vendored-js.json"

julia --project=packages/grove bin/merge-cdx.jl "$output" "$work/cargo-core.json" "$work/cargo-mcp.json" "$work/cargo-desktop.json" "$work/julia.json" "$work/vendored-js.json"

echo "SBOM written to $output"
