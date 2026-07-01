---
name: beam-observability-debugging
description: |
  Operational guide for BEAM observability and runtime debugging — `sys` inspection,
  `trace` tracing, introspection BIFs, ETS, NIFs, and validation hooks. Load when
  diagnosing a crash, hang, memory leak, hot loop, or scheduler pressure. Does NOT
  cover gen_server callback contracts (see `beam-gen-server`) or error-handling
  semantics (see `beam-errors-failures`).
---

## Triggers

- Debugging a live BEAM system: hangs, crashes, memory leaks, hot loops, mailbox backlog.
- Inspecting process state, tracing calls/messages, or sampling scheduler/memory stats.
- Choosing between ad-hoc tracing and persistent logging.

## References

- `docs/beam/runtime-debugging.md`
  - https://www.erlang.org/doc/apps/stdlib/sys.html
  - https://www.erlang.org/doc/apps/erts/erlang.html
  - https://www.erlang.org/doc/apps/kernel/trace.html
- `docs/beam/proc-lib-and-sys.md`
  - https://www.erlang.org/doc/system/spec_proc.html
  - https://www.erlang.org/doc/apps/stdlib/proc_lib.html
  - https://www.erlang.org/doc/apps/stdlib/sys.html
- `docs/beam/ets-data.md`
  - https://www.erlang.org/doc/apps/stdlib/ets.html
- `docs/beam/nifs.md`
  - https://www.erlang.org/doc/apps/erts/erl_nif.html
  - https://www.erlang.org/doc/system/nif.html
- `docs/beam/validation.md`
  - https://www.erlang.org/doc/apps/stdlib/sys.html
  - https://www.erlang.org/doc/apps/erts/erlang.html
  - https://www.erlang.org/doc/apps/kernel/trace.html
  - https://www.erlang.org/doc/apps/stdlib/ets.html
  - https://www.erlang.org/doc/apps/stdlib/timer.html
  - https://www.erlang.org/doc/system/commoncaveats.html
  - https://www.erlang.org/doc/apps/erts/erl_nif.html
  - https://www.erlang.org/doc/system/nif.html
  - https://www.erlang.org/doc/system/distributed.html
  - https://www.erlang.org/doc/apps/kernel/global.html
  - https://www.erlang.org/doc/apps/kernel/net_kernel.html
  - https://www.erlang.org/doc/apps/kernel/erpc.html

## Key Rules

- `sys` debugging functions (default timeout 5000 ms; on timeout caller exits with `exit({timeout, {M,F,A}})`):
  - `sys:get_state/1` — callback state (`gen_server` → State; `gen_statem` → `{State, Data}`).
  - `sys:get_status/1` — full status `{status, Pid, {module, Module}, [Items]}`.
  - `sys:trace/2` — print all system events to `standard_io`.
  - `sys:statistics/2` — `get` returns `start_time`, `current_time`, `reductions`, `messages_in`, `messages_out`.
  - `sys:no_debug/1` — remove ALL debug functions (must do when done — performance damage).
  - `sys:suspend/1` / `sys:resume/1` — suspend/resume the process loop.
  - `sys:replace_state/2` — replace state via `StateFun` (if `StateFun` crashes, original unchanged).
  - `sys:install/2` / `sys:remove/2` — install/remove debug functions.
- `sys:change_code/4,5` requires the process to be suspended first. `sys:get_state` and `sys:replace_state` are debugging-only.
- Tracing: the `trace` module (OTP 27+) is the source-verified local-node API. `trace:session_create/3`, `trace:process/4`, `trace:function/4`, `trace:send/3`, `trace:recv/3`, `trace:system/3`, `session_destroy/1`. `function/4` call tracing also needs the `call` flag via `process/4`; `send/3`/`recv/3` need `send`/`'receive'` flags. `trace` is local-node only. (The docs note `dbg` as a higher-level tracer, but its API is not detailed in the crawled corpus.)
- `observer:start/0` is the standard GUI for process trees, per-process info, applications, ETS, allocators, and tracing (Observer backend is not detailed in the crawled corpus).
- `process_info/2` key items: `reductions`, `heap_size`, `total_heap_size`, `stack_size`, `message_queue_len`, `messages`, `links`, `monitors`, `monitored_by`, `priority`, `trap_exit`, `status`, `current_function`, `initial_call`, `garbage_collection`, `dictionary`, `registered_name`.
- `system_info/1`: `schedulers`, `schedulers_online`, `dirty_cpu_schedulers`, `dirty_io_schedulers`, `process_count`, `port_count`, `ets_count`, `logical_processors`, `atom_count`, `atom_limit`.
- `statistics/1`: `reductions` → `{Total, SinceLast}`, `run_queue` → total run-queue length, `scheduler_wall_time` (enable via `erlang:system_flag(scheduler_wall_time, true)` first), `garbage_collection`, `context_switches`.
- Crash dumps (`erl_crash.dump`) are written on VM crash; `crashdump_viewer:start/0` parses/visualizes them. Crash-dump analysis is not detailed in the crawled corpus.
- Message queue inspection: `process_info(Pid, message_queue_len)` and `process_info(Pid, messages)` — a growing queue indicates a slow consumer or hot loop.
- Reduction counting for hot loops: sample `process_info(Pid, reductions)` twice with a sleep between; fastest-growing process is likely in a hot loop.
- `erlang:memory/0` — VM-wide breakdown: `processes`, `atom`, `binary`, `ets`, `code`, `system`.
- ETS: `ets:new/2`, `ets:insert/2`, `ets:lookup/2`, `ets:match/2`, `ets:match_object/2`, `ets:select/2`, `ets:info/1,2`, `ets:tab2list/1`. Types: `set`, `ordered_set`, `bag`, `duplicate_bag`. Access: `public`, `protected`, `private`.
- NIFs: last-resort optimization. Too much work per call degrades VM responsiveness; long-running NIFs must yield or run dirty (`erts/erl_nif.html#lengthy_work`). Load with `erlang:load_nif/2`.
- Validation combines `sys` tracing, `process_info`, `system_info`, `statistics`, ETS inspection, timer audit, common-caveat checks, NIF review, and distribution checks. Coverage gaps in the crawled corpus include `erlc` flags, Dialyzer, XRef, `code:is_loaded/1`, `supervisor:which_children/1`/`count_children/1`, `application:which_applications/0`, Observer backend internals, and crash-dump analysis.
- Tracing vs logging: use `trace`/`sys:trace` for ad-hoc runtime diagnosis with low overhead when filtered; use `logger` for persistent structured events. Disable tracing when not actively debugging.

## Quick Commands

```erl
observer:start().
sys:get_state(Pid).
sys:trace(Pid, true).
sys:no_debug(Pid).
process_info(Pid, [reductions, message_queue_len, heap_size, status]).
erlang:system_info(schedulers_online).
erlang:statistics(run_queue).
erlang:memory().
{ok,S} = trace:session_create(dbg, self(), []),
trace:process(S, all, true, [call, silent]),
trace:function(S, {m, '_', '_'}, [{['_'],[],[{silent,false}]}], [local]).
crashdump_viewer:start().
```

## Anti-patterns

- Leaving `sys` debug/trace on in production.
- Using `sys:get_state` in production logic.
- Unfiltered `trace`/`dbg` tracing (output flood).
- Calling `which_children/1` on a supervisor with millions of children under memory pressure.
- Tracing when logging would suffice.
- Not enabling `scheduler_wall_time` before sampling.
- Treating NIFs as a default optimization path.

## Related Skills

- `beam-processes`
- `beam-gen-server`
- `beam-errors-failures`
- `beam-logger-config`
