#!/usr/bin/env bash

set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "error: $*" >&2; exit 1; }

usage() {
  echo "usage: md-format.sh [--check] [file.md ...]" >&2
  exit 2
}

pretty_flags=(--prose-wrap always --print-width 80)
exclude_re='^(packages/desktop/ui/js/vendor/PROVENANCE\.md|\.grove/)'

check=0
args=()
while [ $# -gt 0 ]; do
  case $1 in
    --check) check=1 ;;
    -h|--help) usage ;;
    -*) usage ;;
    *) args+=("$1") ;;
  esac
  shift
done

files=()
if [ ${#args[@]} -gt 0 ]; then
  while IFS= read -r f; do
    [ -f "$f" ] || die "file not found: $f"
    files+=("$f")
  done < <(printf '%s\n' "${args[@]}" | grep -Ev "$exclude_re")
else
  while IFS= read -r f; do files+=("$f"); done \
    < <(git ls-files -- '*.md' | grep -Ev "$exclude_re")
fi
[ ${#files[@]} -gt 0 ] || die "no markdown files to format"

command -v prettier >/dev/null 2>&1 || \
  die "prettier not found on PATH: install it globally (bun add --global prettier or npm install --global prettier)"

if [ "$check" -eq 1 ]; then
  if out=$(prettier --check "${pretty_flags[@]}" -- "${files[@]}" 2>&1); then
    echo "markdown formatting: ok"
  else
    printf '%s\n' "$out" >&2
    die "markdown formatting drift: run bin/md-format.sh to reflow"
  fi
else
  prettier --write "${pretty_flags[@]}" -- "${files[@]}"
fi
