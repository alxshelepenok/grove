#!/usr/bin/env bash
set -u
REPO=/y/Projects/grove
BIN="$REPO/target/release/grove.exe"
WORK=$(mktemp -d)
PROJ="$WORK/proj"
mkdir -p "$PROJ/.grove"
cp "$REPO/.grove/state.lock" "$REPO/.grove/journal.log" "$REPO/.grove/glossary.md" "$PROJ/.grove/"
JHOME="$WORK/jhome"
RHOME="$WORK/rhome"
mkdir -p "$JHOME" "$RHOME"
unset GROVE_PROJECT GROVE_SESSION

run_j() { local R=$1 C=$2; shift 2; (cd "$REPO" && GROVE_HOME="$JHOME" julia --project=packages/grove bin/grove.jl "$C" --root="$R" "$@"); }
run_r() { local R=$1 C=$2; shift 2; GROVE_HOME="$RHOME" "$BIN" "$C" --root="$R" "$@"; }

echo "warming up julia..." >&2
(cd "$REPO" && GROVE_HOME="$JHOME" julia --project=packages/grove -e 'using grove; println("warm ok")')

run_j "$PROJ" status >"$WORK/pre.j.out" 2>"$WORK/pre.j.err"; pj=$?
run_r "$PROJ" status >"$WORK/pre.r.out" 2>"$WORK/pre.r.err"; pr=$?
if [ "$pj" -ne 0 ] || [ "$pr" -ne 0 ]; then
  echo "PREFLIGHT FAIL: julia rc=$pj rust rc=$pr (refusing vacuous differential)"
  echo "--- julia stdout ---"; cat "$WORK/pre.j.out"
  echo "--- julia stderr ---"; cat "$WORK/pre.j.err"
  echo "--- rust stdout ---"; cat "$WORK/pre.r.out"
  echo "--- rust stderr ---"; cat "$WORK/pre.r.err"
  exit 1
fi
echo "preflight ok (both status rc=0)"

cmds=(
"status"
"check"
"list"
"show W-09"
"show G-03"
"show --json W-09"
"packet W-09"
"packet W-09 --cone"
"dor W-09"
"triage"
"stats"
"graph"
"log"
"next"
"deps W-09"
"impact W-08"
"path"
"projects"
"diff"
)

echo "WORK=$WORK"
printf '%-20s | %-9s | %-9s | %-10s | %s\n' "command" "stdout" "stderr" "exit" "note"
echo "---------------------|-----------|-----------|------------|------"
i=0
for c in "${cmds[@]}"; do
  i=$((i+1))
  read -ra argv <<< "$c"
  run_j "$PROJ" "${argv[@]}" >"$WORK/j.out" 2>"$WORK/j.err"; jrc=$?
  run_r "$PROJ" "${argv[@]}" >"$WORK/r.out" 2>"$WORK/r.err"; rrc=$?
  so=identical; cmp -s "$WORK/j.out" "$WORK/r.out" || so=DIFFERS
  se=identical; cmp -s "$WORK/j.err" "$WORK/r.err" || se=DIFFERS
  ex="ok($jrc)"; [ "$jrc" = "$rrc" ] || ex="J=$jrc,R=$rrc"
  note=""
  if [ "$so" = DIFFERS ]; then
    cp "$WORK/j.out" "$WORK/d$i.j.out"; cp "$WORK/r.out" "$WORK/d$i.r.out"; note="saved d$i.*"
  fi
  if [ "$se" = DIFFERS ]; then
    cp "$WORK/j.err" "$WORK/d$i.j.err"; cp "$WORK/r.err" "$WORK/d$i.r.err"; note="$note saved-err d$i.*"
  fi
  printf '%-20s | %-9s | %-9s | %-10s | %s\n' "$c" "$so" "$se" "$ex" "$note"
done

echo
echo "=== git-backed diff variant (fresh temp repo, worktree mutated after commit) ==="
GITP="$WORK/gitproj"
mkdir -p "$GITP/.grove"
cp "$REPO/.grove/state.lock" "$REPO/.grove/journal.log" "$REPO/.grove/glossary.md" "$GITP/.grove/"
(
  cd "$GITP"
  git init -q
  git add .grove/state.lock .grove/journal.log .grove/glossary.md
  git -c user.email=diff@probe -c user.name=diffprobe commit -qm init
)
run_r "$GITP" add q --title="Diff probe question" >/dev/null 2>&1
run_j "$GITP" diff --since=HEAD >"$WORK/gj.out" 2>"$WORK/gj.err"; gjrc=$?
run_r "$GITP" diff --since=HEAD >"$WORK/gr.out" 2>"$WORK/gr.err"; grrc=$?
gso=identical; cmp -s "$WORK/gj.out" "$WORK/gr.out" || gso=DIFFERS
gse=identical; cmp -s "$WORK/gj.err" "$WORK/gr.err" || gse=DIFFERS
printf '%-20s | %-9s | %-9s | %-10s | %s\n' "diff --since=HEAD" "$gso" "$gse" "J=$gjrc,R=$grrc" "git repo variant"
echo "done. WORK=$WORK"
