---
name: workflow-elixir-code-review-04-review
description: |
  Use only for the review phase of the Elixir code-review workflow. Perform the
  human-judgment review using per-doc checklists across naming, OTP, error
  handling, typespecs, and documentation. Do not use for scoping, analysis,
  running gates, or issuing a verdict.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-code-review
  org.phase: review
  org.phase_order: "04"
---

# Phase 04: review (Elixir code review)

## Phase purpose

Perform the human-judgment review using the per-doc checklists across naming,
OTP, error handling, typespecs, and documentation. Walk each topic doc's
`## Review checklist` section explicitly and cite items rather than
paraphrasing.

## Steps to perform

1. **Naming and pattern matching** — consult `docs/elixir/naming-conventions.md`
   and `docs/elixir/language-fundamentals.md` and the `elixir-coding` skill.
   Look for snake_case/CamelCase violations, `?`/`!` suffix misuse, pipe usage,
   and control-flow clarity.
2. **OTP patterns** (only if OTP is present) — consult
   `docs/elixir/otp-supervision.md`, `docs/beam/supervision.md`,
   `docs/beam/gen-server.md`, and the `elixir-otp` skill. Check child specs,
   restart strategies, and that no `spawn/1` is used where a supervised process
   belongs.
3. **Error handling** — consult `docs/elixir/error-handling.md`,
   `docs/beam/links-monitors-and-exits.md`, and the `elixir-error-handling`
   skill. Look for swallowed errors, bang functions without non-bang
   counterparts, and `raise`/`throw` on recoverable conditions.
4. **Typespecs** — consult `docs/elixir/typespecs-and-dialyzer.md`. Check
   `@spec` on public functions (per repo policy) and `@type t` where a module
   owns a type.
5. **Documentation** — consult `docs/elixir/documentation-and-publishing.md`.
   Check `@moduledoc`/`@doc` on public modules/functions, no `false` moduledocs
   without reason, and `@moduledoc false`/`@doc false` used consistently.
6. Walk each topic doc's `## Review checklist` section explicitly; record
   pass/fail/N-A per item. **Cite checklist items; do not paraphrase them.**

## Docs to consult

- `docs/elixir/naming-conventions.md`
- `docs/elixir/language-fundamentals.md`
- `docs/elixir/core-modules.md`
- `docs/elixir/otp-supervision.md`
- `docs/elixir/testing-exunit.md`
- `docs/elixir/typespecs-and-dialyzer.md`
- `docs/beam/supervision.md`
- `docs/beam/gen-server.md`
- `docs/beam/common-mistakes.md`
- `docs/beam/validation.md`

## Operational skills to load

- `elixir-coding`
- `elixir-error-handling`
- `elixir-otp` (if OTP present)
- `beam-supervision` (if OTP supervision present)
- `beam-gen-server` (if `GenServer` callbacks present)
- `beam-errors-failures` (if error/exit paths present)

## Constraints to apply

- `constraint-elixir-style` — enforce naming conventions, pipe usage, and
  idiomatic pattern matching; flag style inconsistencies.
- `constraint-beam-failure` — flag swallowed errors, missing error tuples at
  boundaries, and inappropriate `raise`/`throw` on recoverable conditions.
- `constraint-beam-supervision` — applies only if OTP is present: verify child
  specs, restart strategies, and that processes are supervised rather than
  bare-spawned.
- `constraint-beam-process-isolation` — applies only if NIFs are present:
  verify NIF scheduler safety and process-isolation invariants.
- `constraint-elixir-otp-api` — verify public API completeness for OTP-facing
  modules: `@spec`, `@doc`, and `@moduledoc` as required by repo policy.
- `constraint-elixir-style` — review only the diff; do not request changes to
  untouched code.

## Validations to run

None — this is a manual review phase that uses per-doc checklists. Automated
validations ran in phase 03 (workflow-elixir-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-elixir-code-review-00-orchestration`. Set:

- `outcome`: `pass` if no findings; `partial` if findings exist but none are
  blockers; `fail` if a blocker-level finding was identified.
- `constraints_applied`: all constraints listed above that applied.
- `risks`: each finding with file:line, the checklist item or doc rule
  violated (cited), and a concrete suggested fix. Severity classification
  happens in phase 05-verdict; here record the raw findings.
- `assumptions`: any N-A checklist items and why.
- `next_phase`: `05-verdict`.
- `next_workflow`: `null`.
