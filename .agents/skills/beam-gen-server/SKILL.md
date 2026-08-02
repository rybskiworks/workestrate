---
name: beam-gen-server
description: |
  Operational guide for the BEAM `gen_server` behaviour — callback contracts,
  return-tuple shapes, call/cast/reply, timeouts, hibernate, naming, and `sys`
  debugging. Load when writing, reviewing, or debugging `gen_server` modules in
  Erlang. Does NOT cover supervision trees (see `beam-supervision`), `gen_statem`
  (see `beam-gen-statem`), exit-signal propagation (see `beam-errors-failures`),
  or hand-rolled special processes (see `beam-observability-debugging`).
---

## Triggers

- Writing, reviewing, or debugging a `gen_server` callback module.
- Implementing client-server processes in Erlang.
- Using `sys` debugging or code-change on a running `gen_server`.

## References

- `docs/beam/gen-server.md`
  - https://www.erlang.org/doc/system/gen_server_concepts.html
  - https://www.erlang.org/doc/apps/stdlib/gen_server.html
- `docs/beam/proc-lib-and-sys.md`
  - https://www.erlang.org/doc/system/spec_proc.html
  - https://www.erlang.org/doc/apps/stdlib/proc_lib.html
  - https://www.erlang.org/doc/apps/stdlib/sys.html

## Key Rules

- `gen_server` models runtime properties (state, serialized access, concurrency isolation, failure containment) — never code organization.
- Client API functions run in the caller; callbacks run in the `gen_server` process.
- `init/1` returns `{ok, State}` | `{ok, State, Action}` | `{stop, Reason}` | `ignore` | `{error, Reason}`. It is synchronous and blocks `start_link`; do heavy work in `handle_continue/2`.
- `start_link/3,4`, `start/3,4`, and `start_monitor/3,4` do not return until `init/1` returns or fails.
- `{error, Reason}` from `init/1` (OTP 26.0+) exits the process with reason `normal` (silent failure); `{stop, Reason}` exits with `Reason`.
- `action()` includes `Timeout`, `hibernate`, `{timeout, Time, Message[, Options]}`, `{hibernate, Time, Message[, Options]}`, and `{continue, Continue}`.
- The legacy `Timeout` action is restarted by system messages; prefer `{timeout, Time, Message}` for reliable timers.
- `handle_call/3` returns `{reply, Reply, NewState}` | `{reply, Reply, NewState, Action}` | `{noreply, NewState}` | `{noreply, NewState, Action}` | `{stop, Reason, Reply, NewState}` | `{stop, Reason, NewState}`.
- `handle_cast/2`, `handle_info/2`, and `handle_continue/2` return `{noreply, NewState}` | `{noreply, NewState, Action}` | `{stop, Reason, NewState}`.
- `handle_info/2` is optional; the default logs unexpected `Info`, drops it, and returns `{noreply, State}`.
- Returning `{continue, Continue}` requires `handle_continue/2`; otherwise the process exits with `undef`.
- `terminate/2` (Reason, State) -> `term()`; the return value is ignored. It is NOT guaranteed on `brutal_kill`/`kill` or linked exits without `trap_exit`. Ports and sockets close automatically on exit.
- `code_change/3` (`OldVsn` | `{down, Vsn}`, State, Extra) -> `{ok, NewState}` | `{error, Reason}`.
- `format_status/1` (OTP 25.0+; `format_status/2` is deprecated) redacts `sys:get_status` output and crash logs.
- `call/2,3` is synchronous, defaults to a 5000 ms timeout, and the caller exits on `timeout` or `noproc`. `cast/2` is async, always returns `ok`, and does not verify that the server exists.
- Deferred reply: return `{noreply, NewState}` from `handle_call/3` and later call `gen_server:reply(From, Reply)`. The caller waits forever if the reply is never sent.
- A callback timeout schedules a `timeout` message to `handle_info/2` only if no other message arrives first; it is not guaranteed even at 0 ms. Use `{continue, Continue}` for immediate follow-up work.
- `gen_server` does NOT trap exits automatically; call `process_flag(trap_exit, true)` in `init/1` if cleanup-on-exit is required.
- Bad return values terminate the process. `throw` is a valid return from callbacks, not an error.
- Naming forms: atom (local), `{global, term()}`, `{via, Module, term()}`. Do NOT use dynamic atoms (atom-table leak).
- `start_link/3,4` links the server (use under supervision); `start/3,4` is unlinked (ad-hoc); `start_monitor/3,4` is standalone with a monitor.
- `sys` debug helpers include `get_state/1,2`, `get_status/1,2`, `trace/2`, `statistics/2`, `no_debug/1,2`, `suspend/1,2`, `resume/1,2`, and `replace_state/2,3`. Default timeout is 5000 ms; on timeout the caller exits with `exit({timeout, {M,F,A}})`. Disable debug in production.

## Quick Commands

```erl
gen_server:start_link({local, my_srv}, my_srv, [], []).
gen_server:call(my_srv, {get, Key}).
gen_server:cast(my_srv, {set, Key, Value}).
gen_server:reply(From, Reply).
sys:get_state(my_srv).
sys:get_status(my_srv).
sys:statistics(my_srv, get).
sys:trace(my_srv, true).
sys:no_debug(my_srv).
sys:suspend(my_srv).
sys:resume(my_srv).
gen_server:stop(my_srv).
```

## Anti-patterns

- Using `gen_server` for pure functions or code organization.
- Heavy work in `init/1` blocking supervisor startup.
- Blocking I/O or long work in `handle_call/3` while callers wait.
- Missing `handle_info/2` so unexpected messages fill the mailbox.
- Returning `{noreply, State}` from `handle_call/3` without a reply path.
- Dynamic atom names (atom-table exhaustion).
- Calling `receive` inside callbacks (the `gen_server` owns the mailbox).
- Using a `timeout: 0` action to mean "run immediately".
- Aggressive hibernation without memory evidence.
- Using `call/2,3` for fire-and-forget work; use `cast/2` instead.
- Relying on `terminate/2` for cleanup without `trap_exit` in `init/1`.
- Using deprecated `format_status/2` instead of `format_status/1`.

## Related Skills

- `beam-supervision`
- `beam-gen-statem`
- `beam-errors-failures`
- `beam-processes`
- `beam-observability-debugging`
