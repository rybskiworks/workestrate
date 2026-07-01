---
name: workflow-elixir-validation-01-scope
description: |
  Use only for the scope phase of the Elixir validation workflow.
  Identify what changed (git diff) and determine which gates apply
  (OTP code? coverage configured? deps audit available?). Do not
  use for compile, test, format, doc, or final reporting.
allowed-tools: Read Bash(git:*) Bash(grep:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-validation
  org.phase: scope
  org.phase_order: "01"
---

# Elixir Validation Workflow — Phase 01 — Scope

## Phase Purpose

Identify what changed and determine which gates apply. Record applicable vs
not-applicable gates so phases 02-04 run only the relevant gates and mark the
rest "not applicable" or "not configured".

## Steps

1. Identify what changed:
   ```sh
   git diff --stat
   git diff --name-only
   ```
2. Determine gate applicability:
   - **OTP code**: search the workspace for `GenServer`, `Supervisor`, `Agent`, `Registry`, `DynamicSupervisor`, or `Application`. If present, runtime validation hooks from `docs/beam/validation.md` apply (noted in phase 05; cross-ref debugging workflow). If absent, mark runtime validation "not applicable".
   - **Coverage**: check whether `mix test --cover` is configured (e.g., `test_coverage` in `mix.exs` or a coverage tool such as `excoveralls`). If not configured, mark coverage "not configured" and skip.
   - **Deps audit**: check whether `mix deps.audit` or `hex_audit` is available. If absent, phase 04 records deps audit as "not configured".
   - **Dialyzer**: check whether the PLT is configured (e.g., `dialyxir` in `mix.exs`, `.dialyzer_ignore.exs`, or `plt_add_apps`). If not configured, mark Dialyzer "not configured".
   - **Feature matrix**: check `mix.exs` for project aliases, extra applications, or mutually-exclusive dependencies; if present, note that `mix test` in phase 03 may need adjustment (consult `docs/elixir/mix-project-structure.md`).
3. Record the list of applicable gates and the list of not-applicable/not-configured gates with reasons.
4. Do NOT modify any code, config, or manifest in this phase. Scope only.

## Docs to Consult

- `docs/elixir/workflows/validation.md`
- `docs/elixir/mix-project-structure.md`
- `docs/elixir/dependencies-and-packages.md`
- `docs/beam/validation.md`

## Operational Skills to Load

- `elixir-project-setup` — Mix project structure, dependencies, aliases, Dialyzer configuration.
- `beam-observability-debugging` (conditional) — if OTP code is present.

## Constraints to Apply

- `constraint-elixir-style` — validation is a gate, not a fix; do not modify code or config to make a gate applicable/inapplicable.

## Validations to Run

None. This phase identifies which gates apply; it does not run validation gates.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (scope determined) | partial (some gates' applicability unclear).
- `files_touched`: `[]` (scope only; no changes).
- `constraints_applied`: `["constraint-elixir-style"]`
- `assumptions`: the list of applicable gates and the list of not-applicable/not-configured gates with reasons; the changed-files list.
- `risks`: feature-matrix or alias adjustments required for `mix test`; OTP code present (runtime validation hooks apply).
- `tests_run`: `["git diff --stat", "git diff --name-only"]`.
- `next_phase`: `02-compile`.
- `next_workflow`: `null`.
- `blockers`: any gate whose applicability could not be determined.
