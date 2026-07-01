# BEAM and OTP Internals

## Purpose

This is a thin **Elixir ↔ BEAM mapping index** plus a retained summary of **BEAM VM internals** that have no authoritative home in `docs/beam/`. For the deep Erlang/OTP runtime semantics (supervision, links/monitors, `:sys`/`:proc_lib`, releases, behaviours, applications), see the `docs/beam/` corpus.

Future agents who touch supervision trees, exit-signal handling, `:sys`/`:proc_lib` debugging, release handling, scheduler/GC tuning, or hot code upgrade should consult this document for the Elixir API mapping, then follow the linked `docs/beam/` doc for the runtime semantics.

## Sources used

- https://www.erlang.org/doc/system/ref_man_processes.html (process reference, states, signals)
- https://www.erlang.org/doc/apps/erts/erlang.html (erlang module BIFs: spawn_opt, process_info, system_info, statistics, etc.)
- https://blog.stenmans.org/theBeamBook/ (The BEAM Book by Erik Stenman — schedulers, reductions, GC, message passing internals)

This page reflects OTP 27–29 semantics (Erlang 27+ renamed `exit/2` to `exit_signal/2`; `exit_signal` is used here for the modern form).

## Related BEAM guidance

This doc is an Elixir ↔ BEAM mapping index; the authoritative BEAM runtime semantics live in `docs/beam/`:

- [../beam/overview.md](../beam/overview.md) — for the ERTS/BEAM/OTP stack overview and OTP design principles.
- [../beam/processes-and-messages.md](../beam/processes-and-messages.md) — for process states, message passing, and the process dictionary.
- [../beam/supervision.md](../beam/supervision.md) and [../beam/gen-server.md](../beam/gen-server.md) — for supervisor flags/strategies/child specs and the gen_server callback contract.
- [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md) — for links, monitors, exit-signal reception rules, and `trap_exit`.

## BEAM Virtual Machine

> **Sourcing note:** The VM-internals below (schedulers, reductions, GC, memory model, timing wheel) are summarized from [The BEAM Book](https://blog.stenmans.org/theBeamBook/) (community-sourced). `docs/beam/` does not yet cover VM internals at this depth — this is a known revisit-later coverage gap. Process-level runtime semantics (states, signals, message passing) ARE covered in [../beam/processes-and-messages.md](../beam/processes-and-messages.md).

### ERTS and the node concept

An Erlang Runtime System (ERTS) instance is an OS process that runs the BEAM emulator and the OTP middleware. A *node* is a named, running ERTS instance. See [../beam/overview.md](../beam/overview.md) for the full stack overview and node/distribution details.

### Schedulers

From [The BEAM Book](https://blog.stenmans.org/theBeamBook/) and the [erlang module](https://www.erlang.org/doc/apps/erts/erlang.html):

- BEAM runs **one scheduler per core** in separate OS threads (when SMP is enabled). Each scheduler has its own **ready queue** (runnable processes) and **waiting queue** (blocked processes).
- Scheduling is **preemptive at the Erlang level** but **cooperative at the C level**: a scheduler runs Erlang processes until they yield by exhausting their reductions (see below), then switches.
- Default: one scheduler thread per enabled core (physical or hyperthreaded), all online.

Inspect at runtime:

```elixir
:erlang.system_info(:smp_support)        # true
:erlang.system_info(:schedulers)          # configured scheduler threads
:erlang.system_info(:schedulers_online)  # online scheduler threads
System.schedulers_online()                # Elixir wrapper
:erlang.system_info(:cpu_topology)        # detected CPU topology
```

### Dirty schedulers

Dirty schedulers run NIFs that may block; two kinds (normal dirty, CPU-native dirty). See [../beam/nifs.md](../beam/nifs.md).

### Reductions and preemption

From [The BEAM Book](https://blog.stenmans.org/theBeamBook/):

- A **reduction** is the unit of scheduling credit. Roughly, each function call counts as one reduction. Each process gets a budget of reductions; when the budget (`fcalls`) reaches 0, the process is preempted and the scheduler picks the next runnable process.
- `CONTEXT_REDS = 4000` (raised from 2000 in OTP-20). This is the per-process reduction budget.
- `INPUT_REDUCTIONS = 2 * CONTEXT_REDS = 8000`. Used for input (message receive) accounting.
- Because Erlang has **no loop construct except recursion and list comprehensions**, it is impossible to loop forever without making a function call — so every long-running computation eventually yields. This is the foundation of fair preemption.
- BIFs/NIFs complicate this: a single BIF call historically could starve the scheduler. The classic example is `binary_to_term`/`term_to_binary` on huge binaries pre-R16, which could run unbounded. Modern BIFs that may be long-running either yield cooperatively or are moved to dirty schedulers.

Inspect and influence reductions:

```elixir
:erlang.process_info(pid, :reductions)   # reductions consumed so far
:erlang.bump_reductions(n)                # explicitly consume n reductions (rare; for NIFs)
```

### Process state machine

From [ref_man_processes.html](https://www.erlang.org/doc/system/ref_man_processes.html), an Erlang process is always in exactly one of these states:

| State | Meaning |
|---|---|
| `free` | Process slot is unused. |
| `runnable` | In a scheduler ready queue, waiting to run. |
| `waiting` | Blocked in a `receive`. |
| `running` | Currently executing on a scheduler. |
| `exiting` | Terminating; releasing resources. |
| `garbing` | Running a garbage collection. |
| `suspended` | Suspended via `:erlang.suspend_process/2`; only responds to system messages. |

The `suspended` state is **only for debugging** (`:erlang.suspend_process/2` / `:erlang.resume_process/1`). Suspension is reference-counted (an `rcount`); a process resumes only when all suspenders have resumed it. Do not use suspension for flow control.

### Priority queues

Each scheduler maintains three priority queues:

| Queue | Contents | Schedule count |
|---|---|---|
| `max` | `priority = max` processes | highest priority |
| `high` | `priority = high` processes | high |
| `normal` + `low` | `priority = normal` and `priority = low` | normal processes scheduled once per pass |

A normal process is scheduled with **schedule count 1**. A low-priority process has schedule count **8**: the scheduler traverses the low queue 7 extra times before scheduling a low-priority process, so low-priority work is heavily deprioritized. Set priority via `:erlang.process_flag(:priority, :low | :normal | :high | :max)` (Elixir: `Process.flag(:priority, ...)`).

### Load balancing

From [The BEAM Book](https://blog.stenmans.org/theBeamBook/):

- Schedulers balance load via **task stealing** and **migration**.
- Strategy: **compact load to as few schedulers as possible without overloading them**. Idle schedulers are parked; busy schedulers steal work.
- The migration path is recalculated every `2000 * CONTEXT_REDS` reductions (i.e. every 8,000,000 reductions with `CONTEXT_REDS = 4000`).
- AMQL (Average Max Queue Length) drives migration decisions: schedulers with long ready queues shed work to those with short ones.

### Timing wheel

BEAM's timer wheel is a fixed-size circular array of timer slots.

- `TIW_SIZE = 65536` slots (or `8192` for small-memory-footprint builds).
- A timer set to fire after `T` milliseconds is inserted at `(tiw_pos + T) % TIW_SIZE`.
- **Timer guarantee**: a timer will **not** trigger before its set time; it **may** trigger later (the wheel is scanned once per tick, and the scheduler may be busy).

This is why `Process.send_after/3` and `:timer.send_interval/3` are best-effort on the late side.

### Ports

Ports are the abstraction for communication outside the VM (sockets, pipes, external programs). See [../beam/ports-io.md](../beam/ports-io.md).

### Process memory model

Each process has its own memory region:

```
+-------------------+ <- high address
|       heap        |   grows upward
|                   |
|   (grows toward   |
|    each other)    |
|                   |
|      stack        |   grows downward
+-------------------+ <- low address
```

- **Stack + heap + mailbox + PCB**. The stack grows toward lower addresses; the heap grows toward higher addresses; they grow toward each other in one contiguous block. A collision triggers garbage collection (or heap growth).
- `min_heap_size` default = **233 words**.
- Key PCB (process control block) fields: `id`, `htop`, `stop`, `heap`, `hend`, `heap_sz`, `min_heap_size`, `fcalls`, `reds`, `priority`, `initial`, `current`.

### Garbage collection

From [The BEAM Book](https://blog.stenmans.org/theBeamBook/):

- BEAM uses a **generational copying GC** per process.
- **Minor collection**: copies live data from the young heap to a new heap; survivors that have survived enough collections are promoted to the old heap via the `high_water` mark.
- **Major collection / full sweep**: triggered when `gen_gcs` reaches `max_gen_gcs` (default **65535**). A full sweep walks the entire heap.
- **Why copying GC works for Erlang**: Erlang terms are **immutable** — there are no cyclic references, so the GC never has to chase pointer cycles. Empirically ~75% of terms are cons cells, and ~99% are short-lived or small, so copying collectors are cheap.
- **Off-heap data and binaries**: large binaries live in a shared off-heap area refcounted per process. Relevant PCB fields: `bin_vheap_sz`, `off_heap`, `mbuf` (message buffer / heap fragment).

Inspect and force GC:

```elixir
:erlang.process_info(pid, :garbage_collection)
:erlang.garbage_collect()                  # GC the calling process
:erlang.garbage_collect(pid)               # GC a specific process
:erlang.process_info(pid, :heap_size)
:erlang.process_info(pid, :total_heap_size)
:erlang.memory()                           # VM-wide memory breakdown
```

### Message passing

Set the `message_queue_data` process flag via Elixir:

```elixir
Process.flag(:message_queue_data, :off_heap)   # or :on_heap
# Or at spawn time:
:erlang.spawn_opt(fn -> :ok end, [:message_queue_data, :off_heap])
```

See [../beam/processes-and-messages.md](../beam/processes-and-messages.md) for the message-passing model.

### Process dictionary

The process dictionary is a process-local key-value store accessible via `Process.get/1,2`, `Process.put/2`, `Process.delete/1`, and `Process.get_keys/0,1`.

> The process dictionary is **discouraged** for application state. It is invisible to `:sys.get_state/1`, bypasses the GenServer callback contract, and is not cleaned up across hot code upgrades in any structured way. Use GenServer state instead. Legitimate uses: request-scoped metadata (e.g. `Logger` metadata via `Logger.metadata/1`), `:initial_call` bookkeeping. See [../beam/processes-and-messages.md](../beam/processes-and-messages.md).

### Key `:erlang` module functions for inspection

| Function | Purpose |
|---|---|
| `:erlang.system_info/1` | `:schedulers`, `:schedulers_online`, `:smp_support`, `:cpu_topology`, `:dirty_cpu_schedulers`, `:dirty_io_schedulers`, `:wordsize`, `:system_version` |
| `:erlang.process_info/2` | `:reductions`, `:heap_size`, `:total_heap_size`, `:stack_size`, `:message_queue_len`, `:links`, `:monitors`, `:monitored_by`, `:priority`, `:trap_exit`, `:status`, `:current_function`, `:initial_call`, `:garbage_collection` |
| `:erlang.garbage_collect/0,1` | Force a full GC of the current/specified process |
| `:erlang.hibernate/3` | Discard the stack and heap, keep only the mailbox; wake on next message (see `:proc_lib.hibernate/3` for OTP-safe variant) |
| `:erlang.spawn_opt/4` | Spawn with options: `:link`, `:monitor`, `{:priority, p}`, `{:fullsweep_after, n}`, `{:min_heap_size, n}`, `{:min_bin_vheap_size, n}`, `{:max_heap_size, bytes}`, `{:message_queue_data, :on_heap | :off_heap}` |
| `:erlang.process_flag/2` | `:trap_exit`, `:priority`, `:save_calls`, `:sensitive`, `:async_dist`, `:max_heap_size`, `:message_queue_data` |
| `:erlang.statistics/1` | `:scheduler_wall_time`, `:reductions`, `:run_queue`, `:garbage_collection`, `:io`, `:context_switches` |

### Connect to Elixir

| Erlang | Elixir |
|---|---|
| `erlang:system_info(schedulers_online)` | `System.schedulers_online/0` |
| `erlang:process_info(Pid, Item)` | `Process.info(pid, item)` |
| `erlang:statistics(reductions)` | `:erlang.statistics(:reductions)` (no direct Elixir wrapper) |
| `erlang:process_flag(trap_exit, true)` | `Process.flag(:trap_exit, true)` |
| `erlang:process_dictionary()` / `put/get` | `Process.get/put` |
| `erlang:processes()` | `Process.list/0` |

## Elixir ↔ BEAM mapping index

The deep Erlang/OTP theory that previously occupied this space (supervision theory, exit-signal theory, `:sys`/`:proc_lib` detail, release handling, design-principles recap) is now authored authoritatively in `docs/beam/`. This index gives the one-line Elixir API → BEAM doc mapping so Elixir developers can cross-reference the runtime semantics without duplicating them here.

### OTP design principles & behaviours

- Behaviours (generic/specific split): `use GenServer` / `use Supervisor` inject `@behaviour` plus a default `child_spec/1`. `-behaviour(Behaviour)` triggers compiler warnings for missing callbacks (Elixir: `@behaviour`). → [../beam/otp-behaviours.md](../beam/otp-behaviours.md), [../beam/overview.md](../beam/overview.md).
- Standard behaviour → Elixir mapping: `gen_server`→`GenServer`; `gen_statem`→`:gen_statem` (no first-party Elixir wrapper; `GenStateMachine` is a library); `gen_event`→`:gen_event`; `supervisor`→`Supervisor`/`DynamicSupervisor`/`PartitionSupervisor`.

### Applications

- `def application` in `mix.exs` compiles to the `.app` resource file; `Application` wraps `:application.*` (`start/2` returns the top supervisor, `stop/1`, `prep_stop/1`, `start_phase/3`, `config_change/3`, `get_env/put_env/compile_env`). → [../beam/applications.md](../beam/applications.md).

### Releases & hot code upgrade

- `mix release` (Elixir 1.9+) produces **immutable releases**; it does NOT use `appup`/`relup` by default. Hot code upgrade via `appup`/`relup` + SASL is an advanced Erlang/OTP feature; `code_change/3` and `@vsn` exist on `GenServer`. `:code.purge/1` / `:code.soft_purge/1` for manual reload; BEAM keeps two module versions simultaneously. → [../beam/releases.md](../beam/releases.md).

### Supervision

- `Supervisor.init(children, strategy:, max_restarts:, max_seconds:, auto_shutdown:)`; input options `:max_restarts`/`:max_seconds` map internally to `intensity`/`period`. Strategies `:one_for_one` / `:one_for_all` / `:rest_for_one` (Elixir drops `:simple_one_for_one` → use `DynamicSupervisor`). Child spec map `%{id, start, restart, shutdown, type, modules, significant}`; `child_spec/1` convention; `Supervisor.child_spec/2` builds+overrides. Restart `:permanent`/`:transient`/`:temporary`; shutdown `:brutal_kill`/integer-ms/`:infinity`; `:auto_shutdown` `:never`/`:any_significant`/`:all_significant`. → [../beam/supervision.md](../beam/supervision.md).

### Error handling & exit signals

- Three exception classes `:error` / `:exit` / `:throw`; `raise/1,2`, `reraise/2,3`, `Kernel.exit/1`, `try/rescue` (matches `:error`), `try/catch` (matches `:exit`/`:throw`). `Process.exit/2` sends a signal to ANOTHER process (distinct from `Kernel.exit/1` which stops the current process). `Process.flag(:trap_exit, true)` converts exit signals to `{:EXIT, from, reason}` messages; `:kill` is untrappable. Elixir exception structs / `defexception` / `Exception` behaviour are Elixir-specific (see `docs/elixir/error-handling.md`). → [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md).

### Links & monitors

- `Process.link/1` (bidirectional), `Process.unlink/1`, `Process.monitor/1` (unidirectional → `{:DOWN, ref, :process, pid, reason}`), `Process.demonitor/2` (pass `[:flush]`), `Kernel.spawn_link/1,3`, `Kernel.spawn_monitor/1,3`. → [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md).

### `:sys` & `:proc_lib` (special processes)

- `:sys.get_state/1`, `:sys.get_status/1`, `:sys.replace_state/2`, `:sys.trace/2`, `:sys.suspend/1` / `:sys.resume/1`, `:sys.no_debug/1` (debugging only; default 5000 ms timeout). `:proc_lib.start_link/3,4,5`, `init_ack/1,2`, `init_fail/2,3` (OTP 26+), `:proc_lib.hibernate/3` (use this, not the raw BIF, for proc_lib processes); special-process `system_*` callbacks (`system_continue/3`, `system_terminate/4`, `system_code_change/4`, `system_get_state/1`, `system_replace_state/2`). `GenServer`/`Supervisor` already implement all of this; you only call `:sys`/`:proc_lib` directly when debugging or writing a custom behaviour. → [../beam/proc-lib-and-sys.md](../beam/proc-lib-and-sys.md), [../beam/runtime-debugging.md](../beam/runtime-debugging.md).

### Erlang → Elixir API mapping (reference tables)

| Erlang | Elixir |
|---|---|
| `link/1` | `Process.link/1` |
| `monitor(process, Pid)` | `Process.monitor/1` |
| `process_flag(trap_exit, true)` | `Process.flag(:trap_exit, true)` |
| `exit/1` | `exit/1` |
| `error/1` | `raise/1` |
| `try ... catch` | `try/catch` / `try/rescue` |
| supervisor (traps exits internally) | `Supervisor` (traps exits internally) |

| Erlang | Elixir |
|---|---|
| `sys:get_state/1` | `:sys.get_state(pid)` |
| `sys:replace_state/2` | `:sys.replace_state(pid, fn)` |
| `sys:suspend/1` | `:sys.suspend(pid)` |
| `sys:resume/1` | `:sys.resume(pid)` |
| `sys:trace/2` | `:sys.trace(pid, true)` |
| `sys:get_status/1` | `:sys.get_status(pid)` |
| `sys:statistics/2` | `:sys.statistics(pid, :get)` |
| `sys:log/2` | `:sys.log(pid, :print)` |
| `sys:terminate/2` | `:sys.terminate(pid, reason)` |
| `proc_lib:start_link/5` | `:proc_lib.start_link/5` |
| `proc_lib:init_ack/1,2` | `:proc_lib.init_ack/1,2` |
| `proc_lib:hibernate/3` | `:proc_lib.hibernate/3` |

## Review checklist

When reviewing code/configs that touch BEAM/OTP internals, verify:

- [ ] `trap_exit` is set only where cleanup-on-shutdown is actually needed.
- [ ] `:sys.*` functions are used **only for debugging**, never in production logic.
- [ ] `message_queue_data` is `:on_heap` unless `:off_heap` was measured to help.
- [ ] No reliance on stacktrace for control flow.
- [ ] `init_ack`/`init_fail` used correctly in any `:proc_lib` special process.
- [ ] `:proc_lib.hibernate/3` (not the raw BIF) used for `:proc_lib` processes.
- [ ] `code_change/3` and `@vsn` are in place if hot upgrades are in scope.

For supervision-specific checklists, see [../beam/supervision.md](../beam/supervision.md) and `docs/elixir/otp-supervision.md`.

## Implementation checklist

When implementing OTP-compliant processes and supervision trees:

- [ ] Call `init_ack` (or return from `init/1` in a GenServer) to acknowledge start.
- [ ] Handle system messages if writing a special process (implement the `system_*` callbacks).
- [ ] Set `trap_exit` only when the process must clean up on a linked process's exit.
- [ ] Provide `child_spec/1` for any module that will be supervised (or rely on `use GenServer`'s default).
- [ ] Implement `code_change/3` if hot upgrades are in scope; set `@vsn`.
- [ ] Prefer automatic shutdown over manual `terminate_child` for "stop the system when X exits".

For supervision implementation details, see [../beam/supervision.md](../beam/supervision.md) and `docs/elixir/otp-supervision.md`.

## Validation hooks

Validate BEAM/OTP behavior at runtime with:

| Goal | Tool |
|---|---|
| Inspect a process's state (debugging) | `:sys.get_state(pid)` |
| Inspect a process's full status | `:sys.get_status(pid)` |
| Inspect process internals | `:erlang.process_info(pid, :reductions)`, `Process.info(pid, :heap_size)` |
| Scheduler utilization | `:erlang.statistics(:scheduler_wall_time)` |
| Reductions / run queue | `:erlang.statistics(:reductions)`, `:erlang.statistics(:run_queue)` |
| GC stats | `:erlang.statistics(:garbage_collection)`, `:erlang.process_info(pid, :garbage_collection)` |
| System info | `:erlang.system_info(:schedulers_online)`, `System.schedulers_online/0` |
| VM-wide memory | `:erlang.memory()` |
| Supervisor child counts | `Supervisor.count_children(sup)` |
| Visual inspection | `:observer.start()` (the Observer GUI: processes, applications, memory, table viewer, trace) |

`:observer` is the single most useful runtime inspection tool — it shows the process tree, per-process reductions/memory/message-queue, application tree, ETS tables, and memory allocators.

## Examples

### 1. Inspecting scheduler/reduction info

```elixir
:erlang.system_info(:smp_support)            # true
:erlang.system_info(:schedulers)             # configured scheduler threads
:erlang.system_info(:schedulers_online)      # online scheduler threads
System.schedulers_online()                    # Elixir wrapper

:erlang.statistics(:reductions)              # {TotalReductions, WordsSinceLastCall}
:erlang.statistics(:run_queue)               # total run-queue length across schedulers

# Enable scheduler_wall_time first, then sample twice:
:erlang.system_flag(:scheduler_wall_time, true)
{_, t0} = :erlang.statistics(:scheduler_wall_time)
Process.sleep(1000)
{_, t1} = :erlang.statistics(:scheduler_wall_time)
# Compute per-scheduler utilization from t0 -> t1
```

### 2. Using `:sys` on a GenServer

```elixir
{:ok, pid} = GenServer.start_link(Stack, [:a, :b, :c])

:sys.get_state(pid)                          # [:a, :b, :c]
:sys.replace_state(pid, fn stack -> [:z | stack] end)  # [:z, :a, :b, :c]
:sys.get_state(pid)                          # [:z, :a, :b, :c]

:sys.trace(pid, true)                        # print all system events to stdout
# ... trigger some calls ...
:sys.no_debug(pid)                           # disable when done

{:status, ^pid, {_, _}, items} = :sys.get_status(pid)
# items include :pdict, :state, :parent, :debug, :misc
```

### 3. Demonstrating `trap_exit` and exit signal handling

```elixir
defmodule TrapDemo do
  def run do
    flag = Process.flag(:trap_exit, true)
    pid = spawn_link(fn ->
      Process.sleep(100)
      exit(:boom)
    end)

    receive do
      {:EXIT, ^pid, reason} -> {:trapped, reason}
    after
      1_000 -> :timeout
    end
  after
    Process.flag(:trap_exit, flag)  # restore
  end
end

TrapDemo.run()   # => {:trapped, :boom}
```

### 4. Forcing GC and inspecting GC info

```elixir
{:ok, pid} = GenServer.start_link(Cache, [])

:erlang.process_info(pid, :heap_size)            # words in the heap
:erlang.process_info(pid, :total_heap_size)      # heap + off-heap
:erlang.process_info(pid, :garbage_collection)   # {min_bin_vheap_size, ...}

:erlang.garbage_collect(pid)                      # force a full GC
:erlang.process_info(pid, :heap_size)             # likely smaller now
```

### 5. Special processes

`GenServer` already implements the full `:sys`/`:proc_lib` special-process protocol. You only need a raw `:proc_lib` loop when implementing a custom behaviour that none of the standard behaviours cover. See [../beam/proc-lib-and-sys.md](../beam/proc-lib-and-sys.md) for the callback contract.

## Common mistakes

1. **Using `:sys.get_state` in production code** — it is a debugging tool with a 5000 ms timeout and suspension semantics. Use a dedicated `handle_call` instead.
2. **Not trapping exits when cleanup is needed** — a linked process dies and the GenServer never gets a chance to flush. Set `Process.flag(:trap_exit, true)` in `init/1`.
3. **Switching to `message_queue_data: :off_heap` without measuring** — increases memory and adds a copy step; only helps for overloaded receivers.
4. **Relying on the stacktrace for logic** — TCO, depth limits, and compiler changes make it unstable. Use it only for debugging.
5. **Expecting `mix release` to do hot upgrades** — `mix release` produces immutable releases. Hot upgrades require `appup`/`relup` and SASL.
6. **Using `erlang:hibernate/3` directly on a `:proc_lib` process** — breaks crash reporting on wake. Use `:proc_lib.hibernate/3`.
7. **Returning a start error via `init_ack`** — the process can exit after `init_ack` returns, blocking VM resources. Use `init_fail/2,3` (OTP 26+).
8. **Forgetting `code_change/3` when hot upgrades are in scope** — the upgrade fails or loses state.
9. **Assuming `:code.purge/1` is safe** — it kills processes still running the old version. Use `:code.soft_purge/1` first.
10. **Treating `timeout: 0` as guaranteed immediate execution** — it is a timeout, not a scheduling guarantee. Use `{:continue, ...}`.

For supervision-specific mistakes, see [../beam/supervision.md](../beam/supervision.md) and `docs/elixir/otp-supervision.md`.

## Strict vs contextual guidance

### Strict (always / never)

- **Never** use `:sys.get_state`/`:sys.replace_state` in production logic — debugging only.
- **Always** call `init_ack` (or return from `init/1`) in a `:proc_lib` process.
- **Always** use `:proc_lib.hibernate/3`, not the raw BIF, for `:proc_lib` processes.
- **Never** rely on the stacktrace for control flow.
- **Never** use `:code.purge/1` when `:code.soft_purge/1` would do.
- **Always** implement `system_*` callbacks if writing a special process.

### Contextual (depends on workload)

- `message_queue_data` (`:on_heap` vs `:off_heap`) — measure before switching.
- Process `:priority` (`:low`/`:normal`/`:high`/`:max`) — only for proven contention cases.
- `:fullsweep_after`, `:min_heap_size`, `:max_heap_size` — tune only with measurement.
- Hibernation — only for long-idle large-heap processes.
- Release strategy — immutable releases (rolling deploys) vs hot upgrades (`appup`/`relup`).
- `trap_exit` — set only when cleanup-on-exit is actually needed.

## Policy decisions for individual repos

Each repo should record its decisions on:

- Whether to use `:off_heap` for any high-throughput mailboxes (and the measurement that justified it).
- GC tuning flags (`:fullsweep_after`, `:min_heap_size`, `:max_heap_size`) — default vs tuned.
- Release strategy: immutable releases with rolling deploys, or hot code upgrade via `appup`/`relup`.
- Monitoring approach: `:observer` in dev, telemetry/`:telemetry` + external dashboards in prod.
- Whether `:sys` debug options are permitted in production code at all.
- Whether `trap_exit` is the default for resource-owning GenServers.
- Naming policy for supervised processes (`__MODULE__` vs explicit atoms vs `:via` Registry).
- Restart policy defaults (`:permanent` vs `:transient` vs `:temporary`) per worker class.
- Whether `code_change/3` and `@vsn` are required (only if hot upgrades are in scope).

## Related docs

- For BEAM runtime semantics, see `../beam/index.md` and the per-topic `docs/beam/*.md` docs.
- `docs/elixir/otp-supervision.md` — OTP abstractions in Elixir (GenServer, Supervisor, DynamicSupervisor, Registry, Application, child specs).
- `docs/elixir/error-handling.md` — Elixir exception classes, `try/catch/rescue`, `raise`/`reraise`/`defexception`.
- `docs/elixir/concurrency-processes.md` — processes, message passing, `Task`, `Agent`.
- `docs/elixir/configuration-and-runtime.md` — runtime configuration, `config/runtime.exs`, releases.
- `docs/elixir/core-modules.md` — `Process`, `Enum`, `Map`, `Keyword`, `IO`, etc.
- `docs/elixir/mix-project-structure.md` — `mix.exs`, `def application`, application specs.

## Related skills

- `beam-supervision`, `beam-gen-server`, `beam-errors-failures`, `beam-processes`, `beam-applications-releases`, `beam-observability-debugging`, `beam-gen-statem`, `beam-logger-config` — BEAM/OTP operational skills in `.agents/skills/`.
- `elixir-otp`, `elixir-error-handling`, `elixir-coding`, `elixir-testing`, `elixir-project-setup` — Elixir-specific operational skills in `.agents/skills/`.
- Use these skills for implementation, review, debugging, and validation work rather than duplicating their guidance here.
