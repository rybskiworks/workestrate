---
name: workflow-gleam-validation-04-format-doc
description: |
  Use only for the format, docs, and release-gate phase of the Gleam
  validation workflow. Run gleam format --check, gleam build, gleam docs
  build, and gleam publish (release only). Do not use for scoping,
  compile, or testing.
allowed-tools: Read Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-validation
  org.phase: format-doc
  org.phase_order: "04"
---

# Gleam Validation Workflow — Phase 04 — Format, Build & Release Gates

## Phase Purpose

Run the format gate and release gates. Record pass/fail and evidence per gate.
`gleam format --check` is a standard validation gate; `gleam build`,
`gleam docs build`, and `gleam publish` are release-only gates (run only when
phase 01 marks them applicable).

## Steps

1. Run the format gate:
   ```sh
   gleam format --check
   ```
   Formatting must be clean across the project. Map to `validation-gleam-format`.
2. Run the build gate (release/extended validation):
   ```sh
   gleam build
   ```
   For multi-target repos, also run `gleam build --target erlang` and `gleam build --target javascript` if required by repo policy.
3. Run the package-interface / doc-build gate (release only):
   ```sh
   gleam docs build
   ```
   This surfaces documentation and public-surface issues that `gleam check` does not. It is a release gate, not a standard CI validation skill.
4. Run the publish gate (release only, after human sign-off):
   ```sh
   gleam publish
   ```
   This is irreversible; only run when phase 01 marks it applicable.
5. For each gate, record pass/fail and the relevant output (diff or clean; warning count or clean; package-interface errors or clean).
6. If a gate fails, continue to phase 05 to give a full picture; the overall result will be fail.
7. Do NOT modify code, `gleam.toml`, or formatting to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/gleam/validation.md`
- `docs/gleam/package-management-and-publishing.md`

## Operational Skills to Load

- `gleam-packages-ffi` — formatting, build, package-interface export, publishing.

## Constraints to Apply

- scope discipline — do not modify code or config files to make a gate pass.

## Validations to Run

- `validation-gleam-format` — command: `gleam format --check`

(Release gates are operational steps, not standard validation skills: `gleam build`, `gleam docs build`, `gleam publish`.)

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any gate red) | partial (a gate release-only/not-applicable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: `["gleam format --check", "gleam build", "gleam docs build", "gleam publish"]` with pass/fail + evidence per gate (mark release-only/not-applicable gates).
- `risks`: any gate red; publish gate failure requiring human decision.
- `next_phase`: `05-report`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
