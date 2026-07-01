---
name: workflow-gleam-validation-01-scope
description: |
  Use only for the scope phase of the Gleam validation workflow.
  Identify what changed (git diff) and determine which gates apply
  (multi-target? package-interface required? publish/release only?). Do not
  use for compile, test, format, doc, or final reporting.
allowed-tools: Read Bash(git:*) Bash(grep:*) Bash(find:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-validation
  org.phase: scope
  org.phase_order: "01"
---

# Gleam Validation Workflow — Phase 01 — Scope

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
   - **Multi-target**: check `gleam.toml` for the declared target and repo policy. If the project supports both Erlang and JavaScript targets, per-target tests apply (`gleam test --target erlang` and `gleam test --target javascript`). If only one target is supported, mark the other target "not applicable".
   - **Package interface / docs build**: required for release validation; for standard CI/merge validation it may be marked "release only". Check repo policy in `CONTRIBUTING.md` or `docs/gleam/validation.md`.
   - **Publish**: release only, after human sign-off. Mark "release only" unless this is a release validation.
   - **BEAM/OTP concerns**: if `gleam_otp`, `gleam_erlang` deps, or `@external erlang` declarations are present, note that runtime validation on the Erlang target may require BEAM debugging (cross-ref `docs/gleam/workflows/debugging.md` and the `gleam-otp-interop` skill). On the JavaScript target, BEAM constraints do not apply.
   - **SBoM / supply chain**: Gleam has no native SBoM command; generation is delegated to ORT consuming `manifest.toml`. Mark as "not a validation gate" unless release policy requires ORT.
3. Record the list of applicable gates and the list of not-applicable/not-configured/release-only gates with reasons.
4. Do NOT modify any code, config, or manifest in this phase. Scope only.

## Docs to Consult

- `docs/gleam/validation.md`
- `docs/gleam/testing.md`
- `docs/gleam/project-structure-and-cli.md`
- `docs/gleam/package-management-and-publishing.md`

## Operational Skills to Load

- `gleam-packages-ffi` — project structure, CLI, target matrix, deps, testing, validation.
- `gleam-otp-interop` (conditional) — if OTP/FFI/Erlang-target code is present.

## Constraints to Apply

- scope discipline — validation is a gate, not a fix; do not modify code or config to make a gate applicable/inapplicable.

## Validations to Run

None. This phase identifies which gates apply; it does not run validation gates.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (scope determined) | partial (some gates' applicability unclear).
- `files_touched`: `[]` (scope only; no changes).
- `constraints_applied`: `["scope discipline"]`
- `assumptions`: the list of applicable gates and the list of not-applicable/not-configured/release-only gates with reasons; the changed-files list.
- `risks`: multi-target policy requiring per-target test runs; OTP/FFI code present (BEAM runtime concerns); release-only gates.
- `tests_run`: `["git diff --stat", "git diff --name-only"]`.
- `next_phase`: `02-compile`.
- `next_workflow`: `null`.
- `blockers`: any gate whose applicability could not be determined.
