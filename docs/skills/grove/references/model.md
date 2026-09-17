# 1. Formal model

## 1.1 Node taxonomy

Development state is the tuple:

```text
Σ ≜ (G, W, D, Q, B, T, Y, A, E)
```

| Set | Symbol | Meaning | ID prefix |
| --- | --- | --- | --- |
| Goals | G | Outcome / requirement; has fitness function. | `G-NN` |
| Work items | W | Executable unit with DoR + DoD. | `W-NN` |
| Decisions | D | ADR; long-lived design choice. | `D-NN` |
| Questions | Q | Open unknown. | `Q-NN` |
| Assumptions | B | Falsifiable assumption with validation method and result. | `B-NN` |
| Artifacts (themes) | T | Grouping of related W (optional). | `T-NN` |
| Discoveries | Y | Curated invariant distilled from process records; never archived. | `Y-NN` |
| Areas | A | Permanent scope skeleton above goals; owns goals; no lifecycle. | `A-NN` |
| Edges | E ⊆ N × LabelE × N | Typed graph edges (§1.3). | – |

with N ≜ G ∪ W ∪ D ∪ Q ∪ B ∪ T ∪ Y ∪ A.

All nodes and edges are stored in `./grove/state.lock` (see [Lockfile](lockfile.md)). There are no per-node files.

## 1.2 Work item type

```text
type(w) ∈ { feature, refactor, bug, spike }
```

- **feature**: new capability; needs hypothesis (HDD) and resolved assumptions when discovery exposed uncertainty.
- **refactor**: structural change, behaviour preserved; needs root cause (causation edge from T).
- **bug**: defect in shipped behaviour; needs reproducible evidence.
- **spike**: investigation only; produces D, Q, or B, not production code.

## 1.3 Edge labels

```text
LabelE = { blocks, causes, implements, asks, tests, supersedes, produces, targets, distills }
```

| Label | Domain → Codomain | Meaning |
| --- | --- | --- |
| `blocks` | N → W | Predecessor must be terminal before successor may start. |
| `causes` | T → W (refactor/bug) | Root cause to symptom. |
| `implements` | W → D | Work item realises an accepted decision. |
| `asks` | Q → N | Open question is raised against the target node. |
| `tests` | B → Q | Assumption operationalises a question into falsifiable validation. |
| `targets` | B → W | Assumption is required by a work item (defines `assumptions(w)`). |
| `produces` | W → D ∪ Q ∪ B ∪ Y | Work item (typically a spike) produced this record. |
| `supersedes` | D → D, Y → Y | New record replaces the old one. |
| `distills` | Y → D ∪ Q ∪ B | Discovery distills content from this process record. |

The graph (N, E) is acyclic on `blocks`. Cycles on other labels are allowed.

## 1.4 Status sets

```text
status(g) ∈ { unverified, partial, verified, declined }
status(w) ∈ { proposed, ready, progress, done, rejected, archived }
status(d) ∈ { proposed, accepted, rejected, superseded }
status(q) ∈ { open, deferred, answered, dropped }
status(b) ∈ { proposed, testing, validated, invalidated_acceptable, invalidated_blocking }
status(t) ∈ { open, done }   (derived per I₆; never set manually)
status(y) ∈ { proposed, active, stale, superseded }
status(a) = present   (structural; no lifecycle, never set manually)
```

## 1.5 Cynefin tag (mandatory on Q, B, and W)

```text
cynefin(n) ∈ { clear, complicated, complex, chaotic }
```

Drives agent behaviour ([Protocol](protocol.md) §2.2). If `chaotic`, stop and escalate.

## 1.6 Core invariants

```text
I₁:  ∀ w ∈ W with status = progress, DoR(w) ≡ ⊤.
I₂:  ∀ w with type = spike ∧ status = done,
      produces(w) ⊆ D ∪ Q ∪ B  ∧  produces(w) ≠ ∅.
I₃:  ∀ w with status = done, ∃ ev ∈ Evidence, satisfies(ev, AC(w)).
I₄:  |{ w ∈ W : status(w) = progress }| ≤ WIP_LIMIT (default 2).
I₅:  ∀ (n₁, blocks, n₂) ∈ E, terminal⁺(n₁) before status(n₂) may transition to progress.
I₆:  ∀ t ∈ T, status(t) = done ⟺ ∀ w ∈ WI(t), status(w) ∈ { done, rejected, archived }.
I₇:  graph (N, E ∩ (· × {blocks} × ·)) is a DAG.
I₈:  ∀ q ∈ Q with cynefin(q) = chaotic, status transitions only via human.
I₉:  ∀ w ∈ W with type = feature, DoR(w) ⇒
      ∀ b ∈ BChain(w), status(b) ∈ { validated, invalidated_acceptable }.
I₁₀: status transition w → done is atomic with applying fitness deltas
      to each g ∈ goals(w) and re-deriving status(g). Either both succeed or
      neither does. The CLI rejects status=done unless deltas are staged
      in the same call (or pre-staged via `grove fitness` since the last
      status mutation of w).
I₁₁: ∀ w ∈ W with status = progress, the session that set it is the only
      session permitted to mutate w until terminal(w) or w leaves `progress`
      (e.g. `revert` or another guarded status change). Persisted as header
      attrs `session` and `session_at` (UTC); `check` rejects a missing token
      (`grove resume` adopts; see protocol §2.6).
I₁₂: ∀ y ∈ Y: (≥1 provenance edge: (w, produces, y) ∨ (y, distills, d/q/b))
      ∧ (surface(y) ≠ ∅ ∨ why(y) ≠ ∅) ∧ tags(y) ≠ ∅ (≥1 glossary term).
      `proposed → active` is refused while any conjunct fails; `stale → active`
      only via `grove revalidate` paid with a fresh anchor.
I₁₃: ∀ g ∈ G: ∃ a ∈ A with area(g) = a.id, recorded as the mandatory `area`
      field and enforced at creation (`grove add g --area=A-NN`); re-partition
      via `grove set G-NN area=A-NN`. An area-less goal in the lock is a
      violation, never silently repaired.
```

with terminality:

```text
terminal(w ∈ W)  ⟺ status(w) ∈ { done, rejected, archived }
terminal⁺(g ∈ G) ⟺ status(g) = verified            -- strict for blocks-edges
terminal(g ∈ G)  ⟺ status(g) ∈ { verified, declined }
terminal(d ∈ D)  ⟺ status(d) ∈ { accepted, rejected, superseded }
terminal(q ∈ Q)  ⟺ status(q) ∈ { answered, deferred, dropped }
terminal(b ∈ B)  ⟺ status(b) ∈ { validated, invalidated_acceptable, invalidated_blocking }
terminal(t ∈ T)  ⟺ status(t) = done
terminal(y ∈ Y)  ⟺ status(y) = superseded      -- stale stays readable, contributes nothing
terminal(a ∈ A)  ⟺ ⊥                            -- areas have no lifecycle
```

`terminal⁺` is the strict variant used for `blocks` edges: a `declined` goal
does not unblock dependents. Other relations use the lax `terminal`.

```text
assumptions(w) ≜ { b ∈ B | (b, targets, w) ∈ E }
BChain(w)      ≜ assumptions(w) ∪ { b ∈ B | ∃ q, (q, asks, w) ∈ E ∧ (b, tests, q) ∈ E }
produces(w)    ≜ { n ∈ D ∪ Q ∪ B ∪ Y | (w, produces, n) ∈ E }
goals(w)       ≜ as recorded in `goals` field of w
WI(t)          ≜ { w ∈ W | theme(w) = t }
```

## 1.7 Definition of Ready

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

`grove dor <ID>` evaluates DoR conjunct-by-conjunct.

The coverage conjunct is opt-in: it is active only when some `g ∈ goals(w)` or
`theme(w)` carries the header attr `requires_coverage`: `true` means θ = 0.5,
a float in `(0,1]` sets θ directly, and several carriers resolve to the maximum
θ. `coverage(w)` is the share of w's declared `surface` (the estimate recorded
via `grove add w --surface` or `grove field W-NN surface add`) covered by the
union of surfaces of **active** discoveries; `proposed` / `stale` /
`superseded` Discoveries and `surface=none` Discoveries contribute nothing. The conjunct
applies only to `feature` work in the `complex` domain; every other shape
passes vacuously. Spikes need no exemption: they are non-feature, so the first
spike in an uncovered area stays the sanctioned way coverage is created (cold
start). DoR keeps its pin-at-transition semantics: a later staleness event
blocks new `progress` transitions but never breaks an in-flight W.

## 1.8 Timestamps

Every node carries `t_created` and `t_updated` (RFC-3339, UTC, second precision).
Every edge carries `t_created`. The CLI assigns and bumps these; agents do not
set them. They are used by `grove log`, `grove diff`, and metric exports.

## 1.9 Type-specific obligations

`grove dor` implements the `type(w)` conjuncts in §1.7 (`hypothesis` + BChain for `feature`;
`repro` / `exit` prose fields for `bug` / `spike`; materialised `T` with `(t, causes, w)` for
`refactor`). Further norms (e.g. spike vs production code, failing-test-first for bugs) are
protocol guidance, not additional CLI conjuncts unless recorded in AC / evidence_strategy.

`repro` and `exit` are first-class prose fields on `w` (see lockfile §6.5).

## 1.10 Areas

An area (kind `a`) is a permanent scope stratum above goals: the skeleton `A`
sits next to the content and answers "where it belongs", never "what is known"
(design D14). Areas carry a fixed `present` status, have no lifecycle, and are
never archived; an area whose goals have all left is a valid dormant scope and
stays as the collapsed stratum.

An area owns goals: every `g` carries a mandatory `area: A-NN` field, required
at creation (`grove add g --area=A-NN`) and movable via `grove set G-NN
area=A-NN` (I₁₃). `w` and `t` carry no area field; their areas are projections,
`areas(w) = union(areas(goals(w)))`, and a theme's areas derive from its
members the same way. `y` carries no area field either: Discovery relevance to a
scope is a pure function of state, never a declared residence.

The anchor sets of an area are derived mechanically:

```text
surface(a) ≜ declared surface on a ∪ ⋃ declared surface(w) over the area's work
tags(a)    ≜ ⋃ tags over the area's goals and their work items
nodes(a)   ≜ the area's non-archived goals ∪ their work items
             ∪ { q, b, d linked by any edge to those work items }
```

Attribution of content to an area comes in two tiers over these sets:

- **Soft tier** (render, per-area C/V): the Discovery relevance predicate of the
  packet (surface ∩ ∨ tags ∩ ∨ cone ∩) applied to the area's anchor sets.
  Only `active` Discoveries count; `stale` contributes zero. It is a relevance view,
  not a partition: a W or Discovery touching two areas counts in both, a W without
  goals counts in none, and the project totals in Content health stay primary.
- **Hard tier** (reserved for gates, not yet consumed by any gate): surface ∩
  alone. A Discovery is a coverage donor for area a iff `surface(Discovery) ∩ surface(a) ≠
  ∅`; tags ∩ and cone ∩ never feed a gate (D14).

The dashboard renders one row per area (including dormant ones) under
**Areas**, restricting the Content health components to `nodes(a)` and adding
the area's soft-attributed active Discoveries to C.
