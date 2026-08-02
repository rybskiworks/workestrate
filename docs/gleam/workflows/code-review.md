# Code Review Workflow

## Purpose

Step-by-step procedure for reviewing a Gleam diff or PR against the corpus checklists and tooling gates. The agent reads the change, loads review checklists from the docs, runs the checks, and reports findings categorized by severity.

## What happens first

1. Understand the change intent: read the PR description, linked issue, or commit message. What is the change trying to accomplish?
2. Identify the blast radius: which modules, tests, and `gleam.toml` entries are touched?
3. Determine whether the change is implementation, refactor, or fix — this selects which sibling workflow's concerns also apply.
4. Identify the target(s): a change touching externals/FFI may behave differently on Erlang vs JavaScript.

## Context gathering

- Read the full diff before commenting on any line.
- Load the "Review checklist" sections from the relevant topic docs (see "Docs consulted" below).
- Check the repo's `CONTRIBUTING.md` for repo-specific gates (target coverage, format enforcement, package-interface export).

## Docs consulted

All docs with review checklists are relevant; prioritize by the changed code's domain:

- `conventions-patterns-antipatterns.md` — naming, type design, anti-patterns to flag.
- `result-option-and-errors.md` — `Result`/`Option` usage, panic vs return, error propagation.
- `externals-and-ffi.md` — external function declarations, FFI safety, target-specific bodies.
- `types-records-and-patterns.md` — custom types, records, pattern matching, making invalid states impossible.

## Skills loaded

- `gleam-language` — naming and function-shape checks, type design review.
- `gleam-otp-interop` — only when the diff touches OTP actors, supervision, or Erlang interop.

## Checks run

```sh
gleam format --check
gleam check
gleam test
```

For multi-target projects, run `gleam test` per target if the repo policy requires it. `gleam check` is always strict and non-configurable; there is no separate lint step.

## Review focus areas

- **Naming**: snake_case for functions/variables, type constructors and module names per `conventions-patterns-antipatterns.md`.
- **Type design**: custom types that make invalid states impossible; no overly broad `Result(a, b)` where a narrower type clarifies intent. See `types-records-and-patterns.md`.
- **Result/Option usage**: errors returned as `Result`, not panicked, except at truly exceptional boundaries; no swallowed `Error` values. See `result-option-and-errors.md`.
- **Externals/FFI safety**: external function declarations are correct per target; FFI bodies are isolated and reviewed for unsafe Erlang/JS. See `externals-and-ffi.md`.
- **Target-specific risks**: code that compiles on both targets but behaves differently (e.g., integer semantics, FFI availability).
- **Type annotations**: public functions annotated; internals annotated where it aids clarity.
- **Pattern matching**: exhaustive matches preferred over catch-all; custom types used where they enforce invariants.
- **Documentation**: module and function docs on public surface.

## Evidence reported

Findings categorized by severity:

- **Blocking**: correctness bugs, swallowed errors, unsafe FFI, missing tests for new behavior, broken type invariants.
- **Should fix**: naming violations, missing annotations on public functions, weak assertions, missing docs, non-exhaustive matches.
- **Optional**: style nits, minor refactors, non-blocking suggestions.

Include the tool output (`gleam check` errors, test failures) verbatim with file:line references.

## Runtime/debugging hooks

- `gleam check` catches type errors and most correctness issues statically; review should still reason about runtime behavior on each target.
- On the **Erlang target**, FFI into Erlang NIFs or raw process operations warrants consulting `docs/beam/` (see BEAM bridge in `index.md`).

## Common mistakes

- **Flagging style that the type system already enforces**: Gleam's type system removes whole classes of issues (null, missing cases); focus review on what the type system cannot catch.
- **Missing target-specific review**: externals/FFI changes must be reviewed for both targets when both are supported.
- **Ignoring error propagation**: a `Result` returned and then discarded loses the error path.

## When to escalate to a human policy decision

- **Design decisions**: whether a new abstraction or type variant is warranted, where a module boundary should sit.
- **Tradeoffs**: `Result` vs panic style, target support scope — the corpus leaves these to repo policy.
- **Acceptable FFI exceptions**: a deliberate unsafe external requires human sign-off.

## Related docs

- `implementation.md` — the workflow that produced the code under review.
- `validation.md` — the full gate suite, run before merge.
- `../conventions-patterns-antipatterns.md`, `../externals-and-ffi.md` — the primary review reference docs.
