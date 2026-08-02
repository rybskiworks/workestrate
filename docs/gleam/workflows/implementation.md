# Implementation Workflow

## Purpose

Step-by-step procedure for writing new Gleam code: a new feature, module, or function. The agent gathers context, follows corpus conventions, runs the standard checks, and reports evidence. This workflow orchestrates; it does not redefine Gleam semantics.

## What happens first

1. Understand the requirement in concrete terms: what input, what output, what side effects, what failure modes.
2. Check existing patterns in the target repo: search for a sibling module or function that already solves a similar problem. Prefer consistency with local conventions over personal preference.
3. Confirm scope: is this a single function, a new module, an external/FFI boundary, or an OTP actor? The scope determines which docs and skills apply.
4. Confirm the target: Erlang, JavaScript, or both. Target choice affects available libraries, FFI, and runtime semantics; see `gleam-toml-and-targets.md`.

## Context gathering

- Read the relevant `docs/gleam/` files (see "Docs consulted" below) for the conventions that govern the code being written.
- Load the relevant skills (see "Skills loaded" below) so the actionable procedures are in context.
- Inspect the target module's existing types, function signatures, and test module to match local style.

## Docs consulted

- `language-fundamentals.md` — variables, immutability, blocks, control flow, labels.
- `types-records-and-patterns.md` — custom types, records, pattern matching, making invalid states impossible.
- `functions-pipelines-and-use.md` — function definitions, the `|>` pipe, `use`, labelled arguments.
- `result-option-and-errors.md` — `Result`/`Option`, error handling strategy, when to panic vs return.
- `project-structure-and-cli.md` — where files live, `gleam.toml`, module naming, the `src`/`test` split.
- `conventions-patterns-antipatterns.md` — naming, snake_case, type design, common anti-patterns.

## Skills loaded

- `gleam-language` — naming, type design, function shape, documentation.
- `gleam-packages-ffi` — when adding modules, `gleam.toml` entries, dependencies, or external/FFI boundaries.

## Checks run

Run these in order; stop and fix before proceeding when a check fails:

```sh
gleam format --check
gleam check
gleam test
```

If the repo records a different policy (e.g., per-target test runs), defer to the repo's `CONTRIBUTING.md`. For multi-target projects:

```sh
gleam test --target erlang
gleam test --target javascript
```

## Process

1. **Skeleton**: module, type definitions, function signatures with type annotations, no bodies yet. Gleam requires type annotations on public functions; annotate internals where it aids clarity.
2. **Happy path**: implement the primary success case; run `gleam check`.
3. **Error paths**: add `Result`/`Option` returns per `result-option-and-errors.md`. Prefer returning `Result` over panicking except at truly exceptional boundaries.
4. **Tests**: write Gleeam cases covering happy and error paths; run `gleam test`.
5. **Docs**: fill in module and function documentation comments; run `gleam format`.
6. **Gates**: run the full check sequence above.

## Runtime/debugging hooks

- Gleam's static type system localizes most errors at compile time; `gleam check` is the primary defect-prevention gate and there is no Dialyzer step.
- On the **Erlang target**, runtime issues (actor crashes, message ordering, supervision) require BEAM tooling — see the BEAM bridge in `index.md` and `debugging.md`.
- On the **JavaScript target**, use Node/browser tooling; BEAM docs do not apply.

## Evidence reported

- `gleam check` output (clean, or the specific errors with file:line).
- `gleam test` results (pass/fail count, any failures with file:line).
- `gleam format --check` status.
- A short summary of what was added and where, including the target(s) targeted.

## Common mistakes

- **Panicking instead of returning `Result`**: `panic` is for impossible states, not for expected failures. See `result-option-and-errors.md`.
- **Ignoring the target**: code that works on Erlang may fail on JavaScript (or vice versa) due to FFI or library availability. Confirm the target early.
- **Unannotated public functions**: Gleam requires annotations on public functions; do not rely on inference for the public surface.
- **Adding a dependency to solve a problem the stdlib solves**: check `stdlib.md` first.
- **Speculative generalization**: do not add type variants or abstractions beyond the requirement.

## When to escalate to a human policy decision

- **Architectural decisions**: introducing a new OTP actor, a supervision tree, or a new module boundary.
- **New dependencies**: adding a Hex package requires justification and a dependency-policy check; see `package-management-and-publishing.md`.
- **Breaking changes**: changing a public function signature, removing a module, or altering return types.
- **Target selection**: choosing to support a second target, or dropping a target, is a policy decision.
- **Ambiguous requirements**: when the spec leaves behavior undefined, surface the question rather than guessing.

## Related docs

- `code-review.md` — what happens after implementation is handed off for review.
- `validation.md` — the full gate suite run before merge.
- `../language-fundamentals.md`, `../types-records-and-patterns.md`, `../result-option-and-errors.md` — the primary semantic references.
