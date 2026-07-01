---
name: constraint-beam-failure
description: |
  Enforces BEAM failure-handling invariants during code execution — let-it-crash
  discipline, trap_exit policy, links-vs-monitors correctness, exit-reception
  rules, and exit/1 vs exit_signal/2. Load when writing or reviewing
  error-handling code in Erlang, Elixir, or Gleam-on-BEAM. Does NOT cover
  supervision tree configuration (see constraint-beam-supervision) or gen_server
  callback contracts (see beam-gen-server).
metadata:
  org.kind: constraint
---

# Constraint: BEAM Failure Handling and Exit Signals

This constraint enforces the let-it-crash philosophy and the exit-signal
propagation rules that underpin OTP fault tolerance. Violations either swallow
failures defensively (defeating supervision), confuse return values with crashes,
or mishandle `kill`/`trap_exit`/monitor semantics.

## Triggers

Load this skill when:

- Writing or reviewing `try/catch`, `exit/1`, `exit_signal/2`, or
  `process_flag(trap_exit, ...)`.
- Choosing between links and monitors for failure detection.
- Debugging an exit-signal cascade or a supervisor restart loop.
- Deciding between an error tuple, an exception, and letting a process crash.

## Rules

1. Let it crash: processes should crash on unexpected input instead of defensive
   programming; supervisors restart them. Fault tolerance comes from isolation
   plus restart, not from catching every error.
2. Do NOT catch exceptions to silence them without fixing the underlying cause.
3. `trap_exit` only in supervisors or resource-owning processes that must clean
   up on linked-process exits; do not set it casually.
4. `exit/1` raises an exception of class `exit` in the CALLING process and stops
   it; use it to stop the current process. `exit_signal/2` (OTP 24+) sends an exit
   signal to ANOTHER process/port without establishing a link — use it (not
   `exit/2`) for new code.
5. `kill` distinction: `exit_signal(Pid, kill)` is UNTRAPPABLE (receiver
   terminates with `killed`); a `kill` signal received via a LINK CAN be trapped
   and is NOT converted to `killed`.
6. A `normal` exit signal is silently dropped unless the receiver is trapping
   exits (then it becomes `{'EXIT', From, normal}`).
7. Use monitors (unidirectional `{'DOWN', Ref, process, Pid, Reason}`) — not
   links — where unidirectional observation is needed; `noproc` is delivered
   immediately if the target did not exist.
8. Always `demonitor(Ref, [flush])` to remove a monitor and discard a queued
   `DOWN`; after `unlink/1` while trapping exits, flush a queued
   `{'EXIT', Id, _}`.
9. Do NOT confuse a crash (abnormal termination propagating exit signals) with a
   return value (even `{error, ...}` is normal control flow and does not
   propagate).
10. Treat the stacktrace as debug-only (except the `undef` guarantee that the
    first entry is the attempted `M,F,A`); never use it for control flow.

## References

- Operational skill: `beam-errors-failures`.
- Docs: `docs/beam/links-monitors-and-exits.md`, `docs/beam/common-mistakes.md`.

## Out of scope

- Supervisor/child-spec configuration — see `constraint-beam-supervision`.
- `gen_server` callback contracts — see `beam-gen-server`.
- NIF crash = VM crash — see `constraint-beam-nif-safety`.

## Violation examples

### Catching an exception to silence it

```erlang
%% FORBIDDEN: swallows the failure instead of fixing the cause
catch do_work(),   %% any error is silently ignored
```

Correct: let the process crash and be restarted by its supervisor, or handle the
specific expected error class with `try ... catch Class:Reason:St -> ...`.

### `exit/2` instead of `exit_signal/2` for new code

```erlang
%% FORBIDDEN (legacy): use exit_signal/2 for new code that signals another process
exit(WorkerPid, shutdown)
```

Correct: `exit_signal(WorkerPid, shutdown)` (OTP 24+); reserve `exit/1` for
stopping the current process.

### Expecting a link-originated `kill` to be untrappable

```erlang
%% FORBIDDEN assumption: a kill received via a LINK can be trapped; only
%% explicit exit_signal(Pid, kill) is untrappable.
```

Correct: a link-originated `kill` is converted to `{'EXIT', Sender, kill}` when
trapping and is NOT converted to `killed`; only `exit_signal/2` `kill` is
untrappable.

### Forgetting to flush a queued `DOWN` after `demonitor/1`

```erlang
%% FORBIDDEN: a DOWN may already be queued
demonitor(Ref),   %% guarantees no FUTURE down, but a queued one remains
```

Correct: `demonitor(Ref, [flush])`.

## How to check

```bash
# Dialyzer / compiler: flag defensive catch-all patterns and exit/2 usage.
# Runtime: process_info(Pid, trap_exit), process_info(Pid, links),
#          process_info(Pid, monitors), is_process_alive/1.
```

Manual review:

- No `catch Expr` that silences errors without handling them.
- `trap_exit` only in supervisors / resource owners.
- `exit_signal/2` (not `exit/2`) for new code signalling another process.
- `demonitor(Ref, [flush])` after monitoring; flush `{'EXIT', Id, _}` after
  `unlink/1` when trapping.
- Stacktrace used for debugging only.
