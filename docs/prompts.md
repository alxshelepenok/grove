# Prompts

Self-contained prompts for driving a project through Grove. Paste one into a fresh agent session. Each assumes `.grove/state.lock` exists in the project, the agent has either the `mcp__grove__*` tools or the `grove` CLI on PATH, and the Grove skill is loaded. Values written into the state (titles, evidence, decision texts) should stay in English, ASCII.

## 1. Bootstrap a new project

When: right after `grove init`, before the first work session.

```text
You are working in <PROJECT DIR>. A grove state file has just been
initialized (.grove/state.lock) and is empty. Bootstrap the project graph
for the product described below.

<2-5 sentences describing the product>

Rules:
- Use only grove commands (or mcp__grove__* tools); never edit .grove/
  files by hand.
- Decompose the product into 3-6 permanent Areas (A). Areas are scope,
  not time: they outlive individual goals.
- Under each area, create the first Goals (G) with a structured
  fitness_kind (count | ratio | boolean | metric) and an explicit
  fitness_target. A goal without a fitness target is a planning error.
- For each goal, create the initial Work items (W) with a full
  Definition of Ready: acceptance criteria (one per line), hypothesis,
  and evidence_strategy. Stage fitness deltas toward their goals.
- Record what you do not know as Questions (Q) and Assumptions (B)
  instead of guessing; tag each with a Cynefin class.
- Do not create a node for everything: if the product description does
  not justify an area or a goal yet, leave it out and record a Q instead.
Finish with `grove check` and a summary of the graph you created.
```

## 2. Work the queue

When: everyday session start; the graph already exists.

```text
You are working in <PROJECT DIR> with grove available (mcp__grove__* tools
or the grove CLI on PATH). This is a work session.

1. Run `grove next` and read the execution packet it proposes
   (`grove packet <W-NN> --cone` if you need the causal neighborhood).
2. Work on exactly that item. Do not pick a different one without
   telling me why.
3. Respect the gates: if the CLI refuses a transition (DoR, evidence,
   session ownership), fix the cause - never work around a refusal.
4. Close the item only with real evidence: what was run, what passed,
   what changed, with file paths and command outputs.
5. If you discover new unknowns or make a design choice mid-work, record
   them as Q / B / D nodes as you go, not as prose in the chat.
6. End with `grove check` and a one-paragraph status summary.
```

## 3. Plan a feature as a goal

When: you have a feature in mind; turn it into a tracked goal instead of a todo list.

```text
You are working in <PROJECT DIR> with grove available. Turn the feature
below into a grove goal with a complete work breakdown. All state goes
into grove; do not write plan markdown files.

Feature: <DESCRIPTION>

Rules:
- One goal (G) under the most fitting area, fitness_kind + fitness_target
  set deliberately (n/a only as an explicit fitness_kind=manual decision).
- Work items (W) with full DoR per item: ac lines, hypothesis,
  evidence_strategy, type, cynefin. Stage fitness deltas.
- Questions (Q) for unknowns, Assumptions (B) for falsifiable beliefs
  that gate work, Decisions (D) for choices already made.
- Wire the graph: blocks edges for real prerequisites, asks/tests/targets
  where they belong. The work order must be derivable from the graph.
- Verify with `grove check`, then show me `grove ready` and the critical
  path (`grove path`).
Do not start implementing; this session is planning only.
```

## 4. Audit a plan or an implementation

When: independent review of a design doc, a goal, or merged code; findings become grove nodes.

```text
You are working in <PROJECT DIR> with grove available. Audit <SUBJECT:
doc path / goal ID / diff range> against the current grove state and the
codebase. Be adversarial: look for factual errors, stale assumptions,
missing failure modes, and claims without evidence.

Rules:
- Verify every load-bearing claim against the actual files before
  reporting it.
- Record findings as grove nodes: Q for open questions, D (proposed) for
  decisions the audit implies, and one Discovery (Y) summarizing the
  audit, anchored with --surface=<files> and glossary tags.
- Do not fix anything; this is a read-only audit. Your output is the
  grove nodes plus a findings summary ordered by severity.
```

## 5. Take over another session's work

When: a provider or client switch, or a new agent continuing an in-progress goal.

```text
You are working in <PROJECT DIR> with grove available. A previous agent
session was interrupted mid-work. Pick up where it left off.

1. Run `grove status` and `grove list w --status=progress` to find
   claimed work; for each item, read `grove packet <W-NN>` fully.
2. Adopt stale claims with `grove resume <W-NN>`; if a claim is fresh
   (<24h), do not touch that item and tell me.
3. Read the evidence trail (`grove log <W-NN>`) before changing
   anything. Continue from the recorded state, not from scratch.
4. If the recorded state contradicts the repository, stop and report the
   contradiction instead of reconciling silently.
```

## 6. Close and archive a goal

When: a goal is verified and its lessons should be distilled before archiving.

```text
You are working in <PROJECT DIR> with grove available. Close out goal
<G-NN>.

1. Confirm every linked work item is terminal (`grove show <G-NN>`,
   `grove list w`).
2. Run `grove distill <G-NN>` and work through the worksheet: for each
   real candidate, create a Discovery (add y --from=<ID>) with glossary
   tags and surfaces. Use `grove distill <G-NN> --null` only when there
   is genuinely nothing to distill.
3. Archive with `grove archive <G-NN>`; verify shared nodes survived
   (they must stay active) and exclusive nodes archived.
4. Finish with `grove check`.
```
