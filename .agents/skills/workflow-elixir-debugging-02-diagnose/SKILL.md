---
name: workflow-elixir-debugging-02-diagnose
description: |
  Use only for the diagnose phase of the Elixir debugging workflow.
  Categorize the defect (compile error, runtime crash/exception, logic error,
  concurrency/race, hang/deadlock, config mismatch, memory/scheduler pressure)
  and identify the root cause. Do not use for reproduction, fixing, regression
  testing, or final verification.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-debugging
  org.phase: diagnose
  org.phase_order: "02"
---

## Phase purpose

Categorize the defect (compile error, runtime crash/exception, logic error, concurrency/race, hang/deadlock, config mismatch, memory/scheduler pressure) and identify the root cause. The category determines which operational skills and docs to load.

## Steps to perform

1. Classify the defect into exactly one of:
   - **Compile error** (`mix compile --warnings-as-errors` fails; read the compiler warning/error and message);
   - **Runtime crash/exception** (code compiles but a process crashes at runtime — exception, `raise`, bad match, `Enum.OutOfBoundsError`, arithmetic error, unexpected exit reason);
   - **Logic error** (code compiles and runs without crashes but produces wrong output);
   - **Concurrency/race condition** (timing-dependent; unsynchronized shared state, missing `GenServer.call`, or `Task.await` ordering);
   - **Hang/deadlock** (a process blocks waiting for a message that never arrives; inspect message queues with `:observer` or `Process.info/2`);
   - **Config mismatch** (wrong value at runtime; check config precedence in `docs/elixir/configuration-and-runtime.md`);
   - **Memory/scheduler pressure** (slow or OOM; inspect with `:observer`; see `docs/elixir/beam-otp-internals.md`).
2. Load skills and docs matching the category:
   - Compile error → `elixir-coding` + `elixir-project-setup` + `docs/elixir/language-fundamentals.md`;
   - Runtime crash/exception → `elixir-error-handling` + `beam-errors-failures` + `docs/elixir/error-handling.md`;
   - Concurrency/race condition → `elixir-otp` + `beam-processes` + `beam-observability-debugging` + `docs/elixir/concurrency-processes.md` + `docs/beam/processes-and-messages.md`;
   - Hang/deadlock → `elixir-otp` + `beam-processes` + `beam-observability-debugging` + `docs/elixir/concurrency-processes.md` + `docs/beam/processes-and-messages.md`;
   - OTP lifecycle (GenServer/Supervisor init, terminate, restart storm) → `elixir-otp` + `beam-supervision` + `beam-gen-server` + `docs/elixir/otp-supervision.md` + `docs/beam/supervision.md`;
   - Config mismatch → `elixir-config` + `docs/elixir/configuration-and-runtime.md`;
   - Memory/scheduler pressure → `beam-observability-debugging` + `docs/elixir/beam-otp-internals.md`;
   - Logic errors → add tests pinning the wrong behavior (these become the regression test in phase `04-regression`); use `IO.inspect/2`, `Logger.debug`, `IEx.pry/0`, `IEx.break!/2`, `:observer`, `:dbg`, `:sys`; consult `docs/elixir/testing-exunit.md`.
3. Load `elixir-error-handling` + `beam-errors-failures` if the defect is an error-handling mistake (swallowed error, wrong error tuple, `raise` on a recoverable condition).
4. Identify the root cause: write a one-paragraph explanation of why the defect occurred. The root cause must explain the mechanism, not just restate the symptom.

## Docs to consult

By category:
- `docs/elixir/error-handling.md` (runtime crash/exception, error-handling mistakes);
- `docs/elixir/beam-otp-internals.md` (memory/scheduler pressure, deep runtime behavior);
- `docs/elixir/concurrency-processes.md` (concurrency/race, hang/deadlock);
- `docs/beam/processes-and-messages.md` (concurrency/race, hang/deadlock);
- `docs/elixir/otp-supervision.md` (OTP lifecycle);
- `docs/beam/supervision.md` (OTP lifecycle);
- `docs/elixir/configuration-and-runtime.md` (config mismatch);
- `docs/beam/runtime-debugging.md` (runtime diagnosis: `sys`, `trace`, `dbg`, `ttb`);
- `docs/beam/links-monitors-and-exits.md` (exit reasons, `trap_exit`, `DOWN`, exit-signal propagation);
- `docs/elixir/testing-exunit.md` (logic errors; test structure).

## Operational skills to load

Conditional by category:
- `elixir-coding` + `elixir-project-setup` (compile error);
- `elixir-error-handling` + `beam-errors-failures` (runtime crash/exception, error-handling mistakes);
- `elixir-otp` + `beam-processes` + `beam-observability-debugging` (concurrency/race, hang/deadlock);
- `elixir-otp` + `beam-supervision` + `beam-gen-server` (OTP lifecycle);
- `elixir-config` (config mismatch);
- `beam-observability-debugging` (memory/scheduler pressure);
- `elixir-testing` (logic errors).

## Constraints to apply

- `constraint-elixir-style` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `# credo:disable-for-next-line` or catching exceptions to silence them. Diagnosis only; do not begin fixing.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 03-fix`, `next_workflow: null`. Record the defect category and the root-cause paragraph in `evidence` (or `assumptions` if the root cause is provisional). Because the continuation policy is `suggest-next`, surface the category and root cause for confirmation before phase `03-fix` is loaded.
