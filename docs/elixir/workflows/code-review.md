# Code Review Workflow

## Purpose

Step-by-step procedure for reviewing an Elixir diff or PR against the corpus checklists and tooling gates. The agent reads the change, loads review checklists from the docs, runs the checks, and reports findings categorized by severity.

## What happens first

1. Understand the change intent: read the PR description, linked issue, or commit message. What is the change trying to accomplish?
2. Identify the blast radius: which modules, tests, and config files are touched?
3. Determine whether the change is implementation, refactor, or fix — this selects which sibling workflow's concerns also apply.

## Context gathering

- Read the full diff before commenting on any line.
- Load the "Review checklist" sections from the relevant topic docs (see "Docs consulted" below).
- Check the repo's `CONTRIBUTING.md` for repo-specific gates (strict mode, coverage threshold, typespec requirements).

## Docs consulted

All docs with "Review checklist" sections are relevant; prioritize by the changed code's domain:

- `naming-conventions.md` — casing, `foo`/`foo!` pairs, module/atom naming.
- `language-fundamentals.md` — pattern matching, pipe usage, control flow.
- `core-modules.md` — `Enum`/`Stream` choice, collection type choice.
- `otp-supervision.md` — child specs, restart strategies, process naming (if OTP code).
- `testing-exunit.md` — test quality, `async: true`, assertions, coverage.
- `typespecs-and-dialyzer.md` — `@spec` presence and correctness, `@type t`.
- `../../beam/supervision.md` — child spec, restart strategy, intensity/period review (when diff touches OTP).
- `../../beam/gen-server.md` — `gen_server` callback contract review.
- `../../beam/common-mistakes.md` — BEAM/Erlang performance/correctness/security pitfalls to flag.
- `../../beam/validation.md` — runtime validation hooks to confirm before merge.

## Skills loaded

- `elixir-coding` — naming and function-shape checks.
- `elixir-static-analysis` — Credo and Dialyzer interpretation.
- `elixir-otp` — only when the diff touches OTP code.

## Checks run

```sh
mix format --check-formatted
mix credo
mix dialyzer
mix test
```

Run `mix dialyzer` even if the repo marks it advisory — its output informs the review. If the repo runs Credo in `--strict`, match that.

## Review focus areas

- **Naming**: snake_case for functions/variables, CamelCase for modules, `?`/`!` suffixes used correctly.
- **Pattern matching**: prefer pattern matching over `case`/`cond` where it clarifies; match on tagged tuples for error returns.
- **OTP patterns**: correct child spec, appropriate restart strategy, no `Process.spawn` where a supervised process belongs.
- **Error handling**: error tuples at boundaries, bang functions only where failure is truly exceptional, no swallowed errors.
- **Test quality**: meaningful assertions, `async: true` where safe, no shared mutable state, tests cover the failure path.
- **Typespecs**: `@spec` on public functions (per repo policy), `@type t` where a module owns a type.
- **Documentation**: `@moduledoc` and `@doc` on public modules/functions; no `false` moduledocs without reason.

## Evidence reported

Findings categorized by severity:

- **Blocking**: correctness bugs, swallowed errors, missing tests for new behavior, broken OTP patterns.
- **Should fix**: naming violations, missing typespecs (per repo policy), weak assertions, missing docs.
- **Optional**: style nits, minor refactors, non-blocking suggestions.

Include the tool output (Credo issues, Dialyzer warnings, test failures) verbatim with file:line references.

## When human judgment is needed

- **Design decisions**: whether a new abstraction is warranted, where a boundary should sit.
- **Tradeoffs**: `Enum` vs `Stream`, error-tuple vs exception style, sync vs async — the corpus leaves these to repo policy.
- **Acceptable exceptions**: a justified `# credo:disable-for-next-line` or a deliberate typespec omission.

## Review sequence

Follow this order so structural issues are caught before cosmetic ones:

1. **Intent**: does the diff do what its description claims?
2. **Correctness**: are the logic and error paths right? Are OTP patterns sound?
3. **Tests**: do tests exist for the new behavior, including the failure path?
4. **Typespecs and docs**: are `@spec`, `@moduledoc`, `@doc` present per repo policy?
5. **Naming and style**: snake_case, `?`/`!` suffixes, pipe usage.
6. **Tooling**: format, Credo, Dialyzer, tests all green.

## Anti-patterns to flag

- **Swallowed errors**: `{:error, _}` matched and discarded without logging or re-raising.
- **Unsupervised processes**: `spawn/1` used where a supervised `Task` or `GenServer` belongs.
- **Weak assertions**: `assert result` where `assert result == expected` is needed.
- **Missing `async: true`**: tests that could run concurrently are serialized without reason.
- **Bang without non-bang**: a `foo!` with no `foo` counterpart, where the repo policy requires the pair.

## Related docs

- `implementation.md` — the workflow that produced the code under review.
- `validation.md` — the full gate suite, run before merge.
- `../static-analysis-credo.md`, `../typespecs-and-dialyzer.md` — tooling reference.
