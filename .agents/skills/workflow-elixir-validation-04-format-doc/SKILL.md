---
name: workflow-elixir-validation-04-format-doc
description: |
  Use only for the format-doc phase of the Elixir validation workflow.
  Run mix format --check-formatted. Do not use for scoping, compile,
  or testing.
allowed-tools: Read Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-validation
  org.phase: format-doc
  org.phase_order: "04"
---

# Elixir Validation Workflow — Phase 04 — Format & Docs

## Phase Purpose

Run the format gate. Record pass/fail and evidence. For release validation,
optionally run dependency auditing if available.

## Steps

1. Run the format gate:
   ```sh
   mix format --check-formatted
   ```
   Formatting must be clean across the project. Map to `validation-elixir-format`.
2. For release validation, optionally run the dependency audit gate:
   ```sh
   mix deps.audit
   ```
   Checks Hex advisories if `hex_audit` or an equivalent audit task is configured. If `mix deps.audit` is not available, record deps audit as "not configured".
3. For each gate, record pass/fail and the relevant output (diff or clean; advisory count or clean).
4. If a gate fails, continue to phase 05 to give a full picture; the overall result will be fail.
5. Do NOT modify code, `mix.exs`, `.formatter.exs`, or `.credo.exs` to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/elixir/workflows/validation.md`
- `docs/elixir/naming-conventions.md`
- `docs/elixir/documentation-and-publishing.md`
- `docs/elixir/dependencies-and-packages.md`

## Operational Skills to Load

- `elixir-docs-publishing` — documentation and Hex publishing policy.
- `elixir-project-setup` — dependencies, formatter configuration.

## Constraints to Apply

- `constraint-elixir-style` — do not modify code or config files to make a gate pass.

## Validations to Run

- `validation-elixir-format` — command: `mix format --check-formatted`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any gate red) | partial (a gate not-configured/not-applicable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: `["mix format --check-formatted"]` with pass/fail + evidence per gate (mark not-configured gates; include `mix deps.audit` if run).
- `risks`: any gate red; supply-chain advisory requiring human judgment on exploitability.
- `next_phase`: `05-report`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
