# Grove protocol primer (compact)

This is the minimal behavioral contract for an agent driving grove through MCP tools only. It is a condensed derivative of `docs/skills/` (model.md, protocol.md, rules.md); the full reference ships as `grove-skill.md` with every release. When this text and the full skill disagree, the full skill wins.

## What grove is

Grove is a graph-driven workflow protocol. All state lives in `.grove/state.lock`, a single line-oriented file with a checksum. The agent never reads or writes it directly; every interaction goes through the CLI, exposed here as MCP tools. Rules written in a prompt are suggestions; rules enforced by the CLI are invariants.

## The dual-track loop

Discovery and Delivery run in parallel, not as phases.

- Discovery resolves open questions (Q), assumptions (B), and decisions (D) before dependent work may start. A question tagged `chaotic` halts you: stop and escalate to the human, do not guess.
- Delivery executes work items (W) that are ready. Use `next` to get the highest-priority ready item on the critical path and `packet <W-NN>` to get exactly the context for that item and nothing more.

## Definition of Ready (DoR)

A work item cannot move to `progress` unless its DoR is satisfied: goals linked, acceptance criteria non-empty, no open blocking questions or invalidated-blocking assumptions, fitness deltas staged, hypothesis set (features), cynefin not chaotic. The CLI evaluates this conjunction on every `status=progress` transition; it cannot be overridden. Use `dor <W-NN>` to see what is missing and fix fields via `field <W-NN> <name> add "..."` instead of forcing the transition.

## Evidence discipline (Definition of Done)

- Before closing, record concrete evidence: `evidence <W-NN> "..."` with facts - files, test counts, measurements, commands run. Not intentions, not summaries of what should be true.
- Only then `set <W-NN> status=done`. The transition is atomic with applying staged fitness deltas to linked goals.
- `check` must stay green. Run it after mutations; treat any violation as a stop signal, not a warning.
- A spike (W with type=spike) closes only when it produced at least one record (D, Q, B, or Y).

## WIP limit and sessions

At most WIP_LIMIT work items (default 2) may be in `progress`. Session tokens (I11) mean only the session that claimed an item may mutate it; if the CLI says another session owns the item, do not fight it - pick another item or ask the human. If you resume after a break, `resume` adopts the token.

## Statuses you will use

- W: proposed -> ready -> progress -> done (rejected, archived are also terminal).
- Q: open -> answered (or deferred/dropped).
- B: proposed -> testing -> validated (or invalidated_acceptable / invalidated_blocking).
- D: proposed -> accepted (or rejected/superseded).
- G (goals): unverified -> partial -> verified (declined).

## Distill and archive cadence

When a goal reaches `verified`, run `distill <G-NN>`: validated assumptions, answered questions, and accepted decisions become Discoveries (Y) - curated domain axioms that outlive the goal and feed future packets. Archive only after distillation; discoveries are never archived, they go stale and get revalidated instead.

## Operational rules

- Minimal diffs; stay on the current work item's topic.
- Code and comments in English; ASCII hyphens, no em/en dashes in code text.
- No comments in code unless the codebase already has them as a convention.
- Never edit `.grove/state.lock` by hand; if the checksum blocks you, stop and report, do not repair without the human.
- Do not commit or push unless the user explicitly asks.
- Temporary files go to the system temp area, not the repo.

## Minimal command map

`next` (what to do), `packet W-NN` (context), `dor W-NN` (what blocks starting), `field W-NN ac add "..."` (fill DoR fields), `fitness W-NN G-NN +1` (stage delta), `evidence W-NN "..."` (record proof), `set W-NN status=progress|done` (transitions), `check` (invariants), `show <ID>` (inspect a node), `distill G-NN` (close out a verified goal).
