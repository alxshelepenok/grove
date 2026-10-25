---
name: grove
description:
    Apply this skill if and only if (a) `.grove/` exists in the project root, or
    (b) the user explicitly asks to "use grove", "init grove", or names a grove
    command. Do NOT apply on inferred non-triviality, and do NOT auto-init on
    greenfield tasks without an explicit request. Covers discovery, features,
    refactoring, bug investigation, and spikes once activated.
---

# Grove

Project work uses the Grove protocol, which turns development with autonomous AI
agents from chaotic guesswork into an executable and deterministic process.
Grove requires the agent to decompose the project into a strictly typed
hierarchy of nodes. No thoughts or architectural ideas may remain hidden in
conversational chats or side files.

Put simply, Grove is a **typed knowledge graph of the project** with
**command-line interface** or **MCP** for exploring that graph and performing
other work.

The project model consists of eight node types:

1. **Area (A-NN):** the permanent scope skeleton of the project that sits above
   goals and never gets archived.
2. **Goal (G-NN):** the desired outcome with a measurable fitness function.
3. **Work item (W-NN):** the executable unit of work of a defined type
   (`feature`, `refactor`, `bug`, or `spike`) with documented readiness
   criteria.
4. **Decision (D-NN):** a recorded architectural choice. Once accepted, it
   becomes immutable and can only be superseded by a new Decision.
5. **Question (Q-NN):** an open unknown that is formally declared rather than
   ignored.
6. **Assumption (B-NN):** a falsifiable hypothesis that gates work items until
   it is validated.
7. **Theme (T-NN):** an optional grouping of related work (`W`).
8. **Discovery (Y-NN):** reusable, evidence-backed knowledge distilled from
   completed goals..

## Workflow for agents

Use Grove to keep reasoning, decisions, execution, and evidence in one queryable
graph. Discovery and Delivery are concurrent loops, not sequential project
phases. Read only the records needed for the current action.

The [Grove specification](../../specification.md) is the normative definition of
the graph, invariants, serialization, lifecycle, and command behavior. Read it
independently with `grove specification` or the `specification` MCP tool when
the packaged relative link is not directly accessible. This skill defines how an
agent applies that protocol. Do not treat examples here as permission to bypass
a protocol guard.

## Non-negotiable rules

- Use `grove` or the equivalent `mcp__grove__*` tools for every graph read and
  mutation. Never read, search, or edit `.grove/state.lock` or
  `.grove/journal.log` directly.
- Never edit `.grove/index.md`. It is a generated projection.
- `.grove/glossary.md` is the only project-state file edited directly. Use
  `grove glossary rename` for term renames so discovery tags remain consistent.
- Do not initialize or bootstrap a project again when `.grove/state.lock`
  already exists. Discovery can still continue as part of normal work.
- Do not start work when Definition of Ready fails. Resolve the reported
  condition through discovery instead of weakening the record to pass a guard.
- Record evidence before marking work done. Never describe unexecuted checks as
  completed evidence.
- Stop and ask the user before acting on a `chaotic` node.
- Keep decisions, questions, assumptions, acceptance criteria, and evidence in
  their graph records, not in parallel planning documents.
- Do not reduce a plan to goals and work when unresolved unknowns need `Q`,
  falsifiable beliefs need `B`, or durable choices need `D`.
- Preserve terminal records. Create a successor instead of rewriting history.

## Command map

Use the CLI form below or its one-to-one MCP equivalent.

```bash
grove status
grove next
grove packet W-12
grove dor W-12
grove show Q-03
grove list q --status=open
grove add w --type=feature --cynefin=clear --goals=G-01 --title="Add sign-in"
grove field W-12 ac add "User can sign in with valid credentials."
grove field W-12 evidence_strategy add "Run the authentication test suite."
grove link Q-03 asks W-12
grove set W-12 status=progress
grove fitness W-12 G-01 +1
grove evidence W-12 "Authentication suite passed; commit abc123."
grove set W-12 status=done
grove check
```

`grove next` selects ready work and returns its execution packet. Use
`grove packet W-NN` when resuming a specific item or when the packet must be
refreshed. Use `grove show`, `list`, `deps`, `impact`, and `path` for targeted
questions. Do not reconstruct graph state through lockfile reads.

## Start or resume a session

### Existing project

1. Run `grove status`.
2. Surface stale or foreign progress claims before selecting new work. Resolve
   each deliberately with `resume`, `revert`, or `handoff`. Ask the user when
   ownership or preservation of interrupted work is unclear.
3. Run `grove next`. If no work is ready, inspect the returned blockers and
   continue Discovery rather than forcing a transition.
4. Read the execution packet and inspect only the additional records required by
   the current uncertainty or acceptance criterion.
5. Ask for alignment when `status` reports a trigger, when the proposed work is
   `complex`, or when the user must choose between materially different
   directions. Otherwise proceed without an approval ritual.

### New project, only after explicit activation

1. Run `grove init`.
2. Elicit outcomes, the fitness model, and scope boundaries with the user.
3. Create Areas first, then Goals linked to those Areas.
4. For each Goal, begin with Discovery. Represent refactor causes with `T` and
   link them to refactor work with `causes`; do not treat an existing codebase
   as a reason to skip unknowns or acceptance criteria.
5. Record questions, assumptions, decisions, and work as separate node types.
   Populate readiness fields while creating the graph instead of deferring all
   of them until execution.

## Choose the operating mode

Classify every `W`, `Q`, and `B` with Cynefin before acting on it.

| Cynefin       | Agent behavior                                                                     |
| ------------- | ---------------------------------------------------------------------------------- |
| `clear`       | Execute against explicit acceptance criteria. Do not invent a spike.               |
| `complicated` | Investigate the relevant material, make the reasoning explicit, then execute.      |
| `complex`     | Run a spike or bounded investigation with exit criteria before production changes. |
| `chaotic`     | Stop and ask the user. Do not experiment or change production state.               |

A complex feature may require active-discovery coverage when its goal (`G`) or
theme (`T`) declares it. Coverage does not replace a hypothesis, evidence
strategy, or any other readiness condition.

## Discovery loop

1. Before opening a question (`Q`), run `grove list q --status=open` and check
   for an existing record with the same scope. Extend the existing record
   instead of creating duplicates. Record contradictions explicitly and request
   alignment when they change the direction of work.
2. Give every question (`Q`) useful context and exit criteria. Investigate only
   what can reduce that uncertainty, then record the outcome.
3. Create an assumption (`B`) when an answer is a falsifiable belief affecting a
   feature's existence, scope, or value. Define its validation method and
   threshold before testing it. Link it with `tests` and `targets` as
   applicable.
4. Create a spike for a complex unknown that needs bounded investigation. Its
   deliverable is a produced `D`, `Q`, or `B`, not production code.
5. Record durable choices as decisions (`D`) with context, options, rationale,
   and consequences. Link implementing work with `implements`.
6. When a question (`Q`) closes, preserve the resulting decision (`D`),
   assumption (`B`), work (`W`), or reason for dropping it. Terminal questions
   (`Q`) remain closed.
7. When an assumption (`B`) becomes `validated` or `invalidated_acceptable`,
   recheck readiness for dependent work (`W`). When it becomes
   `invalidated_blocking`, revise or reject affected work (`W`) and request
   alignment. Never relabel an assumption (`B`) merely to pass readiness.
8. Revisit discovery whenever delivery exposes a surprise, a failed readiness
   condition, or a missing decision.

## Delivery loop

1. Select Work with `grove next`, or use an explicitly assigned `W-NN`.
2. Run `grove dor W-NN`. Resolve every failed conjunct before progress.
3. Refresh the execution packet and confirm that acceptance criteria and the
   evidence strategy describe the requested outcome.
4. Enter `progress`. The command applies readiness, predecessor, WIP, and
   session guards.
5. Perform the work and collect the evidence promised by the strategy. Open new
   graph records for newly discovered uncertainty instead of hiding it in prose.
6. Stage a signed fitness delta for every linked Goal. Use zero only for genuine
   enabling work and explain the indirect contribution in `why`.
7. Record concrete results with `grove evidence`. Include commands, outputs,
   relevant artifact paths, and commit identifiers when available.
8. Mark Work `done` only after every acceptance criterion is satisfied and all
   required deltas and produced records are present.
9. When a Goal becomes verified, distill durable learning into Discoveries or
   record a null distillation. A `--distill-deferred` note can defer the
   reminder, but it does not replace the distillation required before archive.
10. Check alignment triggers, then select the next item or stop for the user.

## Evidence standard

The protocol checks structural completion conditions. The agent is responsible
for semantic sufficiency: evidence must demonstrate every acceptance criterion
on the actual deliverable, not merely show that a module parses or imports.
Follow the host project's existing test and verification conventions.

### Feature

- Prefer dynamic tests that exercise the changed behavior and record runner
  results.
- When dynamic verification is not applicable, combine the strongest available
  static checks, interface-contract traces, and end-to-end exercise. Explain why
  that combination demonstrates the criterion.
- A successful build alone is insufficient evidence of behavior.

### Bug

- Preserve a reproducible failure before the fix when the defect can be tested.
- Run the same reproduction after the fix and run adjacent regression checks.
- If a pre-fix automated reproduction is impossible, record the concrete reason
  and the alternative before-and-after evidence in `why` or `evidence`.

### Refactor

- Record relevant behavior checks before and after the structural change.
- Show that public behavior and contracts are preserved, including exported API
  surface when applicable.
- Resolve or verify the symptom represented by the causing theme (`T`) when it
  has an observable symptom.
- If existing checks cannot establish behavior preservation, create and finish
  predecessor Work that adds the necessary checks, linked with `blocks`.

### Spike

- Link every produced `D`, `Q`, or `B` with `produces`.
- Answer each exit criterion in evidence and reference the produced record.
- Do not merge throwaway implementation into production. Preserve only useful
  research artifacts and identify them in evidence when the user wants them
  kept.

Statements such as "looks correct", self-assessment without an executed check,
or unrelated green tests do not constitute sufficient evidence.

## Graph and record discipline

- Keep the graph synchronized with actual outcomes, not intended outcomes.
- Give Work explicit goals `G`, acceptance criteria, evidence strategy, and
  type-specific fields before progress. Declare surface when it supports
  scoping, coverage, or later auditing.
- Use `blocks` only for causal or required predecessor clearance. Do not encode
  arbitrary ordering as false dependencies.
- Use `causes`, `implements`, `asks`, `tests`, `targets`, `produces`,
  `supersedes`, and `distills` for their defined meanings, not as generic links.
- Replace an accepted decision (`D`) by creating its successor with
  `--supersedes` and transitioning the old decision (`D`) to `superseded`. Do
  not silently rewrite the old choice.
- Preserve reasons for rejected Work and dropped Questions. If circumstances
  change, create a new record with current context.
- Add new domain terms to `.grove/glossary.md` when they enter the project
  vocabulary. Keep glossary terms lowercase and join words with single hyphens.
- Activate a discovery (`Y`) only with provenance, a fresh anchor, and glossary
  tags. Revalidate stale discoveries (`Y`) instead of treating them as current
  coverage.
- Promote a discovery (`Y`) to copy it into another project with provenance.
  Never create live references between independent project locks.

## Interrupted and parallel work

Session ownership protects a progress item, while file locking protects storage.
They solve different problems and neither replaces the other.

- Use `resume` to adopt an interrupted claim after inspecting its current state.
- Use `revert` to return claimed work to ready when it should be replanned.
- Use `handoff` to transfer ownership to another known session token.
- Use disjoint ID stride and offset lanes for independent worktrees when branch
  allocation could collide.
- Never mutate another session's progress item by changing tokens merely to
  bypass ownership.

## Merge and history recovery

Direct lockfile editing is not a workflow. Conflict resolution is the narrow
exception permitted so that `repair` can recanonicalize deliberately combined
state.

1. Resolve textual conflicts without inventing timestamps or manually repairing
   a checksum. Preserve both independent records when the grammar permits.
2. Run `grove repair --confirm` after the combined content has been inspected.
3. Run `grove check` and resolve structural violations through Grove commands.
4. Use `renumber` for ID collisions. If evidence prevents renumbering, preserve
   that history and resolve the collision without falsifying evidence.
5. Regenerate the index and include the resulting state in the intended commit.

Use `undo` for recent reversible mutations. It is not a substitute for version
control, and archive, repair, and null distillation have their own lifecycle
rules.

## Typography

Apply these rules to skill content, glossary content, titles, and prose fields.
Serialized identifiers, status values, commands, and grammar-defined values keep
their protocol spelling.

1. Write user-facing and stored documentation in English.
2. Use sentence case. Start titles and phrases with a capital letter.
3. End prose bullets, numbered items, and checklist entries with a full stop.
4. Do not use em dashes. Use hyphens for compound words and en dashes only as
   empty table-cell placeholders.
5. Keep arrows in code, formal notation, and Mermaid, not in prose.
6. Do not add phase or stage prefixes to titles. Encode required ordering with
   `blocks` edges.
7. Use a short noun or gerund phrase for titles. Put scope qualifiers in a final
   parenthesized comma-separated list when needed.
8. Keep prose factual and verifiable. Prefer artifact paths and executed results
   over unsupported claims.

## End-of-session checklist

1. Run `grove render` if generated output may be stale, then run `grove check`.
2. Confirm that no progress claim is abandoned without an intentional handoff,
   resume, or revert decision.
3. Confirm that open questions (`Q`) have exit criteria and unresolved
   assumptions (`B`) have validation methods and thresholds.
4. Confirm that done work (`W`) has semantic evidence for all acceptance
   criteria and fitness deltas for all linked goals (`G`).
5. Confirm that verified goals (`G`) have discoveries (`D`), null distillation,
   or an explicit `--distill-deferred` note.
6. Confirm that new domain terms are in the glossary and active discoveries
   (`D`) have current anchors and tags.
7. Confirm that rejection and dropping reasons are recorded, and that no
   terminal record was reopened or rewritten.
8. Report unresolved guards, stale claims, and missing verification plainly.
