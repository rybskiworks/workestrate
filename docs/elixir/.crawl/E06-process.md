# Crawl: hexdocs.pm/elixir/Process.html
- seed_url: https://hexdocs.pm/elixir/Process.html
- canonical_url: https://elixir.hexdocs.pm/Process.html
- family: Elixir core module
- fetch: 200
- elixir_version: v1.20.2
- feeds_docs: concurrency-processes.md

## Purpose
`Process` provides conveniences for working with processes and the process dictionary.
It is a thin Elixir wrapper over Erlang BIFs (`:erlang.*`), almost every function is
"Inlined by the compiler" (i.e. it compiles directly to the corresponding BIF call with
no runtime function-call overhead).

The module doc explicitly states developers typically use abstractions such as
`Agent`, `GenServer`, `Registry`, `Supervisor`, and `Task` for building systems and
resort to `Process` for gathering information, trapping exits, links, and monitoring.

Kernel auto-imports basic process functions: `Kernel.spawn/1,3`,
`Kernel.spawn_link/1,3`, `Kernel.spawn_monitor/1,3`, `Kernel.self/0`, `Kernel.send/2`.

Aliases (Erlang/OTP 24+): a way to refer to a PID that can be deactivated even while the
aliased process still runs. Messages to a deactivated alias are silently dropped.
`alias/0`, `alias/1`, `unalias/1`. Useful for request/response scenarios.

## spawn/send/alive/list
- `spawn(fun, opts)` / `spawn(mod, fun, args, opts)` — spawns with options; returns
  `pid()` or `{pid, reference}` when `:monitor` option given. Delegates to
  `:erlang.spawn_opt/4`.
- `send(dest, msg, options)` — sends to a PID/port/registered name/`{name, node}`.
  Options: `:noconnect` (don't auto-connect to remote node), `:nosuspend` (don't block
  sender). Returns `:ok | :noconnect | :nosuspend`. Inlined.
- `send_after(dest, msg, time, opts \\ [])` — schedules `msg` to `dest` after `time` ms.
  Returns a timer reference readable via `read_timer/1` and cancellable via
  `cancel_timer/1`. `dest` PID must be local (dead or alive); if atom, name resolved at
  delivery time. Timer auto-cancels if `dest` PID exits (NOT when dest is an atom).
  Option `:abs` treats `time` as absolute Erlang monotonic time.
- `read_timer(timer_ref)` — ms left until expiry, or `false` if not found.
- `cancel_timer(timer_ref, options \\ [])` — returns ms left, `false`, or `:ok`.
  Options `:async` (default false), `:info` (default true). With `:async` true +
  `:info` true, sends `{:cancel_timer, timer_ref, result}` message.
- `alive?(pid)` — whether process alive on local node. Inlined (`:erlang.is_process_alive`).
- `list()` — PIDs of all processes on local node. A process that is exiting is
  considered to exist but not be alive (so `alive?/1` returns false but PID is in
  `list/0`). Delegates to `:erlang.processes/0`.
- `sleep(timeout)` — `:infinity` or ms. Since v1.18 accepts arbitrarily high integers
  (previously capped at 2^32-1). Discouraged; prefer message passing / `Task.await` /
  `monitor`.

## link/monitor/demonitor + exit/trap_exit model
- `link(pid_or_port)` — bidirectional link. Only one link between two processes. Linking
  to self does nothing. Linked processes receive each other's exit signals. Delegates to
  `:erlang.link/1`.
- `unlink(pid_or_port)` — removes link; no-op if none; always returns `true`. Delegates
  to `:erlang.unlink/1`.
- `monitor(item)` / `monitor(item, options)` — starts monitoring; returns a reference.
  On monitored process death, delivers `{:DOWN, ref, :process, object, reason}` where
  `object` is the pid or `{name, node}`. If already dead, `:DOWN` delivered immediately.
  `monitor/2` (since 1.15.0) accepts options delegating to `:erlang.monitor/3`, e.g.
  `alias: :reply_demonitor` returns a reference that is also an alias.
- `demonitor(monitor_ref, options \\ [])` — turns off monitoring; options `[:flush, :info]`.
  Returns boolean. Delegates to `:erlang.demonitor/2`.
- `exit(pid, reason)` — sends an exit signal to `pid`. Returns `true`. Behavior by reason:
  - `:normal` — pid does not exit unless pid == caller (caller exits `:normal`); if
    trapping exits, signal becomes `{:EXIT, from, :normal}` message.
  - `:kill` — untrappable exit signal; pid unconditionally exits with reason `:killed`.
  - any other term — if not trapping exits, pid exits with that reason; if trapping,
    signal becomes `{:EXIT, from, reason}` message.
  Distinct from `Kernel.exit/1` (stops current process, catchable with `try/1`).
  `Process.exit/2` sends a signal to another process; only handleable via trap_exit and
  not when reason is `:kill`.
- `flag(:trap_exit, boolean)` — enables/disables exit-signal trapping for the calling
  process. Returns old value.

## flag/get_keys, register/whereis, info, process dictionary, hibernate
- `flag(flag, value)` — sets process flag for calling process. Flags: `:error_handler`,
  `:max_heap_size`, `:message_queue_data` (`:off_heap | :on_heap`), `:min_bin_vheap_size`,
  `:min_heap_size`, `:priority`, `:save_calls` (0..10000), `:sensitive`, `:trap_exit`.
  Delegates to `:erlang.process_flag/2`.
- `flag(pid, flag, value)` — sets flag for given pid; only `:save_calls` allowed; raises
  `ArgumentError` if pid not local. Delegates to `:erlang.process_flag/3`.
- Process dictionary:
  - `put(key, value)` — stores; returns previous value or nil.
  - `get(key, default \\ nil)` — returns value or default.
  - `get()` — all `{key, value}` pairs.
  - `get_keys()` — all keys.
  - `get_keys(value)` — keys with given value.
  - `delete(key)` — deletes; returns deleted value or nil.
- `register(pid_or_port, name)` — registers under atom name. Fails with `ArgumentError`
  if not alive locally, name already registered, or pid already registered under another
  name. Reserved names: `nil`, `false`, `true`, `:undefined`.
- `unregister(name)` — removes registration; raises `ArgumentError` if not registered.
- `registered()` — list of registered names.
- `whereis(name)` — returns pid/port or nil.
- `info(pid)` / `info(pid, spec)` — process info for debugging; nil if not alive.
  Delegates to `:erlang.process_info/1,2`.
- `group_leader()` / `group_leader(pid, leader)` — get/set group leader.
- `hibernate(mod, fun_name, args)` — puts calling process into hibernation (memory
  reduced); `no_return()`. Delegates to `:erlang.hibernate/3`.
- `set_label(label)` (since 1.17.0) / `get_label(pid \\ self())` (since 1.20.0) —
  descriptive term shown in Observer and crash logs.
- `alias()` / `alias(options)` / `unalias(alias)` — process aliases (OTP 24+).

## Process ↔ BEAM erlang BIFs mapping (→ docs/beam/processes-and-messages.md, links-monitors-and-exits.md)
Nearly every `Process` function is "Inlined by the compiler" and maps 1:1 to an
`:erlang.*` BIF:

| Elixir Process | Erlang BIF |
|---|---|
| `spawn/2,4` | `:erlang.spawn_opt/4` |
| `send/3` | `:erlang.send/3` |
| `send_after/4` | `:erlang.send_after/3,4` (via timer BIFs) |
| `read_timer/1` | `:erlang.read_timer/1` |
| `cancel_timer/2` | `:erlang.cancel_timer/2` |
| `alive?/1` | `:erlang.is_process_alive/1` |
| `list/0` | `:erlang.processes/0` |
| `link/1` | `:erlang.link/1` |
| `unlink/1` | `:erlang.unlink/1` |
| `monitor/1,2` | `:erlang.monitor/2,3` |
| `demonitor/2` | `:erlang.demonitor/2` |
| `exit/2` | `:erlang.exit/2` |
| `flag/2,3` | `:erlang.process_flag/2,3` |
| `put/2`, `get/0,1,2`, `get_keys/0,1`, `delete/1` | `:erlang.put/2`, `:erlang.get/1,2`, `:erlang.get_keys/0,1`, `:erlang.erase/1` |
| `register/2`, `unregister/1`, `registered/0`, `whereis/1` | `:erlang.register/2`, `:erlang.unregister/1`, `:erlang.registered/0`, `:erlang.whereis/1` |
| `info/1,2` | `:erlang.process_info/1,2` |
| `group_leader/0,2` | `:erlang.group_leader/0,2` |
| `hibernate/3` | `:erlang.hibernate/3` |
| `alias/0,1`, `unalias/1` | `:erlang.alias/0,1`, `:erlang.unalias/1` |
| `set_label/1`, `get_label/1` | `:proc_lib.set_label/1`, `:proc_lib.get_label/1` |

Exit-signal model (cross-ref `docs/beam/links-monitors-and-exits.md`):
- Links are bidirectional; an abnormal exit propagates to linked processes unless they
  trap exits.
- Monitors are unidirectional; deliver `{:DOWN, ref, :process, object, reason}`.
- `:kill` is untrappable; all other reasons can be caught via `trap_exit` (delivered as
  `{:EXIT, from, reason}` messages).
- `:normal` reason does not propagate via links to non-trapping processes.

## Strict rules
- `Process.exit/2` sends a signal to ANOTHER process; `Kernel.exit/1` stops the CURRENT
  process (catchable with `try/1`). Do not confuse them.
- `:kill` exit signal is untrappable — cannot be handled even with `trap_exit`.
- `send_after/4` timer is NOT auto-canceled when `dest` is an atom (resolution happens at
  delivery); it IS auto-canceled when `dest` is a PID that exits.
- `register/2` reserved names: `nil`, `false`, `true`, `:undefined`.
- `flag/3` (per-pid) only allows `:save_calls`; raises `ArgumentError` for non-local pid.
- `list/0` includes exiting processes (alive? returns false but PID still listed).
- `sleep/1` is discouraged; prefer message passing, `Task.await`, or `monitor`.
- `info/1,2` is for debugging only.
- `hibernate/3` returns `no_return()` — the process is resumed only on next message.

## Verbatim quotes
- "Conveniences for working with processes and the process dictionary."
- "developers typically use abstractions such as Agent , GenServer , Registry ,
  Supervisor and Task for building their systems and resort to this module for gathering
  information, trapping exits, links and monitoring."
- "Aliases are a feature introduced in Erlang/OTP 24. An alias is a way to refer to a
  PID in order to send messages to it. The advantage of using aliases is that they can
  be deactivated even if the aliased process is still running."
- "If :kill , which occurs when Process.exit(pid, :kill) is called, an untrappable exit
  signal is sent to pid which will unconditionally exit with reason :killed ."
- "The functions Kernel.exit/1 and Process.exit/2 are named similarly but provide very
  different functionalities."
- "Links are bidirectional."
- "Once the monitored process dies, a message is delivered to the monitoring process in
  the shape of: {:DOWN, ref, :process, object, reason}"
- "If the process is already dead when calling Process.monitor/1 , a :DOWN message is
  delivered immediately."
- "Note that if a process is exiting, it is considered to exist but not be alive."
- "The timer will be automatically canceled if the given dest is a PID which is not alive
  or when the given PID exits. Note that timers will not be automatically canceled when
  dest is an atom (as the atom resolution is done on delivery)."
- "Use this function with extreme care . For almost all situations where you would use
  sleep/1 in Elixir, there is likely a more correct, faster and precise way of
  achieving the same with message passing."

## Version notes
- Page built with ExDoc v0.40.3 for Elixir v1.20.2.
- `monitor/2` (options variant): since 1.15.0.
- `alias/0,1`, `unalias/1`: Erlang/OTP 24+ (Elixir exposes them).
- `set_label/1`: since 1.17.0.
- `get_label/1`: since 1.20.0.
- `sleep/1` accepts arbitrarily high integers since v1.18 (previously capped at 2^32-1).

## Discovered links
### Relevant (crawl later)
- Kernel.spawn/1,3 / spawn_link/1,3 / spawn_monitor/1,3 / self/0 / send/2 (elixir Kernel)
- Agent, GenServer, Registry, Supervisor, Task modules
- Erlang reference manual: Process Aliases section
- :erlang.spawn_opt/4, :erlang.monitor/2,3, :erlang.demonitor/2, :erlang.process_flag/2,3,
  :erlang.process_info/1,2, :erlang.hibernate/3, :erlang.link/1, :erlang.unlink/1,
  :erlang.exit/2, :erlang.processes/0, :erlang.whereis/1, :erlang.register/2,
  :erlang.unregister/1, :erlang.registered/0
- :proc_lib.set_label/1, :proc_lib.get_label/1
- System module (Erlang monotonic time / time concepts)

### Skipped
- ExDoc / ePub / llms.txt / package docs / search UI links
- "View Source" GitHub link
- Settings / project switcher UI
