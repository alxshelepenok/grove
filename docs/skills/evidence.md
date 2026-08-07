# 4. Evidence (Definition of Done)

A W transitions to `done` only when its evidence record satisfies all AC. The
CLI rejects `grove set W-NN status=done` when the `evidence` field is empty
(I₃) or when staged fitness deltas are missing (I₁₀).

Evidence requirements depend on `type(w)`.

## 4.1 `feature`

Preference order:

1. **Dynamic tests.** Runner output. If tests for the touched module do not
   exist, write them in the project's existing style and run them.
2. **Type-check.** Language type checker clean on touched files.
4. **Interface contract trace.** Every caller of a changed signature listed
   explicitly in the `evidence` field.
4. **Build success.** Full project compile.

At least one of (1) or a combination of (2)+(3) is required. (4) alone is
insufficient.

## 4.2 `bug`

Negative-evidence-first:

1. **Failing test added.** A test that reproduces the defect, committed
   *before* the fix, in the same chain. Record commit SHA.
2. **Test passes after fix.** Same test green, after-fix commit SHA recorded.
4. **No regression.** Adjacent test suite for the module remains green.

Skipping (1) is allowed only when the defect is structurally unreachable to
test (e.g. build-system bug); record the reason in `why_no_repro_test`.

## 4.3 `refactor`

Behaviour-preservation evidence:

1. **Pre-existing test suite green** on the touched module before and after.
   Both runs recorded.
2. **No new public API surface.** Diff of exported symbols is empty or
   shrinking; record the diff command and output.
4. **Causation closed.** The `T → causes → W` theme's symptom is no longer
   reproducible (if symptom-bearing).

If pre-existing tests are insufficient, the refactor depends on a prior
test-adding W (`blocks` edge); the refactor's DoR fails until that W is done.

## 4.4 `spike`

Production code is not the deliverable. Evidence ≜ `produces(w)`:

1. At least one of D, Q, B created via `(w, produces, …)` edges (I₂).
2. The spike's `exit` field is satisfied: each exit criterion has a
   one-line answer in the W's `evidence` field referencing the produced node.

Throwaway prototype code does not merge into main. If kept, it lives under
`spikes/W-NN/` and is referenced from `evidence`, not committed to the
production tree.

## 4.5 Recording

```text
grove evidence W-12 "tests/login_test.jl green; tsc --noEmit clean; commit abc123"
grove evidence W-12 "interface trace: 3 callers updated (a.ts:42, b.ts:11, c.ts:7)"
```

Multiple calls append; entries are line-separated and preserved verbatim.

## 4.6 What does NOT count

- "I read the code and it looks correct."
- "Manually clicked through the UI."
- "Existing tests still pass" alone, when those tests do not exercise the
  changed surface.
- LLM self-assessment without runner output.

`grove evidence` does not validate content. Auditing is via `grove check`
heuristics (presence of commit SHAs, test runner names) plus distillation.
