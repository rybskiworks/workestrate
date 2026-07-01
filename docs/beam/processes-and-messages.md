# Processes and Messages

## Purpose
Define the BEAM process abstraction: creation (`spawn`/`spawn_opt` + options),
message passing (`!`/`send`/`receive`/selective/`after`), registration, process
flags, process states, reductions/preemption, and signal ordering. This is the
language-level concurrency reference.

## Sources used
- Crawl file: `crawl/06-ref-man-processes.md` — canonical URL: `https://www.erlang.org/doc/system/ref_man_processes.html`
- Crawl file: `crawl/17-erlang-bifs.md` — canonical URL: `https://www.erlang.org/doc/apps/erts/erlang.html`
- Documented OTP version: OTP 29.0.2 (ERTS v17.0.2)

## Core guidance
An **Erlang process** is "a lightweight, isolated execution entity with its
own memory, message queue, and process dictionary." (06) A receiver is
identified by a **PID**, a **registered name**, or a **process alias**.

**Signals** are the universal mechanism: "All communication between Erlang
processes and Erlang ports is done by sending and receiving asynchronous
signals." (06) Signal types include `message`, `link`, `unlink`, `exit`,
`monitor`, `demonitor`, `down`, `change`, `group_leader`,
`spawn_request`/`spawn_reply`, `alive_request`/`alive_reply`, and port signals.

**Signal reception is automatic:** "Signals are received asynchronously and
automatically. There is nothing a process must do to handle the reception of
signals, or can do to prevent it. In particular, signal reception is *not* tied
to the execution of a `receive` expression, but can happen anywhere in the
execution flow of a process." (06)

**Signal ordering guarantee:** "if an entity sends multiple signals to the same
destination entity, the order is preserved; that is, if `A` sends a signal `S1`
to `B`, and later sends signal `S2` to `B`, `S1` is guaranteed not to arrive
after `S2`. Note that `S1` may or may not have been lost." (06) There is no
guarantee on delivery time, and signals to a terminated receiver do not arrive.

**Runtime-services caveat:** "Since each service consists of multiple
independently executing entities, the order between multiple signals sent from
one service to one process is *not* preserved. Note that this does *not* violate
the signal ordering guarantee of the language." (06)

## Practical rules

### Process creation
A process is created via `spawn/3` (or `spawn/1,2,3,4`, `spawn_link/1,2,3,4`,
`spawn_monitor/1,2,3,4`, `spawn_opt/2,3,4,5`, `spawn_request/1,2,3,4,5`). The
new process "begins execution in `Module:Name(Arg1,...,ArgN)`." (06) "With
`spawn_link`, `spawn_opt`, and `spawn_request`, the spawn and link operations
are performed atomically." (06)

`spawn_opt_option()` (the full option set, from 17):
```
link | {link, [link_option()]} |
monitor | {monitor, [monitor_option()]} |
{priority, Level :: priority_level()} |   %% low | normal | high | max
{fullsweep_after, Number :: non_neg_integer()} |
{min_heap_size, Size :: non_neg_integer()} |
{min_bin_vheap_size, VSize :: non_neg_integer()} |
{max_heap_size, Size :: max_heap_size()} |
{message_queue_data, MQD :: message_queue_data()} |  %% on_heap | off_heap
{async_dist, Enabled :: boolean()}
```
- `monitor` is **disallowed together with `link`** — raises `badarg`. (17)
- `{priority, Level}` sets priority before first execution (equivalent to
  `process_flag(priority, Level)` in the start function). (17)
- If `Node` does not exist, `spawn/2,4`, `spawn_link/2,4`, `spawn_opt/3,5`
  return "a useless pid" (and `spawn_link`/`spawn_opt` send a `noconnection`
  exit signal). (17)

### Message passing
- `Dest ! Msg` is `erlang:send(Dest, Msg)`; returns `Msg`. (17)
- `send_destination()` = `pid() | reference() | port() | atom() | {atom(), node()}`. (17)
- `send/2` "fails with a badarg run-time error if `Dest` is an atom name, but
  this name is not registered. This is the only case when send fails for an
  unreachable destination." (17)
- `send/3` options: `nosuspend | noconnect | priority` (priority since OTP 28).
  (17)
- `send_nosuspend/2,3` returns `true`/`false` instead of blocking — "Use with
  extreme care." (17)

**`receive` semantics:** "A `receive` expression scans the message queue from
the start and selects the first matching message." (06) Without priority
messages, "every message is appended to the end of the message queue, preserving
the order in which the corresponding signals were received. Because signal
ordering is preserved per sender, messages from the same sender retain send
order." (06)

**Priority messages** (OTP 28+): "accepted priority messages are inserted
after the last accepted priority message, while ordinary messages remain at the
end. Priority messages do **not** violate signal ordering ... A `receive` still
scans from the start and cannot distinguish priority from ordinary messages."
(06) "You *very seldom* need to resort to usage of priority messages. Receiving
processes have *not* been optimized for handling large amounts of priority
messages." (06)

**Blocking over distribution:** "When sending a signal over a distribution
channel, the sending process may be suspended ... due to the built-in flow
control ... When the size of the output buffer ... reaches the *distribution
buffer busy limit*, processes sending on the channel will be suspended." (06)

### Registration
- `register(Name, PidOrPort)` — `Name` must be an atom; "automatically
  unregistered when the process terminates." (06, 17) The registered name is a
  "Directly Visible Erlang Resource." (17)
- `whereis(Name)` → `pid() | port() | undefined`. (17)
- `registered/0` → list of registered names. (17)
- `unregister(Name)` — note "you can still receive signals associated with the
  registered name after it has been unregistered as the sender may have looked
  up the name before sending." (17)

### Process flags (`process_flag/2`)
Full flag list (from 17):
- `trap_exit` (boolean) — convert exit signals to `{'EXIT',From,Reason}`.
- `priority` (`low | normal | high | max`).
- `save_calls` (`0..10000`).
- `sensitive` (boolean).
- `message_queue_data` (`on_heap | off_heap`).
- `min_heap_size`, `min_bin_vheap_size`, `max_heap_size`, `fullsweep_after`.
- `async_dist` (boolean) — fully async distributed signaling (no flow-control
  blocking).
- `error_handler` (module).
- `monitor_nodes`.
- `process_flag/3` sets flags for another process but "only a subset ...
  namely `save_calls`." (17)

### Process states and reductions
- "A process always terminates with an *exit reason* (any term)." (06) It
  "terminates *normally* only if its exit reason is the atom `normal`." (06)
- `processes/0` returns all pids; "an exiting process exists, but is not alive.
  That is, `is_process_alive/1` returns false for an exiting process, but its
  process identifier is part of the result returned from `processes/0`." (17)
- `is_process_alive(Pid)` guarantees that signals sent from the caller before the
  call are delivered before aliveness is checked. (17)
- Reductions are tracked via `statistics(reductions)` and
  `statistics(exact_reductions)`. (17)
- `hibernate/3` discards the call stack and shrinks the heap; `hibernate/0`
  (OTP 28+) does not discard the call stack. (17) For `proc_lib`-started
  processes use `proc_lib:hibernate/3` instead. (17)

### Process dictionary
"Each process has its own process dictionary, accessed by calling the following
BIFs: `put(Key, Value)`, `get(Key)`, `get()`, `get_keys(Value)`, `erase(Key)`,
`erase()`." (06)

## Review checklist
- [ ] Is `spawn_opt` used (not bare `spawn`) when options are needed?
- [ ] Are `link` and `monitor` not both passed to `spawn_opt`?
- [ ] Is `receive` selective (matching specific messages) to avoid scanning
  unbounded queues?
- [ ] Are registered names atoms, and is `whereis/1` used defensively?
- [ ] Are process flags set deliberately (especially `trap_exit`,
  `max_heap_size`, `message_queue_data`)?
- [ ] Is priority messaging avoided unless strictly necessary?

## Implementation checklist
- [ ] Choose `spawn_link`/`spawn_monitor`/`spawn_opt` per link/monitor need.
- [ ] Set `min_heap_size`/`max_heap_size` for known workload shapes.
- [ ] Use `send/3` with `nosuspend`/`noconnect` only where non-blocking send is
  required.
- [ ] Prefer `proc_lib:spawn_link`/`start_link` for supervision-tree processes
  (see `proc-lib-and-sys.md`).

## Runtime / debugging checklist
- [ ] `processes/0` / `is_process_alive/1` to enumerate and check liveness.
- [ ] `process_info(Pid, Item)` for `message_queue_len`, `status`,
  `reductions`, `links`, `monitors`, `trap_exit`, `priority`,
  `message_queue_data`, `total_heap_size`. (17)
- [ ] `statistics(reductions)` / `statistics(run_queue)` for scheduler load.
- [ ] `erlang:system_info(dist_buf_busy_limit)` for distribution backpressure.

## Validation hooks
- Signal-ordering guarantee is per sender→destination pair only.
- `send/2` to an unregistered local name raises `badarg` (synchronous check).
- `register/2` raises `badarg` if the name is in use, the process is already
  registered, or `RegName` is the atom `undefined`. (17)

## Examples
```erlang
%% spawn with options (priority + max heap)
Pid = spawn_opt(worker, run, [Arg], [{priority, high}, {max_heap_size, 100000}]),

%% spawn_link atomically links
Pid2 = spawn_link(worker, run, [Arg]),

%% spawn_monitor returns {Pid, Ref}
{Pid3, Ref} = spawn_monitor(worker, run, [Arg]),

%% selective receive with after
receive
    {reply, Ref, Result} -> Result;
    {'DOWN', Ref, process, Pid3, Reason} -> {error, Reason}
after 5000 ->
    {error, timeout}
end,

%% registration
register(my_server, Pid),
whereis(my_server),     %% => Pid | undefined
unregister(my_server),
```

## Common mistakes
- Passing both `link` and `monitor` to `spawn_opt` — raises `badarg`.
- Assuming cross-sender message ordering — only per-sender order is guaranteed.
- Assuming `send` is always non-blocking — distribution flow control can
  suspend the sender.
- Using priority messages casually — "you may cause issues instead of solving
  issues if not used with care." (17)
- Relying on `process_info/1` for non-debug purposes — "intended for debugging
  only. For all other purposes, use `process_info/2`." (17)

## Strict vs contextual guidance
- **Strict:** Never pass `link` and `monitor` together to `spawn_opt`. Never
  assume ordering across senders. Treat priority messages as a last resort.
- **Contextual:** `message_queue_data` (`on_heap` vs `off_heap`) tuning depends
  on workload; `min_heap_size`/`fullsweep_after` are performance knobs.

## Policy decisions for individual repos
- Default `max_heap_size` per process class (memory-bounding policy).
- Whether `async_dist` is enabled for latency-sensitive senders (trades flow
  control for non-blocking sends).
- Whether priority messaging is permitted at all.

## Related docs
- [links-monitors-and-exits.md](links-monitors-and-exits.md)
- [proc-lib-and-sys.md](proc-lib-and-sys.md)
- [timers.md](timers.md)
- [runtime-debugging.md](runtime-debugging.md)
- [distribution.md](distribution.md)
- [common-mistakes.md](common-mistakes.md)

## Related skills
- `beam-processes`
- `beam-observability-debugging`
