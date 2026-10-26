# Grove

| Document property   | Value                    |
| ------------------- | ------------------------ |
| Status              | Normative specification. |
| Lockfile generation | `@grove 1`.              |
| Journal generation  | `1`.                     |

## Abstract

Grove is a graph state protocol. It defines a typed reasoning graph, node
lifecycles, readiness and completion invariants, a checksummed text
serialization, an undo journal, and equivalent CLI and MCP command interfaces.

## 1. General provisions

### 1.1 Requirement notation

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** are to be
interpreted as described in RFC 2119 and RFC 8174 when, and only when, they
appear in all capitals.

### 1.2 Scope and conformance

The protocol covers:

- Invariants I₁ through I₁₄.
- Lockfile and journal formats.
- The graph model and its derived relations.
- Readiness, completion, fitness, and status transitions.
- CLI and MCP behavior, including errors and JSON envelopes.
- Archive, distillation, revalidation, promotion, and undo.

An implementation MUST preserve the behavior defined by this document. A
producer MUST emit canonical lockfiles as defined in section 6. An
implementation MUST reject an unknown lockfile generation rather than infer its
semantics. The conformance corpus provides executable evidence; absence of a
scenario does not make a normative requirement optional.

## 2. Formal model

### 2.1 State tuple

The development state is the tuple:

```text
Σ ≜ (G, W, D, Q, B, T, Y, A, E)
N ≜ G ∪ W ∪ D ∪ Q ∪ B ∪ T ∪ Y ∪ A
E ⊆ N × LabelE × N
```

| Set         | Symbol | Meaning                                            | ID prefix |
| ----------- | ------ | -------------------------------------------------- | --------- |
| Goals       | `G`    | Outcome or requirement with a fitness function.    | `G-NN`    |
| Work items  | `W`    | Executable unit with DoR and DoD.                  | `W-NN`    |
| Decisions   | `D`    | Long-lived design choice.                          | `D-NN`    |
| Questions   | `Q`    | Open unknown.                                      | `Q-NN`    |
| Assumptions | `B`    | Falsifiable assumption with validation and result. | `B-NN`    |
| Themes      | `T`    | Optional grouping of related work items.           | `T-NN`    |
| Discoveries | `Y`    | Curated invariant distilled from process records.  | `Y-NN`    |
| Areas       | `A`    | Permanent scope owning goals.                      | `A-NN`    |

All nodes and edges are stored in `.grove/state.lock`. There are no per-node
record files.

### 2.2 Work types and Cynefin values

```text
type(w) ∈ { feature, refactor, bug, spike }
cynefin(n) ∈ { clear, complicated, complex, chaotic }, n ∈ W ∪ Q ∪ B
```

The work types have these meanings:

| Type       | Meaning                                                              |
| ---------- | -------------------------------------------------------------------- |
| `feature`  | New capability with a hypothesis and resolved discovery assumptions. |
| `refactor` | Behavior-preserving structural change with a root-cause `T`.         |
| `bug`      | Defect in shipped behavior with reproducible evidence.               |
| `spike`    | Investigation producing `D`, `Q`, or `B`, not production code.       |

### 2.3 Edge algebra

```text
LabelE = { blocks, causes, implements, asks, tests, targets, produces, supersedes, distills }
```

| Label        | Domain and codomain              | Meaning                                                     |
| ------------ | -------------------------------- | ----------------------------------------------------------- |
| `blocks`     | `N → W`                          | Predecessor clearance gates successor start and completion. |
| `causes`     | `T → W` for refactor or bug work | Root cause to symptom.                                      |
| `implements` | `W → D`                          | Work realizes a decision.                                   |
| `asks`       | `Q → N`                          | Question applies to a target.                               |
| `tests`      | `B → Q`                          | Assumption operationalizes a question.                      |
| `targets`    | `B → W`                          | Work depends on an assumption.                              |
| `produces`   | `W → D ∪ Q ∪ B ∪ Y`              | Work produced a record.                                     |
| `supersedes` | `D → D` or `Y → Y`               | A successor replaces a record.                              |
| `distills`   | `Y → D ∪ Q ∪ B`                  | Discovery distills a process record.                        |

The projection of `E` onto `blocks` MUST be acyclic. Cycles under other labels
are permitted. Every edge endpoint MUST exist and satisfy the declared types.
Identical edges are idempotent.

Pure relationships are represented only by `e` records. A node field containing
an identifier exists only when it carries information beyond an edge:

- `goals` on `W` declares fitness and readiness targets.
- `theme` on `W` declares single-theme membership used by I₆.
- `fitness` on `W` stores signed deltas rather than a plain relationship.

The following derived sets are used by the protocol:

```text
assumptions(w) ≜ { b ∈ B | (b, targets, w) ∈ E }

BChain(w) ≜ assumptions(w) ∪ { b ∈ B | ∃q ∈ Q: (q, asks, w) ∈ E ∧ (b, tests, q) ∈ E }

produces(w) ≜ { n ∈ D ∪ Q ∪ B ∪ Y | (w, produces, n) ∈ E }

provenance(y) ⟺ (∃w ∈ W: (w, produces, y) ∈ E) ∨ (∃n ∈ D ∪ Q ∪ B: (y, distills, n) ∈ E)

goals(w) ≜ identifiers recorded in the goals field of w

WI(t) ≜ { w ∈ W | theme(w) = t }
```

### 2.4 Areas

Every `g ∈ G` has exactly one `area(g) = a.id`, where `a ∈ A`. An area-less or
dangling goal reference violates I₁₃. Areas have structural `present` status, no
lifecycle, and no archive representation.

`W` and `T` do not declare areas. Their areas are projections:

```text
areas(w) ≜ ⋃ { area(g) | g ∈ goals(w) }
areas(t) ≜ ⋃ { areas(w) | w ∈ WI(t) }
```

`Y` has no declared area. An Area with no current goals remains a valid dormant
scope.

The mechanical anchor sets used for area relevance are:

```text
surface(a) ≜ declared surface(a) ∪ ⋃ { declared surface(w) | w belongs to a }

tags(a) ≜ ⋃ tags over goals in a and their Work items

nodes(a) ≜ non-archived goals in a ∪ their Work items ∪ { q, b, d linked by any edge to those Work items }
```

Surface intersection is the gate-grade area attribution relation. Tag and cone
intersections are relevance relations, not gate-grade coverage relations.

## 3. Status algebra

### 3.1 Status sets

```text
status(g) ∈ { unverified, partial, verified, declined }
status(w) ∈ { proposed, ready, progress, done, rejected, archived }
status(d) ∈ { proposed, accepted, rejected, superseded }
status(q) ∈ { open, deferred, answered, dropped }
status(b) ∈ { proposed, testing, validated, invalidated_acceptable, invalidated_blocking }
status(t) ∈ { open, done }
status(y) ∈ { proposed, active, stale, superseded }
status(a) = present
```

`status(t)` is derived by I₆ and cannot be assigned through `set`. `status(a)`
is structural and cannot be assigned through `set`.

### 3.2 Terminal predicates

```text
terminal(w ∈ W) ⟺ status(w) ∈ { done, rejected, archived }
terminal(g ∈ G) ⟺ status(g) ∈ { verified, declined }
terminal(d ∈ D) ⟺ status(d) ∈ { accepted, rejected, superseded }
terminal(q ∈ Q) ⟺ status(q) ∈ { answered, deferred, dropped }
terminal(b ∈ B) ⟺ status(b) ∈ { invalidated_acceptable, invalidated_blocking }
terminal(t ∈ T) ⟺ status(t) = done
terminal(y ∈ Y) ⟺ status(y) = superseded
terminal(a ∈ A) ⟺ ⊥
```

`validated` is not terminal for `B`. `stale` is not terminal for `Y`; it remains
readable but contributes nothing to active coverage or relevance.

The strict predicate for `blocks` predecessors is:

```text
terminal⁺(g ∈ G) ⟺ status(g) = verified
terminal⁺(n)     ⟺ terminal(n), n ∉ G
```

A `declined` goal therefore does not clear a `blocks` successor.

## 4. Invariants

| ID  | Formal statement                                                                                        | Enforcement                                                                                                                                        |
| --- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| I₁  | `∀ w ∈ W with status = progress, DoR(w) ≡ ⊤`                                                            | Checked on entry to `progress` and by `check`; coverage is pinned at entry.                                                                        |
| I₂  | `∀ w with type = spike ∧ status = done, produces(w) ⊆ D ∪ Q ∪ B ∧ produces(w) ≠ ∅`                      | Checked on entry to `done` and by `check`.                                                                                                         |
| I₃  | `∀ w ∈ W: status(w) = done ⇒ ∃ev ∈ Evidence(w): satisfies(ev, AC(w))`                                   | Presence checked on entry to `done` and by `check`; satisfaction is semantic.                                                                      |
| I₄  | `card({w ∈ W ∣ status(w) = progress}) ≤ WIP_LIMIT`, where `WIP_LIMIT = 2`                               | Checked on entry to `progress` and by `check`.                                                                                                     |
| I₅  | `∀ (n₁, blocks, n₂) ∈ E: terminal⁺(n₁)` before `n₂` enters `progress` or `done`                         | Checked on both transitions and by `check`.                                                                                                        |
| I₆  | `∀ t ∈ T: status(t) = done ⟺ ∀ w ∈ WI(t): status(w) ∈ {done, rejected, archived}`                       | Derived after relevant mutations; direct `set` is rejected.                                                                                        |
| I₇  | `DAG(N, E ∩ (N × {blocks} × N))`                                                                        | Checked by `link` and `check`.                                                                                                                     |
| I₈  | `∀ q ∈ Q: cynefin(q) = chaotic ⇒ human_only(status_transitions(q))`                                     | Status changes require a human decision.                                                                                                           |
| I₉  | `∀ w ∈ W: type(w) = feature ∧ DoR(w) ⇒ ∀b ∈ BChain(w): status(b) ∈ {validated, invalidated_acceptable}` | Checked for `ready`, `progress`, and by `check`.                                                                                                   |
| I₁₀ | `atomic(w → done, apply_deltas(w), rederive(goals(w)))`                                                 | `done` requires every delta staged in the same call or after the last status mutation; failure rolls back the transition.                          |
| I₁₁ | `∀ w ∈ W: status(w) = progress ⇒ exclusive_owner(w, session(w))`                                        | Session guards apply while `W` is `progress`; `session` and `session_at` are mandatory.                                                            |
| I₁₂ | `∀ y ∈ Y: status(y) = active ⇒ provenance(y) ∧ (surface(y) ≠ ∅ ∨ why(y) ≠ ∅) ∧ tags(y) ≠ ∅`             | Checked for `proposed → active` and by `check`; `stale → active` requires `revalidate` with a fresh anchor.                                        |
| I₁₃ | `∀ g ∈ G: ∃a ∈ A: area(g) = a.id`                                                                       | Enforced at creation and by `check`; missing or dangling areas are not silently repaired.                                                          |
| I₁₄ | `∀ n ∈ N: terminal(n) ⇒ ¬user_mutable(n)`                                                               | `set`, `field`, `fitness`, and `evidence` are refused; `undo`, `repair`, `renumber`, derivation, and accepted-`D` supersession are separate lanes. |

## 5. Derived functions and transitions

### 5.1 Definition of Ready

For `w ∈ W`, `DoR(w)` is:

```text
DoR(w) ≜
  (goals(w) ≠ ∅) ∧
  (AC(w) ≠ ∅) ∧
  (∀ q ∈ asks(w), status(q) ∈ { answered, deferred, dropped }) ∧
  (type(w) = feature ⇒ ∀ b ∈ BChain(w), status(b) ∈ { validated, invalidated_acceptable }) ∧
  (∀ g ∈ goals(w), contributes_to_fitness(w, g) ≠ ⊥) ∧
  (evidence_strategy(w) ≠ ∅) ∧
  (type(w) = feature ⇒ hypothesis(w) ≠ ⊥) ∧
  (type(w) = bug ⇒ repro(w) has a non-empty prose line) ∧
  (type(w) = spike ⇒ exit(w) has a non-empty prose line) ∧
  (type(w) = refactor ⇒ ∃ t ∈ T, ¬archived(t) ∧ (t, causes, w) ∈ E) ∧
  (cynefin(w) ≠ chaotic) ∧
  (requires_coverage(w) ∧ type(w) = feature ∧ cynefin(w) = complex ⇒ coverage(w) ≥ θ)
```

`grove dor W-NN` evaluates the conjunction by conjunct. Entering `ready` applies
I₉ to features. Entering `progress` applies the full conjunction plus I₄, I₅,
and I₁₁.

Coverage is required only when all conditions hold:

```text
type(w) = feature
cynefin(w) = complex
∃c ∈ goals(w) ∪ { theme(w) }: requires_coverage(c) is present
```

For a carrier `c`, `requires_coverage=true` denotes θ 0.5; a numeric value in
`(0,1]` denotes that threshold. Multiple carriers resolve to:

```text
θ(w) ≜ max thresholds declared by linked goals and the theme
```

Let `S_active` be the union of `surface` values on active discoveries:

```text
coverage(w) ≜ |surface(w) ∩ S_active| / |surface(w)|
```

Proposed, stale, superseded, and surface-less discoveries contribute nothing. An
empty `surface(w)` fails an active coverage requirement. Other work shapes pass
this conjunct vacuously. Later discovery staleness affects new `progress`
transitions but does not invalidate coverage already pinned for an in-flight
`W`.

### 5.2 Fitness model

`fitness(w)` maps each linked `G` to a signed integer delta. A later staging
operation for the same `(w,g)` pair replaces the previous delta. Zero is a valid
delta. I₁₀ requires a key for every `g ∈ goals(w)` at completion.

The aggregate for goal `g` is:

```text
total(g) ≜ Σ fitness(w)[g] over non-archived w where status(w) = done and g ∈ goals(w)
```

Structured goal kinds and targets are:

| `fitness_kind` | `fitness_target`        | Verification condition                                       |
| -------------- | ----------------------- | ------------------------------------------------------------ |
| `count`        | Non-negative integer N. | `total(g) ≥ N`.                                              |
| `ratio`        | `a/b` or integer N.     | Denominator of `a/b`, otherwise N; then `total(g) ≥ target`. |
| `boolean`      | Ignored.                | `total(g) ≥ 1`.                                              |
| `metric`       | Non-negative integer N. | `total(g) ≥ N`.                                              |
| `manual`       | Optional label.         | No automatic derivation.                                     |

For structured non-manual goals, `fitness_current` is synchronized from the
aggregate. When the target parses and is not met:

```text
total(g) > 0 ⇒ status(g) = partial
total(g) ≤ 0 ⇒ status(g) = unverified
```

When the target is met, `status(g) = verified`. These derivations do not
overwrite `declined`. `manual` goals derive neither current value nor status;
only they permit direct `set G-NN status=verified`.

Changing `fitness_target` through `field` or `set` refreshes a structured goal.
`field G-NN fitness_current` is rejected unless `fitness_kind=manual`. Rejecting
or archiving a contributor rederives its cited goals. A verified goal whose
aggregate falls below target demotes through `partial` to `unverified`. A
badge/aggregate disagreement produces `verified goal below target` in `status`.

Without `fitness_kind`, the legacy `fitness="..."` attribute uses the first
`d/d` denominator as its integer threshold. New writes of the legacy attribute
are rejected; `set G-NN fitness=` removes it.

### 5.3 Transition tables

The tables define `set ID status=target`. Invalid status names return exit 1;
guard refusals return exit 4. Creation, undo, repair, renumber, and system
derivation are separate operations. Terminal freeze includes same-status writes.
A permitted nonterminal same-status write still runs target guards.

#### Goals

| Current status          | Fitness kind         | Permitted targets                                  |
| ----------------------- | -------------------- | -------------------------------------------------- |
| `unverified`, `partial` | Non-manual or legacy | `unverified`, `partial`, `declined`.               |
| `unverified`, `partial` | `manual`             | `unverified`, `partial`, `verified`, `declined`.   |
| `verified`, `declined`  | Any                  | None through `set`; system derivation is separate. |

#### Work

| Current status                 | Permitted targets                                               | Target guards                                   |
| ------------------------------ | --------------------------------------------------------------- | ----------------------------------------------- |
| `proposed`, `ready`            | `proposed`, `ready`, `progress`, `done`, `rejected`, `archived` | I₁ through I₅ and I₉ through I₁₁ as applicable. |
| `progress`                     | `progress`, `ready`, `proposed`, `done`, `rejected`, `archived` | Session release plus target guards.             |
| `done`, `rejected`, `archived` | None through `set`                                              | Terminal freeze.                                |

`ready` checks I₉ for features. `progress` checks DoR, I₄, I₅, and session
ownership, then writes `session` and `session_at`. `done` checks I₂, I₃, I₅, and
I₁₀. Leaving `progress` clears the claim. Every `W` status mutation rederives
cited goals.

#### Decisions, Questions, and Assumptions

| Kind and current status                              | Permitted targets                                                                     |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `D` `proposed`                                       | `proposed`, `accepted`, `rejected`, `superseded`.                                     |
| `D` `accepted`                                       | `superseded`.                                                                         |
| `D` `rejected`, `superseded`                         | None.                                                                                 |
| `Q` `open`                                           | `open`, `deferred`, `answered`, `dropped`.                                            |
| `Q` `answered`, `deferred`, `dropped`                | None.                                                                                 |
| `B` `proposed`, `testing`, `validated`               | `proposed`, `testing`, `validated`, `invalidated_acceptable`, `invalidated_blocking`. |
| `B` `invalidated_acceptable`, `invalidated_blocking` | None.                                                                                 |

Entering `invalidated_blocking` emits dependent-work warnings and does not
change dependent statuses.

#### Discoveries, Themes, and Areas

| Kind and current status | Permitted targets      | Additional guard                           |
| ----------------------- | ---------------------- | ------------------------------------------ |
| `Y` `proposed`          | `active`, `superseded` | Activation requires I₁₂.                   |
| `Y` `active`            | `stale`, `superseded`  | Direct same-status assignment is rejected. |
| `Y` `stale`             | `superseded`           | `active` requires `revalidate`.            |
| `Y` `superseded`        | None.                  | Terminal.                                  |
| `T` `open`, `done`      | None.                  | Derived by I₆.                             |
| `A` `present`           | None.                  | Structural status.                         |

### 5.4 Session ownership

Every transition into `progress` writes `session` and `session_at` in RFC 3339
UTC second precision. Subsequent mutations require the same effective token
until `W` leaves `progress` or becomes terminal.

The effective token is selected in this order:

1. `--session=<token>`.
2. `GROVE_SESSION`.
3. `host:hex16(sha256(norm_root))`, using `COMPUTERNAME`, `HOSTNAME`, or `HOST`.

`resume` adopts a progress claim, `handoff --to=<token>` transfers it, and
`revert` returns `W` to `ready`. Undo restores prior claim snapshots. `status`
marks a progress claim stale when its token differs or its timestamp is more
than 24 hours old. Two sessions cannot simultaneously hold the same progress
`W`.

### 5.5 Alignment triggers

`status` MUST report alignment triggers using the active graph:

```text
align ⟺
  (∃ q ∈ Q, cynefin(q) = chaotic) ∨
  (∃ b ∈ B, status(b) = invalidated_blocking) ∨
  (∃ w ∈ W, status(w) = done ∧ significant(w)) ∨
  (∃ g ∈ G, status(g) = verified) ∨
  (Ready = ∅ ∧ ((∃ q ∈ Q, status(q) = open) ∨
    (∃ b ∈ B, status(b) ∈ { proposed, testing })))

significant(w) ⟺
  (∃ d ∈ D, (w, implements, d) ∈ E ∧ status(d) = accepted) ∨
  (w lies on the current critical path) ∨
  (type(w) = refactor) ∨
  (type(w) = spike ∧ (cynefin(w) = complex ∨ produces(w) contains a D))
```

The current critical path is the chain selected by `path`. These triggers are
advisory alignment signals; they do not change node status or graph state.

## 6. Lockfile format

### 6.1 Storage files

| Path                 | Protocol role                                |
| -------------------- | -------------------------------------------- |
| `.grove/state.lock`  | Canonical graph state with SHA-256 checksum. |
| `.grove/index.md`    | Generated projection of graph state.         |
| `.grove/glossary.md` | Domain vocabulary used by discovery tags.    |
| `.grove/journal.log` | JSON-lines inverse and audit history.        |

`state.lock` is the source of truth. `index.md` is regenerated after mutations.
The registry at `$GROVE_HOME/projects.toml`, defaulting to
`~/.grove/projects.toml`, is an unchecked convenience index rather than
checksummed project state. Separate projects have independent locks and no live
cross-lock references.

Within one project, mutation commands MUST hold an exclusive file lock on
`.grove/state.lock` for the duration of the call. Read commands MUST hold a
shared lock. An implementation MUST break a lock older than 60 seconds and emit
a warning when doing so. File locking serializes storage access; session
ownership under section 5.4 independently guards mutations of progress work.

### 6.2 Envelope and lexical rules

```text
@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:<64-lowercase-hex>
# @grove-id stride=<N> offset=<K> pad=<W>
<records...>
```

The allocator line is optional. `N` and `K` are decimal integers at least 1; `W`
is at least 2. Defaults are `stride=1`, `offset=1`, and `pad=2`. The first
suffix in an empty family is `offset`; later suffixes are the prior numeric
maximum plus `stride`.

Line 1 accepts exactly `@grove 1`. Lines 2 and 3 are mandatory. The checksum is
SHA-256 over the canonical body after line 3, including allocator metadata.
Files use UTF-8, LF line endings, a final newline, and no trailing whitespace. A
checksum mismatch returns exit 2 on verified load. `repair --confirm` recomputes
the checksum and canonical serialization.

Each record consists of a header and zero or more fields. Field indentation is
two spaces; prose indentation is four spaces. Comments begin with `#` at column
zero, are preserved on read, and are not created by the CLI outside the
envelope. Blank lines separate records and collapse to one on serialization.
Unknown fields are parse errors.

### 6.3 EBNF grammar

```ebnf
file           = magic NL comment NL checksum NL [ allocator NL ] { NL }
                 { record } [ archive ] ;
magic          = "@grove 1" ;
comment        = "#" { text-character } ;
checksum       = "# checksum: sha256:" hex-64 ;
allocator      = "# @grove-id stride=" positive
                 " offset=" positive " pad=" pad ;
positive       = nonzero { digit } ;
pad            = digit digit { digit } ;  (* numeric value >= 2 *)
record         = node | edge ;
node           = node-kind SP id { SP attr } [ SP qstring ] NL { field } ;
node-kind      = "g" | "w" | "d" | "q" | "b" | "t" | "y" | "a" ;
edge           = "e" SP id SP label SP id { SP attr } NL ;
label          = "blocks" | "causes" | "implements" | "asks" | "tests"
               | "targets" | "produces" | "supersedes" | "distills" ;
attr           = key "=" attr-value ;
attr-value     = bare-word | qstring | timestamp ;
key            = letter { letter | digit | "_" | "-" } ;
bare-word      = ( letter | "_" ) { letter | digit | "_" | "-" } ;
qstring        = DQUOTE { qchar | escape } DQUOTE ;
qchar          = ? UTF-8 except DQUOTE, reverse solidus, CR, or LF ? ;
escape         = "\\\"" | "\\\\" | "\\n" ;
id             = family "-" digit digit { digit } ;
family         = "G" | "W" | "D" | "Q" | "B" | "T" | "Y" | "A" ;
timestamp      = four-digits "-" two-digits "-" two-digits "T"
                 two-digits ":" two-digits ":" two-digits "Z" ;
field          = "  " key ":" [ SP inline-value ] NL { prose-line } ;
inline-value   = single-value | ref-list | fitness-list ;
single-value   = text ;
ref-list       = list-item { "," SP list-item } ;
list-item      = { ? UTF-8 except comma, CR, or LF ? } ;
fitness-list   = fitness-entry { "," SP fitness-entry } ;
fitness-entry  = id "=" [ "+" | "-" ] digit { digit } ;
prose-line     = "    | " text NL ;
text           = { text-character } ;
text-character = ? UTF-8 except CR or LF ? ;
archive        = ":archive" NL { record } ;
hex-64         = hex-digit { hex-digit } ;  (* exactly 64 digits *)
hex-digit      = digit | "a" | "b" | "c" | "d" | "e" | "f" ;
four-digits    = digit digit digit digit ;
two-digits     = digit digit ;
letter         = ? ASCII uppercase or lowercase letter ? ;
NL             = %x0A ;
SP             = %x20 ;
DQUOTE         = %x22 ;
digit          = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
nonzero        = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
```

Quoted strings are required for attribute values containing whitespace, quote,
or reverse solidus. Titles are always quoted. Text after the four-space pipe
prefix is literal; pipe, quote, and reverse solidus require no escaping.

### 6.4 Header attributes

| Kind | Required attributes         | Optional attributes                 | Trailing title |
| ---- | --------------------------- | ----------------------------------- | -------------- |
| `G`  | `status`                    | `fitness_kind`, `requires_coverage` | Yes.           |
| `W`  | `type`, `status`, `cynefin` | `session`, `session_at`             | Yes.           |
| `D`  | `status`                    | None.                               | Yes.           |
| `Q`  | `status`, `cynefin`         | None.                               | Yes.           |
| `B`  | `status`, `cynefin`         | None.                               | Yes.           |
| `T`  | `status`                    | `requires_coverage`                 | Yes.           |
| `Y`  | `status`                    | None.                               | Yes.           |
| `A`  | `status=present`            | None.                               | Yes.           |

Every node also has `t_created` and `t_updated`; every edge has `t_created`.
These are RFC 3339 UTC timestamps with second precision, assigned by the CLI.
They feed `log`, `diff`, and metrics.

`T` is materialized with user-set title and tags but derived status. `Y` is
never archived. `A` has structural `present` status and is never archived.

### 6.5 Fields

Field forms are prose, reflist, single, and fitness. Unknown kind/field pairs
are invalid. `tags` is a reflist on every kind except `A`.

| Kind | Fields in canonical order                                                                                                           |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `W`  | `goals`, `theme`, `fitness`, `tags`, `surface`, `ac`, `hypothesis`, `repro`, `exit`, `evidence_strategy`, `evidence`, `plan`, `why` |
| `D`  | `tags`, `context`, `options`, `decision`, `consequences`, `validation`                                                              |
| `Q`  | `tags`, `why`, `hypothesis`, `exit`, `log`, `outcome`                                                                               |
| `B`  | `tags`, `vm`, `threshold`, `result`                                                                                                 |
| `G`  | `area`, `tags`, `fitness_target`, `fitness_current`, `notes`                                                                        |
| `T`  | `tags`, `notes`                                                                                                                     |
| `Y`  | `tags`, `surface`, `invariant`, `why`, `skill_updates`, `glossary_updates`, `revalidation`                                          |
| `A`  | `surface`                                                                                                                           |

`area`, `theme`, `fitness_target`, and `fitness_current` are single fields.
`goals`, `surface`, and `tags` are reflists. `fitness` has its dedicated form.
All other fields are prose.

| Field                             | Protocol meaning                                                      |
| --------------------------------- | --------------------------------------------------------------------- |
| `W.goals`                         | Targeted `G-NN` identifiers.                                          |
| `W.theme`                         | Single `T-NN` membership.                                             |
| `W.fitness`                       | Staged `G-NN=±N` deltas.                                              |
| `W.surface`                       | Declared files read or written.                                       |
| `W.ac`                            | One acceptance criterion per line.                                    |
| `W.hypothesis`                    | Feature hypothesis.                                                   |
| `W.repro`                         | Bug reproducer.                                                       |
| `W.exit`                          | Spike exit criteria.                                                  |
| `W.evidence_strategy`             | Evidence collection contract.                                         |
| `W.evidence`                      | Completion evidence.                                                  |
| `D.options`                       | Options prefixed `OC1:`, `OC2:`, and so on.                           |
| `Q.exit`                          | Question exit criteria.                                               |
| `Q.log`                           | Investigation log.                                                    |
| `B.vm`, `B.threshold`, `B.result` | Validation method, threshold, and result.                             |
| `G.fitness_current`               | Derived except when `fitness_kind=manual`.                            |
| `G.notes`                         | Goal notes, including optional `--distill-deferred`.                  |
| `Y.surface`                       | Diffable origin anchor; absence requires `why` and gives no coverage. |
| `Y.invariant`                     | Curated distilled content.                                            |
| `Y.skill_updates`                 | Process learning from the goal.                                       |
| `Y.glossary_updates`              | Term changes from distillation.                                       |
| `Y.revalidation`                  | One line per revalidation.                                            |
| `A.surface`                       | Gate-grade area attribution anchor.                                   |

The notation `W.goals` in this table names the `goals` field on kind `W`; it is
not a separate serialized key.

### 6.6 Canonical ordering and identifiers

Serialization order is deterministic:

1. Active `g`, `w`, `d`, `q`, `b`, `t`, `y`, `a`, sorted by ID within kind.
2. Edges sorted lexicographically by `(from, label, to)`.
3. Optional `:archive`, followed by archived records in the same kind order.

Fields follow section 6.5 order. Prose preserves insertion order. Empty fields
are omitted. Each family has a monotonically increasing ID allocator. IDs are
never reused, including after archive. Disjoint stride/offset lanes permit
independent worktree allocation without changing family semantics.

## 7. Journal and persistent lifecycle

### 7.1 Journal envelope

`.grove/journal.log` contains one JSON object per line:

```json
{
    "v": 1,
    "ts": "2026-07-19T12:35:19Z",
    "cmd": "set",
    "inv": { "op": "set_status_plain", "id": "B-01", "old_status": "testing" },
    "session": "host:0123456789abcdef"
}
```

| Key       | Meaning                                          |
| --------- | ------------------------------------------------ |
| `v`       | Journal generation, currently `1`.               |
| `ts`      | UTC timestamp with second precision.             |
| `cmd`     | Writing command.                                 |
| `inv`     | Undo inverse or audit payload.                   |
| `session` | Effective session token; `none` when unresolved. |

Readers MUST tolerate missing `session` on legacy records. Mutation inverse
families include `rm_node`, `unlink_edge`, `restore_*`, `set_*`, `field_*`,
`renumber_swap`, `revalidate_restore`, and `glossary_rename_restore`.

`gate`, `distill`, `dor_reject`, `undo`, and `archive` are audit-only records.
They are never inverted, counted as mutations, or truncated by undo. In this
classification, non-mutation means excluded from undo selection; `archive` still
changes the lock.

### 7.2 Undo

`undo --steps=N`, default 1, selects the last N mutation records while skipping
audit-only records. It applies inverses newest first, removes the selected
mutation lines, and appends an undo audit record. It restores prior session and
`session_at` snapshots. There is no redo. `init` and `repair` do not journal.
Archive and null distillation are not undoable.

A failed DoR guard on entry to `progress` appends `dor_reject` with failed
conjunct labels and does not change the lock.

### 7.3 Distillation and archive

`distill G-NN` requires `status(g)=verified`. Its worksheet contains validated
`B`, answered `Q`, and accepted `D` from the goal-exclusive archive set, with
suggested discovery provenance. `distill G-NN --null` appends a null
attestation.

When a `W → done` transition makes a linked goal `verified`, the mutation MUST
emit a one-line stderr reminder to run `distill G-NN` or `distill G-NN --null`.
A `notes` line containing `--distill-deferred` suppresses this reminder. The
reminder does not create a discovery or satisfy the archive precondition.

`archive G-NN` requires:

1. A verified `G`.
2. Either at least one discovery provenance-linked into the goal-exclusive
   archive set or a null-distill attestation.
3. No session-guard refusal on affected progress work.

For archiving, each node has a goal-reference set derived from `goals` and
propagated through `implements`, `produces`, `asks`, `tests`, `targets`,
`causes`, `theme`, and bidirectional `supersedes`. Archiving then proceeds in
two steps:

1. **Archive the goal-exclusive set.** This set contains the closing goal and
   affinity-connected `W`, `D`, `Q`, `B`, and `T` whose goal-reference set
   contains exactly that goal. A resource linked to any other goal is excluded.
2. **Archive resources that have no active goals left.** After the first step,
   archive every non-archived `W`, `D`, `Q`, or `B` whose goal-reference set is
   non-empty but contains no non-archived goal. The closing goal counts as
   archived for this test. Consequently, a shared resource is archived when its
   last active goal is archived. A resource whose goals were all archived
   earlier is archived during the next archive operation.

Themes are archived only as members of the goal-exclusive set. Discoveries (`Y`)
and areas (`A`) are never archived. A node with an empty goal-reference set is
not archived by the second step.

Both sets are computed before any mutation. Session guards are checked over both
sets at the same time. If any affected progress `W` is held by a foreign
session, the operation archives nothing. Archive does not rewrite `goals`
fields: references from an archived resource to archived goals remain valid.
Archived nodes are excluded from algebra and read paths by default.

A successful archive appends:

```json
{ "op": "archive", "id": "G-NN", "ids": ["..."] }
```

as the `inv` payload of an audit-only `archive` record. The `ids` list contains
the goal-exclusive set and the resources archived by the second step.

### 7.4 Discovery revalidation and decay

Discovery activation requires provenance, surface or why, and at least one
glossary tag. `active → stale` is permitted directly. `stale → active` requires
`revalidate` with fresh surface and/or provenance. Revalidation surface paths
MUST exist under the project root. Provenance MUST reference an eligible `W` or
`D`/`Q`/`B` that is not superseded or invalidated. Each revalidation appends a
`revalidation` line.

Decay checks cover dead surface paths, superseded `D` origins, invalidated `B`
origins, and missing glossary terms for proposed or active `Y`.

### 7.5 Promotion

`promote Y-NN --to=<project>` copies a discovery into another initialized
project. The copy:

- Receives a fresh target `Y-NN` and starts `proposed`.
- Carries title, tags, surface, invariant, why, skill updates, and glossary
  updates.
- Does not carry revalidation history or edges.
- Records `origin_project`, `origin_id`, and `origin_version`.
- Adds copied tags missing from the target glossary.

Duplicate `(origin_project, origin_id)` promotion is refused with exit 4. The
source lock is read-only. Target-side undo removes the copy.

## 8. Command interface

### 8.1 Invocation, root selection, and flags

```text
grove <command> [args...]
```

Root selection uses the first applicable source:

1. `--root=<path>`, without registry-name lookup.
2. `--project=<directory-or-name>`.
3. `GROVE_PROJECT=<directory-or-name>`.
4. The nearest ancestor containing `.grove/state.lock`.
5. The current directory.

An unknown registry name returns exit 5. Registry rows contain `name`, `path`,
`created`, and `last_opened`. Names default to the directory basename with
numeric collision suffixes. Missing registries read as empty; malformed
registries produce warnings and degraded registry behavior rather than command
crashes.

Global flags are:

| Flag                      | Meaning                                         |
| ------------------------- | ----------------------------------------------- |
| `--root=<path>`           | Explicit project root with highest precedence.  |
| `--project=<dir-or-name>` | Directory or registry-name selection.           |
| `--quiet`                 | Suppress informational messages, not errors.    |
| `--json`                  | Select supported machine-readable output.       |
| `--no-render`             | Skip automatic index generation after mutation. |
| `--session=<token>`       | Override the effective session token.           |

`init` additionally accepts `--id-stride`, `--id-offset`, and `--id-width`.
Other command flags are `packet --cone --cone-depth --cone-max`, `diff --since`,
`log --limit`, `undo --steps`, and `gate --theta --n`.

### 8.2 Exit codes

| Code | Meaning                                                   |
| ---- | --------------------------------------------------------- |
| 0    | Success.                                                  |
| 1    | Generic error, including bad arguments and missing files. |
| 2    | Checksum mismatch on verified load.                       |
| 3    | Invariant violation from `check` or failed undo inverse.  |
| 4    | Guard refusal.                                            |
| 5    | Not found where the command performs that check.          |

### 8.3 Mutation commands

Successful mutations serialize a fresh checksum and regenerate `index.md` unless
`--no-render` is active.

| Command                    | Contract                                                           |
| -------------------------- | ------------------------------------------------------------------ |
| `init`                     | Create lock, index, and glossary; refuse an existing lock.         |
| `add KIND`                 | Create a node and initial structural links; print its assigned ID. |
| `set ID key=value`         | Change a supported scalar or status under section 5.               |
| `field ID FIELD add VALUE` | Append prose or a reflist item.                                    |
| `field ID FIELD rm INDEX`  | Remove a 1-based entry.                                            |
| `field ID FIELD clear`     | Empty a field.                                                     |
| `link FROM LABEL TO`       | Add a typed edge and enforce I₇.                                   |
| `unlink FROM LABEL TO`     | Remove an edge.                                                    |
| `evidence W-NN TEXT`       | Append one evidence line.                                          |
| `fitness W-NN G-NN DELTA`  | Stage or replace one signed delta.                                 |
| `renumber ID --to=NEW-ID`  | Rewrite an ID and every structured reference.                      |
| `resume W-NN`              | Adopt a progress claim.                                            |
| `handoff W-NN --to=TOKEN`  | Transfer a progress claim.                                         |
| `revert W-NN`              | Return progress work to ready.                                     |
| `undo --steps=N`           | Apply journal inverses.                                            |
| `glossary rename OLD NEW`  | Atomically rename a glossary term and discovery tags.              |
| `skill`                    | Print the skill or install it with the embedded specification.     |
| `archive G-NN`             | Apply section 7.3.                                                 |
| `distill G-NN [--null]`    | Emit a worksheet or null attestation.                              |
| `revalidate Y-NN`          | Apply section 7.4.                                                 |
| `render`                   | Regenerate the index projection.                                   |
| `repair --confirm`         | Recanonicalize and re-checksum without proving invariants.         |

Creation options are:

| Kind | Required options                                                        | Optional options                                        |
| ---- | ----------------------------------------------------------------------- | ------------------------------------------------------- |
| `G`  | `--title`, `--area`, `--fitness-kind`; target for count, metric, ratio. | `--fitness-target`, `--status=unverified`.              |
| `W`  | `--title`, `--type`, `--cynefin`.                                       | `--goals`, `--theme`, `--surface`, `--status=proposed`. |
| `D`  | `--title`.                                                              | `--supersedes`, `--status=proposed`.                    |
| `Q`  | `--title`, `--cynefin`.                                                 | `--targets`, `--status=open`.                           |
| `B`  | `--title`, `--cynefin`.                                                 | `--tests`, `--targets`, `--status=proposed`.            |
| `T`  | `--title`.                                                              | `--status=open`.                                        |
| `Y`  | `--title`, `--tags`, `--from`, and `--surface` xor `--why`.             | None.                                                   |
| `A`  | `--title`.                                                              | `--surface`.                                            |

`Y` creation starts `proposed`; `--from=W-NN` creates `produces`, while
`--from=D-NN/Q-NN/B-NN` creates `distills`. CSV options `--tags`, `--surface`,
and `--goals` reject duplicates after trimming surrounding whitespace. Reflist
`field add` rejects an existing value with exit 4 and no mutation.

Scalar `set` keys are `status`, `cynefin`, `type`, `title`, `fitness_kind`,
`fitness_target`, `area`, and `requires_coverage`, restricted by kind. Empty
`fitness_target` clears it. `requires_coverage` accepts `true` or a float in
`(0,1]` on `G` and `T`. The legacy `fitness` key is write-retired except for
empty-value removal.

`renumber` refuses an ID occurring verbatim in evidence on a done `W`.
`glossary rename` refuses an unknown old term or an already existing new term.

### 8.4 Query and report commands

| Command                     | Result and persistent effect                                                          |
| --------------------------- | ------------------------------------------------------------------------------------- |
| `status`                    | Session claims, alignment triggers, and invariant warnings.                           |
| `ready`                     | Currently ready work, sorted by critical-path membership, descending impact, then ID. |
| `next`                      | One execution packet from Ready on the critical path, with Ready fallback.            |
| `packet W-NN`               | Work record, implemented `D`, BChain, outcomes of `Q` linked by `asks`, and DoR.      |
| `deps ID`                   | Transitive `blocks` predecessors in topological order.                                |
| `impact ID`                 | Transitive `blocks` successors.                                                       |
| `path`                      | A longest unfinished-work `blocks` chain.                                             |
| `dor W-NN`                  | Conjunct-by-conjunct DoR report.                                                      |
| `show ID`                   | One node record.                                                                      |
| `list KIND`                 | Nodes with optional status and Cynefin filters.                                       |
| `graph`                     | Mermaid projection.                                                                   |
| `check`                     | Checksum and machine-checkable invariants, edge integrity, anchors, and decay.        |
| `stats`                     | Read-only telemetry from journal and current lock.                                    |
| `diff --since=REF`          | Structural changes against a Git ref, default `HEAD`.                                 |
| `log [ID] --limit=N`        | Newest-first node, edge, and journal timeline; zero disables the cap.                 |
| `projects`                  | Registry rows.                                                                        |
| `triage`                    | Advisory ranking without lock, journal, or index writes.                              |
| `gate`                      | Distillation-gate report and an audit-only journal record.                            |
| `promote Y-NN --to=PROJECT` | Apply section 7.5.                                                                    |
| `specification`             | Print the embedded normative specification without project-state effects.             |

`packet --cone` adds backward and forward `blocks` cones, contraction order,
per-goal vertex-disjoint-path fragility, relevant active `Y`, and a truncation
marker. Defaults are depth 4 and maximum 50 nodes.

`ready` contains only status-ready work with cleared predecessors and current
DoR. `next` chooses the first critical-path member from that set, otherwise its
first member. `triage` ranks nonterminal work by ascending active-discovery
coverage, descending uncertainty, then ID. Uncertainty combines open `Q`,
unresolved `BChain`, and failed DoR conjuncts. Declared coverage is `0.00` when
no surface is declared. Fragility means at most one vertex-disjoint `blocks`
path from a goal. Triage never feeds `next` or an invariant.

`gate` computes active-graph treewidth using a min-fill upper bound, its delta
from the previous gate record, and done-work count. The done threshold defaults
to 5. It reports Git surface overflow above theta, invalidated `B`, and accepted
`D`. Overflow theta defaults to zero. `gate` never changes the lock or creates
discoveries. Empty results still append an audit record with `empty:true`.

`diff` groups added, removed, and changed nodes and edges and ignores pure
reordering. `log ID` also matches identifiers inside inverse payloads. `stats`
never writes project state; missing or malformed journal data is tolerated and
counted where applicable.

### 8.5 JSON envelopes

Read commands supporting `--json` emit one UTF-8 JSON object on success and
include `command`. Exit codes are unchanged. `check` failure returns exit 3 with
`ok:false` and an `errors` array without duplicate stderr errors. Most mutations
ignore `--json`; `add y`, `distill`, `revalidate`, `glossary rename`, and
`promote` define small JSON results.

A diagnostic is a structured value with `severity`, `code`, `text`, and optional
`details`. Severity is `error`, `warning`, or `info` and remains independent of
the diagnostic category. Human rendering is `severity code text`, followed by
indented `key=value` lines for details in their deterministic insertion order.
JSON diagnostics are objects with the same field names. The `errors` array from
`check` contains diagnostic objects rather than untyped strings.

The diagnostic code set is closed and contains exactly these serialized values.

| Code          | Meaning                                                                                                                         |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `SYSTEM`      | Infrastructure, environment, filesystem, external processes, registries, integrity, checksums, and session locks.               |
| `INTERFACE`   | Caller input and request interpretation, including arguments, commands, tools, unsupported keys, and missing requested objects. |
| `INVARIANT`   | Graph logic, integrity invariants, blocking rules, lifecycle rules, ownership rules, and business rules.                        |
| `OPERATIONAL` | Expected lifecycle outcomes and ordinary notices that need no error recovery.                                                   |

`SYSTEM` describes the source rather than criticality and may carry warning or
error severity. `INVARIANT` covers both operations rejected to protect a rule
and checks that discover an already violated rule. `OPERATIONAL` normally
carries info severity and may carry warning severity for a non-error condition.

Borderline cases follow the action available to the caller. Correcting the
request means `INTERFACE`. Changing graph state or following another protocol
operation means `INVARIANT`. Fixing the environment or infrastructure means
`SYSTEM`. No recovery action means `OPERATIONAL`. A missing requested node is
`INTERFACE`, while a missing project lock is `SYSTEM`.

Numeric process exits remain independent of diagnostic categories and are
defined by section 8.2. An implementation MUST NOT derive an exit from the
category or invent a category from an exit. MCP forwards the same category as
the equivalent command failure and MUST NOT reclassify it from the transport,
exit code, tool name, or rendered text.

Authored diagnostic text MUST NOT contain semicolons, colons, em dashes, or CLI
command invocations. Numbered invariant identifiers MAY remain in text or
details without creating additional code values. Raw dynamic values such as
paths and timestamps belong in `details` and are exempt from punctuation
rewriting. Documents, pure data, packets, skill and specification payloads, and
user-authored graph content are not diagnostics and remain unchanged.

Adding a diagnostic category requires a new accepted decision. A new condition
within an existing category changes only text or details and MUST NOT expand the
code set. Legacy condition-specific codes are not valid diagnostic codes.

| Command          | Principal keys besides `command`                                                                                 |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- |
| `ready`          | `items: [{id,title,critical}]`.                                                                                  |
| `next`, `packet` | `work`, `packet_markdown`; packet also has optional `cone`.                                                      |
| `deps`           | `id`, `predecessors`.                                                                                            |
| `impact`         | `id`, `successors`.                                                                                              |
| `path`           | `chain`.                                                                                                         |
| `dor`            | `work`, `conjuncts:[{label,ok,detail}]`, `dor`.                                                                  |
| `show`           | `record` with `kind`, `ID`, `title`, `status`, optional `archive`/type/Cynefin, attrs, fields.                   |
| `list`           | `kind`, `rows`, optional `filter_*`.                                                                             |
| `graph`          | `mermaid`.                                                                                                       |
| `log`            | `limit`, `rows:[{ts,sort,line}]`, optional `id_filter`.                                                          |
| `gate`           | `baseline`, `tw_now`, `tw_delta`, `dones`, `due`, `overflows`, `invalidated`, `accepted`, `empty`, `theta`, `n`. |
| `triage`         | `rows` with work, title, coverage, declared, uncertainty, fragile, suggestion.                                   |
| `distill`        | `goal`, `precondition_met`, `linked_das`, `null_attested`, `candidates`; null form has `goal`, `null`, `empty`.  |
| `check`          | `ok`, `errors`.                                                                                                  |
| `status`         | `progress`, `alignment_triggers`, `invariants`.                                                                  |
| `specification`  | `mode`, `markdown`.                                                                                              |
| `diff`           | `since`, `semantic_change`, `nodes`, `edges`.                                                                    |
| `projects`       | `projects`.                                                                                                      |
| `promote`        | `id`, `origin_project`, `origin_id`.                                                                             |

Packet `cone` contains `backward`, `order`, `forward`, `fragility`,
`relevant_discoveries`, `truncated`, `depth`, and `max`. `fragility` entries
contain `goal` and `paths`.

`stats` returns `records`, `mutations`, `cycle_time`, `dor`, `bets`,
`discovery`, `gates`, `undo`, `audit`, `rework`, `distill_yield`, `surprise`,
`surprise_series`, `cv_series`, and `replay_failures`. Undefined rates are null.
Replay failures are counted rather than fatal.

### 8.6 MCP transport

`grove-mcp` exposes the command interface one-to-one as `mcp__grove__*` tools
over newline-delimited JSON-RPC on stdio. Supported protocol versions are
`2024-11-05`, `2025-03-26`, and `2025-06-18`.

Methods are `initialize`, `notifications/initialized`, `ping`, `tools/list`, and
`tools/call`. Tool argument and Grove guard failures are tool results with
`isError:true`. Protocol failures are JSON-RPC errors. Roles and disabled tools
are configured through `$GROVE_HOME/mcp.toml`.
