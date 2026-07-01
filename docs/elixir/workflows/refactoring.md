# Refactoring Workflow

## Purpose

Step-by-step procedure for refactoring Elixir code while preserving external behavior. The agent establishes a green test baseline, makes incremental changes, runs tests after each step, and reports the before/after evidence.

## What happens first

1. Understand what needs refactoring and why: readability, performance, OTP correctness, removing duplication, or preparing for a future change.
2. Confirm the change is a refactor, not a behavior change. If behavior must change, switch to the implementation workflow and document the behavior delta.
3. Identify the safety net: does the code have tests? If not, write characterization tests before refactoring.

## Context gathering

- Read the code to refactor in full, including its callers.
- Read its test module: what does the test suite assert about the current behavior?
- Load the relevant docs (see "Docs consulted" below) for the patterns the refactor targets.

## Docs consulted

- `core-modules.md` — `Enum`/`Stream` choice, collection type choice, common collection refactorings.
- `otp-supervision.md` — supervision tree restructuring, child spec changes, restart strategy adjustments.
- `language-fundamentals.md` — pattern matching, pipe operator, control-flow simplification.
- `../../beam/supervision.md` — supervision tree restructuring, child spec, restart strategy semantics.
- `../../beam/proc-lib-and-sys.md` — special-process contract (when refactoring hand-written processes).

## Skills loaded

- `elixir-coding` — naming and function-shape conventions for the refactored code.
- `elixir-static-analysis` — to verify the refactor introduces no new Credo/Dialyzer issues.

## Checks run

Run the test suite **before** starting (establish baseline) and **after** each step:

```sh
mix test                      # baseline, then after each step
mix credo                     # no new issues introduced
mix dialyzer                  # no new type warnings introduced
```

If the baseline is red, stop and fix the baseline first. Do not refactor on top of a failing suite.

## Process

1. **Establish baseline**: `mix test` is green; record the pass count.
2. **Plan the steps**: decompose the refactor into the smallest sequence of behavior-preserving steps. Each step should be independently committable.
3. **Make one change at a time**: a single extraction, rename, or collection swap per step.
4. **Run tests after each step**: `mix test`. If red, revert the step and reconsider.
5. **Run Credo and Dialyzer after the final step**: confirm no regressions in static analysis.
6. **Preserve behavior**: public function signatures, return types, and side effects must be unchanged unless the task explicitly permits otherwise.

## Evidence reported

- Diff summary: what was extracted, renamed, or restructured.
- Test results before: pass count and any pre-existing failures.
- Test results after: pass count, confirming no regressions.
- Credo/Dialyzer status before and after: no new issues.

## When human judgment is needed

- **Behavior change is unavoidable**: when the refactor surfaces a latent bug or the old behavior was wrong. Stop, switch to the implementation workflow, and document the behavior delta explicitly.
- **Refactoring spans multiple modules**: when the change crosses a boundary (e.g., moving logic from a controller to a context), confirm the boundary decision with a human before proceeding.
- **Performance-driven refactors**: `Enum` → `Stream` or introducing `Task.async_stream` changes memory/time characteristics; confirm the tradeoff is intended.
- **OTP tree changes**: restructuring a supervision tree changes failure semantics; review restart strategy and child ordering with a human.

## Common refactoring patterns

- **Extract function**: move a block into a named function when it appears more than once or has a distinct responsibility. Name it per `naming-conventions.md`.
- **`Enum` → `Stream`**: when a pipeline materializes large intermediate collections, switch to `Stream` and verify memory behavior. See `core-modules.md`.
- **Consolidate clauses**: replace nested `case`/`cond` with multi-clause function heads using pattern matching. See `language-fundamentals.md`.
- **Restructure supervision**: move a child under a different supervisor or change restart strategy only after confirming the failure semantics; see `otp-supervision.md`.
- **Introduce `@type t`**: when a module owns a primary data structure, add `@type t` and update callers; see `typespecs-and-dialyzer.md`.

## Anti-patterns to avoid

- **Big-bang refactors**: a single large diff is hard to review and hard to revert. Decompose.
- **Refactoring on red**: never refactor while the test suite is failing; you lose the safety net.
- **Behavior drift**: silently changing a return type or side effect during a "refactor" is a behavior change, not a refactor.
- **Renaming public API without a deprecation path**: in libraries, rename via deprecation, not deletion.

## Related docs

- `implementation.md` — when the refactor surfaces a needed behavior change.
- `validation.md` — the full gate suite, run before merge.
- `../core-modules.md`, `../otp-supervision.md` — the primary refactoring reference docs.
