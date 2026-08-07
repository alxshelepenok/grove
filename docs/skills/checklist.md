# 9. Quality checklist

Before ending a session, run `grove check`. It enforces:

- [ ] Lock checksum is valid (no manual edits).
- [ ] Every `done` W has a non-empty `evidence` field (I₃).
- [ ] Every `done` W has fitness deltas applied to each linked G (I₁₀, atomic).
- [ ] Every `progress` W carries a `session` token; `grove status` surfaces stale claims (I₁₁).
- [ ] Every B linked to a `feature` W is `validated` or `invalidated_acceptable` before that W is `ready` (I₉).
- [ ] `WIP count ≤ WIP_LIMIT` (I₄).
- [ ] No DoR violations on `progress` items (I₁).
- [ ] `blocks` graph is a DAG (I₇).
- [ ] No orphan edges (every endpoint exists).
- [ ] Every goal carries an `area` field referencing an existing `A-NN` (I₁₃).

Manual items the CLI cannot check:

- [ ] `index.md` is in sync with the lock (rerun `grove render` if stale).
- [ ] Every Q with `status = open` has cynefin tag and exit criteria.
- [ ] If a Goal is verified, distillation is done (a Discovery linked into the goal's mass OR a null-distill attestation) OR a `--distill-deferred` note is present in the goal's `notes` field (lazy distill policy, rules.md).
- [ ] New domain terms added to `glossary.md`.
- [ ] Typography ([Typography](typography.md)) respected in prose fields.
- [ ] Rejection reasons recorded for `rejected` / `dropped` nodes.
- [ ] No planning context leaked into side markdown files; decisions, questions, and assumptions are D/Q/B nodes ([Planning](planning.md)).
- [ ] Plans are not G+W-only: blocking unknowns have Q nodes, unverified beliefs have B nodes, long-lived choices have D nodes.
- [ ] Every goal created this session has `fitness_kind` + `fitness_target` (no accidental `n/a`); every work item created this session has full DoR fields.
