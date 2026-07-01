# Concurrency and Processes

## Purpose

This document is a project-independent reference for Elixir/BEAM concurrency primitives, centered on the `Process` module and the core ideas every future agent needs when writing, reviewing, refactoring, or debugging Elixir code. It covers spawning, messaging, links, monitors, timers, process inspection, process flags, registration, the process dictionary, and the `Task`/`Task.Supervisor`, `Agent`, `Node`, and `Port` abstractions, aligned with the official Elixir and Erlang documentation.

## Sources used

- https://hexdocs.pm/elixir/Process.html (PRIMARY)
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html
- https://hexdocs.pm/elixir/processes.html
- https://www.erlang.org/doc/reference_manual/processes.html
- https://www.erlang.org/doc/apps/erts/erlang.html
- https://hexdocs.pm/elixir/Task.html
- https://hexdocs.pm/elixir/Task.Supervisor.html
- https://hexdocs.pm/elixir/GenServer.html
- https://hexdocs.pm/elixir/Agent.html
- https://hexdocs.pm/elixir/compatibility-and-deprecations.html
- https://hexdocs.pm/elixir/Node.html
- https://hexdocs.pm/elixir/Port.html
- https://www.erlang.org/doc/system/distributed.html
- https://www.erlang.org/doc/system/ports.html
- https://www.erlang.org/doc/apps/kernel/net_kernel.html
- https://www.erlang.org/doc/apps/kernel/net_adm.html
- https://www.erlang.org/doc/apps/erts/epmd_cmd.html
- https://www.erlang.org/doc/apps/erts/erl_cmd.html
- https://hexdocs.pm/elixir/System.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/agent.ex
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/agent/server.ex

This page reflects Elixir v1.20.2 / OTP 29 docs.

## Related BEAM guidance

The Elixir `Process`/`Task`/`Agent`/`Node`/`Port` API below is the value of this doc; the underlying BEAM concurrency runtime semantics live in `docs/beam/`:

- [../beam/processes-and-messages.md](../beam/processes-and-messages.md) — for spawn/spawn_opt, message passing, process states, and the process dictionary.
- [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md) — for links vs monitors, exit-signal reception rules, `trap_exit`, and `:kill`.
- [../beam/distribution.md](../beam/distribution.md) — for distributed Erlang nodes, cookies, and EPMD.
- [../beam/ports-io.md](../beam/ports-io.md) — for ports, connected processes, and external I/O.

## Process Module and Core Concurrency

### The BEAM concurrency model

All Elixir code runs inside lightweight, isolated BEAM processes that communicate by asynchronous message passing; messages are copied between heaps except large binaries (>64 bytes) which are reference-counted and shared. Each process has its own heap, stack, mailbox, and process dictionary; there is no shared mutable memory. For the full BEAM concurrency model (scheduling, reductions, process states, message passing internals), see [../beam/processes-and-messages.md](../beam/processes-and-messages.md); for VM-internal scheduler/GC/memory detail see `docs/elixir/beam-otp-internals.md`.

For large shared data, prefer `ETS` or `persistent_term` instead of repeatedly sending huge terms between processes. ETS tables live in a separate memory area and can be accessed by many processes; `persistent_term` is optimized for rarely-changing global configuration. (processes.html)

### Spawning processes: spawn, spawn_link, spawn_monitor

`spawn/1,3`, `spawn_link/1,3`, and `spawn_monitor/1,3` live in `Kernel`, are auto-imported, and are inlined to the corresponding `:erlang` BIFs. (Kernel.html#spawn/1)

`Process.spawn/2,4` provides the same behavior with additional options. (Process.html#spawn/2)

Exact signatures:

```elixir
spawn((-> any)) :: pid
spawn(module, atom, [any]) :: pid
spawn_link((-> any)) :: pid
spawn_link(module, atom, [any]) :: pid
spawn_monitor((-> any)) :: {pid, reference}
spawn_monitor(module, atom, [any]) :: {pid, reference}
```

`Process.spawn/2,4` accepts a list of spawn options:

```elixir
:link
:monitor
{:priority, :low | :normal | :high}
{:fullsweep_after, non_neg_integer}
{:min_heap_size, non_neg_integer}
{:min_bin_vheap_size, non_neg_integer}
{:max_heap_size, heap_size}
{:message_queue_data, :off_heap | :on_heap}
```

Note that `:max` priority is NOT available as a spawn option; use `Process.flag(:priority, :max)` after spawning if truly necessary. (Process.html#spawn/2)

Spawn family comparison:

| Function | Returns | Link? | Monitor? |
|---|---|---|---|
| `spawn/1,3` | `pid` | no | no |
| `spawn_link/1,3` | `pid` | yes (atomic) | no |
| `spawn_monitor/1,3` | `{pid, ref}` | no | yes (atomic) |
| `Process.spawn/2,4` w/ `:link` | `pid` | yes | no |
| `Process.spawn/2,4` w/ `:monitor` | `{pid, ref}` | no | yes |

Atomic link/monitor creation means the link or monitor is established as part of the spawn operation. There is no race window where the new process can crash before the caller links or monitors it. (Process.html#spawn_link/1)

MUST: use `spawn_link/1,3` only when you intend shared fate between the caller and the spawned process. If you only want to observe failure, prefer `spawn_monitor/1,3` or `Task`.

### send and receive

`send/2` is defined in `Kernel` and auto-imported:

```elixir
send(dest, msg) :: msg
```

`dest` can be a `pid`, a `port`, an atom registered locally, or a tuple `{name, node}`. (Kernel.html#send/2)

`Process.send/3` adds options:

```elixir
Process.send(dest, msg, options) :: :ok | :noconnect | :nosuspend
```

Options are `:noconnect` and `:nosuspend`. (Process.html#send/3)

Important: `send/2` always returns the message, even if the destination is dead or unknown. It does NOT raise `:noproc`. This is a common source of bugs; use a monitor to detect liveness.

`receive` is a special form:

```elixir
receive do
  pattern [when guard] -> body
after
  timeout -> body
end
```

(Kernel.SpecialForms.html#receive/1)

- `after 0` performs a non-blocking poll.
- `after n` waits up to `n` milliseconds.
- `:infinity` waits forever (the default if no `after` clause is given).

The mailbox is FIFO for a given sender, but `receive` performs selective receive: it scans the mailbox from the head, matching each message against the clauses in order. Non-matching messages remain in the mailbox. This scan is O(n) in the worst case and can become expensive when the mailbox grows large. (processes.html)

Guards in `receive` must use guard-safe BIFs and pure expressions only.

Flushing the mailbox idiom with `after 0`:

```elixir
flush do
  _ -> flush()
after
  0 -> :ok
end
```

In practice a loop that recursively calls `receive` with `after 0` drains all pending messages and returns when the mailbox is empty.

### Timers: send_after, read_timer, cancel_timer

`Process.send_after/4` schedules a message to be delivered later:

```elixir
Process.send_after(dest, msg, time, options \\ []) :: reference
```

`time` is in MILLISECONDS. It returns a timer `reference`. (Process.html#send_after/4)

Options:

```elixir
[{:abs, boolean()}]
```

With `abs: true`, `time` is interpreted as Erlang monotonic absolute time. By default it is relative to the current time.

Auto-cancel behavior:

- If `dest` is a PID that dies before the timer fires, the timer is automatically canceled.
- If `dest` is a registered atom, there is NO auto-cancel. The name is resolved at delivery time; if the name is unregistered, the message is dropped.

There is no ordering guarantee between a `send_after` and a direct `send` scheduled for the same time.

`Process.cancel_timer/2` cancels a timer:

```elixir
Process.cancel_timer(ref, options \\ []) :: non_neg_integer | false | :ok
```

Return values:

- milliseconds remaining if the timer existed and had not fired,
- `false` if the timer does not exist or has already fired/canceled,
- `:ok` when `async: false, info: false` is passed.

Options: `[async: bool, info: bool]`. (Process.html#cancel_timer/2)

`Process.read_timer/1` returns milliseconds left or `false`. (Process.html#read_timer/1)

MUST: capture the `reference` returned by `send_after/4` and use it with `cancel_timer/2`. You cannot cancel a timer by matching on the scheduled message content.

### Inspecting processes: alive?, info, list

`Process.alive?/1` tests whether a local pid is alive:

```elixir
Process.alive?(pid) :: boolean
```

It raises `ArgumentError` for a remote pid. Use `:erpc.call/4` or distributed inspection when checking remote processes. (Process.html#alive?/1)

`Process.info/1,2` retrieves process metadata:

```elixir
Process.info(pid) :: keyword | nil
Process.info(pid, item) :: {item, term} | nil
Process.info(pid, [items]) :: [{item, term}] | nil
```

`nil` is returned if the process is dead. (Process.html#info/2)

From the docs:

> "Use this only for debugging information."

Common info keys:

| Key | Returns |
|---|---|
| `:registered_name` | atom or `[]` |
| `:current_function` | `{m, f, a}` |
| `:initial_call` | `{m, f, a}` |
| `:status` | `:running \| :runnable \| :garbage_collecting \| :waiting \| :suspended \| :exiting` |
| `:message_queue_len` | integer (cheap, O(1)) |
| `:messages` | list of all messages (expensive, copies mailbox) |
| `:links` | list of linked pids/ports |
| `:monitors` | monitored items |
| `:dictionary` | process dictionary as `[{k, v}]` |
| `:trap_exit` | boolean |
| `:priority` | `:low \| :normal \| :high \| :max` |
| `:heap_size` | words |
| `:stack_size` | words |
| `:total_heap_size` | words |
| `:memory` | approximate bytes |
| `:reductions` | reductions consumed |
| `:garbage_collection` | keyword of GC info |
| `:group_leader` | pid |
| `:current_stacktrace` | stacktrace |

`:message_queue_len` is cheap; `:messages` copies the entire mailbox and is expensive. MUST check `:message_queue_len` before pulling `:messages`.

`Process.list/0` returns all pids on the local node, including exiting processes:

```elixir
Process.list() :: [pid]
```

(Process.html#list/0)

Iterating `Process.list/0` and calling `Process.info/2` on every pid can be expensive on large systems; use `:observer` or `recon` for production profiling.

### Links vs monitors

```elixir
Process.link(pid_or_port) :: true
Process.unlink(pid_or_port) :: true
Process.monitor(item) :: reference
Process.monitor(item, options) :: reference
Process.demonitor(ref, options \\ []) :: boolean
```

Links are bidirectional and idempotent; monitors are unidirectional. A monitored process death delivers `{:DOWN, ref, :process, object, reason}` (immediately if already dead). Always pass `[:flush]` to `demonitor/2` to remove stale DOWN messages.

Comparison table:

| Aspect | Link | Monitor |
|---|---|---|---|
| Direction | Bidirectional | Unidirectional |
| Identity | anonymous | `reference()` |
| Failure | linked process exits with same reason (unless trapping) | owner gets `{:DOWN, ref, :process, pid, reason}` |
| trap_exit | without trap, signal crashes receiver; with trap becomes `{:EXIT, from, reason}` | always a message; no trap interaction |
| Multiplicity | one per pair | multiple independent monitors allowed |
| Ports | linkable | monitorable (`{:DOWN, ref, :port, object, reason}`) |
| Atomic spawn form | `spawn_link/1,3` | `spawn_monitor/1,3` |
| One-off work | NO (couples fates) | YES (observe without dying) |

For link/monitor semantics, exit-signal propagation, and the full reception rules, see [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md). SHOULD prefer monitors over links from the observer/caller side; use links for supervision trees and tightly-coupled process pairs.

### trap_exit and exit signals

`Process.flag(:trap_exit, true)` makes exit signals become `{:EXIT, from, reason}` messages instead of killing the current process. `Process.exit/2` sends an exit signal to another process and always returns `true`.

```elixir
Process.flag(:trap_exit, true) :: boolean
Process.exit(pid, reason) :: true
```

Reason semantics:

- `:normal` — the target does NOT exit. If the target is trapping exits, it receives `{:EXIT, from, :normal}`.
- `:kill` — UNTRAPPABLE. The target exits with reason `:killed`.
- any other term — the target exits with that reason unless it is trapping exits, in which case the signal becomes a `{:EXIT, from, reason}` message.

Contrast with `Kernel.exit/1`:

```elixir
Kernel.exit(reason) :: no_return
```

`Kernel.exit/1` raises an exception in the current process; it can be caught with `try/catch`, but still terminates the process once the stack unwinds unless caught and handled carefully.

For exit-signal reception rules, `:kill`/`:normal`/`:shutdown` propagation, and `trap_exit` semantics, see [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md). OTP supervisors and GenServers use `trap_exit` to receive `{:EXIT, child_pid, reason}`; see `docs/elixir/otp-supervision.md`.

MUST NOT expect `{:EXIT, _, _}` messages without first setting `trap_exit`.

### Process flags (flag/2, flag/3)

`Process.flag/2` sets a flag on the current process and returns the old value:

```elixir
Process.flag(flag, value) :: term
```

`Process.flag/3` sets a flag on another process, but only `:save_calls` is allowed for other pids; attempting to set other flags on a non-local pid raises. (Process.html#flag/2)

| Flag | Values | Default | Effect |
|---|---|---|---|
| `:trap_exit` | boolean | false | exit signals become messages |
| `:priority` | `:low \| :normal \| :high \| :max` | `:normal` | scheduler priority (NOT mailbox order) |
| `:save_calls` | 0..10000 | 0 | keep recent calls for crash reports |
| `:sensitive` | boolean (OTP 21+) | false | suppress error logs for this process |
| `:max_heap_size` | heap_size() | `:off` | kill/log when heap exceeds limit |
| `:message_queue_data` | `:off_heap \| :on_heap` | context-dependent | mailbox placement vs GC |
| `:min_heap_size` | non_neg_integer | 233 | force minimum heap |
| `:min_bin_vheap_size` | non_neg_integer | 32768 | minimum virtual binary heap |
| `:fullsweep_after` | non_neg_integer | 65535 | force fullsweep after N minor GCs |

`:max_heap_size` value shapes: `:off`, an integer (soft cap), `{integer, :kill}`, `{integer, :error_logger}`, or with boolean variants. (Process.html#flag/2)

SHOULD NOT set `:high` or `:max` priority in application code; reserve `:max` for the runtime. Priority affects scheduler frequency, not preemption and not mailbox order.

`:max` priority is NOT available as a spawn option; it can only be set via `Process.flag/2` after the process is running.

### Process dictionary

The process dictionary is a per-process key-value store:

```elixir
Process.put(key, value) :: value | nil
Process.get(key, default \\ nil) :: value | default
Process.get() :: [{term, term}]
Process.get_keys(value \\ nil) :: [term]
Process.delete(key) :: value | nil
```

(Process.html#put/2)

The process dictionary is generally discouraged. It is hidden global state, lives on the process heap, creates concurrency hazards, and harms testability. Credo often flags its use.

Legitimate uses include:

- per-request metadata such as Plug/Phoenix request ids,
- `:logger.metadata`,
- telemetry context,
- transient per-process scratch data,
- OTP-managed entries such as `$ancestors` and `$callers`.

MUST NOT use the process dictionary for shared or cross-process state, nor as a replacement for GenServer state. See `docs/elixir/otp-supervision.md`.

### Registration

`Process.register/2`, `Process.unregister/1`, `Process.whereis/1`, and `Process.registered/0` manage local names:

```elixir
Process.register(pid_or_port, name) :: true
Process.unregister(name) :: true
Process.whereis(name) :: pid | port | nil
Process.registered() :: [atom]
```

(Process.html#register/2)

Reserved names: `nil`, `false`, `true`, `:undefined` cannot be used.

`register/2` raises `ArgumentError` if the pid is dead, the name is already taken, or the pid is already registered.

For dynamic names, use `Registry` via `:via`. MUST NOT generate atoms from user input with `String.to_atom/1` to name processes; that causes atom-table leaks. See `docs/elixir/otp-supervision.md`.

### Heap, stack, reductions, and garbage collection

Process info keys expose internals:

| Key | Unit | Meaning |
|---|---|---|
| `:heap_size` | words | size of the youngest heap generation |
| `:stack_size` | words | current stack size |
| `:total_heap_size` | words | total heap size including old generation |
| `:memory` | bytes | approximate memory used by the process |
| `:reductions` | count | total reductions consumed |
| `:garbage_collection` | keyword | GC statistics |

Heap sizes are in WORDS, not bytes; `:memory` is in BYTES. `:reductions` are the unit of scheduler work. (Process.html#info/2)

`Process.hibernate/3` minimizes memory for long-idle processes by discarding the call stack and reducing the heap; the next message invokes `mod.fun(args)`. Use it only for idle processes that currently hold a large heap.

```elixir
Process.hibernate(mod, fun, args) :: no_return
```

`:message_queue_data: :off_heap` reduces GC pressure for processes that receive many large messages, because mailbox data is stored outside the process heap.

For BEAM GC internals and the scheduler/reduction model, see [../beam/processes-and-messages.md](../beam/processes-and-messages.md) (process-level) and `docs/elixir/beam-otp-internals.md` (VM-internal scheduler/GC detail).

### Other Process functions (brief)

`Process.sleep/1`:

```elixir
Process.sleep(timeout) :: :ok
```

It accepts `:infinity` and any non-negative integer. The docstring explicitly warns not to use `sleep` to wait for another process. Use `receive` plus a monitor instead. (Process.html#sleep/1)

`Process.group_leader/0` and `Process.group_leader/2` manage I/O redirection. The group leader receives standard I/O from the process.

`Process.pid/3` constructs a pid from three integers. It is rarely needed; use it only when reconstructing a pid from external data.

`Process.set_label/1` and `Process.get_label/1` (Elixir 1.17+) attach a label to a process for Observer and crash logs. Labels need not be unique.

Process aliases (`Process.alias/0,1` and `Process.unalias/1`, OTP 24 / Elixir 1.15+) create deactivatable references. Sending to a deactivated alias silently drops the message. They are useful for request/response patterns where a late reply must not pollute the mailbox. (Process.html#alias/1)

`Process.flag(:save_calls, n)` keeps a ring buffer of recent calls for crash reports and inspection.

## Task and Task.Supervisor

### Overview: async-and-await model

`Task.async/1,3` spawns a process that is **both linked and monitored** by the caller. The `%Task{}` struct carries `:mfa`, `:owner`, `:pid`, and `:ref` (the monitor reference). (Task.html#async/1)

From the docs:

> "An asynchronous task should be thought of as an extension of the caller process rather than a mechanism to isolate it from all errors."

The link is intentional: it preserves the same fault-boundary properties as ordinary sequential code. If the caller crashes, its async tasks crash; if an async task crashes, the caller exits with the same reason. `Task.await/2` then consumes both the result message and the `{:DOWN, ref, :process, pid, reason}` message. See `## Process Module and Core Concurrency` above for link/monitor fundamentals.

### Task.async / Task.await

Exact signatures:

```elixir
Task.async(fun) :: t()
Task.async(module, function_name, args) :: t()
Task.await(task, timeout \\ 5000) :: term()
```

- `await/2` default timeout is `5000` ms; `:infinity` is accepted. (Task.html#await/2)
- The task sends `{ref, result}` back to the owner; `await/2` also consumes the monitor `DOWN` message.
- If the task exits abnormally, the **caller also exits with the same reason**, because the task is linked.
- `await/2` MUST be called at most once per task; for polling, use `Task.yield/2`.

```elixir
task = Task.async(fn -> heavy_computation() end)
result = Task.await(task)
```

MUST NOT:

- Set `:trap_exit` to "handle" `async/await` failures. The link still propagates the exit; `await/2` does not turn failure into a return value.
- Unlink an `async/await` task. Unlinking severs the fault boundary and leaves a dangling task.

### Task.start vs Task.start_link vs Task.async

Exact signatures:

```elixir
Task.start(fun) :: {:ok, pid}
Task.start(module, function_name, args) :: {:ok, pid}
Task.start_link(fun) :: {:ok, pid}
Task.start_link(module, function_name, args) :: {:ok, pid}
Task.async(fun) :: t()
Task.async(module, function_name, args) :: t()
```

Comparison:

| Function | Returns | Links caller? | Monitored? | Awaits result? | Typical use |
|---|---|---|---|---|---|
| `Task.start/1,3` | `{:ok, pid}` | no | no | no | Unsupervised side-effects (discouraged in production) |
| `Task.start_link/1,3` | `{:ok, pid}` | yes | no | no | Fire-and-forget under static supervision |
| `Task.async/1,3` | `%Task{}` | yes | yes | yes (`Task.await/2`) | Concurrent work with result, shared fate |

Neither `start/1,3` nor `start_link/1,3` returns a `%Task{}` or supports `await/2`; they are fire-and-forget. `Task.child_spec/1` defaults `:restart` to `:temporary`, which is unique among OTP behaviours (most default to `:permanent`).

SHOULD prefer `Task.Supervisor.start_child/2` over `Task.start/1,3` in production, because node shutdown will not wait for unsupervised tasks.

### Statically supervised tasks

`use Task` adds the `Task` behaviour and the default `child_spec/1`. You can place `{Task, fn -> ... end}` under a regular `Supervisor`. (Task.html#module-child-specification)

Child-spec options:

| Option | Default | Notes |
|---|---|---|
| `:id` | module atom | Used by the supervisor |
| `:restart` | `:temporary` | `:temporary`, `:transient`, or `:permanent` |
| `:shutdown` | `5000` | `:brutal_kill` or milliseconds |

The function receives no arguments and its result is discarded. This is the static-supervision analogue of `Task.start_link/1,3`.

### Task.Supervisor

`Task.Supervisor` is a supervisor for dynamically spawned tasks. In Elixir v1.20.x it is implemented as a `DynamicSupervisor` with strategy `:one_for_one`. See `docs/elixir/otp-supervision.md` for `DynamicSupervisor` fundamentals.

Start it in a supervision tree:

```elixir
{Task.Supervisor, name: MyApp.TaskSupervisor}
```

Name registration follows `GenServer` rules (atom, `:via`, or `:global`). `Task.Supervisor` is a single process; for high throughput, run it under `PartitionSupervisor` with `child_spec: Task.Supervisor`. (Task.Supervisor.html#module-name-registration)

Functions:

```elixir
Task.Supervisor.start_child(supervisor, fun, options \\ [])
Task.Supervisor.start_child(supervisor, module, function_name, args, options \\ [])
  :: DynamicSupervisor.on_start_child()

Task.Supervisor.async(supervisor, fun, options \\ [])
Task.Supervisor.async(supervisor, module, function_name, args, options \\ [])
  :: Task.t()

Task.Supervisor.async_nolink(supervisor, fun, options \\ [])
Task.Supervisor.async_nolink(supervisor, module, function_name, args, options \\ [])
  :: Task.t()

Task.Supervisor.children(supervisor) :: [pid]
Task.Supervisor.terminate_child(supervisor, pid) :: :ok | {:error, :not_found}
```

- `start_child/2,3,4,5`: spawned process is linked **only** to the supervisor, not the caller. Options: `:restart` (`:temporary` default | `:transient` | `:permanent`) and `:shutdown` (`:brutal_kill` or ms, default `5000`). The task must trap exits for the graceful shutdown timeout to take effect.
- `async/3,5`: task IS linked to the caller, like plain `Task.async/1,3`. Only option is `:shutdown` (default `5000`). Raises if `:max_children` is reached.
- `async_nolink/3,5`: task is **not** linked to the caller; only monitored. Requires the supervisor's restart strategy to be `:temporary` (the default), because `async_nolink` holds a direct `%Task{}` reference that is lost if the supervisor restarts the child.

For distributed tasks, use the MFA arities (`async/5`, `async_nolink/5`, `start_child/5`) because anonymous functions require the same module version on all nodes.

MUST NOT pass `:restart` or `:shutdown` to `Task.Supervisor.start_link/1`; those options are deprecated. Pass them per `start_child/2,3,4,5` instead.

### Crash isolation and exception handling

The crucial distinction is linking:

- `Task.async/1,3` and `Task.Supervisor.async/3,5` link the task to the caller. If the task raises or exits, the caller exits with the same reason. There is no `{:exit, _}` return unless the task body is wrapped in `try/rescue`.
- `Task.Supervisor.async_nolink/3,5` does not link. If the task crashes, the caller survives and learns the reason through `Task.yield/2` or `Task.shutdown/2`.

```elixir
Task.yield(task, timeout \\ 5000) :: {:ok, term} | {:exit, term} | nil
Task.shutdown(task, timeout \\ :brutal_kill) :: {:ok, term} | {:exit, term} | nil
Task.ignore(task) :: {:ok, term} | {:exit, term} | nil
```

- `yield/2` is non-destructive: the monitor stays active and it may be called multiple times. Returns `nil` on timeout. Returns `{:exit, reason}` when the task is not linked (e.g. `async_nolink`), or the caller is trapping exits, or the task exited `:normal`. (Task.html#yield/2)
- `shutdown/2` unlinks and shuts down the task. On timeout it first sends `:shutdown`, then kills; `:brutal_kill` kills immediately. If the task terminates abnormally, `shutdown/2` still exits the caller with that reason. (Task.html#shutdown/2)
- `ignore/1` (since 1.13) unlinks and demonitors the task but leaves it running — fire-and-continue. (Task.html#ignore/1)

MUST NOT `Task.async/1,3` a task and immediately `Task.ignore/1` it; use `Task.Supervisor.start_child/2` instead.

Canonical timeout/cleanup idiom:

```elixir
case Task.yield(task, timeout) || Task.shutdown(task) do
  {:ok, result} ->
    result

  nil ->
    Logger.warning("Failed to get a result in #{timeout}ms")
    nil
end
```

Comparison:

| Function | Destructive? | Leaves monitor? | Return on task death | Blocks? |
|---|---|---|---|---|
| `await/2` | yes | no | caller exits | yes |
| `yield/2` | no | yes | `{:exit, reason}` or `nil` | yes (up to timeout) |
| `shutdown/2` | yes | no | `{:exit, reason}` or caller exits | yes (up to timeout) |
| `ignore/1` | yes (unlink/demonitor) | no | `{:exit, reason}` if already dead | no |

### Task.async_stream

Exact signatures:

```elixir
Task.async_stream(enumerable, fun, options \\ []) :: Enumerable.t()
Task.async_stream(enumerable, module, function_name, args, options \\ []) :: Enumerable.t()

Task.Supervisor.async_stream(supervisor, enumerable, fun, options \\ []) :: Enumerable.t()
Task.Supervisor.async_stream(supervisor, enumerable, module, function_name, args, options \\ []) :: Enumerable.t()

Task.Supervisor.async_stream_nolink(supervisor, enumerable, fun, options \\ []) :: Enumerable.t()
Task.Supervisor.async_stream_nolink(supervisor, enumerable, module, function_name, args, options \\ []) :: Enumerable.t()
```

Options (plain and Task.Supervisor variants share the first five; `:shutdown` is Task.Supervisor only):

| Option | Type | Default | Notes |
|---|---|---|---|
| `:max_concurrency` | `pos_integer` | `System.schedulers_online/0` | Backpressure: no new item until a slot frees |
| `:ordered` | `boolean` | `true` | `false` can be faster when order is irrelevant |
| `:timeout` | `timeout` | `5000` | `:infinity` allowed |
| `:on_timeout` | `:exit` or `:kill_task` | `:exit` | `:exit` kills the caller; `:kill_task` emits `{:exit, :timeout}` |
| `:zip_input_on_exit` | `boolean` | `false` | Since 1.14; emits `{:exit, {input, reason}}` |
| `:shutdown` | `:brutal_kill` or `integer` | `5000` | Task.Supervisor variants only |

The return is a lazy enumerable yielding `{:ok, value}` or `{:exit, reason}`. Plain `async_stream/3,5` and `Task.Supervisor.async_stream/4,6` link to the caller; `async_stream_nolink/4,6` does not link but still guarantees cleanup of in-flight tasks when the stream halts.

```elixir
items
|> Task.async_stream(&do_work/1, max_concurrency: 4, on_timeout: :kill_task)
|> Enum.to_list()
```

MUST combine `async_stream` with `Stream.take/2` when only some results are needed; otherwise it consumes the whole enumerable and may spawn work that is immediately discarded.

### When to use Task vs Task.Supervisor

From the docs for dynamically supervised tasks:

- `Task.Supervisor.start_child/2` — use for fire-and-forget work where the result is not needed.
- `Task.Supervisor.async/2` + `Task.await/2` — use for concurrent work with a result where the caller should crash on task failure.
- `Task.Supervisor.async_nolink/2` + `Task.yield/2` + `Task.shutdown/2` — use for concurrent work with a result where the caller should survive and receive the failure reason.

Decision summary:

| Use case | Recommended API |
|---|---|
| Short-lived concurrency with shared fate | `Task.async/1,3` + `Task.await/2` |
| Fire-and-forget, supervised | `Task.Supervisor.start_child/2` |
| Concurrent result, caller survives failures | `Task.Supervisor.async_nolink/2` + `yield`/`shutdown` |
| Bulk bounded concurrency | `Task.async_stream/3` or `Task.Supervisor.async_stream/4,6` |

SHOULD prefer supervised tasks in production for visibility and configurable shutdown. Inside OTP behaviours such as `GenServer`, use `Task.Supervisor.async_nolink/2` and handle `{ref, result}` and `{:DOWN, ref, :process, pid, reason}` in `handle_info/2` instead of calling `Task.await/2` inside a callback, which would block the behaviour.

```elixir
def handle_call(:work, _from, state) do
  task = Task.Supervisor.async_nolink(MyApp.TaskSupervisor, fn -> do_work() end)
  {:noreply, Map.put(state, task.ref, :pending)}
end

def handle_info({ref, result}, state) when is_reference(ref) do
  Process.demonitor(ref, [:flush])
  state = Map.update!(state, ref, fn _ -> {:ok, result} end)
  {:noreply, state}
end

def handle_info({:DOWN, ref, :process, _pid, reason}, state) do
  state = Map.update!(state, ref, fn _ -> {:error, reason} end)
  {:noreply, state}
end
```

### Ancestor and caller tracking

Supervised tasks set `$ancestors` and `$callers` in the process dictionary. These entries preserve the logical caller chain for crash reports and logging, so a failure in a supervised task is attributed to the originating code, not just the anonymous supervisor process.

This is a legitimate use of the process dictionary: the entries are OTP-managed and read-only for observability. See the `### Process dictionary` subsection above for why such OTP-managed entries are acceptable while ad-hoc process-dictionary state is not.

## Agent

### Overview: state container over a GenServer

`Agent` is a simple abstraction around a single piece of state held in a process.

> "Agents are a simple abstraction around state… The Agent module provides a basic server implementation that allows state to be retrieved and updated via a simple API." (Agent.html)

Every agent IS a `GenServer`. It is implemented via the private `Agent.Server` module (`use GenServer`) with `init/1`, `handle_call/3`, `handle_cast/2`, and `code_change/3`. Consequently all `GenServer` semantics apply: linking, supervision, timeouts, `:global` and `:via` registration, and graceful shutdown. See `docs/elixir/otp-supervision.md` for `Supervisor` and `child_spec` details.

The agent's state is a single Erlang term:

```elixir
@type state :: term
```

There is one mailbox; all operations serialize through it.

### Starting and stopping an agent

Exact signatures (Elixir v1.20.2):

```elixir
Agent.start_link((-> term), GenServer.options()) :: Agent.on_start()
Agent.start_link(module, atom, [term], GenServer.options()) :: Agent.on_start()
Agent.start((-> term), GenServer.options()) :: Agent.on_start()
Agent.start(module, atom, [term], GenServer.options()) :: Agent.on_start()
Agent.stop(agent, reason \\ :normal, timeout \\ :infinity) :: :ok
```

Types:

```elixir
@type name :: atom | {:global, term} | {:via, module, term}
@type agent :: pid | {atom, node} | name
@type on_start :: {:ok, pid} | {:error, {:already_started, pid} | term}
@type state :: term
```

The initial function (or MFA) runs **inside** the agent process; its return value becomes the initial state. `Agent.start_link/2,4` links the agent to the caller and is meant for supervision trees. `Agent.start/2,4` does not link.

Options are the same as `GenServer.options()`: `:name`, `:timeout` (init deadline), `:debug`, and `:spawn_opt`. Returns are `{:ok, pid}`, `{:error, {:already_started, pid}}`, or `{:error, reason}`. `Agent.stop/3` delegates to `GenServer.stop/3`.

```elixir
{:ok, _pid} = Agent.start_link(fn -> 0 end, name: :counter)
```

MUST: use `start_link/2,4` for agents that live under a supervisor.

### The state API: get, update, get_and_update, cast

Exact signatures:

```elixir
Agent.get(agent, (state -> a), timeout \\ 5000) :: a
Agent.get(agent, module, atom, [term], timeout \\ 5000) :: a

Agent.get_and_update(agent, (state -> {a, state}), timeout \\ 5000) :: a
Agent.get_and_update(agent, module, atom, [term], timeout \\ 5000) :: a

Agent.update(agent, (state -> state), timeout \\ 5000) :: :ok
Agent.update(agent, module, atom, [term], timeout \\ 5000) :: :ok

Agent.cast(agent, (state -> state)) :: :ok
Agent.cast(agent, module, atom, [term]) :: :ok
```

Default timeout for `get/3,5`, `get_and_update/3,5`, and `update/3,5` is `5000` ms; `:infinity` is accepted. There is no separate `update/4` or `get_and_update/4` overload. The timeout is the third argument with a default of `5000`. Anonymous-function arities are `/3`; MFA arities are `/5`. To set a custom timeout, pass it as the third argument:

```elixir
Agent.update(agent, fn s -> s + 1 end, :infinity)
```

`Agent.get_and_update/3,5` requires the function to return a 2-tuple `{reply, new_state}`. The first element is returned to the caller; the second becomes the new state. The server enforces this shape: any other return value causes `{:bad_return_value, other}` and stops the agent. The `{:halt, reply, new_state}` 3-tuple is NOT supported by `Agent`; that pattern belongs to `Stream`/`Cont`.

`get/3,5`, `update/3,5`, and `get_and_update/3,5` are synchronous `GenServer.call`s. They block the caller and return the actual result (or `:ok` for `update`). `cast/2,4` returns `:ok` immediately.

> "Note that `cast` returns `:ok` immediately, regardless of whether `agent` (or the node it should live on) exists." (Agent.html)

```elixir
Agent.get(:counter, & &1)
#=> 0

Agent.update(:counter, &(&1 + 1))
#=> :ok

Agent.get_and_update(:counter, fn n -> {n, n + 1} end)
#=> 0
Agent.get(:counter, & &1)
#=> 1

Agent.cast(:counter, fn n -> n + 1 end)
#=> :ok   # fire-and-forget; the update may not have run yet
```

MUST NOT use `Agent.cast/2,4` when you need to confirm the write succeeded or need a return value — use `Agent.update/3,5` or `Agent.get_and_update/3,5` instead.

### Naming and registration

Naming follows the same rules as `GenServer`. The `:name` option accepts an atom (local), `{:global, term}`, or `{:via, module, term}`. Once registered, address the agent by name (or `{atom, node}`) instead of pid.

For dynamic names, use `Registry` via `:via`. See `## Process Module and Core Concurrency` → Registration and `docs/elixir/otp-supervision.md`.

MUST NOT generate atoms from user input with `String.to_atom/1` to name agents; that causes atom-table leaks.

### Synchronous vs asynchronous operations

| Function | Mechanism | Blocks caller? | Returns | Confirms write? |
|---|---|---|---|---|
| `get/3,5` | `GenServer.call` | yes | computed value | n/a (read) |
| `update/3,5` | `GenServer.call` | yes | `:ok` | yes |
| `get_and_update/3,5` | `GenServer.call` | yes | `reply` | yes |
| `cast/2,4` | `GenServer.cast` | no | `:ok` immediately | NO |

With `cast/2,4` the function may not have run, may never run (dead agent), and you cannot get a return value. All four serialize through the single agent process regardless.

### `use Agent` and supervision

`use Agent` injects the behaviour and generates a `child_spec/1` so the module can be a supervisor child. Options such as `use Agent, restart: :transient, shutdown: 10_000` configure the generated spec. Because an agent is a `GenServer`, it follows standard child-spec and supervision rules.

```elixir
defmodule CounterAgent do
  use Agent

  def start_link(opts \\ []) do
    Agent.start_link(fn -> 0 end, opts)
  end

  def increment(pid), do: Agent.update(pid, &(&1 + 1))
  def value(pid), do: Agent.get(pid, & &1)
end
```

In a supervision tree:

```elixir
{CounterAgent, name: CounterAgent}
```

See `docs/elixir/otp-supervision.md` for `Supervisor`, `child_spec`, and restart semantics.

### Agent for simple key-value stores

A canonical KV store built on `Agent` keeps a map and transforms it with pure functions:

```elixir
defmodule KV do
  use Agent

  def start_link(opts \\ []),
    do: Agent.start_link(fn -> %{} end, [name: __MODULE__] ++ opts)

  def put(key, value),
    do: Agent.update(__MODULE__, &Map.put(&1, key, value))

  def fetch(key),
    do: Agent.get(__MODULE__, &Map.fetch(&1, key))

  def delete(key),
    do: Agent.update(__MODULE__, &Map.delete(&1, key))
end
```

Atomic read-modify-write via `get_and_update/3,5`:

```elixir
def get_and_put(key, value) do
  Agent.get_and_update(__MODULE__, fn state ->
    {Map.get(state, key), Map.put(state, key, value)}
  end)
end
```

For high-throughput or large datasets, `ETS` (shared, lock-free reads) is better than a single serialized agent. See `## Process Module and Core Concurrency` → "For large shared data, prefer ETS or persistent_term".

### When to use Agent vs GenServer

> "Agents provide a segregation between the client and server APIs (similar to GenServers)." (Agent.html)

Use `Agent` for a simple state container: a single value read and transformed with pure functions, where you do not need custom message handling. Use `GenServer` (or `:gen_statem`, `ETS`, `Registry`) when you need multiple distinct request types, custom `handle_info`, `:continue`/timeouts, explicit state machines, rich `code_change` migrations, or higher throughput than one serialized mailbox.

| Need | Choose |
|---|---|
| Single shared value, pure-function transforms | `Agent` |
| Multiple request types / custom messages / `handle_info` | `GenServer` |
| Explicit state machine with transitions | `GenServer` or `:gen_statem` |
| High read throughput, shared data | `ETS` / `persistent_term` |
| Dynamic key→process registry | `Registry` |

### Limitations and when NOT to use Agent

- **Serialization.** All operations pass through one process mailbox; there is no intra-agent parallelism.
- **No custom callbacks.** You cannot implement `handle_call/3`, `handle_cast/2`, or `handle_info/2`; the message set is fixed by `Agent.Server` (`{:get, fun}`, `{:get_and_update, fun}`, `{:update, fun}`, `{:cast, fun}`).
- **Client functions run inside the agent.** Closures capture the caller's environment, and expensive work blocks the whole agent. The docs warn about computing "in the client" vs "in the server" to avoid races.
- **Distributed/rolling-upgrade caveat.** Anonymous-function APIs require the caller and agent to share the same module version. For distributed agents, prefer the MFA variants: `get/5`, `update/5`, `get_and_update/5`, and `cast/4`.
- **Deprecation status.** The `Agent` module is fully supported through Elixir v1.20.2; it is not listed on the official "Compatibility and Deprecations" page, and the `Agent.html` page carries no deprecation banner. Some community guidance (including core-team members on the Elixir forum) suggests reaching for `GenServer` directly for non-trivial state, but that is guidance, NOT a deprecation.

MUST NOT use `Agent` as a substitute for `GenServer` when you need custom message handling, multiple request types, or complex logic — graduate to `GenServer` (see `docs/elixir/otp-supervision.md`).

## Node

### Overview: distributed Erlang nodes

For the BEAM distribution model, see [../beam/distribution.md](../beam/distribution.md).

> "In Erlang, a node is a running Erlang runtime system with a name." (distributed.html)

A distributed Erlang system is a set of such runtime systems communicating over TCP/IP. Each node is identified by the atom `name@host`, assigned at startup with `--sname` (short name) or `--name` (long/FQDN name). The `Node` module is a thin, mostly-inlined wrapper over Erlang BIFs (`:erlang.*`) and the `net_kernel` process; most calls compile directly to the underlying BIF. (Node.html)

### Node names: node(), self/0, alive?/0

`Kernel.node/0` returns the local node atom, or `:nonode@nohost` if the node is not part of a distributed system. It is allowed in guards. (Kernel.html#node/0)

```elixir
node() :: node()
```

`Kernel.node/1` returns the node where a `pid`, reference, or port lives. (Kernel.html#node/1)

```elixir
node(pid | port | reference) :: node()
```

`Node.self/0` returns the current node name; it is equivalent to `node()`. (Node.html#self/0)

```elixir
Node.self() :: t()
```

`Node.alive?/0` returns `true` only when the local node participates in a distributed system. It delegates to `:erlang.is_alive/0`. (Node.html#alive?/0)

```elixir
Node.alive?() :: boolean()
```

Types:

```elixir
@type t() :: node()
@type state() :: :visible | :hidden | :connected | :this | :known
```

### Listing nodes: list/0,1

`Node.list/0` is equivalent to `Node.list(:visible)`. It returns all currently connected visible nodes; the local node is **not** included. (Node.html#list/0)

```elixir
Node.list() :: [t()]
Node.list(state | [state]) :: [t()]
```

`Node.list/1` accepts a single state atom or a list of state atoms (list arguments are OR-ed). It delegates to `:erlang.nodes/1`. (Node.html#list/1)

| State | Meaning |
|---|---|
| `:visible` | Visible, connected nodes (default). |
| `:hidden` | Hidden, connected nodes. |
| `:connected` | Visible **and** hidden connected nodes. |
| `:this` | The local node only. |
| `:known` | All nodes ever known to this node. |

### Connecting nodes: ping, connect, disconnect

`Node.connect/1` attempts to connect to a remote node. (Node.html#connect/1)

```elixir
Node.connect(node) :: boolean() | :ignored
```

- Returns `true` on success.
- Returns `false` on failure.
- Returns `:ignored` if the local node is not alive.

It delegates to `:net_kernel.connect_node/1`.

`Node.disconnect/1` forces a disconnect. The remote node perceives the disconnect as a local node crash. (Node.html#disconnect/1)

```elixir
Node.disconnect(node) :: boolean() | :ignored
```

It delegates to `:erlang.disconnect_node/1`.

`Node.ping/1` sets up a connection and returns `:pong` on success or `:pang` on failure. (Node.html#ping/1)

```elixir
Node.ping(node) :: :pong | :pang
```

It wraps `:net_adm.ping/1`, which calls `net_kernel:connect_node/1`.

Auto-connect behavior: the first use of another node's name—such as `Node.spawn/2`, `send/2` to a remote pid, or `Node.ping/1`—triggers an automatic connection attempt unless the node was started with `dist_auto_connect: never`. Connections are transitive by default: if A connects to B and B connects to C, A and C also connect. Disable transitivity with `-connect_all false`. (distributed.html)

### Spawning on remote nodes

The `Node.spawn` family is inlined to the corresponding `:erlang` BIFs. (Node.html#spawn/2)

Exact signatures:

```elixir
Node.spawn(node, (-> any)) :: pid
Node.spawn(node, (-> any), Process.spawn_opts()) :: pid | {pid, reference}
Node.spawn(node, module, atom, [any]) :: pid
Node.spawn(node, module, atom, [any], Process.spawn_opts()) :: pid | {pid, reference}

Node.spawn_link(node, (-> any)) :: pid
Node.spawn_link(node, module, atom, [any]) :: pid

Node.spawn_monitor(node, (-> any)) :: {pid, reference}   # since 1.14
Node.spawn_monitor(node, module, atom, [any]) :: {pid, reference}
```

The tuple `{pid, reference}` is returned when the spawn options include `:monitor`.

If the target node does not exist, `Node.spawn/2,4`, `Node.spawn_link/2,4`, and `Node.spawn_monitor/2,4` still return a "useless PID" rather than raising. For `Node.spawn_link`, the link causes the caller to receive an exit signal with reason `:noconnection`. (Node.html#spawn_link/2)

Comparison:

| Function | Returns | Link? | Monitor? | Failed target behavior |
|---|---|---|---|---|
| `Node.spawn/2,4` | `pid` or `{pid, ref}` | no | optional | useless PID |
| `Node.spawn_link/2,4` | `pid` | yes | no | useless PID + `{:EXIT, _, :noconnection}` |
| `Node.spawn_monitor/2,4` | `{pid, ref}` | no | yes | useless PID; `{:DOWN, _, :process, _, :noconnection}` |

### Monitoring nodes: monitor/2,3

`Node.monitor/2,3` turns per-node monitoring on or off for the caller. It delegates to `:erlang.monitor_node/2,3`. (Node.html#monitor/2)

```elixir
Node.monitor(node, flag) :: true
Node.monitor(node, flag, options) :: true
```

`flag` is a boolean. The main option is `:allow_passive_connect`. When monitoring is enabled, the caller receives `{nodedown, node}` if the node goes down. For cluster-wide up/down events—observing any node joining or leaving—prefer `:net_kernel.monitor_nodes/2`, which sends `{nodeup, node}` and `{nodedown, node}` (plus optional extra metadata) to the caller. (net_kernel.html)

### Cookies and security

Distributed Erlang nodes authenticate with a shared "magic cookie" (an atom). During the handshake, cookies are compared via a hashed challenge/response; the cookie itself is never transmitted over the wire. (distributed.html)

```elixir
Node.get_cookie() :: atom()
Node.set_cookie(node, atom) :: true
```

- `Node.get_cookie/0` returns the local cookie, or `:nocookie` if the node is not alive. (Node.html#get_cookie/0)
- `Node.set_cookie/2` sets the cookie used for `node` (defaulting to `Node.self()`). If `node` is the local node, it also sets the cookie for all other currently unknown nodes. It raises `FunctionClauseError` if the local node is not alive. (Node.html#set_cookie/2)

At startup, the `auth` server looks for `.erlang.cookie` in the user's home directory, then in `filename:basedir(user_config, "erlang")`. If neither exists, it creates `$HOME/.erlang.cookie` with mode `0400` and a random value. (distributed.html) Set the cookie at startup with `--cookie` (Elixir) or `-setcookie` (Erlang), or call `Node.set_cookie/2` after the node is alive.

> "Starting a distributed node without also specifying `-proto_dist inet_tls` will expose the node to attacks that may give the attacker complete access to the node and by extension the cluster." (distributed.html)

Distribution uses cleartext TCP by default; cookies are **not** cryptographically secure. **MUST** use `-proto_dist inet_tls` for production distribution and run the cluster on an isolated network. **MUST NOT** rely on the cookie as the sole security control. Consider `:net_kernel.allow/1` for node allow-listing.

### EPMD

EPMD (Erlang Port Mapper Daemon) maps symbolic node names (`name@host`) to a host address and port. It listens on TCP port `4369` by default; override with `-port` or the `ERL_EPMD_PORT` environment variable. (epmd_cmd.html)

`erl`/`iex` auto-starts EPMD when a node becomes distributed if no EPMD is already running. You can start it manually with `epmd -daemon`, and `epmd -names` lists registered names (wrapped by `:net_adm.names/0,1`). Environment variables include `ERL_EPMD_ADDRESS` (bind addresses) and `ERL_EPMD_PORT`. The `-no_epmd` flag runs a distributed node without EPMD, but requires an alternate `-proto_dist` module. On Windows, a single EPMD instance supports roughly 60 nodes. (epmd_cmd.html)

EPMD tracks only the *Name* part of a node name; the *Host* part is implicit from which host's EPMD is contacted.

### Starting and naming a node

Common CLI invocation:

```bash
iex --sname foo
iex --name foo@host.example.com --cookie secret --hidden
```

Use `--erl "-flag"` to pass raw Erlang flags. Erlang equivalents: `-sname`, `-name`, `-setcookie`, `-hidden`, `-connect_all false`. (erl_cmd.html)

**Long-named and short-named nodes cannot communicate with each other.** Choose one naming scheme for the whole cluster.

Programmatically, `Node.start/2` turns a non-distributed node into a distributed one, and `Node.stop/0` reverses it. (Node.html#start/2)

```elixir
Node.start(name, options) :: {:ok, pid} | {:error, term}
Node.stop() :: :ok | {:error, :not_allowed | :not_found}
```

Options include `:name_domain` (`:shortnames` or `:longnames`, default `:longnames`), `:net_ticktime`, `:dist_listen` (default `true`; `false` implies hidden), and `:hidden` (default `false`). Passing `:undefined` as the name enables dynamic node name mode (OTP 23+), which implies `dist_listen: false`, `hidden: true`, and `dist_auto_connect: never`.

`Node.start/2` is rarely used in production; prefer the CLI flags at launch.

### Hidden nodes and dynamic node names

Hidden nodes (`-hidden`) are not transitive, do not appear in `Node.list/0` (`:visible`), are not tracked by `:global`, and must be connected explicitly. Use `Node.list(:hidden)` or `Node.list(:connected)` to see them. They are useful for O&M or inspection nodes that must not disturb the cluster. (distributed.html)

Dynamic node name mode (`-name undefined` or `-sname undefined`) lets the node request its name from the first node it connects to. It implies `dist_listen: false`, `hidden: true`, and `dist_auto_connect: never`, and requires OTP 23+ on both sides.

### When distributed mode matters

Use distributed Erlang for horizontal scaling, fault tolerance across machines, distributed `:mnesia`/ETS tables, `:global` name registration, `:rpc`/`:erpc` calls, `GenServer.call({name, node}, ...)`, and remote `Task.Supervisor`.

Tradeoffs include TCP/serialization overhead, network-partition (split-brain) behavior, a larger security surface, EPMD dependency, and operational complexity. **SHOULD NOT** enable distribution unless cross-node messaging is actually required—a standalone node (`:nonode@nohost`) is simpler and more secure.

Summary:

- **MUST** use `inet_tls` and an isolated network for production distribution.
- **SHOULD** prefer `:erpc` over `:rpc` for cluster calls.
- **MUST NOT** rely on cookies as the sole security control.

```elixir
# From node B
Node.ping(:foo@host)
#=> :pong

# Spawn on :foo@host and send the result back to the caller
parent = self()
Node.spawn(:foo@host, fn -> send(parent, {:remote_result, Node.self()}) end)
```

```elixir
Node.alive?()
#=> true

Node.get_cookie()
#=> :my_cluster_cookie
```

## Port

### Overview: ports for external OS processes

For BEAM ports and external I/O, see [../beam/ports-io.md](../beam/ports-io.md).

> "Ports provide a byte-oriented interface to the external world." (ports.html)

A `Port` is a byte-oriented channel between the BEAM and an external OS program, driver, or file descriptor. All communication to and from the port goes through a single owner process. The Elixir `Port` module is a thin, mostly-inlined wrapper over Erlang port BIFs such as `:erlang.open_port/2`. (Port.html)

### Port.open/2: name types and mechanisms

`Port.open/2` is the entry point. It returns a bare port identifier, **not** `{:ok, port}`. It delegates to `:erlang.open_port/2`. (Port.html#open/2)

```elixir
Port.open(name(), list()) :: port()
```

The `name()` type:

```elixir
@type name() ::
  {:spawn, charlist() | binary()}
  | {:spawn_driver, charlist() | binary()}
  | {:spawn_executable, :file.name_all()}
  | {:fd, non_neg_integer(), non_neg_integer()}
```

- `{:spawn, command}` — runs the program with arguments parsed by spaces; looked up via `$PATH`. It **cannot** handle spaces in the executable path or arguments, so `{:spawn_executable, path}` is usually preferable. (Port.html)
- `{:spawn_executable, filename}` — runs the executable at the given absolute path. Pass arguments with the `:args` option. Resolve `$PATH` entries with `System.find_executable/1`. (System.html)
- `{:spawn_driver, command}` — spawns a linked-in driver; advanced VM-internal use.
- `{:fd, fd_in, fd_out}` — accesses the VM's own file descriptors; only for reimplementing runtime internals.

### Port options

`Port.open/2` accepts a list of options delegated to `:erlang.open_port/2`. Common options:

| Option | Meaning |
|---|---|
| `:binary` | Deliver port data as binaries rather than charlists. Strongly preferred in Elixir. |
| `:packet` (1, 2, or 4) | Frame messages with an N-byte length prefix. |
| `:line` | Deliver data one line at a time; CRLF is stripped. Messages have an `{:line, ...}` shape. |
| `:stream` | No framing; data is delivered as soon as it arrives. |
| `:use_stdio` (default `true`) / `:nouse_stdio` | Communicate over the program's stdio or not. |
| `:stderr_to_stdout` | Merge stderr into stdout; requires `:use_stdio`. |
| `:in` / `:out` | Direction(s) of communication. |
| `:eof` | Close the port on EOF from the external program. |
| `:connected` (default `true`) | The opening process is the connected process; `false` creates an unconnected port. |
| `:args` | Argument list for `:spawn_executable`. |
| `:env` | List of `{name, value}` tuples for the child environment. |
| `:hide` | Hide the spawned process window (Windows only). |
| `:parallelism` | Enable port-scheduler parallelism. |

`Port.open/2` fails synchronously—raising an error such as `:enoent`—if the OS cannot spawn the program.

### The owner process and connected process

The process that calls `Port.open/2` is the port **owner** and, by default, the **connected** process. All messages from the port are delivered to the owner. (ports.html)

`Port.connect/2` reassigns the connected process. (Port.html#connect/2)

```elixir
Port.connect(port, pid) :: true
```

It delegates to `:erlang.port_connect/2` and replies `{port, :connected}` to the **old** owner.

If the owner terminates, the port closes (and a correctly written external program should exit when its stdio closes). Setting `:connected: false` creates an unconnected port, but there is still a single owner.

### Communicating with a port

Function API:

```elixir
Port.command(port, iodata(), options \\ []) :: boolean()
Port.close(port) :: true
```

`Port.command/3` sends data to the port. Options are `:force` and `:nosuspend`; with `:nosuspend` it returns `false` instead of suspending the caller. It delegates to `:erlang.port_command/3`. (Port.html#command/3)

`Port.close/1` closes the port synchronously. It does **not** send a `{port, :closed}` reply. It delegates to `:erlang.port_close/1`. (Port.html#close/1)

Equivalent message API via `send/2`:

```elixir
{pid, {:command, data}}
{pid, :close}
{pid, {:connect, new_pid}}
```

Inspection:

```elixir
Port.list/0 :: [port()]
Port.info/1 :: keyword() | nil
Port.info/2 :: {item, term} | nil
```

`Port.list/0` returns all ports on the node. `Port.info/1,2` returns metadata such as `:name`, `:links`, `:connected`, `:input`, `:output`, and `:os_pid`, or `nil` if the port is closed. Both delegate to `:erlang.port_info/1,2`. (Port.html#info/2)

### Messages received from a port

The owner receives these messages:

| Message | Meaning |
|---|---|
| `{port, {:data, data}}` | Data from the port. `data` is a binary with `:binary`, a charlist without it, or `{:line, ...}` with `:line`. |
| `{port, :closed}` | Reply to a `:close` request. |
| `{port, :connected}` | Reply to a `:connect` request. |
| `{:EXIT, port, reason}` | Exit signal when the port crashes or the external program terminates. |

### Linked ports, exit signals, and trap_exit

A port is automatically **linked** to its owner when opened. If the external program or the port itself terminates, the owner receives an exit signal `{:EXIT, port, reason}`. (ports.html)

Per the Elixir docs, if the reason is not `:normal`, this message is only received when the owner is trapping exits. (Port.html) Therefore the owner **MUST** call `Process.flag(:trap_exit, true)` and handle `{:EXIT, port, reason}` to survive port crashes. See `### trap_exit and exit signals` in the Process section above for the fundamentals.

```elixir
port = Port.open({:spawn_executable, "/bin/cat"}, [:binary, :stream])
Process.flag(:trap_exit, true)
Port.command(port, "hello\n")

receive do
  {^port, {:data, data}} ->
    IO.inspect(data)

  {:EXIT, ^port, reason} ->
    IO.puts("port exited: #{inspect(reason)}")
end
```

### Monitoring ports

`Port.monitor/1` creates a monitor on a port. It returns a reference and delivers `{:DOWN, ref, :port, object, reason}` when the port exits. It has been available since Elixir 1.6. (Port.html#monitor/1)

```elixir
Port.monitor(port | {name, node} | name) :: reference()
```

`Port.demonitor/2` removes the monitor. (Port.html#demonitor/2)

```elixir
Port.demonitor(ref, options \\ []) :: boolean()
```

Options are `:flush` and `:info`. **MUST** pass `[:flush]` to remove a stale `DOWN` message, mirroring `Process.demonitor/2`.

**SHOULD** prefer `Port.monitor/1` over trapping exits when you only want to observe a port's lifetime from one caller.

### Orphan processes and cleanup (CRITICAL caveat)

Closing a port closes the child's stdin/stdout. However, if the VM **crashes**, a long-running external child is **not** automatically terminated; it merely loses its stdio. Not all CLI tools exit when their parent dies. (Port.html)

**MUST** design long-running port children to self-terminate on EOF or parent loss. The documented bash wrapper pattern kills the child when stdin closes:

```bash
#!/bin/bash
exec "$@" &
pid=$!
while read; do :; done
kill -KILL $pid
```

Do not assume the BEAM will reap external processes.

### Windows argument caveat

On Unix, arguments are passed as an array. On Windows, the child parses the raw command line, so `.bat`/`.com` files run through `cmd.exe` whose parsing allows argument injection. **MUST NOT** pass untrusted arguments to `.bat`/`.com` files; prefer explicit `.exe` extensions. (System.html / Port.html)

### Port vs NIF vs System.cmd

| Approach | Process boundary | Overhead | Failure isolation | Build complexity | Streaming I/O | Sync/Async | Use when |
|---|---|---|---|---|---|---|---|
| Port | separate OS process | high (serialization) | crash isolated | no C needed | async message stream | async | external programs, sandboxes |
| NIF | in-process (VM) | lowest | NIF crash can take down the VM | C/build chain required | synchronous | sync | measured, perf-critical native code |
| `System.cmd/3` | separate OS process (Port wrapper) | high | crash isolated | no C needed | collected into memory | sync | one-shot "run and collect output" |

> "Also consider using `System.cmd/3` if all you want is to execute a program and retrieve its return value." (Port.html)

`System.cmd/3` returns `{output, exit_status}` synchronously and accepts options such as `:into`, `:lines`, `:cd`, `:env`, `:stderr_to_stdout`, and `:use_stdio`. (System.html#cmd/3)

**SHOULD** use `System.cmd/3` for one-shot commands; use a raw `Port` when you need streaming, long-lived, or interactive I/O; use a NIF only when you have measured a need and accept the VM-crash risk.

```elixir
# One-shot command via System.cmd/3
{output, 0} = System.cmd("elixir", ["--version"])
```

```elixir
# Streaming port with trapped exits
port =
  Port.open(
    {:spawn_executable, System.find_executable("cat")},
    [:binary, :stream, :eof]
  )

Process.flag(:trap_exit, true)
Port.command(port, "hello\n")
Port.close(port)

receive do
  {^port, {:data, data}} -> IO.inspect(data)
  {:EXIT, ^port, reason} -> IO.puts("port exited: #{inspect(reason)}")
end
```

## Review checklist

- [ ] All spawn targets are linked, monitored, or supervised; no orphan processes remain.
- [ ] Monitors are demonitored with `[:flush]` once no longer needed.
- [ ] No reliance on `send/2` to detect liveness; a monitor is used instead.
- [ ] No `{:EXIT, _, _}` receive without `trap_exit` set.
- [ ] No `:high` or `:max` priority in application code.
- [ ] `:message_queue_len` is checked before pulling `:messages`.
- [ ] No busy-loops; every loop blocks on `receive` or otherwise yields.
- [ ] No `Process.sleep/1` used to wait for another process to finish.
- [ ] No process dictionary used for shared or cross-process state.
- [ ] No dynamic atom registration via `String.to_atom/1`.
- [ ] Large terms are shared via ETS/`persistent_term`, not sent repeatedly.
- [ ] `send_after/4` return values are captured for cancellation when cancellation is needed.

## Implementation checklist

- [ ] Prefer `Task` / `Task.Supervisor` over raw `spawn` for concurrent work.
- [ ] Use `spawn_monitor/1,3` when you need the worker's result or exit from the caller.
- [ ] Use `spawn_link/1,3` only for shared-fate process pairs inside a supervision tree.
- [ ] Always pair `monitor/1` with `demonitor(ref, [:flush])`.
- [ ] Detect completion/failure with `{:DOWN, ref, :process, pid, reason}`.
- [ ] Keep mailboxes small; design consumers to keep up and use demand-driven backpressure.
- [ ] Use `Process.flag(:trap_exit, true)` only in supervisor-like processes.
- [ ] Use `:message_queue_data: :off_heap` for processes receiving many large messages.
- [ ] Capture `send_after` references in state when timers must be cancelable.
- [ ] Use `:max_heap_size` with `:kill` to bound runaway memory in workers.
- [ ] Reserve `:priority` changes for documented, justified cases.
- [ ] Put per-request metadata (logger/request id) in the process dictionary; keep everything else in message-passed state.

## Validation hooks

- `:observer` / `:observer.start()` — inspect live processes, mailboxes (`:messages`), links, monitors, memory, reductions, and state.
- `Process.info(pid)` / `Process.info(pid, :message_queue_len)` — quick liveness and mailbox check.
- `:recon` / `:recon_proc` (external library) — production process and memory profiling.
- `mix format --check-formatted` — formatting.
- `mix credo` — flags process-dictionary misuse, complex code, and dynamic-atom warnings.
- `mix dialyzer` — type-checks return and argument types of Process calls.
- Note: there is no automated "concurrency-specific" gate; the hooks are general Elixir tooling plus runtime inspection.

## Examples

### spawn and message passing

```elixir
defmodule Counter do
  def start(initial) do
    spawn(fn -> loop(initial) end)
  end

  defp loop(count) do
    receive do
      {:increment, from} ->
        new_count = count + 1
        send(from, {:count, new_count})
        loop(new_count)

      {:get, from} ->
        send(from, {:count, count})
        loop(count)
    end
  end
end

pid = Counter.start(0)
send(pid, {:increment, self()})
receive do
  {:count, n} -> IO.puts("count: #{n}")
end
```

### spawn_monitor: getting a result or failure

```elixir
{pid, ref} =
  spawn_monitor(fn ->
    result = heavy_computation()
    send(parent, {:result, result})
  end)

receive do
  {:result, value} ->
    IO.inspect(value)
    Process.demonitor(ref, [:flush])

  {:DOWN, ^ref, :process, ^pid, reason} ->
    IO.puts("worker died: #{inspect(reason)}")
end
```

### link vs monitor under failure

```elixir
# Using a link: if the worker exits abnormally, the caller also exits
# unless the caller traps exits.
linked_pid = spawn_link(fn ->
  Process.sleep(100)
  exit(:boom)
end)

# Using a monitor: the caller observes the failure as a message.
{monitored_pid, ref} = spawn_monitor(fn ->
  Process.sleep(100)
  exit(:boom)
end)

receive do
  {:DOWN, ^ref, :process, ^monitored_pid, reason} ->
    IO.puts("monitored worker died: #{inspect(reason)}")
end
```

### trap_exit and {:EXIT, _, _}

```elixir
pid =
  spawn(fn ->
    Process.flag(:trap_exit, true)
    linked = spawn_link(fn -> exit(:normal_reason) end)

    receive do
      {:EXIT, ^linked, reason} ->
        IO.puts("linked process exited with #{inspect(reason)}")
    end
  end)

Process.sleep(200)
Process.alive?(pid)
#=> true
```

### send_after with cancellation

```elixir
defmodule TimeoutState do
  defstruct [:timer_ref, :value]

  def schedule(timeout \\ 5_000) do
    ref = Process.send_after(self(), :timeout, timeout)
    %__MODULE__{timer_ref: ref, value: nil}
  end

  def cancel(%__MODULE__{timer_ref: ref} = state) do
    Process.cancel_timer(ref)
    flush_timeout()
    %{state | timer_ref: nil}
  end

  defp flush_timeout do
    receive do
      :timeout -> flush_timeout()
    after
      0 -> :ok
    end
  end
end

state = TimeoutState.schedule(5_000)
state = TimeoutState.cancel(state)
```

### Flushing the mailbox

```elixir
def flush do
  receive do
    _msg -> flush()
  after
    0 -> :ok
  end
end

send(self(), :noise)
flush()
```

### Inspecting a process

```elixir
pid = spawn(fn ->
  Process.register(self(), :inspector_example)
  receive do
    :stop -> :ok
  end
end)

Process.info(pid, [:registered_name, :message_queue_len, :trap_exit])
#=> [
#=>   registered_name: :inspector_example,
#=>   message_queue_len: 0,
#=>   trap_exit: false
#=> ]

send(pid, :stop)
```

### monitor + demonitor([:flush])

```elixir
{pid, ref} = spawn_monitor(fn -> Process.sleep(100) end)

# Later, when we no longer care:
Process.demonitor(ref, [:flush])

# The DOWN message, if already sent, has been removed from the mailbox.
receive do
  {:DOWN, ^ref, _, _, _} -> :unexpected
after
  0 -> :clean
end
#=> :clean
```

## Common mistakes

1. **Busy-loop without receive or sleep** — fix: block on `receive`; if polling is truly required, use `send_after` with a small interval rather than a tight loop.
2. **Unbounded mailbox growth** — fix: design consumers to keep up; apply demand-driven backpressure or use bounded buffers.
3. **Leaking monitors (never demonitored)** — fix: always call `Process.demonitor(ref, [:flush])` when the monitor is no longer needed.
4. **Linking to arbitrary processes** — fix: monitor from the observer side instead; reserve links for shared-fate pairs and supervision trees.
5. **Sending huge terms repeatedly** — fix: share data through ETS or `persistent_term` and send only references or small messages.
6. **Selective receive scanning cost on big mailboxes** — fix: keep mailboxes small; separate message types into different processes or use dedicated consumers.
7. **Expecting `{:EXIT, _, _}` without trap_exit** — fix: set `Process.flag(:trap_exit, true)` before receiving exit signals.
8. **Relying on `send/2` to detect liveness** — fix: use `Process.monitor/1` or `Process.alive?/1` for local pids.
9. **Using `Process.sleep/1` to wait for another process** — fix: use `receive` plus a monitor or a `Task.await/2`.
10. **Using the process dictionary for shared state** — fix: pass state in messages or use a GenServer/ETS.
11. **Dynamic atom registration** — fix: use `Registry` via `:via` and `String.to_existing_atom/1` if atoms are unavoidable.
12. **Calling `Process.info/2` on a remote pid** — fix: remote pids raise `ArgumentError`; use `:erpc.call/4` to inspect remote processes.
13. **Forgetting `:flush` on demonitor** — fix: always pass `[:flush]` unless you intentionally want to handle a stale DOWN.
14. **Sending `send_after` to a registered atom that gets unregistered** — note: there is no auto-cancel for atom destinations; the message is dropped if the name is gone at delivery time.

## Strict vs contextual guidance

### Strict

- MUST monitor or link/supervise every spawned process; no orphans.
- MUST demonitor with `[:flush]`.
- MUST set `trap_exit` before expecting `{:EXIT, _, _}`.
- MUST NOT use `send/2` to detect liveness.
- MUST NOT use `:high` or `:max` priority in application code.
- MUST NOT use the process dictionary for shared state.
- MUST NOT use `String.to_atom/1` for process names.
- MUST NOT busy-loop without yielding.
- MUST NOT rely on `terminate`/send_after atom-cancellation semantics without verifying them.

### Conventions (not enforced by the compiler/OTP)

- Prefer `Task` / `Task.Supervisor` over raw `spawn` for concurrent work.
- Capture `send_after` references in state when cancelable timers are required.
- Use `:off_heap` for processes that receive many large messages.
- Use `:max_heap_size` to bound worker memory.
- Reserve hibernation for long-idle processes that hold a large heap.

### Contextual tradeoffs

- `spawn_link` vs `spawn_monitor` — choose shared fate vs observation.
- `:trap_exit` in a GenServer-like process vs supervisors only — supervisors need it; most workers should not trap exits unless they own linked resources.
- Process dictionary for per-request metadata (acceptable) vs state (not acceptable).
- `:fullsweep_after` tuning — lower values mean more CPU and less memory; raise with care.

## Policy decisions for individual repos

- Whether raw `spawn*` is allowed at all, or only via `Task`/supervisor.
- Required monitor cleanup helper or convention.
- Whether `:trap_exit` is permitted outside supervisors.
- Whether the process dictionary is banned or scoped to logger/request metadata.
- `:max_heap_size` policy for workers.
- Priority policy (e.g., never touch).
- Timer management convention (capture refs in state, named timers, etc.).

## Related docs

- `docs/elixir/otp-supervision.md` — GenServer, Supervisor, Registry (`:via`), supervision trees, and `trap_exit` under supervisors.
- `docs/elixir/core-modules.md` — Enum/Stream/Map (data transformation, not concurrency).
- `docs/elixir/language-fundamentals.md` — pattern matching, guards, and control flow used in `receive`.
- `docs/elixir/testing-exunit.md` — testing concurrent and asynchronous code.
- `docs/elixir/static-analysis-credo.md` — Credo warnings for process dictionary misuse and more.
- `docs/elixir/typespecs-and-dialyzer.md` — typing Process return values.
- `docs/elixir/configuration-and-runtime.md` — runtime and scheduler configuration.

## Related skills

- None defined yet.
