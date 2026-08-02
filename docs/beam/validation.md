# Validation

## Purpose

Cross-cutting BEAM validation hooks: compile-time gates, runtime sanity checks,
introspection assertions, and observability monitors that catch regressions early.
This doc synthesizes validation hooks from the crawled corpus — `sys`, `trace`,
introspection BIFs, ETS, timers, NIFs, and distribution — into a single
checklist. It cites crawl files for each hook and flags items referenced in the
task scope that are NOT covered by the crawled corpus as gaps.

## Sources used

- Crawl `15-sys.md` — stdlib `sys.html` — https://www.erlang.org/doc/apps/stdlib/sys.html
- Crawl `17-erlang-bifs.md` — erts `erlang.html` — https://www.erlang.org/doc/apps/erts/erlang.html
- Crawl `31-trace.md` — kernel `trace.html` — https://www.erlang.org/doc/apps/kernel/trace.html
- Crawl `24-ets.md` — stdlib `ets.html` — https://www.erlang.org/doc/apps/stdlib/ets.html
- Crawl `16-timer.md` — stdlib `timer.html` — https://www.erlang.org/doc/apps/stdlib/timer.html
- Crawl `34-commoncaveats.md` — system `commoncaveats.html` — https://www.erlang.org/doc/system/commoncaveats.html
- Crawl `25-erl-nif.md` — erts `erl_nif.html` — https://www.erlang.org/doc/apps/erts/erl_nif.html
- Crawl `33-nif-guide.md` — system `nif.html` — https://www.erlang.org/doc/system/nif.html
- Crawl `26-distributed.md` — system `distributed.html` — https://www.erlang.org/doc/system/distributed.html
- Crawl `27-global.md` — kernel `global.html` — https://www.erlang.org/doc/apps/kernel/global.html
- Crawl `28-net-kernel.md` — kernel `net_kernel.html` — https://www.erlang.org/doc/apps/kernel/net_kernel.html
- Crawl `30-erpc.md` — kernel `erpc.html` — https://www.erlang.org/doc/apps/kernel/erpc.html

> **Coverage gaps:** The following validation-relevant facilities are referenced
> in the task scope but NOT detailed in the crawled corpus: `erlc` flags
> (`+warnings_as_errors`), Dialyzer, XRef, `code:is_loaded/1`/clash detection,
> `supervisor:which_children/1`/`count_children/1`, `application:
> which_applications/0`, the Observer backend, and crash-dump analysis. They are
> standard BEAM facilities but their source-verified detail requires crawling
> `erlc`, `dialyzer`, `xref`, `code`, `supervisor`, `application`, `observer`, and
> the ERTS "Crash Dump" chapter. Revisit.

## Core guidance

### Compile-time gates (gap — not crawled)

`erlc` with `+warnings_as_errors` treats warnings as errors, and Dialyzer/XRef
provide static analysis at the BEAM level. These are referenced in the task
scope but NOT covered by the crawled corpus. Revisit: crawl `erlc`, `dialyzer`,
`xref` for source-verified flags and workflows.

### Runtime sanity checks via `sys` (crawl 15)

- `sys:get_state(Name)` / `get_state(Name, Timeout)` — assert a behaviour's
  state after a transition. "These functions are intended only to help with
  debugging."
- `sys:get_status(Name)` — assert `SysState` is `running` (not stuck
  `suspended`); inspect parent, debug opts, and `Misc`.
- `sys:statistics(Name, get)` — assert reductions/messages_in/messages_out are
  within expected bounds.
- `sys:no_debug(Name)` — ensure no lingering debug settings after a test.

### Process introspection (crawl 17)

- `process_info(Pid, status)` — assert `running`/`waiting`/`garbage_collecting`.
- `process_info(Pid, message_queue_len)` — assert mailbox is not backing up.
- `process_info(Pid, reductions)` — assert fair scheduling / cost bounds.
- `process_info(Pid, total_heap_size)` / `memory` — assert memory bounds.
- `process_info(Pid, links)` / `monitors` / `monitored_by` — assert expected
  topology.
- `is_process_alive(Pid)` — assert a process is up (with signal-ordering
  guarantee: prior signals from the caller are delivered before the check).
- `processes/0` / `ports/0` — assert process/port counts are within bounds.

### System introspection (crawl 17)

- `system_info(process_count)` / `process_limit` — process-count pressure.
- `system_info(port_count)` / `port_limit` — port-count pressure.
- `system_info(atom_count)` / `atom_limit` — atom-table pressure (default limit
  1,048,576; atoms are never GC'd — see crawl 34).
- `system_info(ets_count)` / `ets_limit` — ETS table-count pressure.
- `system_info(schedulers)` / `schedulers_online` — scheduler topology.
- `system_info(dirty_cpu_schedulers)` / `dirty_io_schedulers` — dirty scheduler
  availability for NIFs.

### Statistics and memory (crawl 17)

- `statistics(reductions)` / `exact_reductions` — reduction accounting.
- `statistics(run_queue)` / `run_queue_lengths` — run-queue backlog.
- `statistics(context_switches)` — scheduling churn.
- `statistics(garbage_collection)` — GC counts and words reclaimed.
- `statistics(io)` — I/O bytes in/out.
- `statistics(scheduler_wall_time)` — per-scheduler utilization.
- `memory(total)` / `memory(processes)` / `memory(system)` / `memory(ets)` /
  `memory(binary)` / `memory(atom)` — memory breakdown.

### Trace-based validation (crawl 31)

- `trace:system/3` (OTP 28.0) — fail-fast on `long_schedule` ("> 1 ms is
  considered a good maximum time for a driver callback or a NIF"),
  `long_message_queue`, `long_gc`, `large_heap`, `busy_port`, `busy_dist_port`.
- `trace:function/4` with `call_count`/`call_time`/`call_memory` — per-function
  profiling assertions (no process trace flags needed).
- `trace:process/4` with `silent` + match-spec `{silent, Bool}` — assert specific
  call patterns fire without flooding.
- Remember: `trace` is local-node only; `session_destroy/1` does not retract
  already-sent messages.

### ETS checks (crawl 24)

- `ets:info(Table)` / `info(Table, Item)` — assert owner, size, memory, type,
  protection.
- `ets:info(Table, safe_fixed_monotonic_time)` — assert no leaked fixations
  (time-warp safe; `safe_fixed` is NOT).
- `ets:member/2` / `lookup/2` — assert expected data.
- `ets:test_ms/2` — validate a match spec before deployment.
- `erlang:system_info(ets_count)` / `ets_limit` — table-count bounds.

### Timer checks (crawl 16, 34)

- `erlang:read_timer/1,2` — assert remaining time on a BIF timer.
- `erlang:cancel_timer/1` — assert `non_neg_integer()` (ms left) or `false`.
- `timer:tc/4` — assert a measured duration is within expected bounds.
- Avoid the timer-module bottleneck: prefer BIF timers at scale; `timer:tc/3`
  and `timer:sleep/1` do NOT use the timer server.

### NIF checks (crawl 25, 33)

- `erlang:load_nif/2` — assert `ok` (module usable) vs `{error, Reason}`.
- NIF stubs raise `erlang:nif_error/1` when the library is not loaded.
- `trace:system/3` `long_schedule` — flag NIFs exceeding ~1 ms.
- Assert dirty NIFs do not stall ordinary schedulers under load.
- "Rewriting Erlang code to a NIF to make it faster should be seen as a last
  resort" (crawl 34).

### Distribution checks (crawl 26, 27, 28, 30)

- `is_alive/0` — assert the node is distributed.
- `node/0` / `nodes/0` / `nodes(hidden)` / `nodes(connected)` — topology.
- `net_kernel:get_state()` (OTP 25.0) — distribution state.
- `net_kernel:get_net_ticktime()` — tick configuration consistency.
- `net_kernel:monitor_nodes/2` — assert `nodeup` precedes remote signals;
  `nodedown` follows them.
- `global:registered_names/0` / `whereis_name/1` — global registry consistency.
- `global:sync/0` — reconcile the name server; assert `ok`.
- `erpc:call/5` / `multicall/5` — assert per-node `{ok, _} | {Class, _}`.
- `erlang:get_cookie/0` — cookie inspection (cleartext distribution caveat).

### Common-caveat guards (crawl 34)

- Atom exhaustion: avoid `list_to_atom/1` / `binary_to_atom/1,2` on untrusted
  input; use `list_to_existing_atom/1` / `binary_to_existing_atom/1,2`. Assert
  `system_info(atom_count)` is well below `atom_limit`.
- `++` quadratic copying: keep the growing accumulator on the right.
- Loss of sharing when spawning/sending funs over records/maps: extract only
  needed fields outside the fun.
- `length/1` is O(n): prefer pattern matching in guards.
- `setelement/3` copies: prefer records or coalesced calls.
- `size/1`: prefer `tuple_size/1` / `byte_size/1` for type info.

## Practical rules

- Use `sys:get_state`/`get_status` for behaviour-state assertions in tests
  (debugging-only — not production logic).
- Use `process_info/2` (not `/1`) in non-debug validation code.
- Use `safe_fixed_monotonic_time` (not `safe_fixed`) in time-warp-safe code.
- Use `trace:system/3` monitors in CI/staging to catch `long_schedule`/
  `large_heap` regressions.
- Assert `erlang:load_nif/2` returns `ok` in NIF module tests.
- Assert `erpc` per-node results explicitly (do not assume all succeeded).
- Assert `global:sync/0` returns `ok` after cluster changes.
- Keep `atom_count` well below `atom_limit`; never convert untrusted input to
  atoms.

## Review checklist

- [ ] Are `sys` state assertions used in behaviour tests?
- [ ] Are `process_info/2` items checked for mailboxes/heap/reductions?
- [ ] Are `system_info` limits (process/port/atom/ets) monitored?
- [ ] Are `trace:system/3` monitors enabled in staging?
- [ ] Are ETS fixations released (no `safe_fixed` leaks)?
- [ ] Are NIF load results asserted in tests?
- [ ] Are `erpc`/`global` results checked per-node?
- [ ] Are atom-creation caveats enforced (existing-atom variants)?

## Implementation checklist

- [ ] Add `sys:get_state` assertions to gen_server/gen_statem test suites.
- [ ] Add `trace:system/3` long_schedule/large_heap monitors to staging config.
- [ ] Add `ets:info(_, safe_fixed_monotonic_time)` checks to table-using tests.
- [ ] Assert `erlang:load_nif/2` in every NIF module's `-on_load` test.
- [ ] Wrap `erpc:multicall/5` results with per-node `{ok,_}|{Class,_}` handling.
- [ ] Add `statistics(run_queue)` / `memory(total)` smoke checks to health probes.

## Runtime / debugging checklist

- [ ] `sys:get_status(Name)` to check a process is not stuck `suspended`.
- [ ] `process_info(Pid, backtrace)` for the call stack.
- [ ] `process_info(Pid, message_queue_len)` for overloaded mailboxes.
- [ ] `statistics(scheduler_wall_time)` for load imbalance.
- [ ] `memory(ets)` / `memory(binary)` for memory pressure.
- [ ] `ets:info(Table, safe_fixed_monotonic_time)` for fixation leaks.
- [ ] `net_kernel:get_state()` / `global:sync/0` for distribution health.

## Validation hooks

- `sys:get_state/1,2` — behaviour state assertion (crawl 15).
- `sys:get_status/1,2` — `running` vs `suspended` (crawl 15).
- `sys:statistics/2` — reductions/messages bounds (crawl 15).
- `process_info/2` — status/mailbox/heap/reductions/links (crawl 17).
- `is_process_alive/1` — liveness with signal ordering (crawl 17).
- `system_info/1` — process/port/atom/ets/scheduler limits (crawl 17).
- `statistics/1` — reductions/run_queue/GC/io/scheduler_wall_time (crawl 17).
- `memory/0,1` — total/processes/system/ets/binary/atom (crawl 17).
- `trace:system/3` — long_schedule/large_heap/busy_port (crawl 31).
- `ets:info/1,2` + `safe_fixed_monotonic_time` (crawl 24).
- `erlang:read_timer/1,2` / `cancel_timer/1` (crawl 16, 17).
- `erlang:load_nif/2` result (crawl 25, 33).
- `is_alive/0`, `nodes/0,1`, `net_kernel:get_state/0` (crawl 26, 28).
- `global:sync/0`, `global:registered_names/0` (crawl 27).
- `erpc:call/5`, `erpc:multicall/5` per-node results (crawl 30).

## Examples

Assert a gen_server's state in a test:
```erlang
?assertMatch(#{phase := ready}, sys:get_state(my_server)).
```

Catch long schedules in staging:
```erlang
{ok, S} = trace:session_create(mon, self(), []),
trace:system(S, long_schedule, 10),   %% 10 ms threshold
trace:system(S, large_heap, 10_000_000).
```

Assert no ETS fixation leaks:
```erlang
?assertEqual(false, ets:info(Tab, safe_fixed_monotonic_time)).
```

Assert NIF load:
```erlang
?assertEqual(ok, erlang:load_nif(?MODULE, 0)).
```

## Common mistakes

- Using `sys:get_state` in production logic (debugging-only).
- Using `process_info/1` (debugging-only) instead of `process_info/2`.
- Using `safe_fixed` (not time-warp safe) instead of
  `safe_fixed_monotonic_time`.
- Forgetting `trace:session_destroy/1` (settings persist).
- Assuming `erpc:multicall` succeeded on all nodes (check per-node results).
- Converting untrusted input to atoms (exhaustion DoS).

## Strict vs contextual guidance

Strict:
- `sys:get_state`/`replace_state` are debugging-only.
- `process_info/1` is debugging-only; use `/2` otherwise.
- `safe_fixed_monotonic_time` for time-warp-safe code.
- `trace` is local-node only.
- Atoms are never GC'd; use existing-atom variants on untrusted input.

Contextual:
- `trace:system/3` thresholds (long_schedule ms, large_heap words) — tune by
  workload.
- `statistics` sampling frequency in health probes.
- Whether `sys` assertions run in CI only or also in staging canaries.

## Policy decisions for individual repos

- Whether `sys:get_state` assertions are permitted in integration tests.
- `trace:system/3` thresholds for staging always-on monitors.
- Required `erlang:load_nif/2` assertion in every NIF module test.
- Atom-count alarm threshold relative to `atom_limit`.
- Whether `erpc` results must be checked per-node in all call sites.

## Related docs

- `runtime-debugging.md` — `sys`, `trace`, introspection BIFs.
- `processes-and-messages.md` — process lifecycle.
- `ets-data.md` — ETS checks.
- `timers.md` — timer validation.
- `nifs.md` — NIF validation.
- `distribution.md` — distribution validation.
- `common-mistakes.md` — common-caveat guards.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
