---
name: workflow-elixir-debugging-03-fix
description: |
  Use only for the fix phase of the Elixir debugging workflow.
  Implement the minimal root-cause fix. Do not use for reproduction,
  diagnosis, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-debugging
  org.phase: fix
  org.phase_order: "03"
---

## Phase purpose

Implement the minimal root-cause fix. Do not suppress the symptom. The fix must address the mechanism identified in phase `02-diagnose`.

## Steps to perform

1. Fix the root cause, not the symptom. Concretely by category:
   - **Compile error** → fix the syntax/pattern-match/type issue per the `elixir-coding` skill and `docs/elixir/language-fundamentals.md`.
   - **Runtime crash/exception** → replace `raise`/bad match/unchecked indexing with proper error handling per `docs/elixir/error-handling.md`.
   - **Concurrency/race condition** → address the actual race (missing `GenServer.call`, wrong `Task.await` ordering, unsynchronized shared state) per `docs/elixir/concurrency-processes.md` and `docs/beam/processes-and-messages.md`, not a workaround that masks timing.
   - **Hang/deadlock** → identify the blocked process and the missing message/timeout per `docs/elixir/concurrency-processes.md` and `docs/beam/processes-and-messages.md`.
   - **OTP lifecycle** → fix the supervision/init/terminate issue per `docs/elixir/otp-supervision.md` and `docs/beam/supervision.md`; do not rewrite unrelated GenServer logic.
   - **Config mismatch** → fix the config precedence or runtime lookup per `docs/elixir/configuration-and-runtime.md`.
   - **Memory/scheduler pressure** → fix the leak or retained state per `docs/elixir/beam-otp-internals.md`.
   - **Logic error** → fix the incorrect computation, not a caller that masks it.
2. Apply `constraint-beam-failure`: ensure errors propagate correctly, let-it-crash where appropriate, and do not swallow errors.
3. If the fix touches OTP code (GenServer/Supervisor), apply `constraint-beam-supervision`: restrict changes to the supervision/init/terminate fix; do not rewrite surrounding logic.
4. If the fix touches a NIF, apply `constraint-beam-process-isolation`: restrict changes to the process-isolation fix at the NIF boundary; do not rewrite unrelated code.
5. Apply `constraint-elixir-style`: fix only the reported defect; record adjacent issues as follow-ups; do not add features; do not suppress with `# credo:disable-for-next-line` or catching exceptions to silence them.

## Docs to consult

By category:
- `docs/elixir/language-fundamentals.md` (compile error);
- `docs/elixir/error-handling.md` (runtime crash/exception, error-handling mistakes);
- `docs/elixir/concurrency-processes.md` (concurrency/race, hang/deadlock);
- `docs/beam/processes-and-messages.md` (concurrency/race, hang/deadlock);
- `docs/elixir/otp-supervision.md` (OTP lifecycle);
- `docs/beam/supervision.md` (OTP lifecycle);
- `docs/elixir/configuration-and-runtime.md` (config mismatch);
- `docs/elixir/beam-otp-internals.md` (memory/scheduler pressure);
- `docs/beam/links-monitors-and-exits.md` (exit reasons, `trap_exit`, `DOWN`, exit-signal propagation).

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

- `constraint-elixir-style` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `# credo:disable-for-next-line` or catching exceptions to silence them.
- `constraint-beam-failure` — Ensure errors propagate correctly, let-it-crash where appropriate, and do not swallow errors.
- `constraint-beam-supervision` — If OTP is touched: restrict changes to the supervision/init/terminate fix; do not rewrite surrounding logic.
- `constraint-beam-process-isolation` — If NIFs are touched: restrict changes to the process-isolation fix at the NIF boundary; do not rewrite unrelated code.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 04-regression`, `next_workflow: null`. Record the fix — what changed and why it addresses the root cause — in `evidence`. List every file changed in `files_touched`. List the constraints actually applied in `constraints_applied`.
