---
name: beam-processes
description: |
  Operational guide for BEAM processes and concurrency — spawn/spawn_opt,
  message passing, links vs monitors, exit signals, process flags, registration,
  process dictionary, and lifecycle. Load when writing or reviewing code that
  spawns processes, sends/receives messages, or uses links/monitors. Does NOT
  cover `gen_server`/`gen_statem` behaviours (see `beam-gen-server` and
  `beam-gen-statem`), supervision trees (see `beam-supervision`), or deep
  observability tooling (see `beam-observability-debugging`).
---

## Triggers

- Writing or reviewing code that uses `spawn`, `spawn_opt`, `send`, or `receive`.
- Reasoning about links vs monitors, exit signals, and `trap_exit`.
- Configuring process flags, priorities, heap sizes, or message-queue layout.
- Working with ports, timers, or distributed Erlang primitives.

## References

- `docs/beam/processes-and-messages.md`
  - https://www.erlang.org/doc/system/ref_man_processes.html
  - https://www.erlang.org/doc/apps/erts/erlang.html
- `docs/beam/timers.md`
  - https://www.erlang.org/doc/apps/stdlib/timer.html
  - https://www.erlang.org/doc/system/commoncaveats.html
  - https://www.erlang.org/doc/apps/erts/erlang.html
- `docs/beam/ports-io.md`
  - https://www.erlang.org/doc/system/ports.html
  - https://www.erlang.org/doc/apps/erts/erlang.html
- `docs/beam/distribution.md`
  - https://www.erlang.org/doc/system/distributed.html
  - https://www.erlang.org/doc/apps/kernel/global.html
  - https://www.erlang.org/doc/apps/kernel/net_kernel.html
  - https://www.erlang.org/doc/apps/kernel/erpc.html

## Key Rules

- `spawn/1,3` and `spawn_opt/2,3,4,5` options include `link`, `monitor`,
  `{priority, low|normal|high|max}`, `{fullsweep_after, n}`, `{min_heap_size, n}`,
  `{max_heap_size, bytes}`, `{message_queue_data, on_heap|off_heap}`,
  `{min_bin_vheap_size, n}`. `link` and `monitor` cannot be combined.
- Message passing: `Pid ! Msg` / `erlang:send/2`; `receive ... after ... end`;
  selective receive scans the mailbox in arrival order and non-matching messages
  stay in place.
- Messages are COPIED from the sender heap into the receiver mailbox; shared
  off-heap binaries pass by reference.
- `message_queue_data`: `on_heap` is the default (sender takes the receiver lock);
  `off_heap` lets the sender write to a heap fragment without the lock (less
  contention, more memory). Measure before switching.
- Links are bidirectional; at most one link exists between two processes. Created
  by `link/1` or `spawn_link`. On termination a process sends an exit signal to
  all linked processes. `link/1` raises `noproc` if the target does not exist and
  the caller is not trapping exits.
- Monitors are unidirectional; `erlang:monitor(process, Pid)` returns a `Ref`. On
  termination the monitoring process receives
  `{'DOWN', Ref, process, Pid, Reason}`; monitoring a dead/non-existent process
  delivers `noproc`. Use `erlang:demonitor(Ref, [flush])`. Repeated monitors are
  independent.
- Process aliases (OTP 24+): `alias/0,1`, `unalias/1`, and `{alias, _}` options on
  `monitor/3` / `spawn_opt/5`. You cannot create an alias for another process,
  look one up, or check whether an alias is still active.
- Process flags: `trap_exit`, `priority`, `save_calls`, `sensitive`,
  `message_queue_data`, `min_heap_size`, `max_heap_size`, `fullsweep_after`,
  `async_dist` (OTP 23+).
- Registration: `register/2`, `whereis/1`, `unregister/1`; names are atoms.
  Reserved names include `nil`, `false`, `true`, `undefined`. Use a `:via` module
  for dynamic names.
- Process dictionary: `put/2`, `get/0,1`, `erase/0,1`; discouraged for application
  state because it is invisible to `sys` and bypasses behaviours.
- Process states: `free`, `runnable`, `waiting`, `running`, `exiting`, `garbing`,
  `suspended`. `suspended` is ONLY for debugging.
- Reductions: `CONTEXT_REDS = 4000` per process; when exhausted the process is
  preempted. Long-running computations must yield.
- Signal ordering: signals from a given sender to a given receiver are delivered
  in order; there is NO global ordering across senders.
- Exit/`DOWN` signals are not sent until all directly visible resources
  (registered names, ETS tables) are released; a pid cannot be reused until
  everything is released.
- Timers: prefer `erlang:send_after/3` and `erlang:start_timer/3` over
  `timer:send_after/3` (single server process that can bottleneck). `timer:tc/3`
  and `timer:sleep/1` do not use the timer server. Use `erlang:cancel_timer/1`
  and `erlang:read_timer/1`.
- Ports: `erlang:open_port/2` with `{spawn, Command}`, `{packet, N}`, and
  `binary` are sourced. Ports communicate via messages: send
  `{Pid, {command, Data}}`, receive `{Port, {data, Data}}` and `{Port, closed}`.
  (Other `open_port/2` options are not covered here.)
- Distribution: nodes run `net_kernel`; use `-setcookie` for the magic cookie.
  `node/0`, `nodes/0`; `global` for cluster-wide names; prefer `erpc:call/4`
  over `rpc:call/4` for remote calls on OTP 23+ peers.

## Quick Commands

```erl
spawn_link(fun() -> ... end).
spawn_opt(fun() -> ... end, [link, {priority, high}]).
erlang:monitor(process, Pid).
erlang:demonitor(Ref, [flush]).
process_flag(trap_exit, true).
process_info(Pid, [reductions, message_queue_len, heap_size, status]).
erlang:processes().
```

## Anti-patterns

- Process dictionary for application state.
- Dynamic atom names (atom leak).
- Raw `spawn` for supervised work (use behaviours or `proc_lib`).
- `trap_exit` casually outside supervisors or resource-owning processes.
- `message_queue_data: off_heap` without measuring contention/memory impact.
- Relying on signal ordering across different senders.
- Using the `suspended` state for flow control.
- `timer:send_after/3` under heavy load.
- `list_to_atom/1` on untrusted input.

## Related Skills

- `beam-gen-server`
- `beam-gen-statem`
- `beam-supervision`
- `beam-errors-failures`
- `beam-observability-debugging`
