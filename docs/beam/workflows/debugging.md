# Debugging Workflow

## Purpose

Step-by-step procedure for debugging a BEAM code defect: a crash, a hang, or a wrong output. The agent reproduces the issue, isolates the cause, diagnoses using BEAM tooling, fixes, and verifies. This workflow is for CODE-LEVEL defects. For live-runtime/operational problems (memory leaks, scheduler pressure, crash dumps), use `runtime-diagnosis.md` instead.

## What happens first

1. Reproduce the issue: obtain a failing test, a reproduction script, or a reliable sequence of steps that triggers the defect.
2. If the issue cannot be reproduced, do not guess at a fix. Gather more information (logs, stack trace, observer state) first.
3. Record the reproduction steps; they become the regression test.

## Context gathering

- Read the error message and full stack trace. Note the module, function, arity, and line.
- Read the failing code and its callers.
- Load the relevant docs (see "Docs consulted" below) for the error class and runtime behavior.

## Docs consulted

- `links-monitors-and-exits.md` — exception classes (error/exit/throw), exit reasons, exit-signal propagation, `trap_exit`, crash vs return value.
- `processes-and-messages.md` — process linking, monitoring, message passing, selective receive, signal ordering.
- `runtime-debugging.md` — sys debugging, `:dbg` tracing, observer, `process_info`, crash dumps.
- `proc-lib-and-sys.md` — `proc_lib`/`sys` for special processes, code loading.
- `supervision.md` — restart cascades, supervisor logs, intensity/period.
- `distribution.md` — distributed call failures, `nodedown`, `noproc` on remote nodes.
- `nifs.md` — VM crashes caused by NIF segfaults (not process crashes).
- `ports-io.md` — port-related hangs (external program blocking, port data flow).

## Skills loaded

- `beam-errors-failures` — error classification and exit-signal propagation.
- `beam-processes` — process inspection, links, monitors, mailboxes.
- `beam-observability-debugging` — sys/dbg/observer/process_info tooling.
- `beam-gen-server` — only when the issue involves a `gen_server` callback or lifecycle.

## Debugging tools

- **Erlang shell**: `erl` for interactive exploration; `l(Mod)` to reload after edits.
- **`:observer`**: `observer:start()` for process count, memory, message queue length, supervision tree.
- **`:dbg`**: `dbg:tracer()` / `dbg:tp/2` / `dbg:p/2` for tracing process messages and function calls.
- **`:sys`**: `sys:get_state/1`, `sys:get_status/1`, `sys:trace/2` for `gen_server`/`gen_statem` introspection.
- **`process_info/2`**: `erlang:process_info(Pid, [message_queue_len, reductions, status, current_function])`.
- **Crash dumps**: `erl_crash.dump` + `crashdump_viewer:start()` for VM-level crashes.
- **Common Test / ExUnit**: failing test cases localize the defect.

## Checks run

```sh
# Run the failing test in isolation (Common Test):
rebar3 ct --suite=test/my_SUITE    # Erlang
# or:
mix test path/to/test.exs:42       # Elixir
# or:
gleam test                         # Gleam

# For a crash reproduction outside the test suite:
erl -pa ebin -s my_app start -eval 'my_app:repro()'
```

## Process

1. **Reproduce**: confirm the defect triggers reliably.
2. **Isolate**: narrow the trigger to the smallest input and the smallest code path.
3. **Diagnose**: identify the root cause — a wrong return value, a race, a crash, an exit-signal cascade, a config mismatch. Use the debugging tools above to inspect state.
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
- **Exit-signal cascade**: when a supervisor cascade is the symptom, the root cause may be in a different subtree.
- **Behavior change required**: if the fix changes observable behavior, switch to the implementation workflow and document the delta.

## Common defect classes

- **Wrong return value**: a callback returns the wrong shape. Check pattern-match clauses and return-tuple contracts in `gen-server.md` or `gen-statem.md`.
- **Crash / exception**: an exception propagates and crashes a process. Read the stack trace; classify via `links-monitors-and-exits.md` (error/exit/throw).
- **Hang / deadlock**: a process blocks waiting for a message that never arrives. Inspect message queues with `process_info(Pid, message_queue_len)` or `:observer`.
- **Exit-signal cascade**: a linked process dies and cascades through the supervision tree. Check `trap_exit` settings and supervisor restart strategy.
- **Race condition**: output depends on timing. Look for unsynchronized shared state, missing `call` (using `cast` instead), or message ordering assumptions.
- **Config mismatch**: wrong value at runtime. Check env precedence in `applications.md`; use `application:get_env/3` at runtime.
- **NIF-related VM crash**: a segfault in a NIF crashes the entire VM (not just a process). Check `erl_crash.dump` for the crash reason; see `nifs.md`.

## Anti-patterns to avoid

- **Fixing the symptom**: catching an exception to silence it without addressing the cause.
- **Debugging on a stale build**: always recompile before reasoning about behavior.
- **Assuming concurrency**: confirm a process is actually involved before reaching for `:sys` tracing.
- **Skipping the regression test**: a fix without a test will regress.
- **Using runtime-diagnosis for code defects**: this workflow is for code-level bugs; use `runtime-diagnosis.md` for operational/live-system problems.

## Related docs

- `../links-monitors-and-exits.md`, `../processes-and-messages.md`, `../runtime-debugging.md`, `../proc-lib-and-sys.md`, `../supervision.md`

## Related workflows

- `runtime-diagnosis.md` — for live-runtime/operational problems (memory, scheduler, crash dumps).
- `implementation.md` — once the root cause is fixed and behavior may need extending.
- `validation.md` — the full gate suite, run before merge.
