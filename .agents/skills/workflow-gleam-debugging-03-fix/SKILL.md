---
name: workflow-gleam-debugging-03-fix
description: |
  Use only for the fix phase of the Gleam debugging workflow.
  Implement the minimal root-cause fix. Do not use for reproduction,
  diagnosis, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-debugging
  org.phase: fix
  org.phase_order: "03"
---

## Phase purpose

Implement the minimal root-cause fix. Do not suppress the symptom. The fix must address the mechanism identified in phase `02-diagnose`.

## Steps to perform

1. Fix the root cause, not the symptom. Concretely by category:
   - **Type-check error** → correct the type model per `gleam-language` (fix annotations, custom types, pattern exhaustiveness) rather than inserting `todo` or catch-all `_` patterns.
   - **Runtime crash** → replace `panic`/`let assert`/`todo` with proper `Result` handling per `docs/gleam/result-option-and-errors.md` (except in OTP libraries where a supervision tree is intended).
   - **Result-Option error model mistake** → return `Result` from fallible functions, chain with `result.try`/`use`, avoid `let assert Ok(x) = res` in libraries.
   - **OTP/actor/supervision issue (Erlang target)** → address the actual message race, wrong `Next`, supervisor restart loop, or child-spec mistake per `docs/gleam/otp-actors-and-supervision.md` and BEAM docs, not a caller-side catch.
   - **FFI/externals issue** → fix the data-shape or return-tuple mismatch, add/strengthen type annotations on `@external` functions, align Erlang/JS mapping rules per `docs/gleam/erlang-interop.md` and `docs/gleam/externals-and-ffi.md`.
   - **Target-specific-semantic issue** → fix the code path for the affected target (or make it target-agnostic), validate on BOTH targets.
   - **Logic error** → fix the incorrect computation, not a caller that masks it.
2. Apply `constraint-gleam-result`: ensure errors propagate correctly (`result.try`, proper error types) rather than being swallowed or panicked.
3. If the fix touches OTP/actor/supervision/interop code on the Erlang target, apply `constraint-beam-supervision`, `constraint-beam-failure`, and `constraint-beam-process-isolation`: restrict changes to the soundness fix and its supervision/failure-context; do not rewrite the surrounding OTP logic. On the JavaScript target these constraints do not apply.
4. Apply scope discipline: fix only the reported defect; record adjacent issues as follow-ups; do not add features; do not suppress with catch-all `_` patterns, `panic`/`let assert` renames, or target-workarounds.

## Docs to consult

By category:
- `docs/gleam/result-option-and-errors.md` (runtime crashes, Result-Option mistakes);
- `docs/gleam/erlang-interop.md` (FFI/externals, Erlang target);
- `docs/gleam/externals-and-ffi.md` (FFI/externals);
- `docs/gleam/otp-actors-and-supervision.md` (OTP/actor/supervision, Erlang target);
- `docs/gleam/javascript-target.md` (JavaScript target);
- `docs/gleam/gleam-toml-and-targets.md` (target configuration).

## Operational skills to load

Conditional by category:
- `gleam-language` (type-check errors, Result-Option mistakes);
- `gleam-otp-interop` (OTP/actor/supervision issues on Erlang target; FFI/externals; target-specific interop).

## Constraints to apply

- Scope discipline — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with catch-all `_` patterns, `panic`/`let assert` renames, or target-workarounds.
- `constraint-gleam-result` — Ensure errors propagate correctly (`result.try`, proper error types) rather than being swallowed or panicked.
- `constraint-beam-supervision`, `constraint-beam-failure`, `constraint-beam-process-isolation` — If OTP/actor/supervision/interop is touched on the Erlang target: restrict changes to the soundness fix and its BEAM failure context; do not rewrite the surrounding OTP logic. Do not apply on the JavaScript target.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 04-regression`, `next_workflow: null`. Record the fix — what changed and why it addresses the root cause — in `evidence`. List every file changed in `files_touched`. List the constraints actually applied in `constraints_applied`.
