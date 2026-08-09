<p align="center">
  <picture>
    <source type="image/webp" srcset=".github/assets/hero.webp" />
    <img src=".github/assets/hero.png" alt="Grove" />
  </picture>
</p>

***G**raph-driven **R**easoning **O**ver **V**erified **E**vidence. A formal workflow protocol that keeps AI coding agents on track through machine-enforced invariants, verified evidence, and structured context. Designed to keep deep, long-running projects coherent across sessions, agents, and months.*

<p align="center">
  <a href="https://notebook.google.com/notebook/434f3efc-c199-4b7e-ac61-92fbd85d655e"><img src="https://img.shields.io/badge/Notebook-q/a-8A2BE2?style=for-the-badge" alt="Notebook" /></a>
  <a href="docs/install.md#verify-before-you-run"><img src="https://img.shields.io/badge/signing-RSA--PSS-8A2BE2?style=for-the-badge" alt="Release signing" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/open%20source-AGPL_3.0-8A2BE2?style=for-the-badge" alt="AGPL 3.0 license" /></a>
  <a href="https://github.com/alxshelepenok/grove/actions/workflows/rust-tests.yml"><img src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Falxshelepenok%2Fgrove%2Fbadges%2F.github%2Fbadges%2Ftests.json&style=for-the-badge" alt="Tests" /></a>
  <a href="packages/grove/conformance"><img src="https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Falxshelepenok%2Fgrove%2Fbadges%2F.github%2Fbadges%2Fconformance.json&style=for-the-badge" alt="Conformance" /></a>
</p>

Grove enforces rules as deterministic invariants stored in a single checksummed lockfile. The agent cannot declare work done without falsifiable evidence, start unready tasks, or hallucinate progress.

Instead of lossy prompt compression or summarization, Grove structures project state into a typed reasoning graph with machine-checkable edges. It routes exactly the execution packet needed for the current step, and nothing more.

Works with Claude Code, Codex, Gemini CLI, Cursor, Windsurf, Cline, GitHub Copilot, and any other agent. Grove includes a CLI, a built-in MCP server, and a drop-in agent skill bundle so you can integrate it into your workspace immediately.

<p align="center">
  <picture>
    <source type="image/webp" srcset=".github/assets/overview.webp" />
    <img src=".github/assets/overview.png" alt="Grove" />
  </picture>
</p>

## Agents lose context as projects grow

Long-running agents suffer from context amnesia. Decisions, assumptions, and dependencies drift out of view as the session grows. A related failure is **unreliable self-reporting**: the agent declares work complete without sufficient evidence. This happens not out of deception, but because nothing in the environment prevents it.

Standard task trackers inherently trust the executor. When a human checks a box in Jira, the work is assumed done. That is a reasonable default for humans. For autonomous agents it is a silent failure mode. Premature "done" declarations compound across long sessions into structural errors, until the codebase no longer matches what anyone believes it to be, and nobody can say why a given line exists.

This is not theoretical. Grove evolved from a crude Markdown hypothesis into a protocol that today manages Merlin Guild, a closed-source production project of 200k+ lines of Rust and TypeScript, heavy on blockchain and cryptography. It scales because the protocol does not rely on the agent remembering or obeying the workflow correctly.

## Structure instead of compression

The obvious response to context amnesia is more tooling: summarization, compaction, retrieval. All of these compress context, and compression loses information while hoping the loss does not matter.

**Grove does not compress context. It structures it.**

As a project evolves, its work is continuously organized into areas, goals, questions, assumptions, decisions, and executable work items. This hierarchy gives the agent a way to identify the context relevant to the current step instead of repeatedly carrying the entire project history into its context window.

The result is not just better continuity; it also reduces token usage. The agent no longer loads unrelated project context simply to recover where it is and why the current task exists.

## One project state, multiple interfaces

> The unit of memory is not the conversation. It is the project.  
> The unit of progress is not the agent's claim. It is verified evidence.

Grove replaces polite prompt instructions with a strict, mechanically enforced protocol. Agents interact with project state through the CLI, MCP, and desktop interfaces. The protocol itself enforces the rules:

- A work item cannot be marked done without an evidence record.
- Work cannot start until every precondition is machine-verified.
- Goal progress cannot be updated by hand; fitness deltas are applied atomically at close time or not at all.

The state either satisfies the protocol, or Grove refuses to advance it. An agent cannot advance protocol state by merely claiming that work is complete.

## Give agents only the context they need

Grove forces the agent to decompose the product into a strict typed hierarchy. Instead of a raw list of tasks, every piece of planning context lives explicitly in a node taxonomy. Nothing hides in side documents or the agent's internal state:

| Node | ID | What it holds |
| --- | --- | --- |
| **Area** | `A-NN` | Permanent scope skeleton above goals; never archived. |
| **Goal** | `G-NN` | Outcome with a measurable fitness function. |
| **Theme** | `T-NN` | Optional grouping of related work items. |
| **Work item** | `W-NN` | Executable unit with Definition of Ready + Definition of Done. |
| **Decision** | `D-NN` | ADR: immutable once accepted, superseded only with recorded rationale. |
| **Question** | `Q-NN` | Open unknown, declared instead of pretended away. |
| **Assumption** | `B-NN` | Falsifiable hypothesis with a validation method and result. |
| **Discovery** | `Y-NN` | Reusable, evidence-backed knowledge distilled from finished work. |

Typed edges connect these nodes into a graph, with blocks remaining acyclic. This structure lets Grove answer both **why does this task exist?** and **what breaks if I change it?** without relying on the agent's internal state.

Once this graph exists, the routing promised above becomes mechanical. `grove next` picks the current step; `grove packet` emits exactly its context. Nothing irrelevant enters the context window.

Before touching a line of code, the agent queries the **causality cone**. `grove packet W-NN --cone` maps the backward cone (everything that must finish first, in topological contraction order), the forward cone (the blast radius if this item changes), and a fragility score per affected goal.

<p align="center">
  <picture>
    <source type="image/webp" srcset=".github/assets/diagram.webp" />
    <img src=".github/assets/diagram.png" alt="The Grove loop: on the left, the Discovery track chains Questions into Assumptions into Discoveries; on the right, the Project holds areas, each chaining Goals into Work into Evidence. Dotted edges close the loop: questions are asked against goals, assumptions target work, discoveries guide goals, and evidence distills back into discoveries." />
  </picture>
</p>

## Turn reasoning into reusable knowledge

Grove introduces a unified methodology for AI-driven software development, informed by Dual-Track Agile, Hypothesis-Driven Development, ADRs, Continuous Discovery, Cynefin, and the Mikado method. These influences are integrated into a single workflow designed around the constraints of autonomous LLM agents and enforced through machine-checkable invariants.

Most AI workflows are tiny waterfalls: specify everything, then build everything. Grove runs discovery and delivery in parallel, each track feeding the other:

- **Discovery** takes open unknowns and operationalizes them. A question becomes a falsifiable assumption; validated outcomes become curated discoveries.
- **Delivery** executes ready work items and writes verified code on top of those assumptions.

The joints are mechanical. Questions are asked against goals; assumptions target work and gate it; discoveries guide the next goals; finished work distills back into discoveries. When a test falsifies an assumption, the plan reshapes at once; dependent work cannot proceed on a broken foundation.

### Knowledge has a lifecycle

Grove does not treat distilled knowledge as permanent truth. Discoveries can become stale as the project evolves; reactivating one requires a fresh anchor. Distillation debt is tracked rather than silently accumulating, while goal fitness is re-derived on every close.

The dashboard reflects actual protocol state after every mutation, not intended state. Grove assumes that a project keeps moving: knowledge changes, assumptions are invalidated, and unfinished reasoning accumulates. The protocol makes those changes visible instead of letting them disappear into project history.

### Ideas behind the protocol

- Discovery and Delivery run in parallel (Dual-Track Agile, Cagan). A work item cannot enter Delivery until every open question and unvalidated assumption that blocks it is resolved in Discovery.
- Every executable unit has explicit acceptance criteria before code is written (HDD, Definition of Ready). The DoR is not a checklist anyone can override; it is a boolean conjunction the CLI evaluates on every `status=progress` transition.
- Long-lived design choices are first-class artifacts (ADR, Nygard). Decisions are immutable once accepted. They cannot be quietly revised; they can only be superseded by a new decision with a recorded rationale.
- Open unknowns are first-class artifacts; agents declare them rather than pretend to know (Continuous Discovery; Cynefin). A question tagged `chaotic` halts the agent and requires human resolution.
- Assumptions are falsifiable gates, not comments. An assumption in state `invalidated_blocking` prevents any dependent work item from becoming ready. The agent cannot proceed by ignoring it.
- Refactoring uses a Mikado-style dependency graph distinguishing causation, sequencing, implementation, and inquiry. This makes the blast radius of a change explicit before the first line is touched.
- Verified goals archive only after distillation: their validated assumptions, answered questions, and accepted decisions become Discoveries (Y), curated domain axioms that are never archived and that feed future packets.
- Areas (A) are a permanent scope skeleton above goals. Every goal belongs to exactly one area, enforced by the CLI at creation (I₁₃); areas are never archived, so the structure outlives any single goal.

## Beyond spec-driven development

Spec-driven development makes specifications explicit. Grove goes one step further: it makes the development process itself executable and verifiable.

A specification describes what the system should do. Grove additionally models the state of the work around it: what is proven, what is assumed, what is blocked, what is ready, what is done, and on what evidence. These are not instructions for the agent to follow; they are protocol state that gates what the agent is allowed to do next.

This changes the workflow from document-driven execution to state-driven execution. There is no single specification to regenerate the project from and no fixed waterfall per feature. Discovery and delivery run continuously, and when an assumption is falsified, the graph and the execution plan change
with it. Every state transition is mechanically gated rather than relying on the agent to follow the process correctly.

Specifications remain useful. In Grove, they can live as decisions, acceptance criteria, and other structured project knowledge attached to the work. What changes is what enforces them: the specification describes the intended outcome; the protocol determines whether the project is allowed to advance.

## Let evidence decide when work is done

A task cannot even start until a Definition of Ready passes. This is a strict boolean conjunction evaluated by the protocol on every `status=progress` transition, not a checklist anyone can override. Once ready, Grove emits an execution packet: exactly the context the step needs (the work item, its acceptance criteria, its open questions, its assumption chain, and the decisions that constrain it) and nothing more.

Closing requires **falsifiable** evidence: test outputs, commit hashes, build logs. A self-reported `finished` is not accepted. The transition to `done` is **atomic**; goal fitness deltas and status re-derivation are applied in the same write, or not at all.

## Versioned reasoning, not just versioned code

All state lives in `.grove/state.lock`, a single line-oriented text file with a SHA-256 checksum rewritten on every mutation. Any manual edit, rogue script, or bad merge is detected on the next protocol operation, and all state transitions are blocked until the file is repaired.

The lockfile can live **inside** the project repository, so the history of project reasoning evolves alongside the code, or **outside** it in a separate directory or repository. Both topologies are supported. When committed to version control, the lockfile's history becomes the history of the project's reasoning, not just its code.

### Merge conflicts and race conditions

A typical race occurs when two branches advance project state independently:

1. Branch `A` merges into `main` and advances the project state.
2. Branch `B` was created earlier and still contains the previous state checksum.
3. Merging the branches produces a checksum mismatch.

Grove detects the divergence instead of silently accepting inconsistent state.

Resolution is mechanical:

1. Resolve textual conflicts, keeping records from both sides.
2. Run `grove repair --confirm` to re-canonicalize and re-checksum the state.
3. Run `grove check` to surface any invariant violations.
4. Run `grove renumber` if ID collisions occurred between branches.

### Parallel work

For parallel worktrees, `grove init --id-stride/--id-offset` allocates disjoint ID ranges so collisions do not happen in the first place.

On a single machine, concurrent mutations are protected by an exclusive `flock`, while claimed work is protected by session tokens.

Multi-machine writes to a single lockfile are out of scope by design. Remote state transitions must be routed through a single canonical writer, such as a primary CI agent.

## Continue when the agent disappears

A work item in progress carries an exclusive session token (only the session that claimed it can mutate it). If a provider goes down mid-goal, or a tough refactor calls for a different model, another agent on another client takes over via `grove handoff`. Alternatively, it adopts the session via `grove resume`.

All reasoning and proof live in the lockfile, so `grove next` and `grove packet` rebuild the working context instantly. The invariants keep the new agent honest; the rest of the progress cannot be faked.

The real test of agent-driven development is exactly this: when the agent stops mid-goal, the next one picks the project up without reconstructing intent from chat logs.

## Preserve why the work exists

The result is not only continuity for agents. It is explainability for the people who own the project.

You can answer why a piece of code exists, what it depends on, what assumptions it rests on, and what evidence justified it months after the original work was completed.

## Getting started

Grove is designed to be installed once and then used as the persistent workflow layer for your project.

The [installation guide](docs/install.md) covers the CLI, MCP server, desktop application, and agent skill bundle. Then run `grove init` in your project root to create the checksummed state file. Connect the MCP server, add the [signed skill](docs/install.md#agent-skill-bundle), and bootstrap your first session with a [prompt template](docs/prompts.md). From then on, `grove next` drives every session.

Not sure if Grove fits your workflow? The [Gemini Notebook](https://notebook.google.com/notebook/434f3efc-c199-4b7e-ac61-92fbd85d655e) holds the full documentation. Start with a simple question, like *"How does Grove stop an agent from marking work done without evidence?"* or *"When is Grove the wrong tool for my project?"*

## Where it fits

Grove is most useful when work is long-running, reasoning-heavy, and performed by autonomous agents.

**Security and research workflows**: security work stretches over months and runs on hypotheses: most leads die, some become critical paths. Grove fits this shape natively. Every closed item carries evidence, so the audit trail is the project itself - the append-only journal, immutable decisions, and evidence-bound closes answer "who concluded what, when, and on which basis" without a separate reporting process. Priorities stop being a feeling: the critical path, per-goal fitness, and DoR gates decide what runs next, formally.

**Architecture and compliance work**: assumptions in Grove carry a validation method and a result; decisions carry rationale; discoveries anchor invariants to concrete surfaces. "Do we still meet our SLOs" becomes a question the state answers through goal fitness metrics, and "why is it built this way" stays answerable months later - each architectural choice points to the questions, assumptions, and evidence that produced it.

**Long refactors and multi-session features**: the Mikado-style dependency graph and the causality cone make blast radius explicit before the first edit, and session continuity lets the work survive agent and provider changes.

Grove is usually the wrong tool for short-lived prototypes, one-prompt tasks, and projects where the code will not outlive the session.

Grove is not a task manager, code-context tool, or multi-agent orchestrator. It is a protocol for maintaining verified project state for autonomous agents.

## Protocol invariants

```text
I₁:  ∀ w ∈ W with status = progress, DoR(w) ≡ ⊤.
I₂:  ∀ w with type = spike ∧ status = done,
      produces(w) ⊆ D ∪ Q ∪ B ∪ Y  ∧  produces(w) ≠ ∅.
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
terminal⁺(g ∈ G) ⟺ status(g) = verified (strict for blocks edges)
terminal(g ∈ G)  ⟺ status(g) ∈ { verified, declined }
terminal(d ∈ D)  ⟺ status(d) ∈ { accepted, rejected, superseded }
terminal(q ∈ Q)  ⟺ status(q) ∈ { answered, deferred, dropped }
terminal(b ∈ B)  ⟺ status(b) ∈ { validated, invalidated_acceptable, invalidated_blocking }
terminal(t ∈ T)  ⟺ status(t) = done
terminal(y ∈ Y)  ⟺ status(y) = superseded
terminal(a ∈ A)  ⟺ ⊥ (areas have no lifecycle)
```

`terminal⁺` is the strict variant used for `blocks` edges: a `declined` goal does not unblock dependents. Other relations use the lax `terminal`.

```text
assumptions(w) ≜ { b ∈ B | (b, targets, w) ∈ E }
BChain(w)      ≜ assumptions(w) ∪ { b ∈ B | ∃ q, (q, asks, w) ∈ E ∧ (b, tests, q) ∈ E }
produces(w)    ≜ { n ∈ D ∪ Q ∪ B ∪ Y | (w, produces, n) ∈ E }
goals(w)       ≜ as recorded in `goals` field of w
WI(t)          ≜ { w ∈ W | theme(w) = t }
```

## FAQ

**How does the state file work?**

All state lives in `.grove/state.lock`, a single line-oriented text file with a SHA-256 checksum on every write. Any manual edit is detected immediately on the next protocol operation, and all state transitions are blocked until the file is repaired. The agent never reads or writes the file directly; it interacts only through Grove interfaces (CLI, MCP).

This design makes the entire workflow auditable and diff-friendly. Every transition is a single atomic write. The lockfile can be committed to version control; its history is the history of the project's reasoning, not just its code.

**How does an agent work with Grove efficiently? Does it need to read the whole skill and write essays into the lock?**

No on both counts. The skill's `index.md` is the minimal safe contract - one screen, complete for operation; every other page is depth you open only when the task touches its topic. And when a command's shape is unclear, the CLI itself is the instructor: refusals like `add g: --area is required` or `DoR ≢ ⊤; see grove dor W-NN` say exactly what is missing. Even an agent that never opened the skill cannot corrupt state, because the invariants (DoR gates, evidence gates, the checksum) are enforced by the protocol, not by the document - partial reading degrades process quality, never integrity.

Writing works by compression, not by transcription. The agent deliberates in its own context for as long as it needs, then stores only the conclusions: one acceptance criterion per line, one sentence per hypothesis, a compact context/options block on a decision node. Dozens of small CLI calls are normal and cheap - they batch into a single shell invocation per node (`add` + fields + `fitness`). What never belongs in the lock is the reasoning itself: if a fact does not change what a future agent does, it is not recorded. And `grove next` / `grove packet` exist precisely so the agent never re-reads the state file to plan.

## License

GNU Affero General Public License v3.0 (AGPL-3.0). Copyright (c) 2026 Alex Shelepenok. Free to use, study, modify, and redistribute under the license terms, including network use: offering Grove as a network service requires offering its source. See [LICENSE](LICENSE) for the full text.
