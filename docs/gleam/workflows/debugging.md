# Debugging Workflow

## Purpose

Step-by-step procedure for debugging a Gleam defect: a crash, a hang, or a wrong output. The agent reproduces the issue, isolates the cause, diagnoses using target-appropriate tooling, fixes, and verifies.

## What happens first

1. Reproduce the issue: obtain a failing Gleeam test, a reproduction script, or a reliable sequence of steps that triggers the defect.
2. If the issue cannot be reproduced, do not guess at a fix. Gather more information (logs, stack trace, runtime state) first.
3. Record the reproduction steps; they become the regression test.
4. Identify the target: Erlang-target bugs use BEAM tooling; JavaScript-target bugs use JS tooling. The workflow differs by target.

## Context gathering

- Read the error message and full stack trace. Note the module, function, and line.
- Read the failing code and its callers.
- Load the relevant docs (see "Docs consulted" below) for the error class and runtime behavior.

## Docs consulted

- `result-option-and-errors.md` — `Result`/`Option` semantics, when `panic` is appropriate, error propagation.
- `erlang-interop.md` — Erlang-target runtime, interop with Erlang code, where BEAM semantics leak in.
- `otp-actors-and-supervision.md` — actor message passing, supervision, process lifecycle (Erlang target).

## Skills loaded

- `gleam-otp-interop` — when the issue involves an actor, supervision, or Erlang interop on the Erlang target.

## Runtime/debugging hooks

Gleam's static type system localizes most errors at compile time; runtime defects are usually in FFI, interop, actor messaging, or target-specific semantics.

**Erlang target (BEAM bridge):** consult the BEAM docs directly:
- Runtime debugging / `sys` / tracing / observer → `docs/beam/runtime-debugging.md`
- Processes / messages / links / monitors → `docs/beam/processes-and-messages.md`
- Exit reasons / error propagation → `docs/beam/links-monitors-and-exits.md`
- Supervision / restart strategies → `docs/beam/supervision.md`

BEAM tools available on the Erlang target: `:observer`, `:dbg`, `:sys.get_state`, Erlang shell tracing, `gleam run` with Erlang shell attached.

**JavaScript target:** use Node/browser debugging tooling (inspector, breakpoints, `console.log`); BEAM docs do not apply.

## Checks run

```sh
gleam test --trace         # if supported; otherwise gleam test with verbose runner
gleam test path/to/module # run a single test module in isolation
gleam check                # confirm the type system is not already flagging the issue
```

For a crash reproduction outside the test suite, write a minimal script and run with `gleam run`.

## Process

1. **Reproduce**: confirm the defect triggers reliably.
2. **Isolate**: narrow the trigger to the smallest input and the smallest code path.
3. **Diagnose**: identify the root cause — a wrong return value, an actor message race, an FFI mismatch, a target-specific semantic difference. Use the target-appropriate debugging tools above.
4. **Fix**: make the smallest change that addresses the root cause, not the symptom.
5. **Verify**: the reproduction now passes; the full suite remains green.
6. **Regression test**: add a Gleeam test that fails without the fix and passes with it.

## Evidence reported

- Root cause: what was wrong, in one or two sentences.
- Fix description: what changed and why this addresses the root cause.
- Test results: the new regression test passes; the full suite is green.
- Target(s) on which the defect was reproduced and verified.
- Any related risks: does the fix affect other call sites or the other target?

## Common mistakes

- **Fixing the symptom**: catching/ignoring an error to silence it without addressing the cause.
- **Debugging on a stale build**: always `gleam build` / `gleam check` before reasoning about behavior.
- **Assuming the type system caught it**: runtime defects live in FFI, interop, actor messaging, and target-specific semantics — areas the type system does not cover.
- **Using BEAM tooling on the JavaScript target**: BEAM docs and `:observer` do not apply to JS-target builds.
- **Skipping the regression test**: a fix without a test will regress.

## When to escalate to a human policy decision

- **Race conditions**: timing-dependent actor bugs may need a design change, not a one-line fix.
- **OTP lifecycle bugs**: crashes during actor init or supervision may indicate a supervision-tree design issue.
- **Cross-actor defects**: when the root cause is in a different actor than the symptom, confirm the fix location with a human.
- **FFI correctness**: defects inside external/FFI bodies may require domain expertise beyond Gleam.
- **Behavior change required**: if the fix changes observable behavior, switch to the implementation workflow and document the delta.

## Related docs

- `implementation.md` — once the root cause is fixed and behavior may need extending.
- `validation.md` — the full gate suite, run before merge.
- `../result-option-and-errors.md`, `../erlang-interop.md` — the primary debugging reference docs.
- `../../beam/runtime-debugging.md`, `../../beam/processes-and-messages.md` — BEAM bridge docs for Erlang-target runtime issues.
