---
name: grove
description: Apply this skill if and only if (a) `.grove/` exists in the project root, or (b) the user explicitly asks to "use grove", "init grove", or names a grove command. Do NOT apply on inferred non-triviality, do NOT auto-init on greenfield tasks without explicit request. Covers greenfield features, refactoring, bug investigation, and spikes once activated.
---

# Graph-driven reasoning over verified evidence

A dual-track, evidence-based workflow for AI agents. State lives in a single line-oriented lock file; the agent reads only what the current step demands. Designed to let weak agents go deep without hallucinating.

Core ideas (analogues in brackets):

- Discovery and Delivery run in parallel [Dual-Track Agile, Cagan].
- Every executable unit has explicit acceptance criteria before code is written [HDD, Definition of Ready].
- Long-lived design choices are first-class artifacts [ADR, Nygard].
- Open unknowns are first-class artifacts; agents declare them rather than pretend to know [Continuous Discovery; Cynefin].
- Refactoring uses a Mikado-style dependency graph distinguishing causation, sequencing, implementation, and inquiry.

## State lives in `.grove/state.lock`

Files inside `.grove/`:

- `state.lock`: the SINGLE source of truth. Line-oriented DSL. Carries a SHA-256 checksum. **Never edit this file by hand.** Any manual edit is detected on the next CLI call and blocks all work until `grove repair --confirm`.
- `index.md`: auto-generated dashboard plus mermaid graph. Regenerated on every mutate command. Manual edits are overwritten.
- `glossary.md`: the only file the agent edits directly (domain terms).
- `journal.log`: append-only JSON-lines journal. Mutation records carry the inverse ops behind `grove undo` and `grove stats`; gate, distill, undo, and DoR-refusal records are non-mutations. `grove undo` truncates the lines it reverts.

Per-node `w-NN.md`, `d-NN.md`, `q-NN.md`, `b-NN.md` files do NOT exist in this skill. All node bodies (acceptance criteria, hypotheses, evidence, ADR context, investigation logs) live as prose fields inside `state.lock`.

## User-level store

`~/.grove/` (overridable via `GROVE_HOME`) holds the project registry `projects.toml` (name, path, created, last_opened; human-editable, not checksummed) and, by convention, one independent subdirectory per project, each a normal grove project with its own lock. Locks never intersect: there are no live cross-lock references. Discoveries move between projects only by copy-with-provenance via `grove promote Y-NN --to=<project>`; each copy starts its own staleness lifecycle in the target. Target a project with `--project=<dir|name>` or the `GROVE_PROJECT` environment variable; list the registry with `grove projects`.

## All access is through the `grove` CLI

Use the `grove` CLI, or the MCP server `grove-mcp` over stdio, which exposes the same commands one-to-one as `mcp__grove__*` tools. The server serves the project containing the current directory (nearest ancestor with `.grove/state.lock`); pass `--root=<dir>` only to serve a different project. The cheat sheet below assumes the CLI.

### Cheat sheet

```bash
grove init                              # bootstrap .grove/
grove next                              # propose the next W (Ready ∩ critical_path)
grove packet W-12                       # full execution context for W-12
grove add w --type=feature --cynefin=clear --goals=G-01 --title="…"
grove field W-12 ac add "User can sign in."
grove link Q-03 asks W-12
grove set W-12 status=progress          # guarded by DoR, WIP, blocks
grove evidence W-12 "tests green; abc123"
grove fitness W-12 G-01 +1
grove set W-12 status=done              # guarded by I₃
grove dor W-12                          # conjunct breakdown
grove path                              # critical path
grove check                             # all invariants; use in pre-commit
```

`grove next` returns the same content as `grove packet <ID>`, a self-contained markdown bundle covering the W, every linked decision, every `BChain` assumption, and the outcome of every blocking question. That bundle is the only context an agent needs.

## Reading order

This page is the minimal safe contract; the pages below are depth, opened when the task touches their topic (see [planning](planning.md) §6). Full versions:

**Must-read on activation:**

1. [Formal model](model.md): nodes, edges, statuses, invariants I₁..I₁₃, DoR.
2. [Protocol](protocol.md): workflow, cynefin gating, session start, discovery / delivery loops, alignment triggers.
3. [Planning](planning.md): planning context lives in the lock, full node vocabulary, required fields, goal fitness.
4. [CLI](cli.md): full CLI reference.
5. [Evidence](evidence.md): DoD per work-item type.
6. [Rules](rules.md): operational rules, merge protocol, pre-commit hook.
7. [Lockfile](lockfile.md): grammar; needed only for tooling outside the CLI.
8. [Typography](typography.md): formatting rules for all prose fields, entity titles, and other text content. Must be followed consistently.
9. [Checklist](checklist.md): end-of-session quality gate.
10. [Diagrams](diagrams/): mermaid for dual-track, top-level workflow, palette.

## Hard constraints

- Never read or write `state.lock` directly. Always go through `grove`.
- Never bypass `grove dor`. If DoR is `⊥`, return to the Discovery loop.
- Never mark `done` without recording evidence via `grove evidence`.
- Never re-run discovery if `state.lock` already exists.
- If `cynefin = chaotic` on any node you touch, stop and escalate.
- Never put planning context into side markdown files; decisions, questions, and assumptions are D/Q/B nodes ([planning](planning.md)).
- Never ship a plan of only G and W nodes; surface unknowns as Q/B and choices as D first.
- Never leave a goal without `fitness_kind` + `fitness_target` or a work item without full DoR fields at creation.
