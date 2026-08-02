# Runtime Diagnosis Workflow

## Purpose

Step-by-step procedure for diagnosing LIVE-RUNTIME or OPERATIONAL problems in a BEAM system: memory leaks, scheduler pressure, runaway processes, mailbox growth, message-queue overload, GC pressure, and crash dumps (`erl_crash.dump`). This workflow is OPS/RUNTIME-oriented — it inspects a running or crashed system to identify operational issues. It is NOT a code-defect root-cause workflow; for code-level bugs (crash, hang, wrong output with a known reproduction), use `debugging.md` instead.

> **Distinction from debugging.md**: `debugging.md` reproduces and fixes a code defect with a known trigger. This workflow diagnoses operational symptoms (slow, OOM, overloaded, crashed VM) where the root cause may be load, configuration, resource exhaustion, or a slow leak — not necessarily a code bug.

## What happens first

1. Determine whether the system is still running, partially degraded, or fully crashed (VM dead).
2. If the VM crashed: locate and preserve `erl_crash.dump` before anything else — it contains the full state at crash time.
3. If the system is running but degraded: connect via Erlang shell (`erl -sname ... -remsh node@host`) or attach to the running node. Do NOT restart the node until diagnostic data is collected.
4. Gather baseline metrics: scheduler utilization, run-queue length, process count, memory breakdown.

## Context gathering

- Read `runtime-debugging.md` for the full tool inventory (sys, dbg, observer, process_info, system_info, statistics, crash dumps).
- Read `proc-lib-and-sys.md` for proc_lib/sys and the code module.
- Read `processes-and-messages.md` for process states, message queues, links, monitors, and process flags.
- Read `logger-and-config.md` if the issue involves log configuration or log flooding.

## Docs consulted

- `runtime-debugging.md` — sys debugging, :dbg, observer, process_info, system_info, statistics, crash dumps, reduction counting, scheduler/GC knobs.
- `proc-lib-and-sys.md` — proc_lib/sys, code module, file module, os module.
- `processes-and-messages.md` — process states, message queues, links, monitors, process flags, message_queue_data.
- `logger-and-config.md` — logger levels, handlers, filters (if log flooding or config issues).
- `ets-data.md` — table-size/ownership diagnosis, ETS inspection (`ets:i/0`, `ets:info/1`).
- `distribution.md` — node-overload, distribution-channel latency, `nodedown` diagnosis.
- `nifs.md` — dirty-scheduler saturation, NIF-related VM crashes.
- `timers.md` — timer-wheel/timer-process bottleneck diagnosis.

## Skills loaded

- `beam-observability-debugging` — the primary skill for this workflow.
- `beam-processes` — for process inspection and message-queue analysis.
- `beam-logger-config` — if the issue involves logging configuration or log flooding.

## Diagnostic tools

- **`observer:start/0`**: GUI showing process tree, per-process info (reductions, memory, message queue), application tree, ETS tables, memory allocators.
- **`erlang:processes/0`**: list all PIDs.
- **`erlang:process_info/2`**: per-process `reductions`, `heap_size`, `total_heap_size`, `message_queue_len`, `messages`, `status`, `current_function`, `garbage_collection`.
- **`erlang:system_info/1`**: `schedulers`, `schedulers_online`, `process_count`, `port_count`, `ets_count`.
- **`erlang:statistics/1`**: `reductions`, `run_queue`, `scheduler_wall_time` (enable first), `garbage_collection`, `context_switches`.
- **`erlang:memory/0`**: VM-wide breakdown (`processes`, `atom`, `binary`, `ets`, `code`, `system`).
- **`sys:get_state/1`**, **`sys:get_status/1`**: inspect gen_server/gen_statem state.
- **`sys:trace/2`**: trace system events for a specific process.
- **`:dbg`**: `dbg:tracer()`, `dbg:tp/2`, `dbg:p/2` for function-call tracing.
- **Crash dumps**: `erl_crash.dump` + `crashdump_viewer:start/0` for VM-level crashes.
- **ETS inspection**: `ets:info/1,2`, `ets:i/0`, `ets:tab2list/1`.

## Checks run

```erl
%% Baseline metrics:
erlang:system_info(process_count).
erlang:statistics(run_queue).
erlang:memory().
erlang:system_flag(scheduler_wall_time, true).

%% Find the busiest processes (by reductions or message queue):
[{P, erlang:process_info(P, reductions)} || P <- erlang:processes()].
[{P, erlang:process_info(P, message_queue_len)} || P <- erlang:processes()].

%% Inspect a specific process:
erlang:process_info(Pid, [status, current_function, heap_size, total_heap_size, message_queue_len, garbage_collection]).

%% Scheduler utilization (sample twice):
{_, T0} = erlang:statistics(scheduler_wall_time).
timer:sleep(1000).
{_, T1} = erlang:statistics(scheduler_wall_time).

%% Crash dump analysis:
crashdump_viewer:start().
```

## Process

1. **Preserve evidence**: if VM crashed, save `erl_crash.dump`. If running, collect baseline metrics before changing anything.
2. **Classify the symptom**: memory growth, scheduler saturation, mailbox growth, process leak, GC pressure, or crash.
3. **Identify the hotspot**: use reduction counting (hot loop), message_queue_len (slow consumer), heap_size (memory leak), or scheduler_wall_time (scheduler saturation).
4. **Isolate the cause**: trace the hotspot process with `sys:trace/2` or `:dbg` to see what it is doing. Check process flags, links, and monitors.
5. **Determine if it's a code bug or an operational issue**: if the code is correct but the system is overloaded, the fix may be configuration (more schedulers, message_queue_data, max_heap_size), load shedding, or scaling. If the code has a bug (infinite loop, unbounded accumulation), switch to `debugging.md`.
6. **Apply the fix or mitigation**: code fix (switch to debugging.md), configuration change (process flags, GC tuning, logger level), or operational action (restart the process, scale out).
7. **Verify**: the symptom is resolved; baseline metrics return to normal.

## Evidence reported

- Symptom: what was observed (memory growth, scheduler saturation, crash, etc.).
- Hotspot: which process(es) or resource(s) were the bottleneck.
- Root cause: code bug, configuration issue, resource exhaustion, or load.
- Fix: code change, configuration change, or operational action.
- Before/after metrics: scheduler utilization, run-queue length, process count, memory breakdown.
- Whether the issue requires a code fix (switch to `debugging.md`).

## When human judgment is needed

- **Production impact**: if the diagnosis requires attaching observer or enabling tracing on a production node, confirm the performance impact is acceptable.
- **Restart decision**: if the only mitigation is restarting a process or node, confirm data loss is acceptable.
- **Capacity planning**: if the root cause is load exceeding capacity, a scaling or capacity-planning decision is needed.
- **Code bug identified**: if the diagnosis reveals a code defect, switch to `debugging.md` for the fix.

## Common operational problem classes

- **Memory leak / heap growth**: a process's `heap_size` or `total_heap_size` grows without bound. Check for unbounded accumulation in state, missing hibernation, or a process that should have terminated. Use `erlang:garbage_collect/1` to test if it's reclaimable.
- **Scheduler pressure / run-queue growth**: `statistics(run_queue)` is high; `scheduler_wall_time` shows saturation. Check for CPU-bound NIFs, hot loops (reduction counting), or too few schedulers. If dirty NIFs are in use, check dirty scheduler utilization separately — see `nifs.md`.
- **Mailbox growth / slow consumer**: `process_info(Pid, message_queue_len)` is high. The process is not consuming messages fast enough — check for a hot loop in a callback, blocking work in `handle_call/3`, or `message_queue_data` contention.
- **Process leak**: `erlang:system_info(process_count)` grows over time. Check for processes that start but never terminate (missing `{:stop, ...}`, missing `trap_exit`, unmonitored spawned processes).
- **GC pressure**: `statistics(garbage_collection)` shows frequent collections. Check for large heap allocations, consider `fullsweep_after` tuning or hibernation for idle processes.
- **Crash dump (VM crash)**: `erl_crash.dump` contains the error reason and full process states. Common causes: out of memory, scheduler death, `erlang:halt/1`. Use `crashdump_viewer:start/0` to analyze.
- **Log flooding**: logger output overwhelms I/O. Check logger level, handler configuration, and filter rules. See `logger-and-config.md`.

## Anti-patterns to avoid

- **Restarting the node before collecting evidence**: the crash dump and live metrics are gone after a restart.
- **Enabling tracing without filtering**: `:dbg` with broad patterns floods output and impacts performance.
- **Using `sys:get_state` in production logic**: it is a debugging tool with a 5000ms timeout and suspension semantics.
- **Leaving `sys` debug on**: excessive debug handlers damage performance. Always `sys:no_debug/1` when done.
- **Confusing operational issues with code bugs**: if the code is correct but the system is overloaded, the fix is configuration or capacity, not a code change.
- **Treating symptoms without root cause**: restarting a process that immediately re-enters the bad state does not fix the underlying issue.

## Related docs

- `../runtime-debugging.md`, `../proc-lib-and-sys.md`, `../processes-and-messages.md`, `../logger-and-config.md`

## Related workflows

- `debugging.md` — if the diagnosis reveals a code-level defect.
- `validation.md` — after applying a fix, run the full gate suite.
