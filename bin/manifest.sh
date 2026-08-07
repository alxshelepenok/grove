#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

usage() {
  echo "usage: manifest.sh --version X.Y.Z --sums SHA256SUMS [--artifacts-dir DIR] [--channel stable] [--ttl-days 180] [--repo alxshelepenok/grove] [--previous manifest.json] [--output manifest.json] [--now EPOCH]" >&2
  exit 2
}

version=""
sums=""
dir="."
channel="stable"
ttl_days=180
repo="alxshelepenok/grove"
previous="manifest.json"
output="manifest.json"
now=""

while [ $# -gt 0 ]; do
  case $1 in
    --version) version=$2; shift 2 ;;
    --sums) sums=$2; shift 2 ;;
    --artifacts-dir) dir=$2; shift 2 ;;
    --channel) channel=$2; shift 2 ;;
    --ttl-days) ttl_days=$2; shift 2 ;;
    --repo) repo=$2; shift 2 ;;
    --previous) previous=$2; shift 2 ;;
    --output) output=$2; shift 2 ;;
    --now) now=$2; shift 2 ;;
    --refresh) refresh=1; shift ;;
    *) usage ;;
  esac
done

fmt_ts() { date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -r "$1" +%Y-%m-%dT%H:%M:%SZ; }
file_size() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1"; }

if [ "${refresh:-0}" -eq 1 ]; then
  [ -f "$previous" ] || die "--refresh requires an existing --previous manifest"
  prev_seq=$(sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$previous")
  [ -n "$prev_seq" ] || die "cannot read sequence from $previous"
  [ -n "$now" ] || now=$(date -u +%s)
  created=$(fmt_ts "$now")
  expires=$(fmt_ts $((now + ttl_days * 86400)))
  sed -e "s/^  \"created_at\": \".*\",$/  \"created_at\": \"$created\",/" \
      -e "s/^  \"expires_at\": \".*\",$/  \"expires_at\": \"$expires\",/" \
      -e "s/^      \"sequence\": $prev_seq,$/      \"sequence\": $((prev_seq + 1)),/" \
      "$previous" > "$output"
  echo "refreshed manifest: sequence $((prev_seq + 1)), expires $expires"
  exit 0
fi

[ -n "$version" ] || die "--version is required"
[ -n "$sums" ] || die "--sums is required"
echo "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || die "version must be X.Y.Z, got: $version"
echo "$channel" | grep -qE '^[a-z0-9_-]+$' || die "invalid channel name: $channel"
[ -f "$sums" ] || die "SHA256SUMS not found: $sums"

[ -n "$now" ] || now=$(date -u +%s)
created=$(fmt_ts "$now")
expires=$(fmt_ts $((now + ttl_days * 86400)))

sequence=1
if [ -f "$previous" ]; then
  prev=$(sed -n 's/^      "sequence": \([0-9][0-9]*\),$/\1/p' "$previous")
  [ -n "$prev" ] && sequence=$((prev + 1))
fi

rows=$(mktemp)
out_tmp=$(mktemp)
trap 'rm -f "$rows" "$out_tmp"' EXIT

: > "$rows"
while read -r hash filename; do
  filename=${filename#\*}
  case $filename in
    grove-desktop-v"$version"-*.tar.gz)
      comp=grove_desktop
      rest=${filename#grove-desktop-v"$version"-}
      ;;
    grove-mcp-v"$version"-*.tar.gz)
      comp=grove_mcp
      rest=${filename#grove-mcp-v"$version"-}
      ;;
    grove-v"$version"-*.tar.gz)
      comp=grove
      rest=${filename#grove-v"$version"-}
      ;;
    *) continue ;;
  esac
  target=${rest%%.*}
  key=$(printf '%s_%s' "$comp" "$target" | tr '-' '_')
  [ -f "$dir/$filename" ] || die "artifact listed in $sums not found: $dir/$filename"
  size=$(file_size "$dir/$filename")
  url="https://github.com/$repo/releases/download/v$version/$filename"
  printf '%s\t%s\t%s\t%s\n' "$key" "$url" "$hash" "$size" >> "$rows"
done < "$sums"

[ -s "$rows" ] || die "no release artifacts matched version $version in $sums"

{
  printf '{\n'
  printf '  "version": "%s",\n' "$version"
  printf '  "created_at": "%s",\n' "$created"
  printf '  "expires_at": "%s",\n' "$expires"
  printf '  "channels": {\n'
  printf '    "%s": {\n' "$channel"
  printf '      "sequence": %d,\n' "$sequence"
  printf '      "artifacts": {\n'
  first=1
  while IFS=$'\t' read -r key url hash size; do
    [ "$first" -eq 1 ] || printf ',\n'
    first=0
    printf '        "%s": {\n' "$key"
    printf '          "url": "%s",\n' "$url"
    printf '          "sha256": "%s",\n' "$hash"
    printf '          "size": %s\n' "$size"
    printf '        }'
  done < <(sort "$rows")
  printf '\n      }\n'
  printf '    }\n'
  printf '  }\n'
  printf '}\n'
} > "$out_tmp"

mkdir -p "$(dirname "$output")"
mv "$out_tmp" "$output"
