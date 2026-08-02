# Refactoring Workflow

## Purpose

Step-by-step procedure for refactoring Gleam code while preserving external behavior. The agent establishes a green `gleam test` baseline, makes incremental changes, runs tests after each step, and reports the before/after evidence.

## What happens first

1. Understand what needs refactoring and why: readability, type design, removing duplication, consolidating externals, or preparing for a future change.
2. Confirm the change is a refactor, not a behavior change. If behavior must change, switch to the implementation workflow and document the behavior delta.
3. Identify the safety net: does the code have tests? If not, write characterization tests before refactoring.

## Context gathering

- Read the code to refactor in full, including its callers.
- Read its test module: what does the test suite assert about the current behavior?
- Load the relevant docs (see "Docs consulted" below) for the patterns the refactor targets.

## Docs consulted

- `stdlib.md` — collection choice (`list`, `dict`, `set`), stdlib functions, avoiding reinventing what the stdlib provides.
- `result-option-and-errors.md` — `Result`/`Option` restructuring, error propagation improvements.
- `conventions-patterns-antipatterns.md` — naming, type design, anti-patterns to remove during refactor.

## Skills loaded

- `gleam-language` — naming and function-shape conventions for the refactored code.

## Checks run

Run the test suite **before** starting (establish baseline) and **after** each step:

```sh
gleam test                  # baseline, then after each step
gleam check                 # type-check after each step
gleam format --check        # format clean at the end
```

If the baseline is red, stop and fix the baseline first. Do not refactor on top of a failing suite. For multi-target projects, run `gleam test` per target if the repo policy requires it.

## Process

1. **Establish baseline**: `gleam test` is green; record the pass count.
2. **Plan the steps**: decompose the refactor into the smallest sequence of behavior-preserving steps. Each step should be independently committable.
3. **Make one change at a time**: a single extraction, rename, collection swap, or type redesign per step.
4. **Run `gleam check` after each step**: the type system catches most regressions immediately.
5. **Run `gleam test` after each step**: if red, revert the step and reconsider.
6. **Run `gleam format --check` after the final step**: confirm formatting is clean.
7. **Preserve behavior**: public function signatures, return types, and side effects must be unchanged unless the task explicitly permits otherwise.

## Runtime/debugging hooks

- Refactoring in Gleam is unusually safe because the static type system localizes the effect of changes; `gleam check` after each step is the primary safety net.
- On the **Erlang target**, refactors that touch actor message shapes or supervision structure require runtime verification via `gleam test` and possibly BEAM debugging (see BEAM bridge in `index.md`).

## Evidence reported

- Diff summary: what was extracted, renamed, or restructured.
- `gleam test` results before: pass count and any pre-existing failures.
- `gleam test` results after: pass count, confirming no regressions.
- `gleam check` status before and after: no new errors.
- `gleam format --check` status.

## Common mistakes

- **Big-bang refactors**: a single large diff is hard to review and hard to revert. Decompose.
- **Refactoring on red**: never refactor while the test suite is failing; you lose the safety net.
- **Behavior drift**: silently changing a return type or side effect during a "refactor" is a behavior change, not a refactor.
- **Renaming public API without a deprecation path**: in libraries, rename via deprecation, not deletion.
- **Skipping `gleam check` between steps**: the type system is the fastest regression catcher; use it.

## When to escalate to a human policy decision

- **Behavior change is unavoidable**: when the refactor surfaces a latent bug or the old behavior was wrong. Stop, switch to the implementation workflow, and document the behavior delta explicitly.
- **Refactoring spans multiple modules**: when the change crosses a boundary, confirm the boundary decision with a human before proceeding.
- **Externals consolidation**: merging or splitting external/FFI declarations changes target-specific risk; review with a human.
- **Making-invalid-states-impossible redesign**: narrowing a type changes the public surface; confirm the tradeoff is intended.

## Related docs

- `implementation.md` — when the refactor surfaces a needed behavior change.
- `validation.md` — the full gate suite, run before merge.
- `../stdlib.md`, `../result-option-and-errors.md` — the primary refactoring reference docs.
