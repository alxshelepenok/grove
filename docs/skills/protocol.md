# 2. Workflow protocol

Two loops run concurrently, not as phases. Either may run in any session. See diagram in [diagrams/dual-track.md](diagrams/dual-track.md) and the top-level flow in [diagrams/workflow.md](diagrams/workflow.md).

## 2.1 Cynefin-driven mode selection

Before doing anything on a node, the agent classifies it.

```text
cynefin(n) = clear        → execute directly; no spike, no Q needed.
cynefin(n) = complicated  → read code or docs, write plan, execute.
cynefin(n) = complex      → spike with explicit exit criteria; outcome ∈ {D, Q, B}.
                            Production code allowed only AFTER spike closes.
cynefin(n) = chaotic      → STOP; escalate to user; do not write code.
```

## 2.2 Session start protocol

**If `.grove/state.lock` exists:**

1. Run `grove status`. If the previous session left a W in `progress` owned by a stale session token (see §2.6), surface it first; do not advance to `grove next` until the user resolves it.
2. Run `grove next`. The CLI returns either an execution packet for a single proposed `W-NN` (chosen from `Ready ∩ critical_path`) or a structured "no-ready" diagnostic.
3. Confirm with the user only if any of: (a) alignment trigger from §2.5 is live, (b) `grove next` returned a fallback outside the critical path, (c) the proposed W's cynefin = `complex`. Otherwise proceed silently.
4. Do not re-run discovery if state exists.

**If `.grove/state.lock` does not exist:**

1. Run `grove init`. This creates `.grove/state.lock`, `.grove/index.md`, and `.grove/glossary.md`.
2. Ask the user for top-level outcome(s) and their scope partition. Create areas via `grove add a --title=…`, then `G-NN` rows via `grove add g --title=… --area=A-NN --fitness-kind=… --fitness-target=…`.
3. For each G, decide track:
   - **greenfield**: start the Discovery loop (impact map, then Q / B / D / W).
   - **existing code**: start a refactor scan (creates `T-NN` and `W:refactor` items).

The agent never reads `state.lock` directly. All reads go through `grove` ([cli.md](cli.md)).

## 2.3 Discovery loop

1. Open `Q-NN` with cynefin tag and exit criteria via `grove add q`.
2. If `complicated`, investigate via reads only; write outcome via `grove field Q-NN outcome add "…"`.
3. If the unknown affects whether a `feature` W should exist or how it should be scoped, open `B-NN` with a falsifiable assumption, validation method, and acceptance threshold.
4. If `complex`, open W with `type=spike`; produce D, Q, or B only.
5. When Q closes, either new B (answer needs validation), new D (a choice was made), new W (action follows), or `dropped` with reason.
6. When B closes as `validated` or `invalidated_acceptable`, run `grove dor` on every dependent W. When B closes as `invalidated_blocking`, revise or reject the dependent W.

## 2.4 Delivery loop

1. `grove next` to pick `w ∈ Ready`.
2. Verify `grove dor W-NN` reports ⊤. If not, return to Discovery on the missing conjunct. Never override DoR silently.
3. `grove packet W-NN` returns the self-contained execution packet (W body + linked D + B + Q.outcome + DoR breakdown). This is the only context the agent needs.
4. `grove set W-NN status=progress` (claims the session token, see §2.6).
5. Implement; collect evidence per strategy.
6. Stage fitness deltas: `grove fitness W-NN G-NN <delta>` for every linked goal (use `0` for enabling work and add a `why` note).
7. `grove evidence W-NN "…"` records evidence.
8. `grove set W-NN status=done`. The CLI atomically (I₁₀): verifies I₃ evidence, applies the staged fitness deltas, re-derives `status(g)` for each linked goal, derives `status(t)` for the theme (I₆), and auto-runs `grove render`. If any sub-step fails, the whole transition is rolled back.
9. `status(g)` re-derivation:
   - `verified` iff fitness function satisfied.
   - `partial` iff progress changed but threshold not met.
   - `unverified` iff no measured progress.
   - `declined` only by explicit user decision.
10. If a Goal becomes `verified`, the CLI prints a **distillation hint** (lazy distill; suppress with a `notes` line containing `--distill-deferred`). Distillation is mandatory before archiving, but the artifact is not: run `grove distill G-NN` for the worksheet (validated B / answered Q / accepted D in the goal's mass, each with a `grove add y --from=…` skeleton) and create Discoveries for what is worth keeping, or attest the empty case with `grove distill G-NN --null`. `grove archive G-NN` refuses until ≥1 Discovery is provenance-linked into the goal's exclusive mass **or** a null-distill attestation exists for the goal.

## 2.5 Triggers for user alignment

Stop ONLY when one of these holds:

```text
align ⟺
  (∃ q ∈ Q, cynefin(q) = chaotic) ∨
  (∃ b ∈ B, status(b) = invalidated_blocking) ∨
  (∃ w ∈ W, status(w) = done ∧ significant(w)) ∨
  (∃ g ∈ G, status(g) = verified) ∨
  (Ready = ∅ ∧ ((∃ q ∈ Q, status(q) = open) ∨ (∃ b ∈ B, status(b) ∈ { proposed, testing })))

where significant(w) ⟺
  (∃ d ∈ D, (w, implements, d) ∈ E ∧ status(d) = accepted) ∨
  (w lies on the current critical path) ∨
  (type(w) = refactor) ∨
  (type(w) = spike ∧ (cynefin(w) = complex ∨ |produces(w)| ≥ 1 with new D))
```

Trivial spikes (cynefin=complicated, no new D) do not trigger checkpoints.

## 2.6 Session tokens and interrupted work

Every `grove set W-NN status=progress` records a session token as the `session`
header attr and stamps `session_at` (RFC3339 UTC). The default token is
deterministic from the worktree path and host env (`COMPUTERNAME`/`HOSTNAME`/`HOST`);
override with `GROVE_SESSION` or each CLI's `--session=<token>`.
Subsequent mutations of that W require the same effective token.

`grove resume`, `grove handoff --to=…`, and `grove revert` adjust the claim (see
HELP). Undo restores prior `session`/`session_at` snapshots from the journal.

When `grove status` finds a `progress` W with a stale token (different session,
or session marker `> 24h`), it reports:

```text
W-12 progress (stale: session abc123, last touch 2d ago)
options:
  grove resume W-12       -- adopt the token in this session
  grove revert W-12       -- back to ready, discards progress notes
  grove handoff W-12 --to=<token>   -- transfer ownership
```

The agent MUST surface this before running `grove next`. Two sessions cannot
hold `progress` on the same W; the CLI rejects concurrent claims.

## 2.7 Checkpoint template

```text
Checkpoint. Reason: [trigger].
Done since last checkpoint: [W-NN, …].
Open: [Q-NN, …]; Proposed decisions: [D-NN, …].

Next options:
  1. [Most logical action].
  2. [Alternative].
  3. Your call.
```
