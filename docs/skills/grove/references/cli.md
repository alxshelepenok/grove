# 4. CLI reference

Invocation: `grove <command> [args...]`.

The CLI reads and writes `.grove/state.lock` and `.grove/index.md` under a project root. Root resolution, first match wins: `--root=<path>` (explicit path, no name lookup; the root must contain or will contain `.grove/`), then `--project=<dir|name>`, then the `GROVE_PROJECT` environment variable (both take an existing directory or a registry name, unknown names exit 5), otherwise a walk-up from the current working directory to the first ancestor containing `.grove/state.lock` (fallback: the cwd itself).

## 3.1 Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. |
| 1 | Generic error (bad args, file missing). |
| 2 | Lock checksum mismatch. Use `grove repair --confirm`. |
| 3 | Invariant violation (`grove check`). |
| 4 | Guard failure (DoR, WIP, evidence missing, etc.). |
| 5 | Not found (unknown ID). |

## 3.2 Read commands

**`grove status`:** session-aware overview. Lists: stale-token `progress` W's
(see [protocol §2.6](protocol.md#26-session-tokens-and-interrupted-work)),
open alignment triggers (§2.5), invariant warnings short of full check.
Run this first in every session.

**`grove ready`:** list work items ready to start. Sorted: critical-path members first, then by descending downstream-blocks count. Output is one line per W: `W-NN  <title>  [crit]`.

**`grove next`:** single proposed W from `Ready ∩ critical_path`. Falls back to any `Ready` member if the intersection is empty. Prints the same packet as `grove packet <ID>` (see below).

**`grove packet <W-NN>`:** execution packet. Self-contained markdown bundle:

- The W record (header + all fields, prose rendered as markdown).
- Every `D-NN` linked by `implements`.
- Every `B-NN` in `BChain(W)`.
- The `outcome` of every `Q-NN` linked by `asks`.
- A DoR breakdown (same as `grove dor`).

`--cone` appends multi-hop structural context on `blocks` (output-only; no lockfile
change). Horizon: `--cone-depth=N` (BFS hops, default 4), `--cone-max=N` (node cap,
default 50). Sections:

- `## Contraction order`: backward cone in topological order: `1. W-NN  status  title`.
- `## Forward cone (impact)`: blast radius, same columns, unordered.
- `## Fragility`: per goal, vertex-disjoint `blocks`-paths G→W: `G-NN: k disjoint blocks-paths`, `G-NN: 1 (brittle)`, or `G-NN: no blocks-path`.
- `## Relevant discoveries`: only when non-empty.
- `> cone truncated (depth=N, max=M)`: final line when the horizon cut the cone.

JSON adds a `cone` object: `backward`, `order`, `forward` (id arrays), `fragility`
(`[{ goal, paths }]`), `relevant_discoveries`, `truncated`, `depth`, `max`.

This is the only context the agent needs to implement the W.

**`grove deps <ID>`:** transitive predecessors on `blocks`. One ID per line, in topological order.

**`grove impact <ID>`:** transitive successors on `blocks` (what does this unblock?).

**`grove path`:** critical path: longest chain of unfinished W on `blocks`, head to tail.

**`grove dor <W-NN>`:** DoR conjunct breakdown:

```text
W-12 DoR:
  ⊤  goals(w) ≠ ∅                      → G-01
  ⊤  AC(w) ≠ ∅                          → 2 entries
  ⊤  ∀ q ∈ asks(w), q terminal          → Q-03 (answered)
  ⊥  BChain validated                   → B-01 testing
  ⊤  fitness deltas set                 → G-01=+1
  ⊤  evidence_strategy ≠ ∅
  ⊤  hypothesis ≠ ⊥
  ⊤  repro(w) ≠ ∅                    → (non-bug)
  ⊤  exit(w) ≠ ∅                     → (non-spike)
  ⊤  (T, causes, w) via materialised T → (non-refactor)
  ⊤  cynefin ≠ chaotic                  → clear
result: ⊥
```

**`grove show <ID>`:** pretty-print one record.

**`grove list <kind> [--status=…] [--cynefin=…]`:** kinds: `w`, `d`, `q`, `b`, `g`, `t`, `y`, `a`. Tabular output.

**`grove graph`:** print the mermaid block to stdout.

**`grove diff [--since=<git-ref>]`:** structured diff of `state.lock`
between the current working copy and `<git-ref>` (default `HEAD`). Output
groups changes by record kind and shows added / removed / changed nodes and
edges, ignoring pure reordering. Designed for PR review.

**`grove projects`:** table of the project registry, one tab-separated row per
project: `name`, `path`, `last_opened`. The registry lives at
`~/.grove/projects.toml` (the user-level store is `~/.grove/`, overridable via
the `GROVE_HOME` environment variable). It is a human-editable convenience
index, not state, and is not checksummed: every CLI invocation whose resolved
root holds `.grove/state.lock` (and every `init`) upserts its row. `name`
defaults to the directory basename, suffixed `-2`, `-3`, ... when another path
already owns it; `created` is preserved across upserts, `last_opened` is
refreshed. A missing registry reads as empty; a malformed one prints a stderr
warning and registry features degrade gracefully, never crash a command.

**`grove log [<ID>] [--limit=N]`:** newest-first merged timeline from node/edge
`t_created`/`t_updated` attrs and `.grove/journal.log` (one tab-separated row per
source; journal rows use middle field `journal`). An `<ID>` filter also matches
inverse payloads in journal records (so IDs only referenced there still work).
`--limit=0` disables the cap.

**`grove gate [--theta=N] [--n=N]`:** report-only distillation gate (design D5/D6,
phase 0). Computes the structural differential since the last gate record:
treewidth of the active graph (min-fill upper bound) and its delta, W→done
transitions since baseline (`due: true` when count ≥ `--n`, default 5), then the
mechanical surprise signals as `would distill` candidates:

- `- overflow W-NN: <paths>`: actual git surface (files touched by commits
  since the baseline whose subject names the exact id `W-NN`; one batched
  `git log --name-only` scan per gate run) minus the W's declared `surface`
  field; shown when the overflow exceeds `--theta` (default 0). Empty when git
  is unavailable or no commit mentions the W.
- `- invalidated B-NN: <title>`: B that moved to `invalidated_*` since baseline.
- `- accepted D-NN: <title>`: D accepted since baseline.

**Report-only:** `gate` never writes `state.lock` and never creates Discovery nodes
(`grove add y` / `grove distill` perform the actual distillation). Its one
persistent write is a gate record appended to `.grove/journal.log`
(`{"v":1,"ts":…,"cmd":"gate","inv":{"op":"gate","tw":N,"dones":N,"empty":bool}}`);
an empty diff leaves a null record (`"empty": true`) for audit. Gate records are
non-mutations: `grove undo` skips them (never inverts, never truncates) and
`grove log` shows them plainly.

**`grove triage`:** read-only discovery recommender (design D12). Ranks every
non-terminal W by where discovery effort is most needed, from mechanical
signals only. One tab-separated row per W, columns `W  cov  χ  fragile
suggestion`: `cov` is the share of the W's declared `surface` covered by
*active* Discovery surfaces (`0.00` when none declared), `χ` counts open Q among
`asks(w)` + BChain entries not yet `validated` / `invalidated_acceptable` +
failed DoR conjuncts, `fragile` is `yes` when any goal has ≤ 1 vertex-disjoint
`blocks`-path into the W. Sorted by coverage ascending, then χ descending,
then id. The suggestion is the first matching rule: declare surface → spike to
create coverage → resolve open Q/B and DoR gaps → add a redundant path
(blocks) → deepen coverage → ready to deliver. Empty project prints
`triage: no open work`. **Advisory only (D12):** triage never feeds
`grove next`, never gates an invariant, and persists nothing: no lock write,
no journal record.

**`grove check`:** run all invariants I₁..I₁₃ plus orphan-edge, edge-type, and Discovery decay checks; the lock checksum is verified on load. Exit code 0 / 2 / 3 as listed in §3.1.

**`grove stats`:** read-only telemetry computed from `.grove/journal.log` plus the
current lock; never writes the lock, the journal, or the index. Sections: cycle
time per cynefin class (ready to done, from status intervals reconstructed
backward through journal inverse records), DoR (rejection events total and per
node, progress entries, first-pass rate, and a first-pass split: progress
entries with no prior reject vs rejects followed by q/b/y churn
(`reject_discovery`) vs plain rejects, plus the discovery share of post-reject
entries), bets (entries into `validated` /
`invalidated_acceptable` / `invalidated_blocking`, ratio), discovery (stale
entries, revalidations, gate runs, empty gate runs, and gate overflow /
invalidated events), undo
(events, undone steps, undos per 100 mutations), audit (commands per journal
`session` token - count, per-session commands, mean/median/max; records
predating the key bucket as `unknown`. Checkpoint latency in hours, two series:
`dor_reject` to the next progress entry of that W, and Discovery proposed to
active. Post-approval invalidation: ever-`validated` B that later held an
invalidated status, count / denominator / rate), rework (per W: DoR reject
counts, split into covered vs uncovered by intersection of the W's declared
`surface` with currently active Discovery surfaces; undo events stay global
because undo drops journaled lines), distill yield (per archived goal: `real`
when a Discovery `distills` into the goal's exclusive archive pool, `null`
when only a null-distill attestation exists, else `none`), surprise
(invalidated B + gate overflows per done W), a surprise series (per done W,
chronological: surprise events since the previous done W, and the replayed C
value at that ts), a `gates` array with one row per gate record
(oldest first: treewidth, dones, empty flag, overflow events, overflow path
total, invalidated events; path total is null on legacy records without
counts), and a C/V series (V as in the Content health dashboard, including
uncovered surface) replayed backward over mutation
records (with a replay-failure count). Missing journal reads as empty;
unparseable or inconsistent records are tolerated and counted, never fatal.
Journal record kinds that feed it: `dor_reject` (appended when the DoR guard
refuses `status=progress`, with the failed conjunct labels in `missing`),
`undo` (appended after a successful undo, with `steps`), gate records (with
`overflows` / `invalidated` id lists and per-W `overflow_counts`), and
`archive` (goal id plus archived id list); all four are non-mutations, skipped
by `grove undo`. Every new record carries a top-level `session` key with the
writing command's effective session token.

**`--json` on read commands:** every read command in §3.2 prints **one JSON object** on stdout (UTF-8) instead of the human-oriented text. Keys always include **`command`** (string, same as the subcommand). Most mutate commands ignore `--json`; `add y`, `distill`, `revalidate`, and `glossary rename` emit a small JSON payload. Exit codes are unchanged; for **`check`**, failures still return **3** with **`"ok": false`** and an **`errors`** array in the JSON body (no duplicate error lines on stderr). Schema details: §3.4.1.

## 3.3 Mutate commands

**All mutate commands** re-serialize the lock with a fresh checksum and call `render` implicitly.

**`grove init`:** creates `.grove/state.lock`, `.grove/index.md`, `.grove/glossary.md`. Idempotent: refuses if the lock already exists.

Optional allocation tuning (persisted once in the optional `# @grove-id stride=…` lock comment; see [§6.1 Lockfile envelope](lockfile.md#61-file-envelope)):

- `--id-stride=<N>` (default `1`): additive gap between successive numeric suffixes (`N≥1`).
- `--id-offset=<K>` (default `1`): first suffix when a family allocator is empty (`K≥1`).
- `--id-width=<W>` (default `2`, or bumped to ≥`3` when stride/offset are non-default without an explicit `--id-width`): minimum digit padding for new IDs (`W≥2`).

**`grove renumber <ID> --to=<NEW-ID>`:** rewrites one record ID and every structured reference (`edges`, structural list fields keyed by IDs, `:fitness` map keys against goals, `:goal`/`:work-items` payloads, `:theme`, etc.). Refuses when the token appears verbatim in prose on **any done** `w` (`evidence` field), signalling that downstream consumers may have anchored on the exported string; resolve manually ([merge protocol](rules.md#merge--rebase-protocol)).

**`grove add <kind> [...]`:** kind ∈ `g w d q b t y a`.

| Kind | Required flags | Optional |
| --- | --- | --- |
| `g` | `--title="…"`, `--area=A-NN`, fitness spec (`--fitness-kind=count\|ratio\|boolean\|metric\|manual`) | `--fitness-target=…` (required for `count` / `metric` / `ratio`), `--status=unverified` |
| `w` | `--title="…"`, `--type=feature\|refactor\|bug\|spike`, `--cynefin=…` | `--goals=G-01,G-02`, `--theme=T-01`, `--surface=p1,p2` (declared estimate, feeds coverage), `--status=proposed` |
| `d` | `--title="…"` | `--supersedes=D-01`, `--status=proposed` |
| `q` | `--title="…"`, `--cynefin=…` | `--targets=W-01`, `--status=open` |
| `b` | `--title="…"`, `--cynefin=…` | `--tests=Q-01`, `--targets=W-01`, `--status=proposed` |
| `t` | `--title="…"` | `--status=open` |
| `y` | `--title="…"`, `--tags=<t1,t2>` (≥1 glossary term), `--from=<W-NN\|D-NN\|Q-NN\|B-NN>` (≥1 provenance record), `--surface=<p1,p2>` xor `--why="…"` | – |
| `a` | `--title="…"` | `--surface=p1,p2` |

`y` starts `proposed`; `--from=W-NN` wires a `produces` edge, `--from=D/Q/B` wires `distills` edges. The CLI prints the assigned ID. CSV list options (`--tags`, `--surface`, `--goals`) refuse duplicate entries (compared after trimming surrounding whitespace) at capture.

Every goal belongs to exactly one area (I₁₃): `add g` refuses a missing or unknown `--area`. An area-less goal in the lock is an I13 violation, fixed via `grove set G-NN area=A-NN`; create real areas with `add a`. `add g` also refuses a missing fitness specification: pass `--fitness-kind` (with `--fitness-target` for `count` / `metric` / `ratio`) or `--fitness-kind=manual` for a deliberate n/a. The legacy `--fitness="…"` label is retired: writes are rejected.

**`grove set <ID> <key>=<value>`:** keys: `status`, `cynefin`, `type`, `title`, `fitness_kind` (G only), `fitness_target` (G only: set or refresh the structured threshold; empty value clears it), `area` (G only: owning area `A-NN`, I₁₃), `requires_coverage` (G/T only: `true` for θ=0.5 or a float in `(0,1]`; opts linked complex features into the DoR coverage conjunct, see [model §1.7](model.md#17-definition-of-ready)). The retired legacy key `fitness` is rejected on writes; `set G-NN fitness=` (empty value) removes a leftover legacy label from an old lock. Status transitions are guarded:

- `W status=progress`: I₁ DoR ≡ ⊤, I₄ WIP, I₅ predecessors `terminal⁺`, I₁₁ no other session holds the token. Records the session token.
- `W status=done`: I₃ evidence non-empty, I₅ predecessors `terminal⁺`, I₁₀ atomic: fitness deltas for every linked goal must be staged via `grove fitness` since the last status mutation; otherwise rejected. On success, applies deltas, re-derives `status(g)` and `status(t)`, runs `grove render`. If a linked goal **newly** reaches `verified`, the CLI prints a **lazy distill** hint to stderr (`grove distill G-NN`, or `grove distill G-NN --null`); goals with a `notes` line containing `--distill-deferred` suppress the hint (see [rules](rules.md) § lazy distillation).
- `D status=accepted`: locks the record from further field edits (rule "Decision immutability"); use `supersedes` to revise.
- `Y status=active`: anchor-gated (I₁₂); `active → stale` is free; `stale → active` only via `grove revalidate`; any non-terminal → `superseded`.
- `B status=invalidated_blocking`: warns about every dependent W; does not auto-reject.
- `T status=…`: rejected (derived per I₆).

**`grove field <ID> <field> add "…"`:** append one prose line (or one list element) to a field. On reflist fields (`tags`, `surface`, `goals`) a value already present is refused (exit 4, no mutation).
**`grove field <ID> <field> rm <index>`:** remove the Nth (1-based) entry.
**`grove field <ID> <field> clear`:** empty the field.

**`grove link <from> <label> <to>`:** adds an edge. Labels: `blocks`, `implements`, `asks`, `tests`, `targets`, `produces`, `causes`, `supersedes`, `distills`. Validates domain/codomain per [Formal model](model.md) §1.3 and DAG-ness for `blocks` (I₇).

**`grove unlink <from> <label> <to>`:** removes the edge.

**`grove evidence <W-NN> "…"`:** appends a line to the W's `evidence` field. Sugar for `grove field W-NN evidence add "…"`.

**`grove fitness <W-NN> <G-NN> <±delta>`:** stages a delta on **`W`** toward **`G`** (I₁₀ at `done`). If **`G`** carries structured **`fitness_kind`**, `index` / lock fields **`fitness_current`** and **`status(G)`** refresh when **`W`** completes (see [lockfile §6.5.1](lockfile.md#651-structured-fitness-goals)). Multiple calls overwrite the staged delta for the same (W, G) pair. Use `+0` for enabling work.

**`grove archive <G-NN>`:** moves the goal and every `w` / `d` / `q` / `b` / `t` whose **goal-reference set equals `{G-NN}`** (`goals` fields + propagation along `implements`, `produces`, `asks`, `tests`, `targets`, `causes`, `theme`, bidirectional `supersedes`) and that is **affinity-connected** to `G-NN` (`goals` backlinks + undirected structural edges among those nodes only). Shared resources (one `d` tied to work under two goals via `implements`, etc.) **stay active**. `:y` records remain outside `:archive` (Discoveries are never archived). Refuses when `status(G) ≠ verified`, when distillation has not happened (the gate requires **either** ≥1 Discovery provenance-linked into the goal's exclusive mass (`produces` from a mass W, or `distills` into a mass D/Q/B) **or** a null-distill attestation (`grove distill G-NN --null`)), or when session guards fail on `progress` work listing `G-NN`. A successful archive appends an audit-only journal record (`cmd: "archive"`, `inv: {"op":"archive","id":"G-NN","ids":[...]}` with the archived id list): a non-mutation, skipped by `grove undo` exactly like gate records and not undoable.

**`grove distill <G-NN> [--null]`:** distillation at `verified` (refuses otherwise and for non-goals). Default prints the worksheet: the goal's distillation candidates (validated B, answered Q, accepted D from the exclusive sweep, or the goal's full reference set when the sweep is just the goal) each with a suggested `grove add y --from=<ID> …` skeleton, plus whether the archive precondition is already met. `--null` writes a null-distill attestation to `.grove/journal.log` (`cmd: "distill"`, `inv: {"op":"distill","goal":"G-NN","empty":true}`), a non-mutation record: `grove undo` skips it, exactly like gate records.

**`grove revalidate <Y-NN> [--surface=p1,p2] [--from=ID,...]`:** `stale` Discovery → `active`, paid with a fresh anchor: `--surface` paths must exist under root, and/or `--from` a W or D/Q/B (not superseded/invalidated) to wire a new provenance edge. Appends one line to the Discovery's `revalidation` field.

**`grove promote <Y-NN> --to=<project>`:** copy a discovery into another project with provenance; locks never intersect, so this copy is the only way Discoveries move between projects. `--to` resolves like `--project` (an existing directory or a registry name; the target must already hold a lock, run `grove init` there first). The copy takes the next free `Y` id in the target, starts its own lifecycle at `proposed`, and carries `title`, `tags`, `surface`, `invariant`, `why`, `skill_updates`, `glossary_updates` (not `revalidation`; no edges). Provenance attrs on the copy: `origin_project` (source registry name, else the source root basename), `origin_id` (source id), `origin_version` (source `t_updated`). Copied tags missing from the target glossary are appended as `| <term> | copied from <origin_project> |` rows. Promoting the same (`origin_project`, `origin_id`) into a target twice is refused with exit 4 (`already promoted as Y-NN`). The source lock is only read; the target write is journaled as `cmd: "promote"` with an `rm_node` inverse, so target-side `grove undo` removes the copy. `--json` emits `{command, id, origin_project, origin_id}`.

**`grove glossary rename <old> <new>`:** rewrites the term in `.grove/glossary.md` and every Discovery's `tags` atomically (undo restores both). Refuses when `old` is neither in the glossary nor used by any Discovery, or when `new` already exists.

**`grove render`:** regenerate `.grove/index.md`. Called automatically by every mutate command; explicit invocation is for after a `repair`. The dashboard opens with **Content health** (global C/V: C = validated B + answered Q + accepted D + active Discovery; V = open Q + pending B + W below DoR + uncovered surface, counting every non-terminal W whose declared `surface` is not fully covered by active Discovery surfaces; a `Decay` row counts Discoveries showing at least one decay signal and appears only when that count is positive) followed by an **Areas** section: one row per `a` node with its C and V, computed by the soft attribution tier ([model §1.10](model.md)). A relevance view, not a partition: a W or Discovery touching two areas counts in both, a W without goals counts in none, and an empty area renders as a dormant row with zero counts. The global totals stay primary.

**`grove repair --confirm`:** re-parse the lock under relaxed checksum, re-canonicalise, write fresh checksum. Use after a deliberate manual edit OR after any git operation that combined two histories of `state.lock` (merge, rebase, cherry-pick); see [rules.md merge protocol](rules.md#merge--rebase-protocol).

**`grove resume <W-NN>`** / **`grove handoff <W-NN> --to=<token>`** / **`grove revert <W-NN>`:** session-token operations on a `progress` W (journal undo restores prior claim tokens). See [protocol §2.6](protocol.md#26-session-tokens-and-interrupted-work).

**`grove undo [--steps=N]`:** reverts the last N journaled mutate operations applied in inverse order by replaying stored inverse ops onto the lock state, then **truncates** those N mutation lines off `.grove/journal.log` (default `N=1`). Non-mutation records (gate records, null-distill attestations) are skipped: never inverted, never counted, never truncated. A successful undo then appends one non-mutation `undo` record (with `steps`) to the journal, counted by `grove stats` and skipped by later undos; there is no built-in redo. Other mutators (`init`, `repair`) do not write journal lines.

## 3.4 Global flags

- `--root=<path>`: base directory containing `.grove/`. Wins over every other root resolution mechanism.
- `--project=<dir|name>`: target a project by directory path or by name in the registry (see `grove projects`). When neither `--root` nor `--project` is given, `GROVE_PROJECT` supplies the same value; when that is also absent, grove walks up from the cwd to the first ancestor containing `.grove/state.lock` and uses it as root (fallback: the cwd).
- `--quiet`: suppress info; only errors.
- `--json`: machine-readable output for read commands (§3.4.1).
- `--no-render`: skip auto-render after a mutate (debugging only).
- `--session=<token>`: override the session token (default: `GROVE_SESSION` if set, else `host:hex16(sha256(norm_root))` from env `COMPUTERNAME`/`HOSTNAME`/`HOST`).
- `--id-stride=<N>` / `--id-offset=<N>`: only valid on `grove init`; sets
  the worktree's ID allocator to step `N` starting from `offset` to avoid
  collisions on parallel branches (see merge protocol).

### 3.4.1 `--json` command shapes

Each response is a single JSON object. Types: **string**, **bool**, **array**, **object** (string keys).

| Subcommand | Extra keys (besides `command`) |
| --- | --- |
| `ready` | `items`: array of `{ id, title, critical }`. |
| `next` | `work`, `packet_markdown`. |
| `packet` | `work`, `packet_markdown`; with `--cone` also `cone`: `{ backward, order, forward, fragility: [{ goal, paths }], relevant_discoveries, truncated, depth, max }`. |
| `deps` | `id`, `predecessors` (strings, topological order). |
| `impact` | `id`, `successors`. |
| `path` | `chain` (W ids on critical path). |
| `dor` | `work`, `conjuncts`: `[{ label, ok, detail }]`, `dor` (bool, overall ⊤/⊥). |
| `show` | `record`: `{ kind, id, title, status, archived?, type?, cynefin?, attrs: { … }, fields: { … } }` (present `fields` keys follow the lockfile catalog; prose/reflists are JSON arrays of strings). |
| `list` | `kind`, `rows`: `[{ id, status, title, cynefin? }]`, optional `filter_*`. |
| `graph` | `mermaid` (full mermaid block text). |
| `log` | `limit`, `rows`: `[{ ts, sort, line }]`, optional `id_filter`. |
| `gate` | `baseline` (`{ ts, tw, dones }` or null), `tw_now`, `tw_delta`, `dones`, `due`, `overflows`: `[{ w, paths }]`, `invalidated`: `[{ id, title, status }]`, `accepted`: `[{ id, title }]`, `empty`, `theta`, `n`. |
| `triage` | `rows`: `[{ w, title, coverage, declared, uncertainty, fragile, suggestion }]`, sorted as above. |
| `distill` | `goal`, `precondition_met`, `linked_das`, `null_attested`, `candidates`: `[{ id, kind, title, skeleton }]`; with `--null`: `goal`, `null`, `empty`. |
| `check` | `ok`, `errors` (strings; empty when `ok`). |
| `stats` | `records`, `mutations`, `cycle_time` (`by_cynefin` per-class `{ n, mean_hours, median_hours, max_hours }`, `durations_seconds`), `dor` (`reject_events`, `reject_per_node`, `progress_entries`, `first_pass`, `first_pass_rate`, `first_pass_split`: `{ no_reject, reject_discovery, reject_plain, discovery_rate }`), `bets` (`validated`, `invalidated_acceptable`, `invalidated_blocking`, `ratio`), `discovery` (`stale_entries`, `revalidations`, `gate_runs`, `gate_empty`, `gate_overflow_events`, `gate_invalidated_events`), `gates` (`[{ ts, tw, dones, empty, overflow_events, overflow_paths, invalidated_events }]`, oldest first; `overflow_paths` null on legacy records without `overflow_counts`), `undo` (`undo_events`, `undone_steps`, `undos_per_100_mutations`), `audit` (`sessions`: `{ count, per_session: [{ session, commands }], mean, median, max }`, `checkpoint_latency`: `{ dor, discovery }` each `{ n, mean_hours, median_hours, max_hours }`, `post_approval_invalidation`: `{ invalidated, ever_validated, rate }`), `rework` (`covered` / `uncovered`: `{ w, rejects, mean_rejects, per_w: [{ id, rejects }] }`), `distill_yield` (`goals_with_real`, `goals_null_attested`, `goals_without`, `goals`: `[{ goal, status, discoveries }]`), `surprise` (`total`, `done_w`, `per_done`), `surprise_series` (`[{ id, ts, delta, c }]`, chronological), `cv_series` (`[{ ts, c, v }]`, oldest first), `replay_failures` (null for undefined rates). |
| `status` | `progress`: session rows; `alignment_triggers`; `invariants`: `{ ok, messages }`. |
| `diff` | `since` (git ref), `semantic_change`, `nodes` (per-kind `added` / `removed` / `changed`), `edges`: `{ added, removed }` with `{ from, label, to }`; same semantic rules as textual diff (`lock_structural_lines`). |
| `projects` | `projects`: `[{ name, path, created, last_opened }]` from the registry. |

## 3.5 Examples

```bash
grove init
grove add a --title="Auth"
grove add g --title="Migrate auth" --area=A-01 --fitness-kind=count --fitness-target=5
grove add w --type=feature --cynefin=clear --goals=G-01 \
          --title="Add login flow"
grove add q --cynefin=complicated --targets=W-01 \
          --title="Which hash algo?"
grove link Q-01 asks W-01
grove field Q-01 outcome add "bcrypt; see D-01"
grove set Q-01 status=answered

grove dor W-01
grove next
grove packet W-01

grove fitness W-01 G-01 +1
grove evidence W-01 "tests/login_test.jl green; commit abc123"
grove set W-01 status=done

grove check
```

Note the order: stage `fitness` before `evidence` before `status=done`. The
`done` transition is the single atomic point that applies everything.

