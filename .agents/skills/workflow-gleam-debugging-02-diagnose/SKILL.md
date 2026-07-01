---
name: workflow-gleam-debugging-02-diagnose
description: |
  Use only for the diagnose phase of the Gleam debugging workflow.
  Categorize the defect (type-check error / Result-Option error model /
  OTP-actor-or-supervision / FFI-or-interop / target-specific-semantic / logic)
  and identify the root cause. Do not use for reproduction, fixing, regression
  testing, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-debugging
  org.phase: diagnose
  org.phase_order: "02"
---

## Phase purpose

Categorize the defect (type-check error / Result-Option error model / OTP-actor-or-supervision / FFI-or-interop / target-specific-semantic / logic) and identify the root cause. The category determines which operational skills and docs to load.

## Steps to perform

1. Classify the defect into exactly one of:
   - **Type-check error** (`gleam check` fails; read the compiler error message and location);
   - **Runtime crash** (code compiles but crashes at runtime — `panic`, `let assert`, `todo`, actor exit, `call` timeout);
   - **Result-Option error model mistake** (wrong error type, swallowed `Error`, returning `Option` from a fallible function, `let assert Ok(x) = res` in library code);
   - **OTP-actor-or-supervision issue** (Erlang target only — message race, wrong `Next`, supervisor restart loop, child-spec mistake, `call`/`send` panic);
   - **FFI-or-interop issue** (`@external` mismatch, Erlang/Elixir/JS data-shape mismatch, wrong return tuple shape, missing type annotation);
   - **Target-specific-semantic issue** (behavior differs between Erlang and JavaScript targets, e.g. `Int` overflow, `Float` `Infinity`/`NaN`, decoder differences);
   - **Logic error** (code compiles and runs without crashing but produces wrong output).
2. Load skills and docs matching the category:
   - Type-check / Result-Option errors → `gleam-language` + `docs/gleam/result-option-and-errors.md`;
   - OTP/actor/supervision issues (Erlang target) → `gleam-otp-interop` + `docs/gleam/otp-actors-and-supervision.md` + BEAM docs (`docs/beam/runtime-debugging.md`, `docs/beam/processes-and-messages.md`, `docs/beam/links-monitors-and-exits.md`, `docs/beam/supervision.md`);
   - FFI/externals → `gleam-otp-interop` + `docs/gleam/erlang-interop.md` + `docs/gleam/externals-and-ffi.md`;
   - Target-specific semantics → relevant target doc (`docs/gleam/erlang-interop.md` or `docs/gleam/javascript-target.md`) + `docs/gleam/gleam-toml-and-targets.md`;
   - Logic errors → add tests pinning the wrong behavior (these become the regression test in phase `04-regression`); use `echo`, `io.debug`, Node inspector, or BEAM tracing; consult `docs/gleam/testing.md`.
3. Note target divergence explicitly:
   - On the **Erlang target**, use BEAM tooling (`:observer`, `:dbg`, `:sys.get_state`, Erlang shell tracing).
   - On the **JavaScript target**, use Node/browser debugging tooling; BEAM docs and `:observer` do NOT apply.
4. Identify the root cause: write a one-paragraph explanation of why the defect occurred. The root cause must explain the mechanism, not just restate the symptom.

## Docs to consult

By category:
- `docs/gleam/result-option-and-errors.md` (type-check errors, Result-Option mistakes);
- `docs/gleam/erlang-interop.md` (FFI/externals, Erlang target semantics);
- `docs/gleam/externals-and-ffi.md` (FFI/externals);
- `docs/gleam/otp-actors-and-supervision.md` (OTP/actor/supervision, Erlang target);
- `docs/gleam/javascript-target.md` (JavaScript target semantics);
- `docs/gleam/gleam-toml-and-targets.md` (target configuration);
- `docs/gleam/testing.md` (logic errors; test structure);
- BEAM docs (Erlang target only): `docs/beam/runtime-debugging.md`, `docs/beam/processes-and-messages.md`, `docs/beam/links-monitors-and-exits.md`, `docs/beam/supervision.md`.

## Operational skills to load

Conditional by category:
- `gleam-language` (type-check errors, Result-Option mistakes);
- `gleam-otp-interop` (OTP/actor/supervision issues on Erlang target; FFI/externals; target-specific interop).

## Constraints to apply

- Scope discipline — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with catch-all `_` patterns, `panic`/`let assert` renames, or target-workarounds. Diagnosis only; do not begin fixing.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 03-fix`, `next_workflow: null`. Record the defect category and the root-cause paragraph in `evidence` (or `assumptions` if the root cause is provisional). Record the target (`erlang` or `javascript`) in `evidence`. Because the continuation policy is `suggest-next`, surface the category and root cause for confirmation before phase `03-fix` is loaded.
