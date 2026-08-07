#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")/.."

die() { echo "error: $*" >&2; exit 1; }

usage() {
  echo "usage: audit.sh [--fail-closed] [--trivy-bin trivy] [--vex docs/security/artifacts/vex.json] [--targets \"Cargo.lock packages/grove/Manifest.toml\"]" >&2
  exit 2
}

fail_closed=0
trivy="trivy"
vex="docs/security/artifacts/vex.json"
targets="Cargo.lock packages/grove/Manifest.toml"

while [ $# -gt 0 ]; do
  case $1 in
    --fail-closed) fail_closed=1; shift ;;
    --trivy-bin) trivy=$2; shift 2 ;;
    --vex) vex=$2; shift 2 ;;
    --targets) targets=$2; shift 2 ;;
    *) usage ;;
  esac
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

findings=0
op_error=0
i=0
for target in $targets; do
  i=$((i+1))
  out="$work/report-$i.json"
  err="$work/trivy-$i.err"
  "$trivy" fs --scanners vuln --format json --output "$out" "$target" 2>"$err"
  rc=$?
  if [ $rc -ne 0 ]; then
    echo "warning: trivy failed for $target (rc=$rc)" >&2
    sed 's/^/  /' "$err" >&2
    op_error=1
    continue
  fi
  julia --project=packages/grove bin/audit-filter.jl "$out" "$vex" || findings=1
done

if [ "$op_error" -eq 1 ] && [ "$fail_closed" -eq 1 ]; then
  die "scanner operational failure in --fail-closed mode"
fi
if [ "$findings" -eq 1 ]; then
  die "audit failed"
fi
echo "audit ok"
