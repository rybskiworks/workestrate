# Validation Workflow

## Purpose

Step-by-step procedure for validating Gleam code before merge, release, or handoff. The agent runs the full gate suite and reports pass/fail status for each gate with specific failure details.

## What happens first

1. Understand what needs validating: is this a pre-merge check, a release check, or a handoff check? Release checks may add package-interface export and SBOM steps.
2. Confirm the repo's recorded policy for each gate (target coverage, format enforcement, package-interface export, SBOM). Read `CONTRIBUTING.md` and the relevant topic doc's "Policy decisions for individual repos" section.
3. Ensure the working tree is clean and dependencies are resolved (`gleam deps` or `gleam run` to fetch).

## Context gathering

- Read the relevant docs (see "Docs consulted" below) for what each gate checks and how to interpret its output.
- Load the relevant skills (see "Skills loaded" below) for the actionable procedures behind each gate.

## Docs consulted

- `validation.md` — the validation gate suite, per-target validation, the static-type guarantee.
- `testing.md` — Gleeam test configuration, test module structure, per-target test runs.
- `project-structure-and-cli.md` — `gleam.toml`, target declaration, module layout.
- `package-management-and-publishing.md` — package-interface export, `gleam publish`, Hex publishing, SBOM.

## Skills loaded

- `gleam-packages-ffi` — package interface, dependency, and publish procedures.

## Checks run (full suite)

Run in order; each gate must pass before the next is meaningful, but run all gates and report every failure:

```sh
gleam format --check
gleam check
gleam test
```

For multi-target projects, validate each target the repo supports:

```sh
gleam test --target erlang
gleam test --target javascript
```

For a release validation, additionally:

```sh
gleam build
gleam docs build      # package-interface export
gleam publish         # only at release time, after sign-off
```

## Validation gates

| Gate | Pass criterion |
|---|---|
| Format | `gleam format --check` exits 0; no unformatted files. |
| Type check | `gleam check` exits 0; no errors. (Strict and non-configurable; no Dialyzer step.) |
| Tests | `gleam test` exits 0; all tests pass. |
| Per-target tests | For each supported target, `gleam test --target <t>` exits 0 (if repo policy requires). |
| Build | `gleam build` exits 0 for each supported target. |
| Package interface | `gleam docs build` succeeds; package interface exports cleanly (release gate). |
| Publish | `gleam publish` succeeds (release only, after human sign-off). |

## The static-type guarantee

Gleam is statically typed with whole-program type inference at `gleam check`. There is **no Dialyzer step** and no separate type-warning category: if `gleam check` passes, the program is type-safe. This means the validation suite is shorter than Elixir's (no Credo, no Dialyzer), but per-target runtime validation (`gleam test` per target) becomes more important because the type system does not catch target-specific runtime differences.

## Runtime/debugging hooks

- `gleam check` is the compile-time guarantee; runtime defects still require `gleam test` and, on the Erlang target, BEAM debugging (see BEAM bridge in `index.md` and `debugging.md`).
- Package-interface export (`gleam docs build`) surfaces documentation and public-surface issues that `gleam check` does not.

## Evidence reported

For each gate, report pass/fail with specifics:

- **Format**: clean, or list the unformatted files.
- **Type check**: clean, or the specific errors with file:line.
- **Tests**: pass/fail count; for each failure, the test name and assertion message.
- **Per-target tests**: per-target pass/fail.
- **Build**: per-target build status.
- **Package interface**: export succeeded or the specific errors.
- **Publish**: succeeded or the specific failure (release only).

End with a one-line summary: `VALIDATION: PASS` or `VALIDATION: FAIL` with the list of failing gates.

## Common mistakes

- **Skipping per-target test runs**: code that type-checks may fail at runtime on one target due to FFI or library differences.
- **Treating `gleam check` as sufficient**: it guarantees type safety, not runtime correctness or target compatibility.
- **Publishing without sign-off**: `gleam publish` is irreversible; require human sign-off.
- **Forgetting package-interface export at release**: `gleam docs build` catches public-surface issues that should be resolved before publish.

## When to escalate to a human policy decision

- **Acceptable target coverage**: whether both targets must pass, or only the project's primary target, is a repo decision.
- **Package-interface export as a gate**: whether `gleam docs build` gates CI is a repo decision.
- **SBOM / supply chain**: whether a dependency audit or SBOM is produced at release time is a repo/release decision.
- **Publish scope**: whether to run `gleam publish` and which version bump is required is a release decision requiring human sign-off.

## Related docs

- `implementation.md`, `code-review.md`, `refactoring.md`, `debugging.md` — workflows whose output is validated here.
- `../validation.md`, `../testing.md`, `../package-management-and-publishing.md` — the primary validation reference docs.
