# Implementation Workflow

## Purpose

Step-by-step procedure for writing new Elixir code: a new feature, module, or function. The agent gathers context, follows corpus conventions, runs the standard checks, and reports evidence. This workflow orchestrates; it does not redefine Elixir semantics.

## What happens first

1. Understand the requirement in concrete terms: what input, what output, what side effects, what failure modes.
2. Check existing patterns in the target repo: search for a sibling module or function that already solves a similar problem. Prefer consistency with local conventions over personal preference.
3. Confirm scope: is this a single function, a new module, or a new OTP process? The scope determines which docs and skills apply.

## Context gathering

- Read the relevant `docs/elixir/` files (see "Docs consulted" below) for the conventions that govern the code being written.
- Load the relevant skills (see "Skills loaded" below) so the actionable procedures are in context.
- Inspect the target module's existing typespecs, module attributes, and test module to match local style.

## Docs consulted

- `language-fundamentals.md` — pattern matching, immutability, pipe operator, control flow.
- `core-modules.md` — `Enum` vs `Stream`, `Map`/`Keyword`/`MapSet`, string types.
- `naming-conventions.md` — casing, module/function/atom naming, `foo`/`foo!` pairs.
- `otp-supervision.md` — only when adding a GenServer, Supervisor, or process; restart strategies, child specs.
- `mix-project-structure.md` — where files live, application/0, dependency declaration.
- `../../beam/supervision.md` — supervisor flags, child specs, restart strategies (when adding OTP processes).
- `../../beam/gen-server.md` — `gen_server` callback contract (when adding a GenServer).
- `../../beam/applications.md` — `.app` resource, application callback, env precedence (when wiring `application/0`).

## Skills loaded

- `elixir-coding` — naming, typespecs, documentation, function shape.
- `elixir-otp` — only when the change involves OTP processes or supervision.
- `elixir-project-setup` — when adding modules, mix.exs entries, or application deps.

## Checks run

Run these in order; stop and fix before proceeding when a check fails:

```sh
mix format --check-formatted
mix compile --warnings-as-errors
mix credo --strict
mix test
```

If the repo records a different policy (e.g., Credo not in strict mode), defer to the repo's `CONTRIBUTING.md`.

## Evidence reported

- Compilation output (clean, or the specific warnings/errors).
- Test results (pass/fail count, any failures with file:line).
- Lint results (Credo issue count by category, or clean).
- A short summary of what was added and where.

## When human judgment is needed

- **Architectural decisions**: introducing a new process, a new supervision tree, or a new boundary (context, boundary, schema).
- **New dependencies**: adding a Hex package requires justification and a dependency-policy check; see `dependencies-and-packages.md`.
- **Breaking changes**: changing a public function signature, removing a module, or altering return types.
- **Ambiguous requirements**: when the spec leaves behavior undefined, surface the question rather than guessing.

## Avoiding unrelated cleanup

- Stay in scope: do not reformat, rename, or refactor code outside the change being made.
- If an unrelated issue is noticed, record it as a follow-up note rather than fixing it inline.
- Do not bump dependencies, update the lockfile, or touch CI config unless the task requires it.
- Keep the diff reviewable: one logical change per branch.

## Implementation order

When the change spans multiple concerns, follow this order to keep each step independently verifiable:

1. **Skeleton**: module, `@moduledoc`, function heads with `@spec`, no bodies yet.
2. **Happy path**: implement the primary success case; run `mix compile --warnings-as-errors`.
3. **Error paths**: add error-tuple returns or bang variants per `error-handling.md`.
4. **Tests**: write ExUnit cases covering happy and error paths; run `mix test`.
5. **Docs**: fill in `@doc` strings; run `mix format`.
6. **Gates**: run the full check sequence above.

## Scope boundaries

- Do not add a dependency to solve a problem the standard library already solves. Check `core-modules.md` first.
- Do not introduce a GenServer when a stateless function suffices. Check `otp-supervision.md` for when a process is justified.
- Do not generalize beyond the requirement. Speculative abstractions broaden the diff and the review surface.

## Related docs

- `code-review.md` — what happens after implementation is handed off for review.
- `validation.md` — the full gate suite run before merge.
- `../naming-conventions.md`, `../language-fundamentals.md`, `../core-modules.md` — the primary semantic references.
