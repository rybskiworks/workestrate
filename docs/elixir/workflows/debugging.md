# Debugging Workflow

## Purpose

Step-by-step procedure for debugging an Elixir defect: a crash, a hang, or a wrong output. The agent reproduces the issue, isolates the cause, diagnoses using BEAM tooling, fixes, and verifies.

## What happens first

1. Reproduce the issue: obtain a failing test, a reproduction script, or a reliable sequence of steps that triggers the defect.
2. If the issue cannot be reproduced, do not guess at a fix. Gather more information (logs, stack trace, observer state) first.
3. Record the reproduction steps; they become the regression test.

## Context gathering

- Read the error message and full stack trace. Note the module, function, arity, and line.
- Read the failing code and its callers.
- Load the relevant docs (see "Docs consulted" below) for the error class and runtime behavior.

## Docs consulted

- `error-handling.md` — exception classes, error tuples, `try/rescue` vs `{:ok, _}` patterns, `@moduledoc`-level error semantics.
- `beam-otp-internals.md` — scheduler, GC, process heap, message queue, exit reasons.
- `concurrency-processes.md` — process linking, monitoring, message passing, race conditions.
- `configuration-and-runtime.md` — config sources, runtime config, environment variables (when the bug is config-related).
- `../../beam/runtime-debugging.md` — `sys`, `trace`, `dbg`, `ttb`, introspection BIFs for runtime diagnosis.
- `../../beam/links-monitors-and-exits.md` — exit reasons, `trap_exit`, `DOWN`, exit-signal propagation.

## Skills loaded

- `elixir-error-handling` — error classification and handling patterns.
- `elixir-otp` — only when the issue involves a GenServer, Supervisor, or process lifecycle.

## Debugging tools

- **IEx**: `iex -S mix` for interactive exploration; `recompile()` to reload after edits.
- **`:observer`**: `:observer.start()` for process count, memory, message queue length.
- **`:dbg`**: `:dbg.tracer()` / `:dbg.p/2` for tracing process messages and function calls.
- **`:sys`**: `:sys.get_state/1`, `:sys.get_status/1`, `:sys.trace/3` for GenServer introspection.
- **`Logger.debug`**: enable debug-level logging to trace execution flow.
- **ExUnit assertions**: failing assertions localize the defect; `assert` messages show the mismatched values.

## Checks run

```sh
mix test --trace              # verbose test output, identifies the failing case
mix test path/to/test.exs:42 # run a single test in isolation
```

For a crash reproduction outside the test suite, write a minimal script and run with `elixir repro.exs` or in `iex`.

## Process

1. **Reproduce**: confirm the defect triggers reliably.
2. **Isolate**: narrow the trigger to the smallest input and the smallest code path.
3. **Diagnose**: identify the root cause — a wrong return value, a race, a crash, a config mismatch. Use the debugging tools above to inspect state.
4. **Fix**: make the smallest change that addresses the root cause, not the symptom.
5. **Verify**: the reproduction now passes; the full suite remains green.
6. **Regression test**: add a test that fails without the fix and passes with it.

## Evidence reported

- Root cause: what was wrong, in one or two sentences.
- Fix description: what changed and why this addresses the root cause.
- Test results: the new regression test passes; the full suite is green.
- Any related risks: does the fix affect other call sites?

## When human judgment is needed

- **Race conditions**: timing-dependent bugs may need a design change, not a one-line fix.
- **OTP lifecycle bugs**: crashes during init or terminate may indicate a supervision-tree design issue.
- **Cross-process defects**: when the root cause is in a different process than the symptom, confirm the fix location with a human.
- **Behavior change required**: if the fix changes observable behavior, switch to the implementation workflow and document the delta.

## Common defect classes

- **Wrong return value**: a function returns the wrong shape or value. Isolate with ExUnit assertions; check pattern-match clauses and error-tuple handling.
- **Crash / exception**: an exception propagates and crashes a process. Read the stack trace; classify via `error-handling.md` (which exceptions are expected vs exceptional).
- **Hang / deadlock**: a process blocks waiting for a message that never arrives. Inspect message queues with `:observer` or `Process.info(pid, :message_queue_len)`.
- **Race condition**: output depends on timing. Look for unsynchronized shared state, missing `GenServer.call`, or `Task.await` ordering; see `concurrency-processes.md`.
- **Config mismatch**: wrong value at runtime. Check config precedence in `configuration-and-runtime.md`; use `Application.get_env/2` at runtime, not compile time.
- **Memory / scheduler pressure**: slow or OOM. Inspect with `:observer`; check for large retained state or process leaks; see `beam-otp-internals.md`.

## Anti-patterns to avoid

- **Fixing the symptom**: catching an exception to silence it without addressing the cause.
- **Debugging on a stale build**: always `mix compile` before reasoning about behavior.
- **Assuming concurrency**: confirm a process is actually involved before reaching for `:sys` tracing.
- **Skipping the regression test**: a fix without a test will regress.

## Related docs

- `implementation.md` — once the root cause is fixed and behavior may need extending.
- `validation.md` — the full gate suite, run before merge.
- `../error-handling.md`, `../beam-otp-internals.md` — the primary debugging reference docs.
