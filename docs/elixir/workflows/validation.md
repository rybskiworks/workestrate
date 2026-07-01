# Validation Workflow

## Purpose

Step-by-step procedure for validating Elixir code before merge, release, or handoff. The agent runs the full gate suite and reports pass/fail status for each gate with specific failure details.

## What happens first

1. Understand what needs validating: is this a pre-merge check, a release check, or a handoff check? Release checks may add dependency auditing and release builds.
2. Confirm the repo's recorded policy for each gate (strict mode, coverage threshold, Dialyzer gating). Read `CONTRIBUTING.md` and the relevant topic doc's "Policy decisions for individual repos" section.
3. Ensure the working tree is clean and dependencies are resolved (`mix deps.get`).

## Context gathering

- Read the relevant docs (see "Docs consulted" below) for what each gate checks and how to interpret its output.
- Load the relevant skills (see "Skills loaded" below) for the actionable procedures behind each gate.

## Docs consulted

- `testing-exunit.md` — test configuration, `async: true`, coverage, tag policy.
- `typespecs-and-dialyzer.md` — Dialyzer flags, PLT configuration, warning categories.
- `static-analysis-credo.md` — Credo categories, strict mode, exit-code bitmask.
- `mix-project-structure.md` — compilation, releases, application/0, dependency declaration.
- `../../beam/validation.md` — shared BEAM runtime validation hooks (sys, trace, introspection, ETS/timer/NIF checks).

## Skills loaded

- `elixir-testing` — ExUnit configuration, assertion quality, coverage.
- `elixir-static-analysis` — Credo and Dialyzer interpretation and configuration.

## Checks run (full suite)

Run in order; each gate must pass before the next is meaningful, but run all gates and report every failure:

```sh
mix format --check-formatted
mix compile --warnings-as-errors
mix credo --strict
mix dialyzer
mix test --cover
```

For a release validation, additionally:

```sh
mix deps.audit        # if hex_audit is available
mix release           # or the repo's release command
```

## Validation gates

| Gate | Pass criterion |
|---|---|
| Format | `mix format --check-formatted` exits 0; no unformatted files. |
| Compilation | `mix compile --warnings-as-errors` exits 0; no warnings. |
| Credo | `mix credo --strict` (or repo-configured mode) exits 0; no issues. |
| Dialyzer | `mix dialyzer` exits 0; no warnings (or only repo-acknowledged ones). |
| Tests | `mix test` exits 0; all tests pass. |
| Coverage | `mix test --cover` meets the repo's threshold (if set). |

## Evidence reported

For each gate, report pass/fail with specifics:

- **Format**: clean, or list the unformatted files.
- **Compilation**: clean, or the specific warnings/errors with file:line.
- **Credo**: clean, or issue count by category with file:line.
- **Dialyzer**: clean, or each warning with file:line and category.
- **Tests**: pass/fail count; for each failure, the test name and assertion message.
- **Coverage**: percentage and whether it meets the repo threshold.

End with a one-line summary: `VALIDATION: PASS` or `VALIDATION: FAIL` with the list of failing gates.

## When human judgment is needed

- **Acceptable coverage level**: the corpus does not mandate a threshold; the repo decides. If coverage is below the repo threshold, a human decides whether to add tests or accept the gap.
- **Acceptable lint exceptions**: a justified `# credo:disable-for-next-line` or a Dialyzer `@dialyzer` attribute requires human sign-off.
- **Dialyzer as gate vs advisory**: if the repo marks Dialyzer advisory, report warnings but do not block on them.
- **Release scope**: whether to run `mix release`, dependency audit, or security scans is a repo/release decision.

## Related docs

- `implementation.md`, `code-review.md`, `refactoring.md`, `debugging.md` — workflows whose output is validated here.
- `../testing-exunit.md`, `../typespecs-and-dialyzer.md`, `../static-analysis-credo.md` — the primary validation reference docs.
