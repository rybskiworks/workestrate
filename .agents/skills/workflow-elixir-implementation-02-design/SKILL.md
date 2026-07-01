---
name: workflow-elixir-implementation-02-design
description: |
  Use only for the design phase of the Elixir implementation workflow. Make
  module structure, function shape, error strategy, and OTP design decisions up
  front. Do not use for scoping, implementation, testing, or final verification.
allowed-tools: Read Write Edit Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-implementation
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Elixir implementation)

## Phase purpose

Make module structure, function shape, error strategy, and OTP design decisions
up front so the implement phase can proceed without mid-flight architectural
changes.

## Steps to perform

1. Load `elixir-coding` — naming, typespecs, documentation, function shape.
2. Load `elixir-error-handling` — error tuples vs exceptions, bang variants,
   `raise`/`rescue` boundaries.
3. Load `elixir-project-setup` for manifest changes, application deps, and
  module layout.
4. Read `docs/elixir/mix-project-structure.md` for module file layout,
   `application/0`, and dependency declaration.
5. Decide the module file convention and function shape up front:
   - Use `def`/`defp` with multi-clause functions and pattern matching rather
     than nested conditionals.
   - Prefer the pipe operator `|>` for sequential transformations.
   - Place `@spec` on public functions and keep `@moduledoc` current.
6. Decide the error strategy:
   - Library boundary → return `{:ok, result}` / `{:error, reason}` tuples for
     expected failures; add a `foo!` bang variant when failure is exceptional.
   - Application boundary → let supervisors handle unexpected failure per
     `docs/elixir/error-handling.md` and `docs/beam/errors-failures.md`.
   - Never use `raise`/`throw` for expected runtime conditions; reserve them
     for genuine invariants and tests.
7. Conditionally load `elixir-otp` if the implementation involves OTP processes
   or supervision. Read `docs/elixir/otp-supervision.md`,
   `docs/beam/supervision.md`, and `docs/beam/gen-server.md` when adding a
   GenServer, Supervisor, or process.

## Docs to consult

- `docs/elixir/mix-project-structure.md`
- `docs/elixir/error-handling.md`
- `docs/elixir/naming-conventions.md`
- `docs/elixir/otp-supervision.md` (if OTP)
- `docs/beam/supervision.md` (if OTP)
- `docs/beam/gen-server.md` (if GenServer)

## Operational skills to load

- `elixir-coding`
- `elixir-error-handling`
- `elixir-project-setup`
- (conditional) `elixir-otp` — only if the implementation involves OTP
  processes or supervision.

## Constraints to apply

- `constraint-elixir-style` — design only what the captured requirements demand.
  Do not introduce speculative abstractions, new dependencies, or modules beyond
  what the new code requires.
- `constraint-elixir-otp-api` — plan documentation for the public API: decide
  which public items need `@moduledoc`, `@doc`, `@spec`, and doctests so the
  implement and verify phases can produce complete docs.

## Validations to run

None — validations run in phase 05 (workflow-elixir-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-elixir-implementation-00-orchestration`. Set:

- `outcome` to `pass` once module convention, function shape, and error strategy
  are decided and recorded.
- `constraints_applied` to include `constraint-elixir-style` and
  `constraint-elixir-otp-api`.
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. process boundary or dependency
  choice) needs human input — in that case set `handoff_requires_hil: true` and
  record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-style
  - constraint-elixir-otp-api
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
