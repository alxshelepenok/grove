#!/usr/bin/env bash
set -u
yml=${1:-.github/workflows/release.yml}

pass=0
fail=0
report() {
  if [ "$1" -eq 0 ]; then pass=$((pass+1)); echo "PASS: $2"; else fail=$((fail+1)); echo "FAIL: $2"; fi
}

cmd=$(sed -n '/gh release create/,/install\.ps1 install\.ps1\.sig/p' "$yml")
assets=$(printf '%s\n' "$cmd" | grep -E '^[[:space:]]+(out/|docs/|manifest\.json|install\.)' \
          | sed 's/^[[:space:]]*//' | tr -d '\\' | tr ' ' '\n' | grep -v '^$')
globs=$(printf '%s\n' "$assets" | grep '\*')
explicits=$(printf '%s\n' "$assets" | grep -v '\*')

for a in manifest.json.sig out/SHA256SUMS.sig install.ps1.sig; do
  printf '%s\n' "$explicits" | grep -qxF "$a"
  report $? "parser anchor present: $a"
done

missing=""
while IFS= read -r e; do
  case "$e" in
    *.sig) ;;
    *)
      printf '%s\n' "$explicits" | grep -qxF "$e.sig" || missing="$missing $e.sig"
      ;;
  esac
done < <(printf '%s\n' "$explicits")
[ -z "$missing" ]
report $? "every explicit asset carries its signature twin (missing:$missing)"

bad=""
while IFS= read -r e; do
  while IFS= read -r g; do
    case "$e" in $g) bad="$bad $e" ;; esac
  done < <(printf '%s\n' "$globs")
done < <(printf '%s\n' "$explicits")
[ -z "$bad" ]
report $? "no explicit asset is covered by a glob (would upload twice:$bad)"

printf '%s\n' "$assets" | grep -qxF 'out/grove-skill.tar.gz.sig'
report $? "the glob-attached skill archive has its signature attached"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
