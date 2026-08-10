# 3. Planning

How to turn an intention into grove state. Applies whenever you create or reshape goals and work items, before any `status=progress` happens.

## 1. Planning context lives in the lock, never in markdown files

Every artifact of planning - a design choice, an open question, a hypothesis, a risk - is a grove node, not a paragraph in a side document. The repo's own history is the precedent: `docs/plans/` and `docs/roadmap.md` were retired precisely because their content moved into D/Q/W records (see D-05 to D-09, Q-03, Q-04 in this project).

If you catch yourself drafting a plan as a markdown file, stop and decompose it into nodes instead:

| You were about to write | Create instead |
| --- | --- |
| "We choose A over B because..." | `grove add d` with `context`, `options`, `decision`, `consequences`, `validation` |
| "We don't know whether X..." | `grove add q` (+ `asks` edges to the nodes it blocks) |
| "We believe Y is true..." | `grove add b` with a validation method (+ `targets` / `tests` edges) |
| "Open question for later..." | `grove add q` (deferred is a status, not a file) |

The only exception is a document the user explicitly asked for. Even then, the operative decisions belong in D records; the document is a view, not the source.

## 2. Use the full node vocabulary

The protocol technically runs on G and W alone. That is a license to be lazy, not a design. A plan containing only goals and work items is a smell: it means unknowns were swallowed, choices went unrecorded, and hypotheses will be re-tested or silently trusted.

Node selection:

- **A (area)**: the permanent scope skeleton. Create rarely, once per durable product area; every goal hangs under exactly one (I13).
- **G (goal)**: an outcome with a fitness function. Never a task list.
- **W (work item)**: the smallest executable unit with its own acceptance criteria.
- **Q (question)**: an open unknown. If planning cannot proceed without the answer, link `asks` to the blocked node - the CLI will gate it for you.
- **B (assumption)**: anything you currently treat as true without having checked: performance, compatibility, user behavior, external API shape. Give it a validation method and let it become `validated` or `invalidated_*` instead of folklore.
- **D (decision)**: a long-lived design choice. Once accepted it is immutable; it can only be superseded with a recorded rationale. This is where architectural reasoning lives permanently.
- **T (theme)**: optional grouping when a goal's work items cluster.
- **Y (discovery)**: not created during planning; it arrives later through distillation or `produces` from a spike.

Heuristics: "we think but haven't verified" -> B. "we don't know and it blocks" -> Q. "we picked one and it must stay picked" -> D. "we will need this grouping in the UI" -> T.

## 3. Required fields: DoR is the floor

For every work item before it becomes `ready`:

- `title` - a concrete outcome, not a topic.
- `goals` - at least one G-NN link.
- `type` and `cynefin` - honest classification; `chaotic` means stop and escalate to the human.
- `ac` - measurable acceptance criteria; each one checkable by a test, a command, or a screenshot, never by adjectives.
- `hypothesis` - why this approach should work.
- `evidence_strategy` - how you will prove the AC at close time.
- `fitness` - a staged delta for every linked goal (`grove fitness W-NN G-NN +N`).
- `blocks` edges - wherever sequencing actually matters.

`grove dor W-NN` must be `⊤` before `status=progress`; the CLI refuses otherwise. Fill the fields at creation, not at claim time.

For every goal at creation:

```bash
grove add g --area=A-NN --title="..." --fitness-kind=count --fitness-target=N
# fitness_target e.g. the number of planned work items
```

`fitness n/a` is legitimate only as a deliberate `fitness_kind=manual` decision. `add g` refuses a missing fitness specification outright (pass `--fitness-kind` with `--fitness-target` for `count` / `metric` / `ratio`, or `--fitness-kind=manual`); the legacy `--fitness="…"` label is retired (writes rejected), and the `set` / `field` backfill above no longer works.

## 4. Planning workflow

1. **Understand the intention.** Restate it as one sentence; if it splits into unrelated parts, those are separate goals.
2. **Anchor scope.** Pick or create the area (`grove list a`, `grove add a`).
3. **Create the goal with fitness** (section 3). The fitness target should be derivable from the decomposition you are about to do.
4. **Surface the unknowns first.** Before writing any W, list what you do not know and what you are assuming - those become Q and B nodes with their edges, because they shape the decomposition.
5. **Record the choices.** Any "we do X, not Y, because Z" becomes a D node while the reasoning is fresh.
6. **Decompose into work items** with full DoR fields (section 3) and `blocks` edges for real dependencies.
7. **Sanity-check.** `grove ready` shows the executable set; `grove dor` on anything that should be ready; `grove path` for the critical chain. Then present the plan to the user before starting.

## 5. Reasoning stays in your context; the lock stores conclusions

Deliberate as long as you need - but what lands in the lock is the compressed outcome, never the deliberation. Fields are atomic facts: one acceptance criterion per `field ... ac add`, one sentence per hypothesis line, no essays. Reasoning behind a choice concentrates in its D node (`context`, `options`, `decision`, `consequences`) and stays compact there too. Evidence at close time is numbers, files, and hashes ("workspace tests 301/0, desktop 121/0, screenshot X"), not a narrative of what you tried.

Practical serialization:

- Think the whole plan through in-context first, then write it down node by node. One batched shell call per node carries everything (`add` + all `field` lines + `fitness`), and one call can chain several nodes with `&&`.
- Never dump your reasoning into a field "for completeness". If a fact does not change what a future agent does, it does not belong in the lock.
- Length is not a workaround trigger: dozens of small CLI calls are normal and cheap compared to re-reading state; `grove packet` / `grove next` exist precisely so you never re-read the lock to plan.

## 6. Reading this skill

`index.md` is the minimal safe contract - it is short by design and complete for operation. The other pages are depth, not prerequisites for every action: open them when the task actually touches their topic. `cli.md` is a reference, not a tutorial; when unsure of a command's shape, run it - every refusal (`add g: --area is required`, `DoR ≢ ⊤; see grove dor W-NN`) is a precise instruction. The CLI's invariants are the last line of defense: partial reading degrades process quality, never state integrity.
