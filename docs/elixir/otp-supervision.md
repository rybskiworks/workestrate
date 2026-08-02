# OTP and Supervision

## Purpose

Define guidance for OTP abstractions in Elixir in this repo: GenServer, Supervisor, DynamicSupervisor, Registry, Application, child specs, and supervision trees. Future agents who write, review, refactor, debug, or validate Elixir processes and supervisors should follow these rules so behavior is consistent, predictable, and aligned with the official Elixir and Erlang documentation.

This document fills the GenServer, Supervisor, DynamicSupervisor, Registry, Application, Child Specs, and Supervision Trees sections in depth. Future agents who write, review, refactor, debug, or validate Elixir processes and supervisors should follow these rules so behavior is consistent, predictable, and aligned with the official Elixir and Erlang documentation.

## Sources used

- https://hexdocs.pm/elixir/GenServer.html (PRIMARY)
- https://www.erlang.org/doc/system/gen_server_concepts.html (Erlang OTP theory)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html (Erlang gen_server module, callback typespecs)
- https://www.erlang.org/doc/system/design_principles.html (OTP design principles, generic/specific split)
- https://hexdocs.pm/elixir/Supervisor.html (cross-reference for child specs)
- https://hexdocs.pm/elixir/Registry.html (PRIMARY for Registry section; `:via` registration, `:unique`/`:duplicate` keys, partitioning, `dispatch/4`)
- https://hexdocs.pm/elixir/Process.html (register/send_after/monitors context)
- https://hexdocs.pm/elixir/DynamicSupervisor.html (PRIMARY for DynamicSupervisor section)
- https://hexdocs.pm/elixir/PartitionSupervisor.html (cross-reference for scaling/partitioning)
- https://www.erlang.org/doc/system/sup_princ.html (Erlang supervisor principles, :simple_one_for_one semantics for migration)
- https://www.erlang.org/doc/apps/kernel/global.html (`:global` cluster-wide name registration; Registry vs `:global`)
- https://github.com/elixir-lang/elixir/blob/main/CHANGELOG.md (Registry `:keys` tuple forms, v1.19.0; `{:duplicate, :key}` `ordered_set` layout, v1.20.0)

This page reflects Elixir v1.20.2 docs.

## Related BEAM guidance

The Elixir callback contracts and option mappings below are the value of this doc; the underlying OTP supervision/gen_server runtime semantics live in `docs/beam/`:

- [../beam/supervision.md](../beam/supervision.md) — for supervisor flags, restart strategies, child specs, intensity/period, and automatic shutdown.
- [../beam/gen-server.md](../beam/gen-server.md) — for the gen_server callback contract, call/cast/reply protocol, and `:sys` debugging.
- [../beam/otp-behaviours.md](../beam/otp-behaviours.md) — for the standard behaviours and the generic/specific split.
- [../beam/links-monitors-and-exits.md](../beam/links-monitors-and-exits.md) — for `trap_exit`, exit-reason propagation, and shutdown semantics.

## GenServer

### What GenServer is / OTP client-server theory

GenServer is Elixir's behaviour for client-server processes: a generic process loop (message dispatch, timeouts, debug/`:sys` support, code change, termination hooks) plus developer-supplied callbacks. For the OTP client-server theory and generic/specific split, see [../beam/gen-server.md](../beam/gen-server.md) and [../beam/otp-behaviours.md](../beam/otp-behaviours.md).

Important properties from the Erlang docs ([stdlib gen_server](https://www.erlang.org/doc/apps/stdlib/gen_server.html)):

- A gen_server "does not trap exit signals automatically — this must be explicitly initiated in the callback module."
- "If a callback function fails or returns a bad value, the `gen_server` process terminates. However, an exception of class `throw` is not regarded as an error but as a valid return, from all callback functions."

### When to use / NOT to use GenServer

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "A GenServer, or a process in general, must be used to model runtime characteristics of your system. A GenServer must never be used for code organization purposes."

> "Use processes only to model runtime properties, such as mutable state, concurrency and failures, never for code organization."

The canonical anti-pattern is the "calculator GenServer":

```elixir
# AVOID
GenServer.start_link(Calculator, [], name: :calculator)
GenServer.call(:calculator, {:add, 1, 2})
GenServer.call(:calculator, {:subtract, 5, 3})
```

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "If you have read somewhere that you must use processes to build APIs, this is likely an anti-pattern. For example, putting `add/2` and `subtract/2` operations behind a single `GenServer` is an anti-pattern not only because it convolutes the calculator logic but also because you put the calculator logic behind a single process that will potentially become a bottleneck."

Rule of thumb:

| Use a GenServer for | Do NOT use a GenServer for |
|---|---|
| Mutable process state | Pure functions / deterministic calculations |
| Serialized access to a shared resource | Code organization or namespacing |
| Concurrency isolation | Trivial wrappers around libraries |
| Failure containment and restarts | Anti-pattern "API servers" over pure logic |

If the operation is pure, just write functions.

### Callback contract

Use `@impl true` for every callback. `use GenServer` provides the `@behaviour` and utility functions. Return tuples MUST match the exact shapes below; bad returns terminate the process.

#### `init/1`

```elixir
@impl true
@spec init(init_arg :: term()) ::
        {:ok, state}
        | {:ok, state, timeout | :hibernate | {:continue, term}}
        | :ignore
        | {:stop, reason}
        | {:error, reason}
def init(init_arg) do
  # ...
end
```

| Return | Meaning |
|---|---|
| `{:ok, state}` | Start successfully with initial state. |
| `{:ok, state, timeout}` | Start and schedule a timeout. |
| `{:ok, state, :hibernate}` | Start and hibernate immediately. |
| `{:ok, state, {:continue, term}}` | Start and immediately run `handle_continue/2`. |
| `:ignore` | Stop silently; `start_link/3` returns `:ignore`. |
| `{:stop, reason}` | Stop with reason; `start_link/3` returns `{:error, reason}`. |
| `{:error, reason}` | `start_link/3` returns `{:error, reason}` (process not started). |

`init/1` is synchronous and blocks `start_link/3`. Do heavy work here only if the supervisor should not finish its own start until the work completes.

#### `handle_call/3`

```elixir
@impl true
@spec handle_call(request :: term(), from :: GenServer.from(), state :: term()) ::
        {:reply, reply, new_state}
        | {:reply, reply, new_state, timeout | :hibernate | {:continue, term}}
        | {:noreply, new_state}
        | {:noreply, new_state, timeout | :hibernate | {:continue, term}}
        | {:stop, reason, reply, new_state}
        | {:stop, reason, new_state}
def handle_call(request, from, state) do
  # ...
end
```

| Return | Meaning |
|---|---|
| `{:reply, reply, new_state}` | Reply to caller and continue. |
| `{:reply, reply, new_state, timeout}` | Reply, continue, schedule timeout. |
| `{:reply, reply, new_state, {:continue, term}}` | Reply then run `handle_continue/2`. |
| `{:noreply, new_state}` | Do not reply now; caller blocks until `GenServer.reply/2`. |
| `{:stop, reason, reply, new_state}` | Reply, then stop. |
| `{:stop, reason, new_state}` | Stop without replying. |

#### `handle_cast/2`

```elixir
@impl true
@spec handle_cast(request :: term(), state :: term()) ::
        {:noreply, new_state}
        | {:noreply, new_state, timeout | :hibernate | {:continue, term}}
        | {:stop, reason, new_state}
def handle_cast(request, state) do
  # ...
end
```

| Return | Meaning |
|---|---|
| `{:noreply, new_state}` | Continue without replying. |
| `{:noreply, new_state, timeout}` | Continue and schedule timeout. |
| `{:noreply, new_state, {:continue, term}}` | Continue then run `handle_continue/2`. |
| `{:stop, reason, new_state}` | Stop the server. |

#### `handle_info/2`

```elixir
@impl true
@spec handle_info(msg :: term(), state :: term()) ::
        {:noreply, new_state}
        | {:noreply, new_state, timeout | :hibernate | {:continue, term}}
        | {:stop, reason, new_state}
def handle_info(msg, state) do
  # ...
end
```

Handles non-system, non-`$gen_call`, non-`$gen_cast` messages. This includes `:timeout` from a callback return, monitor `:DOWN` messages, `Process.send_after/4` deliveries, and `:EXIT` messages when the process is trapping exits. You MUST implement `handle_info/2` explicitly if the process receives any raw messages; the default implementation logs and drops unexpected messages.

#### `handle_continue/2`

```elixir
@impl true
@spec handle_continue(continue :: term(), state :: term()) ::
        {:noreply, new_state}
        | {:noreply, new_state, timeout | :hibernate | {:continue, term}}
        | {:stop, reason, new_state}
def handle_continue(continue, state) do
  # ...
end
```

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "Useful for performing work after initialization or for splitting the work in a callback in multiple steps, updating the process state along the way."

A `{:continue, term}` returned from `init/1`, `handle_call/3`, `handle_cast/2`, or `handle_info/2` causes `handle_continue/2` to run before any external message is processed. If you return `{:continue, _}` but do not implement `handle_continue/2`, the process exits with `undef`.

#### `terminate/2`

```elixir
@impl true
@spec terminate(reason :: :normal | :shutdown | {:shutdown, term} | term, state :: term()) ::
        term()
def terminate(reason, state) do
  # ...
end
```

Return value is ignored. `reason` is one of `:normal`, `:shutdown`, `{:shutdown, term}`, or any other term for an unexpected exit.

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "it is not guaranteed that `terminate/2` is called when a GenServer exits. Therefore, important cleanup should be done using process links and/or monitors."

`terminate/2` is NOT called on `:brutal_kill`, on `:kill` exit, or when a linked process exits and the GenServer does not trap exits. Ports, sockets, and file descriptors auto-close on exit.

#### `code_change/3`

```elixir
@impl true
@spec code_change(old_vsn :: term(), state :: term(), extra :: term()) ::
        {:ok, new_state} | {:error, reason}
def code_change(old_vsn, state, extra) do
  # ...
end
```

`old_vsn` is the previous `@vsn` value during a hot-code upgrade, or `{:down, vsn}` during a downgrade.

#### `format_status/1`

```elixir
@impl true
@spec format_status(status :: :gen_server.format_status()) :: :gen_server.format_status()
def format_status(status) do
  # ...
end
```

`format_status/1` was introduced in Elixir 1.17.0. The status map contains keys such as `:state`, `:message`, `:reason`, and `:log`. It is called by `:sys.get_status/1` and on abnormal termination. Use it to redact secrets from `:sys.get_status/1` output and crash logs. `format_status/2` is deprecated.

### call vs cast vs info

| Mechanism | Function | Synchronicity | Callback | Guarantees |
|---|---|---|---|---|
| `call` | `GenServer.call(server, request, timeout \\ 5000)` | Synchronous | `handle_call/3` | Blocks caller; caller exits on timeout, no process, etc. |
| `cast` | `GenServer.cast(server, request)` | Asynchronous | `handle_cast/2` | Always returns `:ok` "regardless of whether the destination server (or node) exists." |
| `info` | `send/2`, `Process.send_after/4`, monitor `:DOWN`, `:EXIT` | Out-of-band | `handle_info/2` | Fire-and-forget raw messages. |

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "`cast/2` returns `:ok` immediately, regardless of whether the destination server (or node) exists."

A `call/3` can exit the caller for reasons including `:timeout`, `:noproc`, `:nodedown`, `:calling_self`, `:shutdown`, and `:normal`.

### Reply patterns

| Pattern | Use when | Risk |
|---|---|---|
| `{:reply, reply, new_state}` | The server can reply synchronously in the callback. | None typical. |
| `{:noreply, new_state}` + `GenServer.reply(from, reply)` | Reply must be deferred or produced by another process. | Caller deadlocks if `GenServer.reply/2` is never called. |
| `{:stop, reason, reply, new_state}` | Reply then terminate. | Stops the server. |
| `{:stop, reason, new_state}` | Terminate without replying. | Caller may receive `{:DOWN, ...}` instead. |

The `:noreply` pattern is powerful but dangerous. If `handle_call/3` returns `{:noreply, state}` and the code path that should call `GenServer.reply(from, reply)` is skipped, the caller blocks forever. Always pair `:noreply` with an explicit reply path.

### handle_continue / why not init

Do not perform slow or blocking work in `init/1` unless the supervisor must wait for it. Returning `{:ok, state, {:continue, term}}` lets `start_link/3` return immediately while still performing setup before processing external messages.

Before (bad — blocks supervisor start):

```elixir
@impl true
def init(arg) do
  # Blocks until connection succeeds or fails
  {:ok, conn} = SomeDatabase.connect(arg)
  {:ok, %{conn: conn}}
end
```

After (good — fast start, then connect):

```elixir
@impl true
def init(arg) do
  {:ok, %{arg: arg, conn: nil}, {:continue, :connect}}
end

@impl true
def handle_continue(:connect, %{arg: arg} = state) do
  case SomeDatabase.connect(arg) do
    {:ok, conn} ->
      {:noreply, %{state | conn: conn}}

    {:error, reason} ->
      {:stop, reason, state}
  end
end
```

### Timeouts

#### Callback-returned timeouts

Every callback that accepts a continuation can return `timeout | :hibernate | {:continue, term}` as a third or fourth element. The default timeout is `:infinity`.

When a callback returns a timeout value, the server schedules `:timeout` to be delivered to `handle_info/2` if no other message arrives first.

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "If the process has any message already waiting when the `timeout` value is returned, the timeout is ignored and the waiting message is handled as usual. This means that even a timeout of `0` milliseconds is not guaranteed to execute."

Therefore, if you need an immediate, unconditional follow-up action, use `{:continue, term}` and `handle_continue/2`, not `timeout: 0`.

#### `call/3` timeout

`GenServer.call/3` takes a `timeout` argument defaulting to `5000` milliseconds. If the server does not reply in time, the caller exits with reason `:timeout`.

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "If the caller catches the failure and continues running, and the server is just late with the reply, it may arrive at any time later into the caller's message queue."

OTP 24+ mitigates this with process aliases, but legacy code or pre-OTP-24 targets must still be careful: catching the `:timeout` exit and continuing leaves a stale `{ref, reply}` message that should be discarded.

### Process naming and registration

The `:name` option accepts:

| Form | Scope | Use case |
|---|---|---|
| `nil` | Anonymous | Default; no registration. |
| atom | Local node | Fixed, well-known name like `MyApp.Queue`. |
| `{:global, term}` | Global (cluster-wide) | Distributed Erlang global registration. |
| `{:via, module, term}` | Custom registry | `Registry`, `Horde`, or custom `:via` module. |

`server/0` (the first argument to `call/2`, `cast/2`, etc.) accepts a PID, a local atom, `{atom, node}`, `{:global, term}`, or `{:via, module, term}`.

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "If there is an interest to register dynamic names locally, do not use atoms, as atoms are never garbage-collected and therefore dynamically generated atoms won't be garbage-collected. For such cases, you can set up your own local registry by using the Registry module."

Use `Registry` for dynamic names:

```elixir
{:ok, _} = Registry.start_link(keys: :unique, name: MyApp.Registry)
name = {:via, Registry, {MyApp.Registry, "some_key"}}
{:ok, pid} = GenServer.start_link(MyServer, arg, name: name)
```

### start_link vs start

| Function | Links to caller? | Use under supervision? | Typical use |
|---|---|---|---|
| `GenServer.start_link/3` | Yes | Yes | Supervised workers. |
| `GenServer.start/3` | No | No | Ad-hoc processes or testing. |

Both are synchronous: they block until `init/1` returns. Options include `:name`, `:timeout` (for `init/1`), `:debug`, `:spawn_opt`, and `:hibernate_after`.

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "`start_link/3` starts a `GenServer` process linked to the current process."

The `on_start/0` type is:

```elixir
@type on_start ::
        {:ok, pid}
        | :ignore
        | {:error, {:already_started, pid} | term}
```

`use GenServer` auto-defines a `child_spec/1` function that accepts `:id`, `:restart`, and `:shutdown`. The defaults are `id: __MODULE__`, `restart: :permanent`, and `shutdown: 5000`.

### Client API functions

| Function | Purpose |
|---|---|
| `GenServer.call/3` | Synchronous request; returns reply or exits. |
| `GenServer.cast/2` | Asynchronous request; always returns `:ok`. |
| `GenServer.reply/2` | Reply to a call that returned `{:noreply, state}`. |
| `GenServer.multi_call/4` | Synchronous call to servers on multiple nodes. |
| `GenServer.abcast/3` | Asynchronous broadcast to named servers on multiple nodes. |
| `GenServer.stop/3` | Stop a server with a reason and timeout. |
| `GenServer.whereis/1` | Resolve a server reference to a PID or `nil`. |

### The canonical client/server module layout

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html), the Stack example:

```elixir
defmodule Stack do
  use GenServer

  # Client API
  def start_link(default) when is_list(default) do
    GenServer.start_link(__MODULE__, default, name: __MODULE__)
  end

  def push(element) do
    GenServer.cast(__MODULE__, {:push, element})
  end

  def pop do
    GenServer.call(__MODULE__, :pop)
  end

  # Server callbacks
  @impl true
  def init(stack) do
    {:ok, stack}
  end

  @impl true
  def handle_call(:pop, _from, [head | tail]) do
    {:reply, head, tail}
  end

  @impl true
  def handle_cast({:push, element}, state) do
    {:noreply, [element | state]}
  end
end
```

Client API functions run in the caller's process; callbacks run in the GenServer process. Keep both in one module by convention.

### Debugging

The `:sys` module provides inspection and tracing for GenServer processes:

| Function | Purpose |
|---|---|
| `:sys.get_state(pid)` | Read the current process state. |
| `:sys.get_status(pid)` | Read formatted status (calls `format_status/1`). |
| `:sys.trace(pid, true)` | Print every message and state change. |
| `:sys.statistics(pid, true)` | Collect message counts. |
| `:sys.no_debug(pid)` | Disable all debug handlers. |
| `:sys.suspend(pid)` | Suspend the process loop. |
| `:sys.resume(pid)` | Resume the process loop. |

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> "It is very important to switch off debugging once we're done. Excessive debug handlers or those that should be turned off, but weren't, can seriously damage the performance of the system."

Use `format_status/1` to redact sensitive fields from `:sys.get_status/1` output and from crash logs.

## Supervisor

### What a Supervisor is / OTP supervision theory

Elixir `Supervisor` is a thin wrapper over the OTP `:supervisor` behaviour. Children start left-to-right; under `:one_for_all`/`:rest_for_one` they terminate/restart right-to-left. For supervision theory, supervisor flags, and restart semantics, see [../beam/supervision.md](../beam/supervision.md).

### Starting a supervisor: start_link/2 and the two forms

`Supervisor.start_link/2`:

```elixir
@spec start_link([child_spec() | module_spec() | :supervisor.child_spec()], [option() | init_option()]) ::
        {:ok, pid()} | {:error, {:already_started, pid()} | {:shutdown, term()} | term()}
```

The `on_start/0` type is:

```elixir
@type on_start ::
        {:ok, pid()}
        | :ignore
        | {:error, {:already_started, pid()} | {:shutdown, term()} | term()}
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> "a supervisor started with this function is linked to the parent process and exits not only on crashes but also if the parent process exits with `:normal` reason."

There are two idiomatic forms:

1. Direct: pass the children list and options directly to `Supervisor.start_link/2`. This is recommended only at the top of the supervision tree, usually inside `Application.start/2`.
2. Module-based: define a module with `use Supervisor` and implement `init/1`, which must return `Supervisor.init(children, strategy: ...)` (or a tuple equivalent).

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> "A general guideline is to use the supervisor without a callback module only at the top of your supervision tree, generally in the `Application.start/2` callback. We recommend using module-based supervisors for any other supervisor in your application, so they can run as a child of another supervisor in the tree."

Module-based supervisor example:

```elixir
defmodule MyApp.WorkerSupervisor do
  use Supervisor

  def start_link(init_arg) do
    Supervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    children = [
      MyApp.DatabaseWorker,
      MyApp.CacheWorker
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end
```

### Supervision strategies

The strategy type is:

```elixir
@type strategy :: :one_for_one | :one_for_all | :rest_for_one
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> `:one_for_one` - if a child process terminates, only that process is restarted.
>
> `:one_for_all` - if a child process terminates, all other child processes are terminated, and then all child processes (including the terminated one) are restarted.
>
> `:rest_for_one` - if a child process terminates, the terminated child process and the rest of the children (those started after it) are terminated and restarted.

For strategy semantics and restart-cascade behavior, see [../beam/supervision.md](../beam/supervision.md).

| Strategy | Use when | Risk |
|---|---|---|
| `:one_for_one` | Children are independent; a failure in one does not corrupt others. | Cascading failures are not isolated if children are actually coupled. |
| `:one_for_all` | Children are tightly coupled and must restart as a unit. | Higher blast radius; all children stop on any single failure. |
| `:rest_for_one` | Children depend on the ones started before them, but earlier children are independent of later ones. | Later children restart more often than earlier ones. |

Historical note: `:simple_one_for_one` was Elixir's old dynamic-child strategy. It was hard-deprecated in Elixir v1.10 in favor of `DynamicSupervisor` (available since Elixir v1.6). Passing `strategy: :simple_one_for_one` still works but emits a deprecation warning; OTP's `:supervisor` still supports the strategy directly. New Elixir code must use `DynamicSupervisor`. See Elixir's [compatibility-and-deprecations.md](https://github.com/elixir-lang/elixir/blob/main/lib/elixir/pages/references/compatibility-and-deprecations.md).

### Restart values (:restart)

The restart type is:

```elixir
@type restart :: :permanent | :transient | :temporary
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> `:permanent` - the child process is always restarted.
>
> `:transient` - the child process is restarted if it exits abnormally, i.e., with an exit reason other than `:normal`, `:shutdown`, or `{:shutdown, term}`.
>
> `:temporary` - the child process is never restarted.

| Value | Restarted on... | Default? |
|---|---|---|
| `:permanent` | Any exit reason (including `:normal` and `:shutdown`). | Yes |
| `:transient` | Only abnormal exit reasons (not `:normal`, `:shutdown`, or `{:shutdown, term}`). | No |
| `:temporary` | Never. Even under `:one_for_all` or `:rest_for_one`, a temporary child is not restarted when a sibling dies. | No |

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> "Notice that when a process exits with reason `:normal`, `:shutdown`, or `{:shutdown, term}`, the supervisor does not log the error. However, if the process exits with any other exit reason, the supervisor will log the error and the state of the process at termination."

When a supervisor reaches its maximum restart intensity, it exits with reason `:shutdown` and is only restarted if its own child spec is `:permanent` (the default). For the full restart-intensity model, see [../beam/supervision.md](../beam/supervision.md).

### Shutdown values (:shutdown)

The shutdown type is:

```elixir
@type shutdown :: timeout() | :brutal_kill
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> `:brutal_kill` - the child process is unconditionally and immediately terminated using `Process.exit(child, :kill)`.
>
> any integer >= 0 - the supervisor tells the child process to terminate by calling `Process.exit(child, :shutdown)` and then waits for an exit signal back. If no exit signal is received within the specified number of milliseconds, the child process is unconditionally terminated using `Process.exit(child, :kill)`.
>
> `:infinity` - the supervisor waits indefinitely for the child process to terminate. This is the default for supervisor children.

If the child process does not trap exits, the `:shutdown` signal terminates it immediately. If it traps exits, `terminate/2` is invoked and the child must finish within the timeout. Defaults differ by `:type`: a `:worker` defaults to `5000` milliseconds; a `:supervisor` defaults to `:infinity`.

Set child supervisors to `:infinity` so their subtrees shut down cleanly; finite timeouts are fine for workers unless cleanup can hang. For shutdown signal semantics, see [../beam/supervision.md](../beam/supervision.md).

### Restart intensity: max_restarts and max_seconds

The `init_option()` type is:

```elixir
@type init_option ::
        {:strategy, strategy()}
        | {:max_restarts, non_neg_integer()}
        | {:max_seconds, pos_integer()}
        | {:auto_shutdown, auto_shutdown()}
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> `:max_restarts` - the maximum number of restarts allowed in a time frame. Defaults to `3`.
>
> `:max_seconds` - the time frame in which `:max_restarts` applies. Defaults to `5`.

Elixir defaults (`3` restarts in `5` seconds) are more tolerant than OTP's own (`1` in `5`). Keep `:max_restarts` conservative; high intensity at every level prevents isolation of cascading failures. For tuning guidance and how intensity compounds across the tree, see [../beam/supervision.md](../beam/supervision.md).

### Child specifications and child_spec/1

A child specification map has the following shape:

```elixir
@type child_spec :: %{
        :id => atom() | term(),
        :start => {module(), function_name :: atom(), args :: [term()]},
        optional(:restart) => restart(),
        optional(:shutdown) => shutdown(),
        optional(:type) => type(),
        optional(:modules) => [module()] | :dynamic,
        optional(:significant) => boolean()
      }
```

The `children` list passed to `Supervisor.start_link/2` or `Supervisor.init/2` accepts four forms. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

> "a child specification map; a tuple with a module as first element and the start argument as second; or an atom representing a module that implements the `child_spec/1` function"

More precisely, the accepted forms are:

| Form | Resolved to |
|---|---|
| `%{id: ..., start: ...}` | Used as-is (map may include other child spec keys). |
| `Module` | Calls `Module.child_spec([])`. |
| `{Module, arg}` | Calls `Module.child_spec(arg)`. |
| `{module, function, args}` legacy tuple | Treated as an Erlang-style supervisor child. |

The `module_spec/0` type is:

```elixir
@type module_spec :: {module(), term()} | module()
```

`use GenServer` auto-defines `child_spec/1` with defaults `id: __MODULE__`, `restart: :permanent`, and `shutdown: 5000`. Cross-reference the GenServer section's `start_link vs start` subsection above. `use Supervisor` likewise auto-defines `child_spec/1`, customizable via the `:id` and `:restart` options passed to `use Supervisor`. Customization at `use` time looks like:

```elixir
use GenServer, restart: :transient, shutdown: 10_000
```

The `:id` field must be unique among a static supervisor's children. Conflicting `:id` values cause `Supervisor.init/2` (or `start_link/2`) to refuse initialization. `DynamicSupervisor` ignores `:id` because it assigns identifiers dynamically.

### Inline vs module-based child specs

Inline map form is explicit but leaks start details into the parent:

```elixir
%{id: Counter, start: {Counter, :start_link, [0]}}
```

The `{Module, arg}` tuple is the most idiomatic form because the module encapsulates its own spec:

```elixir
{Counter, 0}
```

A bare module is concise when the default argument is `[]`:

```elixir
Counter
```

Use the inline map when you need local overrides (a different `:id`, `:restart`, or `:shutdown`) without changing the module. Use `{Module, arg}` for ordinary supervised workers. Use a bare module when the worker needs no init argument.

### Supervisor.child_spec/2 — overriding defaults

```elixir
@spec child_spec(child_spec() | module_spec(), child_spec_overrides()) :: child_spec()
```

`Supervisor.child_spec/2` retrieves the spec (by calling `child_spec/1` on a module or tuple, or using the map directly) and then shallow-merges the given overrides. Unknown override keys raise.

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html):

```elixir
Supervisor.child_spec({Agent, fn -> :ok end}, id: {Agent, 1})
#=> %{id: {Agent, 1}, start: {Agent, :start_link, [fn -> :ok end]}}
```

This is the canonical way to start multiple children from the same module under one supervisor (give each a distinct `:id`) or to override `:shutdown`/`:restart` without editing the module.

### start_link / registration options (:name)

The `option()` type is:

```elixir
@type option :: {:name, name()}
```

where `name()` follows the same rules as GenServer:

```elixir
@type name :: atom() | {:global, term()} | {:via, module(), term()}
```

Cross-reference the GenServer section's "Process naming and registration" subsection above. Local atoms, `{:global, term}`, and `{:via, module, term}` are all valid. For dynamic names, prefer `Registry` or another `:via` module rather than atoms.

### Automatic shutdown (:auto_shutdown, since OTP 24 / Elixir 1.12)

```elixir
@type auto_shutdown :: :never | :any_significant | :all_significant
```

The default is `:never`. When set to `:any_significant` or `:all_significant`, the supervisor shuts down automatically when significant children terminate. Only children whose `:restart` is `:transient` or `:temporary` may be marked `significant: true`; `:permanent` children cannot be significant.

Do not make an auto_shutdown supervisor a permanent child of another supervisor, or its shutdown will be undone by the parent restart. For the full auto-shutdown semantics, see [../beam/supervision.md](../beam/supervision.md).

### Building supervision trees

Supervision trees are built by nesting module-based supervisors in a parent's children list:

```elixir
defmodule MyApp.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      MyApp.WorkerSupervisor,
      MyApp.Registry,
      {MyApp.Endpoint, []}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: MyApp.Supervisor)
  end
end

defmodule MyApp.WorkerSupervisor do
  use Supervisor

  def start_link(init_arg) do
    Supervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    children = [
      MyApp.DatabaseWorker,
      MyApp.CacheWorker
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end
```

`Supervisor.start_child/2` appends a child to the end of a running supervisor's children list, preserving the right-to-left termination semantics required by `:rest_for_one`. `Supervisor.which_children/1` returns a list of tuples:

```elixir
[{id :: term(), child :: pid() | :restarting | :undefined, type :: :worker | :supervisor, modules :: [module()] | :dynamic}]
```

where `child` is the PID, `:restarting` if the child is currently restarting, or `:undefined` if it has not started or has been terminated.

Patterns and best practices:

| Situation | Recommendation |
|---|---|
| Independent workers | Use `:one_for_one`. |
| Tightly coupled workers that must restart together | Use `:one_for_all`. |
| Pipeline / dependency chain | Use `:rest_for_one`; order children from least-dependent to most-dependent. |
| Unknown or highly variable number of children | Use `DynamicSupervisor`; see the `## DynamicSupervisor` section below. |
| Need horizontal scaling across schedulers | Use `PartitionSupervisor` (Elixir 1.14+). |
| High restart intensity everywhere | Keep values conservative; intensity compounds across levels. |

### DynamicSupervisor

`DynamicSupervisor` is the modern replacement for the deprecated `:simple_one_for_one` strategy. It starts with no children; children are added at runtime via `DynamicSupervisor.start_child/2`. It supports only the `:one_for_one` strategy, ignores the `:id` field, and adds `:max_children` and `:extra_arguments` options. To scale horizontally, use `PartitionSupervisor` (Elixir 1.14+). Full coverage is in the `## DynamicSupervisor` section below.

### Common mistakes

1. **Using a non-module supervisor everywhere** — fix: use module-based supervisors for any supervisor that is itself a child of another supervisor; reserve the direct `Supervisor.start_link/2` form for the top of the tree.
2. **Conflicting `:id` values** — fix: ensure each child `:id` is unique under a static supervisor; use `Supervisor.child_spec/2` to disambiguate.
3. **Finite `:shutdown` for a child supervisor** — fix: set child supervisors to `:infinity` shutdown so their own children terminate cleanly.
4. **Using `:simple_one_for_one`** — fix: use `DynamicSupervisor`.
5. **Tuning `:max_restarts` too high** — fix: keep intensity conservative; cascading failures should be isolated, not absorbed.
6. **Forgetting `:transient`/`:temporary` change restart behavior** — fix: a `:temporary` child never restarts, even when siblings die under `:one_for_all` or `:rest_for_one`; a `:transient` child does not restart on `:normal`/`:shutdown`.
7. **Forgetting the supervisor itself needs `:permanent` to be restarted** — fix: default child specs are `:permanent`, so a supervisor that hits max intensity is restarted by its parent. If you override its spec to `:temporary` or `:transient`, it will not be restarted after hitting max intensity.
8. **Expecting `start_child/2` to preserve `:id` semantics for DynamicSupervisor** — fix: `DynamicSupervisor` ignores `:id`; identify children by PID.

### Cross-references

- GenServer section's `start_link vs start` subsection above: `use GenServer` auto-defines `child_spec/1` with `id: __MODULE__`, `restart: :permanent`, `shutdown: 5000`.
- GenServer section's "Process naming and registration" subsection above: same `:name` rules apply to supervisors.
- `## Child Specs` below: covers the child specification map and custom `child_spec/1` implementations.
- `## Supervision Trees` below: covers tree design, restart intensity, and fault-isolation boundaries.
- `## DynamicSupervisor` below: covers `DynamicSupervisor.start_child/2`, `terminate_child/2`, `which_children/1`, and scaling patterns.

## DynamicSupervisor

### What DynamicSupervisor is / when to use it

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> A supervisor optimized to only start children dynamically.
>
> The `Supervisor` module was designed to handle mostly static children that are started in the given order when the supervisor starts. A `DynamicSupervisor` starts with no children. Instead, children are started on demand via `start_child/2` and there is no ordering between children. This allows the `DynamicSupervisor` to hold millions of children by using efficient data structures and to execute certain operations, such as shutting down, concurrently.

When to choose each supervisor:

| Situation | Use |
|---|---|
| Children are known at boot, started in a fixed order, and need `:one_for_all`/`:rest_for_one` restart coupling | Static `Supervisor` |
| Children must start left-to-right and terminate right-to-left for dependency ordering | Static `Supervisor` |
| Children are created on demand at runtime; number and identity are not known at boot | `DynamicSupervisor` |
| Same child template spawned many times (e.g. per-connection, per-request, per-session workers) | `DynamicSupervisor` |
| Need to cap concurrency with `:max_children` or inject shared `:extra_arguments` | `DynamicSupervisor` |

### Supported strategy and ignored :id

`DynamicSupervisor` supports ONLY `:one_for_one`. From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> `:strategy` - the restart strategy option. The only supported value is `:one_for_one` which means that no other child is terminated if a child process terminates. You can learn more about strategies in the `Supervisor` module docs.

```elixir
@type strategy() :: :one_for_one
```

The child spec `:id` is ignored. From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> Note that while the `:id` field is still required in the spec, the value is ignored and therefore does not need to be unique. Unlike `Supervisor`, this module does not return `{:error, {:already_started, pid}}` for child specs given with the same id. `{:error, {:already_started, pid}}` is returned however if a duplicate name is used when using name registration.

Cross-reference the Supervisor section's "Child specifications and child_spec/1" subsection above, where `:id` uniqueness is enforced for static supervisors.

### Starting a DynamicSupervisor: the two start_link forms

There are two idiomatic forms.

Form 1 — direct, as a child of another supervisor:

```elixir
@spec start_link([init_option() | GenServer.option()]) :: Supervisor.on_start()
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> This function is typically not invoked directly, instead it is invoked when using a `DynamicSupervisor` as a child of another supervisor.

Canonical example:

```elixir
children = [
  {DynamicSupervisor, name: MyApp.DynamicSupervisor, strategy: :one_for_one}
]

Supervisor.start_link(children, strategy: :one_for_one)
```

Form 2 — module-based, with `use DynamicSupervisor`:

```elixir
@spec start_link(module(), term(), [GenServer.option()]) :: Supervisor.on_start()
```

Module-based example:

```elixir
defmodule MyApp.DynamicSupervisor do
  # Automatically defines child_spec/1
  use DynamicSupervisor

  def start_link(init_arg) do
    DynamicSupervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end
end
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> When you `use DynamicSupervisor`, the `DynamicSupervisor` module will set `@behaviour DynamicSupervisor` and define a `child_spec/1` function, so your module can be used as a child in a supervision tree.

Note the key difference between the two forms:

> Options specific to `DynamicSupervisor` must be returned from the `init/1` callback.

i.e. for `start_link/3` the DynamicSupervisor-specific options (`:strategy`, `:max_children`, `:extra_arguments`, etc.) go in the `init/1` callback's `DynamicSupervisor.init/1` call, NOT in `start_link/3`'s opts. For the direct `start_link/1` form they go inline.

Cross-reference the Supervisor section's "Starting a supervisor" guidance above: module-based supervisors are recommended when the supervisor is itself a child of another supervisor.

### init options

```elixir
@type init_option() ::
        {:strategy, strategy()}
        | {:max_restarts, non_neg_integer()}
        | {:max_seconds, pos_integer()}
        | {:max_children, non_neg_integer() | :infinity}
        | {:extra_arguments, [term()]}
```

| Option | Default | Description (verbatim) |
|---|---|---|
| `:strategy` | — | "the restart strategy option. The only supported value is `:one_for_one` which means that no other child is terminated if a child process terminates." |
| `:max_restarts` | `3` | "the maximum number of restarts allowed in a time frame." |
| `:max_seconds` | `5` | "the time frame in which `:max_restarts` applies." |
| `:max_children` | `:infinity` | "the maximum amount of children to be running under this supervisor at the same time. When `:max_children` is exceeded, `start_child/2` returns `{:error, :max_children}`." |
| `:extra_arguments` | `[]` | "arguments that are prepended to the arguments specified in the child spec given to `start_child/2`." |

`:name` follows the same GenServer registration rules — cross-reference the GenServer section's "Process naming and registration" subsection above. `init/1` "accepts the same `options` as `start_link/1` (except for `:name`)".

`:max_restarts`/`:max_seconds` behave exactly as in the static Supervisor — cross-reference the Supervisor section's "Restart intensity" subsection above. `:auto_shutdown` and `:significant` children (covered in the Supervisor section) are NOT supported by DynamicSupervisor.

### start_child/2

```elixir
@spec start_child(
        Supervisor.supervisor(),
        Supervisor.child_spec()
        | {module(), term()}
        | module()
        | (old_erlang_child_spec :: :supervisor.child_spec())
      ) :: on_start_child()
```

The accepted forms are identical to how the static Supervisor resolves children — a child spec map, `{Module, arg}`, bare module, or legacy Erlang tuple. Cross-reference the Supervisor section's "Inline vs module-based child specs" subsection above.

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> This function will block the `DynamicSupervisor` until the child initializes. When starting too many processes dynamically, you may want to use a `PartitionSupervisor` to split the work across multiple processes.

```elixir
@type on_start_child() ::
        {:ok, pid()}
        | {:ok, pid(), info :: term()}
        | :ignore
        | {:error, {:already_started, pid()} | :max_children | term()}
```

| Return | Meaning |
|---|---|
| `{:ok, pid}` | Child started; spec and PID added to the supervisor. |
| `{:ok, pid, info}` | Child started with extra info (e.g. from `GenServer` `{:ok, state}` start). |
| `:ignore` | Child's start function returned `:ignore`; nothing added to the tree. |
| `{:error, {:already_started, pid}}` | Returned ONLY when a duplicate registered NAME is used — NOT for duplicate child spec `:id`. |
| `{:error, :max_children}` | Supervisor already has `:max_children` running children. |
| `{:error, term}` | Child's start function returned an error, failed, or returned an erroneous value. |

### terminate_child/2 — keyed by pid, not by id

```elixir
@spec terminate_child(Supervisor.supervisor(), pid()) :: :ok | {:error, :not_found}
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> Terminates the given child identified by `pid`.
>
> This function will block the `DynamicSupervisor` until the child terminates, which may take an arbitrary amount of time if the child is trapping exits and implements its own terminate callback. For this reason, it is often better to ask the child process itself to terminate, often by declaring in its child spec it has a restart strategy of `:transient` (or `:temporary`) and then sending it a message to stop with reason `:shutdown`.

This differs from the static `Supervisor.terminate_child/2`, which takes a `child_id` (term). Because DynamicSupervisor ignores `:id`, you MUST identify children by PID. Cross-reference the Supervisor section above.

### which_children/1 and count_children/1

`which_children/1`:

```elixir
@spec which_children(Supervisor.supervisor()) :: [
        {:undefined, pid() | :restarting, :worker | :supervisor,
         [module()] | :dynamic}
      ]
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> - `id` - it is always `:undefined` for dynamic supervisors
> - `child` - the PID of the corresponding child process or the atom `:restarting` if the process is about to be restarted
> - `type` - `:worker` or `:supervisor` as defined in the child specification
> - `modules` - as defined in the child specification

> Note that calling this function when supervising a large number of children under low memory conditions can bring the system down due to an out of memory error.

`count_children/1`:

```elixir
@spec count_children(Supervisor.supervisor()) :: %{
        specs: non_neg_integer(),
        active: non_neg_integer(),
        supervisors: non_neg_integer(),
        workers: non_neg_integer()
      }
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> - `:specs` - the number of children processes
> - `:active` - the count of all actively running child processes managed by this supervisor
> - `:supervisors` - the count of all supervisors whether or not the child process is still alive
> - `:workers` - the count of all workers, whether or not the child process is still alive

Canonical example:

```elixir
DynamicSupervisor.count_children(MyApp.DynamicSupervisor)
#=> %{active: 2, specs: 2, supervisors: 0, workers: 2}
```

### stop/3

```elixir
@spec stop(Supervisor.supervisor(), reason :: term(), timeout()) :: :ok
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> Synchronously stops the given supervisor with the given `reason`.
>
> It returns `:ok` if the supervisor terminates with the given reason. If it terminates with another reason, the call exits.
>
> This function keeps OTP semantics regarding error reporting. If the reason is any other than `:normal`, `:shutdown` or `{:shutdown, _}`, an error report is logged.

### :extra_arguments — prepended, not appended

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> `:extra_arguments` - arguments that are prepended to the arguments specified in the child spec given to `start_child/2`. Defaults to an empty list.

Example: if the child spec is `{Counter, 0}` (so the start args are `[0]`) and `:extra_arguments` is set to `[:shared]`, the actual start call becomes `Counter.start_link(:shared, 0)` — i.e. `[:shared] ++ [0]`.

This differs from Erlang's deprecated `:simple_one_for_one`, which APPENDS via `apply(M, F, A ++ List)`. This is a migration hazard; see the next subsection.

### Migrating from :simple_one_for_one

`:simple_one_for_one` was Elixir's old dynamic-child strategy, hard-deprecated in Elixir v1.10 in favor of `DynamicSupervisor` (cross-reference the Supervisor section's historical note about `:simple_one_for_one`). The Elixir `Supervisor` typespec never included `:simple_one_for_one` (it only lists `:one_for_one | :one_for_all | :rest_for_one`), and the DynamicSupervisor page itself does NOT contain an explicit migration note — the mapping below is derived by comparing the two APIs.

| `:simple_one_for_one` mechanism | DynamicSupervisor equivalent |
|---|---|
| `supervisor:start_child(Sup, List)` — `List` appended to MFA args via `apply(M, F, A ++ List)` | `DynamicSupervisor.start_child(sup, {Mod, per_child_arg})`; for shared args use `:extra_arguments` (which PREPEND, not append) |
| Single child template reused for all children | Pass any child spec to `start_child/2`; reuse `{Mod, arg}` per child |
| Child spec `:id` must be unique | `:id` is ignored — reuse freely |
| `supervisor:terminate_child(Sup, Pid)` — terminate by pid | `DynamicSupervisor.terminate_child(sup, pid)` — same pid-keyed shape |
| `:simple_one_for_one` shuts down all children asynchronously | DynamicSupervisor also "execute[s] certain operations, such as shutting down, concurrently" |

Before (deprecated Erlang `:simple_one_for_one`, illustrative):

```elixir
# Deprecated: a single child spec template + appended args
:supervisor.start_child(sup, [connection_id])
# started via apply(ConnectionWorker, :start_link, [] ++ [connection_id])
```

After (DynamicSupervisor):

```elixir
# Idiomatic: pass the per-child argument as the child spec arg
{:ok, pid} =
  DynamicSupervisor.start_child(MyApp.DynamicSupervisor, {ConnectionWorker, connection_id})

# Or, for arguments shared by all children, set :extra_arguments at init time
# (remember: they are PREPENDED to the child spec args):
DynamicSupervisor.init(
  strategy: :one_for_one,
  extra_arguments: [shared_config]
)
```

### Dynamic child specs

Each child passed to `start_child/2` must be a valid child spec. Because `use GenServer` (and `use Supervisor`) auto-define `child_spec/1`, passing `{MyWorker, arg}` works directly — the worker module encapsulates its own spec (cross-reference the Supervisor section's "Child specifications and child_spec/1" and the GenServer section's `start_link vs start`). The `:restart`, `:shutdown`, and `:type` keys in each child spec still apply per-child. `:transient` or `:temporary` restart is commonly combined with `terminate_child/2` or self-shutdown for workers that should not be restarted after a normal finish.

### Scalability and partitioning (PartitionSupervisor)

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> The `DynamicSupervisor` is a single process responsible for starting other processes. In some applications, the `DynamicSupervisor` may become a bottleneck. To address this, you can start multiple instances of the `DynamicSupervisor` and then pick a "random" instance to start the child on.

From the docs:

```elixir
children = [
  {PartitionSupervisor,
   child_spec: DynamicSupervisor,
   name: MyApp.DynamicSupervisors}
]
```

```elixir
DynamicSupervisor.start_child(
  {:via, PartitionSupervisor, {MyApp.DynamicSupervisors, self()}},
  {Counter, 0}
)
```

From [DynamicSupervisor.html](https://hexdocs.pm/elixir/DynamicSupervisor.html):

> In the code above, we start a partition supervisor that will by default start a dynamic supervisor for each core in your machine. Then, instead of calling the `DynamicSupervisor` by name, you call it through the partition supervisor, using `self()` as the routing key.

`PartitionSupervisor` requires Elixir 1.14+.

### Registry integration (:via naming for dynamic children)

The DynamicSupervisor page itself does not show a Registry example, but the canonical pattern is: each child registers itself under a `:via` name in a `:unique` Registry inside its own `start_link/1` (cross-reference the GenServer section's "Process naming and registration" and "Registry :via naming" example). Because DynamicSupervisor ignores `:id`, you MUST use a Registry (or other `:via` module) if you need to address children by a meaningful logical key rather than by PID.

From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> Once the registry is started with a given name using `Registry.start_link/1`, it can be used to register and access named processes using the `{:via, Registry, {registry, key}}` tuple.
> Only registries with unique keys can be used in `:via`.

Standard pattern combining `start_child/2` with a child that registers via Registry:

```elixir
# In your application supervision tree:
children = [
  {Registry, keys: :unique, name: MyApp.WorkerRegistry},
  {DynamicSupervisor, name: MyApp.WorkerSupervisor, strategy: :one_for_one}
]

defmodule MyApp.Worker do
  use GenServer

  def start_link(key) do
    GenServer.start_link(__MODULE__, key,
      name: {:via, Registry, {MyApp.WorkerRegistry, key}})
  end

  # ...init/handlers...
end
```

```elixir
# Start a worker by logical key:
{:ok, _pid} =
  DynamicSupervisor.start_child(MyApp.WorkerSupervisor, {MyApp.Worker, "user:42"})

# Address it later by the same key from anywhere:
GenServer.call({:via, Registry, {MyApp.WorkerRegistry, "user:42"}}, :ping)
```

A second `start_child/2` with the same registered key returns `{:error, {:already_started, pid}}` — this is the ONLY case where `start_child/2` returns `:already_started`.

### Common mistakes

1. **Using DynamicSupervisor where static Supervisor is needed** — fix: if children are known at boot and need `:one_for_all`/`:rest_for_one` ordering, use a static `Supervisor`; `DynamicSupervisor` only supports `:one_for_one`.
2. **Expecting duplicate child `:id` to error** — fix: `:id` is ignored; `start_child/2` does NOT return `{:already_started, pid}` for duplicate ids. Use a `:via` Registry for logical-key uniqueness.
3. **Calling `terminate_child(sup, child_id)` with an id** — fix: DynamicSupervisor's `terminate_child/2` takes a PID, not an id. Capture the PID from `start_child/2` or look it up via a Registry.
4. **Expecting `:extra_arguments` to append** — fix: DynamicSupervisor PREPENDS `:extra_arguments` to the child spec args; Erlang's `:simple_one_for_one` appended. Order matters when migrating.
5. **Forgetting `:max_children` is a hard concurrency cap** — fix: once `:max_children` is reached, `start_child/2` returns `{:error, :max_children}`; this is independent of restart intensity.
6. **Single DynamicSupervisor as a global bottleneck** — fix: wrap in a `PartitionSupervisor` (Elixir 1.14+) to spread `start_child/2` across cores.
7. **Calling `which_children/1` on a supervisor with millions of children under memory pressure** — fix: the docs warn this can cause out-of-memory; prefer `count_children/1` or a Registry lookup for targeted access.
8. **Expecting `:auto_shutdown` / `:significant` support** — fix: DynamicSupervisor does not support `:auto_shutdown` or significant children; those are static-`Supervisor` features.

## Registry

### What Registry is

From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> A local, decentralized and scalable key-value process storage.

A `Registry` maps `{key, value}` pairs to the process that registered them. Keys can be any term (strings, integers, tuples, structs) — not just atoms. When a registered process exits (including a crash), its entries are removed automatically, so the registry is self-cleaning without a manual `unregister/2` on the crash path.

From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> The registry uses one ETS table plus two ETS tables per partition.

So a `Registry` is a specialized, supervised wrapper over ETS. It is itself a process started under supervision, and lookups read ETS directly rather than funnelling through a single owner process — that is the "decentralized" property. It is local to a single node: the docs describe a "local, ... non-distributed ... storage." For cluster-wide process registration, Registry is not the tool; reach for `:global`, `Horde`, `:syn`, or distributed `Phoenix.PubSub` adapters.

Cross-reference the GenServer section's "Process naming and registration" subsection: Registry is the recommended backend for dynamic local names via `:via` tuples, because "atoms are never garbage-collected and therefore dynamically generated atoms won't be garbage-collected."

### When to use Registry vs Process.register/2 vs :global

| Need | Use |
|---|---|
| One well-known process with a fixed name, same node | A local atom as `:name` on `start_link/3` (equivalent to `Process.register/2`). |
| Many processes addressed by dynamic/arbitrary keys, same node | `Registry` via `:via` tuples. |
| A single process name that must resolve to one PID cluster-wide | `{:global, term}`. |
| Cluster-wide dynamic / many-to-many naming | Beyond Registry; use `Horde`, `:syn`, etc. |
| Many-to-many mappings, fan-out, or local pub/sub | `Registry` with `:duplicate` keys + `dispatch/4`. |

Comparison of the three mechanisms:

| | `Process.register/2` | `:global` | `Registry` |
|---|---|---|---|
| Scope | Local node | Cluster-wide (replicated) | Local node |
| Key type | Atom only | Any term | Any term |
| Mapping | 1 name → 1 PID | 1 name → 1 PID | 1 key → many PIDs (`:duplicate`); 1 key → 1 PID (`:unique`) |
| Metadata per entry | None | None | Arbitrary `value` term |
| Auto-cleanup on crash | Yes (linked) | Yes | Yes (monitored) |
| Scales across cores | N/A (one table) | N/A | Yes (`:partitions`) |
| Cost | Cheapest | Heaviest (global lock, fully connected mesh) | Moderate |

From [Process.html](https://hexdocs.pm/elixir/Process.html#register/2): `Process.register/2` requires an atom name, fails with `ArgumentError` on a duplicate, and reserves `nil`, `false`, `true`, and `:undefined`. Use it only for a small, fixed set of well-known names.

### start_link/1 and putting Registry in a supervision tree

```elixir
@spec start_link([start_option()]) :: {:ok, pid()} | {:error, term()}
```

```elixir
@type keys() :: :unique | :duplicate | {:duplicate, :key} | {:duplicate, :pid}

@type start_option() ::
        {:keys, keys()}
        | {:name, registry()}
        | {:partitions, pos_integer()}
        | {:listeners, [atom()]}
        | {:meta, [{meta_key(), meta_value()}]}
```

`Registry.start_link/1` (since 1.5.0) requires `:keys` and `:name`; the rest are optional. `Registry.child_spec/1` accepts the same options, so Registry plugs directly into a supervision tree as a named child:

```elixir
children = [
  {Registry, keys: :unique, name: MyApp.Registry},
  {DynamicSupervisor, name: MyApp.WorkerSupervisor, strategy: :one_for_one}
]

Supervisor.start_link(children, strategy: :one_for_one)
```

| Option | Required | Default | Meaning |
|---|---|---|---|
| `:keys` | yes | — | `:unique`, `:duplicate`, `{:duplicate, :key}`, or `{:duplicate, :pid}`. See "Unique vs duplicate keys" below. |
| `:name` | yes | — | The atom that identifies the registry (used in `:via` tuples and every API call). |
| `:partitions` | no | `1` | Number of partition processes / ETS-table groups. Scale with `System.schedulers_online()` for hot registries. |
| `:listeners` | no | `[]` | Named processes notified with `{:register, ...}` / `{:unregister, ...}` tuples. |
| `:meta` | no | `[]` | Static `{key, value}` metadata attached to the registry at start (read via `Registry.meta/2`). |

Ordering: the Registry must start before any process that registers with or addresses it via `:via`. Cross-reference the Supervisor section's "Building supervision trees": under `:one_for_one`, a crash-restart of a worker that uses the registry resolves the name against an already-running registry, but at boot the registry child must precede the children that depend on it.

### Unique vs duplicate keys

The `:keys` option has four values, in two families:

| `:keys` | Mapping | Partitioning | Typical use |
|---|---|---|---|
| `:unique` | 1 key → at most 1 process | by key | Name lookup / `:via` naming |
| `:duplicate` (alias for `{:duplicate, :pid}`) | 1 key → many processes | by pid | Pub/sub with few keys, many subscribers |
| `{:duplicate, :key}` | 1 key → many processes | by key | Pub/sub with many keys, few subscribers each |

From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> `:duplicate` or `{:duplicate, :pid}` - Use `:pid` partitioning (default) when you have keys with many entries (e.g., one topic with many subscribers) ...
>
> `{:duplicate, :key}` - Use `:key` partitioning when entries are spread across many different keys (e.g., many topics with few subscribers each). This makes key-based lookups more efficient as they only need to check a single partition instead of all partitions. This option uses a different internal ETS table type (`ordered_set` instead of `duplicate_bag`) ...

Behavioral differences:

- On a `:unique` registry, `register/3` for an already-taken key returns `{:error, {:already_registered, pid}}`. Re-registering the same key from the same process is also rejected.
- On a `:duplicate` registry, any number of processes (and the same process multiple times) may register under one key — this is what enables fan-out and pub/sub.
- `{:duplicate, :key}` and `{:duplicate, :pid}` were introduced in Elixir 1.19.0; the `ordered_set` composite-key layout landed in 1.20.0. Both are first-class values in 1.20.2.

Only `:unique` registries can back a `:via` name. From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> Only registries with unique keys can be used in `:via`.

### register/3, unregister/2, lookup/2 — the core API

```elixir
@spec register(registry(), key(), value()) ::
        {:ok, pid()} | {:error, {:already_registered, pid()}}

@spec unregister(registry(), key()) :: :ok

@spec lookup(registry(), key()) :: [{pid(), value()}]
```

`register/3` registers the calling process under `key` with `value`. It returns `{:ok, owner}` where `owner` is the partition process; that owner is automatically linked to the caller and unlinked when the caller has no remaining keys. `lookup/2` returns a list of `{pid, value}` (a list, not a single tuple, because `:duplicate` registries can hold several entries per key).

```elixir
{:ok, _} = Registry.start_link(keys: :unique, name: MyApp.Registry)

{:ok, _owner} = Registry.register(MyApp.Registry, "agent", nil)
Registry.lookup(MyApp.Registry, "agent")
#=> [{self(), nil}]

# A :unique registry rejects a second registration of the same key:
Registry.register(MyApp.Registry, "agent", :later)
#=> {:error, {:already_registered, self()}}
```

Duplicate registry — same key held by many processes:

```elixir
{:ok, _} =
  Registry.start_link(
    keys: :duplicate,
    name: MyApp.PubSub,
    partitions: System.schedulers_online()
  )

{:ok, _} = Registry.register(MyApp.PubSub, "topic:42", [])
# ... from another process ...
Registry.lookup(MyApp.PubSub, "topic:42")
#=> [{pid_a, []}, {pid_b, []}, ...]
```

`unregister/2` always returns `:ok`. Because Registry monitors registered processes, entries are removed automatically when a process exits — you do not need to call `unregister/2` defensively in a `terminate/2` to survive crashes.

### :via tuples — Registry as a name backend

A `{:via, Registry, {registry, key}}` tuple is the supported way to use Registry as the name-resolution backend for `GenServer`, `Agent`, `Task`, and `DynamicSupervisor` children. Two forms exist:

| Form | Stored value |
|---|---|
| `{:via, Registry, {registry, key}}` | `nil` |
| `{:via, Registry, {registry, key, value}}` | `value` (custom metadata) |

```elixir
# With metadata:
name = {:via, Registry, {MyApp.Registry, "agent", :hello}}
{:ok, agent_pid} = Agent.start_link(fn -> 0 end, name: name)
Registry.lookup(MyApp.Registry, "agent")
#=> [{agent_pid, :hello}]

# Both the with- and without-metadata forms resolve to the same pid:
Agent.get({:via, Registry, {MyApp.Registry, "agent"}}, & &1)
#=> 0
```

Registry satisfies the `:via` callback contract described in [GenServer.html](https://hexdocs.pm/elixir/GenServer.html) by exporting `register_name/2`, `unregister_name/1`, `whereis_name/1`, and `send/2`. From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html):

> The `:via` option expects a module that exports `register_name/2`, `unregister_name/1`, `whereis_name/1` and `send/2`.

If the `:via` name is already taken, `start_link` returns `{:error, {:already_started, pid}}`. Cross-reference the GenServer section's "Process naming and registration" subsection and the DynamicSupervisor section's "Registry integration (`:via` naming for dynamic children)" subsection.

### Partitioning and scale

The `:partitions` option (default `1`) starts one partition process per partition, each owning its own ETS tables. Writes are routed to a partition by hashing the key (`:unique`, `{:duplicate, :key}`) or the registering PID (`{:duplicate, :pid}`).

```elixir
# Hot unique registry, sized to core count:
Registry.start_link(
  keys: :unique,
  name: MyApp.Registry,
  partitions: System.schedulers_online()
)

# Many-topic duplicate registry, partitioned by key so each lookup hits one partition:
Registry.start_link(
  keys: {:duplicate, :key},
  name: MyApp.TopicRegistry,
  partitions: System.schedulers_online()
)
```

Throughput benefits are largest for registries with many entries, at the cost of one extra process (and ETS tables) per partition. Each partition is a real process, so values in the tens (`System.schedulers_online()`, typically 4–32) are reasonable; hundreds or thousands of partitions are operationally expensive.

### match/4 and select/2 — ETS-style queries

```elixir
@spec match(registry(), key(), match_pattern(), guards()) :: [{pid(), term()}]

@spec select(registry(), spec()) :: [term()]
```

`match/4` matches against the `value` registered under a specific `key` (use `:_` to ignore, `:"$1"`/`:"$2"` to capture, guards as tuples like `{:>, :"$1", 1}`). `select/2` (since 1.9.0) runs a full match spec over `{key, pid, value}` entries.

```elixir
# All entries under "key" whose value is an integer > 1:
Registry.match(MyApp.Registry, "key", :"$1", [{:>, :"$1", 1}])

# All registered keys:
Registry.select(MyApp.Registry, [{{:"$1", :_, :_}, [], [:"$1"]}])
```

From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> Do not use special match variables `:"$_"` and `:"$$"`, because they might not work as expected. In particular, `{:duplicate, :key}` registries use a different internal ETS layout, so match specs that reference the underlying entry structure via `:"$_"` will return different results. Use named variables like `:"$1"`, `:"$2"`, `:"$3"` instead.

Related counters: `count/1` (since 1.7.0, total entries, O(1)), `count_match/4` (since 1.7.0), `count_select/2` (since 1.14.0). Per-process introspection: `keys/2` returns the keys a pid holds; `values/3` (since 1.12.0) returns the values for a specific `{key, pid}` pair.

### dispatch/4 — pub/sub and fan-out

```elixir
@spec dispatch(registry(), key(), (entries :: [{pid(), value()}] -> term()), dispatch_opts()) :: :ok
```

On a `:duplicate` registry, `dispatch/4` (since 1.4.0) hands the caller all `{pid, value}` entries for a key; the callback decides what to do (typically `send/2` to each pid). From [Registry.html](https://hexdocs.pm/elixir/Registry.html):

> Registries can also be used to implement a local, non-distributed, scalable PubSub by relying on the `dispatch/3` function.

```elixir
Registry.dispatch(MyApp.PubSub, "topic:42", fn entries ->
  for {pid, _} <- entries, do: send(pid, {:broadcast, :hello})
end)
```

Options: `parallel: true` dispatches across partitions concurrently — but, per the docs, "`:parallel` ... is only meaningful for `{:duplicate, :pid}` registries" because a `{:duplicate, :key}` registry keeps every entry for a key in a single partition. For production-grade distributed pub/sub use `Phoenix.PubSub`; Registry-based pub/sub is local-only.

### Metadata, listeners, locks (advanced)

- `meta/2`, `put_meta/3`, and `delete_meta/2` (since 1.11.0) read/write/delete arbitrary metadata attached to the registry itself (separate from per-entry `value`s). `:meta` on `start_link/1` seeds initial metadata.
- The `:listeners` option notifies named processes on register/unregister. Listeners receive `{:register, ...}` / `{:unregister, ...}` messages; to learn of crashes they must `Process.monitor/1` the registered process themselves.
- `update_value/3` (since 1.4.0) atomically updates the value for a key on a `:unique` registry, returning `{new, old}` or `:error`; it raises on a `:duplicate` registry.
- `unregister_match/4` (since 1.5.0) conditionally removes entries matching a pattern.
- `lock/3` (since 1.18.0) provides an out-of-band lock on a separate namespace, for coordination you cannot defer to a process.

### Registry + DynamicSupervisor (dynamic named workers)

The canonical pattern: a `:unique` Registry and a `DynamicSupervisor` both named by atom; each worker builds its own `:via` name from a logical key in `start_link/1`. Cross-reference the DynamicSupervisor section's "Registry integration (`:via` naming for dynamic children)" subsection for the full worked example. Two consequences worth restating:

1. Because DynamicSupervisor ignores the child-spec `:id`, the `:via` Registry key is the canonical logical identifier for a child. A second `start_child/2` for an already-registered key returns `{:error, {:already_started, pid}}` — the only `:already_started` case for DynamicSupervisor.
2. `DynamicSupervisor.terminate_child/2` takes a PID, not a name, so resolve the key first:

```elixir
case Registry.lookup(MyApp.Registry, "user:42") do
  [{pid, _}] -> DynamicSupervisor.terminate_child(MyApp.WorkerSupervisor, pid)
  [] -> {:error, :not_found}
end
```

The registry entry is removed automatically once the worker exits, so no manual `unregister/2` is needed after termination.

### Common mistakes

1. **Using `:via` with a `:duplicate` registry** — fix: `:via` requires a `:unique` registry; use a separate `:unique` registry for naming and a `:duplicate` registry for pub/sub.
2. **Using dynamic atoms (`String.to_atom/1`) as process names** — fix: a `:unique` Registry via `:via`; atoms are never garbage-collected.
3. **Expecting Registry to be cluster-wide** — fix: Registry is local per node; for cluster-wide naming use `:global`, `Horde`, or `:syn`.
4. **Forgetting to start the Registry before dependents** — fix: list the Registry child before any worker that registers with or addresses it via `:via`.
5. **Passing a `:via` tuple to `DynamicSupervisor.terminate_child/2`** — fix: it requires a PID; resolve via `Registry.lookup/2` first.
6. **Using `:"$_"` / `:"$$"` in `select/2` on a `{:duplicate, :key}` registry** — fix: use named match variables (`:"$1"`, `:"$2"`, ...); the internal ETS layout differs.
7. **Calling `update_value/3` on a `:duplicate` registry** — fix: it is unique-only and raises otherwise; store and re-`register/3` instead.
8. **Running a hot registry with `partitions: 1`** — fix: scale with `partitions: System.schedulers_online()` when reads/writes are a bottleneck.

## Application

### What an Application is / OTP application theory

From [Application.html](https://hexdocs.pm/elixir/Application.html):

> "A module for working with applications and defining application callbacks."

From the Erlang [applications.html](https://www.erlang.org/doc/system/applications.html):

> "After creating code to implement a specific functionality, you might consider transforming it into an *application* — a component that can be started and stopped as a unit, as well as reused in other systems."

From [Application.html](https://hexdocs.pm/elixir/Application.html):

> "Applications are the idiomatic way to package software in Erlang/OTP. To get the idea, they are similar to the 'library' concept common in other programming languages, but with some additional characteristics."
>
> "An application is a component implementing some specific functionality, with a standardized directory structure, configuration, and life cycle. Applications are *loaded*, *started*, and *stopped*. Each application also has its own environment, which provides a unified API for configuring each application."

An OTP application is NOT a running process; it is a component (compiled code plus a `.app` resource file and an optional callback module) that the runtime can load, start, and stop as a unit. When started, an application boots a supervision tree from a single top supervisor. Cross-reference the Supervisor section's "Building supervision trees" subsection above, which shows `Application.start/2` returning `Supervisor.start_link(children, ...)`.

### The Application behaviour and use Application

`Application` is a behaviour. Its callbacks are:

| Callback | Required? | Purpose |
|---|---|---|
| `start/2` | Yes | Start the supervision tree; return the top supervisor PID. |
| `stop/1` | Yes | Clean up after the application has stopped. |
| `prep_stop/1` | No | Run before the application stops, while it is still fully running. |
| `start_phase/3` | No | Run named startup phases, used with included applications. |
| `config_change/3` | No | React to application environment changes during a release upgrade/downgrade. |

```elixir
@callback start(start_type(), start_args :: term()) ::
        {:ok, pid()}
        | {:ok, pid(), state()}
        | {:error, reason :: term()}

@callback prep_stop(state()) :: state()

@callback start_phase(phase :: term(), start_type(), phase_args :: term()) ::
        :ok | {:error, reason :: term()}

@callback stop(state()) :: term()

@callback config_change(changed, new, removed) :: :ok
        when changed: keyword(), new: keyword(), removed: [atom()]
```

From [Application.html](https://hexdocs.pm/elixir/Application.html#module-use-application):

> "When you `use Application`, the `Application` module will set `@behaviour Application` and define an overridable definition for the `stop/1` function, which is required by Erlang/OTP."
>
> "`use Application` provides no default implementation for the `start/2` callback."
>
> "By using `Application`, modules get a default implementation of `stop/1` that ignores its argument and returns `:ok`, but it can be overridden."

So a minimal application callback module must implement `start/2`; `stop/1` is provided by default. Use `@impl true` on each callback you write (cross-reference the GenServer section's callback-contract guidance).

### start/2 and the top supervisor

`start/2` must return the PID of the application's top-level supervisor. From [Application.html](https://hexdocs.pm/elixir/Application.html#c:start/2):

> "This function should either return `{:ok, pid}` or `{:ok, pid, state}` if startup is successful. `pid` should be the PID of the top supervisor. `state` can be an arbitrary term, and if omitted will default to `[]`; if the application is later stopped, `state` is passed to the `stop/1` callback (see the documentation for the `stop/1` callback for more information)."

The `start_type` argument is:

```elixir
@type start_type() :: :normal | {:takeover, node()} | {:failover, node()}
```

From [Application.html](https://hexdocs.pm/elixir/Application.html#c:start/2):

> "`start_type` defines how the application is started:
> - `:normal` - used if the startup is a normal startup or if the application is distributed and is started on the current node because of a failover from another node and the application specification key `:start_phases` is `:undefined`.
> - `{:takeover, node}` - used if the application is distributed and is started on the current node because of a failover on the node `node`.
> - `{:failover, node}` - used if the application is distributed and is started on the current node because of a failover on node `node`, and the application specification key `:start_phases` is not `:undefined`."

For non-distributed applications `start_type` is always `:normal`, so `_type` is commonly ignored. From the Erlang [applications.html](https://www.erlang.org/doc/system/applications.html):

> "`start/2` is called when starting the application and is to create the supervision tree by starting the top supervisor. It is expected to return the pid of the top supervisor and an optional term, `State`, which defaults to `[]`. This term is passed as is to `stop/1`."

Canonical application callback module:

```elixir
defmodule MyApp.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      MyApp.WorkerSupervisor,
      {Registry, keys: :unique, name: MyApp.Registry}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: MyApp.Supervisor)
  end

  # stop/1 is provided by default; override only if you need cleanup
  # that must run after the supervision tree has terminated.
end
```

`{:error, reason}` from `start/2` aborts application startup.

### Application environment

Each application carries its own key/value environment. From [Application.html](https://hexdocs.pm/elixir/Application.html):

```elixir
@spec get_env(app(), key(), value()) :: value()
@spec fetch_env(app(), key()) :: {:ok, value()} | :error
@spec fetch_env!(app(), key()) :: value()
@spec put_env(app(), key(), value(), timeout: timeout(), persistent: boolean()) :: :ok
@spec get_all_env(app()) :: [{key(), value()}]
@spec delete_env(app(), key(), timeout: timeout(), persistent: boolean()) :: :ok
```

> "`get_env/3` - Returns the value for `key` in `app`'s environment. If the configuration parameter does not exist, the function returns the `default` value." — https://hexdocs.pm/elixir/Application.html#get_env/3

> "`fetch_env/2` - Returns the value for `key` in `app`'s environment in a tuple. If the configuration parameter does not exist, the function returns `:error`." — https://hexdocs.pm/elixir/Application.html#fetch_env/2

> "`fetch_env!/2` - Returns the value for `key` in `app`'s environment. If the configuration parameter does not exist, raises `ArgumentError`." — https://hexdocs.pm/elixir/Application.html#fetch_env!/2

> "`put_env/4` - Puts the `value` in `key` for the given `app`." — https://hexdocs.pm/elixir/Application.html#put_env/4

> "`get_all_env/1` - Returns all key-value pairs for `app`." — https://hexdocs.pm/elixir/Application.html#get_all_env/1

> "`delete_env/3` - Deletes the `key` from the given `app` environment. It receives the same options as `put_env/4`. Returns `:ok`." — https://hexdocs.pm/elixir/Application.html#delete_env/3

Reading at compile time uses the `compile_env` family (since 1.10.0):

> "`compile_env/3` - Reads the application environment at compilation time. Similar to `get_env/3`, except it must be used to read values at compile time. This allows Elixir to track when configuration values change between compile time and runtime." — https://hexdocs.pm/elixir/Application.html#compile_env/3

```elixir
@spec compile_env(app(), key() | list(), value()) :: value()         # macro, since 1.10.0
@spec compile_env!(app(), key() | list()) :: value()                 # macro, since 1.10.0
@spec compile_env(Macro.Env.t(), app(), key() | list(), value()) :: value()  # macro, since 1.14.0
```

Reading `fetch_env!/2` in a module body (i.e. at compile time) is discouraged:

> "warning: Application.fetch_env!/2 is discouraged in the module body, use Application.compile_env/3 instead" — https://hexdocs.pm/elixir/Application.html#module-compile-time-environment

And `put_env/4` must not be used to change values read via `compile_env`:

> "Do not use this function to change environment variables read via `Application.compile_env/2`. The compile environment must be exclusively set before compilation, in your config files." — https://hexdocs.pm/elixir/Application.html#put_env/4

Defaults come from the `:env` key of `def application` (see below); runtime overrides come from `config/config.exs` and `config/runtime.exs`. In libraries, prefer passing configuration through function arguments over the application environment:

> "If you are writing a library to be used by other developers, it is generally recommended to avoid the application environment, as the application environment is effectively a global storage." — https://hexdocs.pm/elixir/Application.html#module-application-environment-in-libraries

### Starting, stopping, and inspecting applications

```elixir
@spec start(app(), restart_type()) :: :ok | {:error, term()}
@spec stop(app()) :: :ok | {:error, term()}
@spec ensure_started(app(), restart_type()) :: :ok | {:error, term()}
@spec ensure_all_started(app() | [app()],
            type: restart_type(), mode: :serial | :concurrent) ::
        {:ok, [app()]} | {:error, term()}
@spec load(app()) :: :ok | {:error, term()}
@spec unload(app()) :: :ok | {:error, term()}
@spec spec(app()) :: [{application_key(), value()}] | nil
@spec app_dir(app()) :: String.t()
@spec get_application(module()) :: app() | nil
```

From [Application.html](https://hexdocs.pm/elixir/Application.html#start/2):

> "Starts the given `app` with `restart_type/0`. If the `app` is not loaded, the application will first be loaded using `load/1`. Any included application, defined in the `:included_applications` key of the `.app` file will also be loaded, but they won't be started. Furthermore, all applications listed in the `:applications` key must be explicitly started before this application is. If not, `{:error, {:not_started, app}}` is returned, where `app` is the name of the missing application."

> "Stops the given `app`. When stopped, the application is still loaded." — https://hexdocs.pm/elixir/Application.html#stop/1

> "Ensures the given `app` is started with `restart_type/0`. Same as `start/2` but returns `:ok` if the application was already started." — https://hexdocs.pm/elixir/Application.html#ensure_started/2

> "Ensures the given `app` or `apps` and their child applications are started." — https://hexdocs.pm/elixir/Application.html#ensure_all_started/2

> "Loads the given `app`. In order to be loaded, an `.app` file must be in the load paths. All `:included_applications` will also be loaded. Loading the application does not start it nor load its modules, but it does load its environment." — https://hexdocs.pm/elixir/Application.html#load/1

`Application.spec/1` reads the `.app` specification; `app_dir/1` returns the application's ebin directory; `get_application/1` maps a module to its application (`nil` if none). The `restart_type/0` passed to `Application.start/2` is:

```elixir
@type restart_type() :: :permanent | :transient | :temporary
```

In tests, prefer `Application.ensure_all_started/1` (idempotent, boots dependencies) over `Application.start/1`.

### mix.exs application config and the .app file

An application is declared in `mix.exs` via `def application`. From [Mix.Tasks.Compile.App](https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html):

> "Writes a `.app` file. A `.app` file is a file containing Erlang terms that defines your application. Mix automatically generates this file based on your `mix.exs` configuration. In order to generate the `.app` file, Mix expects your project to have both the `:app` and the `:version` keys. Furthermore, you can configure the generated application by defining an `application/0` function in your `mix.exs` that returns a keyword list."

The canonical shape:

```elixir
def application do
  [
    extra_applications: [:logger, :crypto, ex_unit: :optional],
    env: [key: :value],
    registered: [MyServer]
  ]
end
```

Key options:

| Key | Meaning |
|---|---|
| `:mod` | The application callback module as `{Mod, args}`. Its `start/2` boots the supervision tree. |
| `:extra_applications` | OTP apps your app depends on that are NOT in `deps/0` (e.g. `:logger`, `:crypto`). Optional deps use `{:ex_unit, :optional}`. Mix starts non-optional ones before your app. |
| `:included_applications` | Apps started as part of THIS app's supervision tree (see "Included applications" below). |
| `:applications` | Runtime dependencies; auto-inferred from `deps/0` unless overridden. |
| `:env` | Default application environment values. |
| `:registered` | Registered process names in this app, for conflict detection. |
| `:start_phases` | Named phases with arguments, run via `start_phase/3`. |
| `:maxT` | Max runtime in ms; app stopped with top supervisor terminated `:normal`. Defaults to `:infinity`. |

From [Mix.Tasks.Compile.App](https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html):

> "`:mod` - specifies a module to invoke when the application is started. It must be in the format `{Mod, args}` where args is often an empty list. The module specified must implement the callbacks defined by the `Application` module."

> "`:extra_applications` - a list of OTP applications your application depends on which are not included in `:deps` (usually defined in `deps/0` in your `mix.exs`). For example, here you can declare a dependency on applications that ship with Erlang/OTP or Elixir, like `:crypto` or `:logger`. Optional extra applications can be declared as a tuple, such as `{:ex_unit, :optional}`. Mix guarantees all non-optional applications are started before your application starts."

> "`:registered` - the name of all registered processes in the application. If your application defines a local GenServer with name `MyServer`, it is recommended to add `MyServer` to this list. It is most useful in detecting conflicts between applications that register the same names."

> "`:env` - the default values for the application environment. The application environment is one of the most common ways to configure applications."

To make an app start a supervision tree, add `:mod`:

```elixir
def application do
  [mod: {MyApp.Application, []}]
end
```

— https://hexdocs.pm/elixir/Application.html#module-the-application-callback-module

This generates an `.app` resource file of the form `{application, Application, [Opt1,...,OptN]}.` (see the Erlang [application resource file](https://www.erlang.org/doc/system/applications.html)). The `:mod` key's start argument becomes the `StartArgs` passed to `start/2`:

> "`StartArgs` is defined by the key `mod` in the application resource file." — https://www.erlang.org/doc/system/applications.html

### Included applications

From [Mix.Tasks.Compile.App](https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html):

> "`:included_applications` - specifies a list of applications that will be included in the application. It is the responsibility of the primary application to start the supervision tree of all included applications, as only the primary application will be started. A process in an included application considers itself belonging to the primary application."

From the Erlang [included_applications.html](https://www.erlang.org/doc/system/included_applications.html):

> "The application controller automatically loads any included applications when loading a primary application, but does not start them. Instead, the top supervisor of the included application must be started by a supervisor in the including application."

Difference:

| | `:extra_applications` | `:included_applications` |
|---|---|---|
| Loaded by controller | Yes | Yes |
| Started independently | Yes (before your app) | No |
| Supervision tree owner | The app itself | The including (primary) app |

Synchronization across an including app and its included apps uses start phases (the `:start_phases` key and the `start_phase/3` callback).

### Releases vs applications

An application is a reusable component; a release is a complete, deployable system assembled from one or more applications. From the Erlang [release_structure.html](https://www.erlang.org/doc/system/release_structure.html):

> "When you have written one or more applications, you might want to create a complete system with these applications and a subset of the Erlang/OTP applications. This is called a *release*."

A release is booted by a `.boot` script that loads and starts its applications in dependency order:

> "When starting Erlang/OTP using the boot script, all applications from the `.rel` file are automatically loaded and started" — https://www.erlang.org/doc/system/release_structure.html

Elixir builds releases with `mix release`. From [Mix.Tasks.Release](https://hexdocs.pm/mix/Mix.Tasks.Release.html#module-why-releases):

> "Releases allow developers to precompile and package all of their code and the runtime into a single unit."

> "Developers can also use `mix release` to build **releases**. Releases are able to package all of your source code as well as the Erlang VM into a single directory. Releases also give you explicit control over how each application is started and in which order." — https://hexdocs.pm/elixir/Application.html#module-tooling

In development (`mix`, `iex -S mix`) applications are started on demand; in a release, the boot script starts them in order. The supervision trees themselves are identical either way — a release only changes packaging and startup ordering.

### Common mistakes

1. **Forgetting `:mod` so the app starts no supervision tree** — fix: add `mod: {MyApp.Application, []}` to `def application`; otherwise the app loads but starts nothing.
2. **Implementing `stop/1` but not `start/2`** — fix: `use Application` gives a default `stop/1` but NO default `start/2`; you must implement `start/2`.
3. **Calling `Application.put_env/4` for values read at compile time** — fix: use `Application.compile_env/3`; `put_env/4` must not change compile-env values.
4. **Reading `Application.fetch_env!/2` in a module body** — fix: it is discouraged at compile time; use `compile_env/3`, or read at runtime inside `init/1`.
5. **Using the application environment for library configuration** — fix: it is effectively global storage; pass configuration through function arguments.
6. **Expecting `:extra_applications` and `:included_applications` to behave the same** — fix: extra apps start independently before your app; included apps' supervision trees must be started by YOUR supervisors.
7. **Confusing `start_type` (`:normal`/`:takeover`/`:failover`) with `restart_type` (`:permanent`/`:transient`/`:temporary`)** — fix: `start_type` describes distributed-app startup; `restart_type` describes application-level restart semantics passed to `Application.start/2`.
8. **Putting heavy work in `Application.start/2`** — fix: `start/2` should only build the tree; do work in child `init/1`/`handle_continue/2` (cross-reference the GenServer section's `handle_continue` guidance).

## Child Specs

This section expands on the Supervisor section's "Child specifications and child_spec/1" and "Inline vs module-based child specs" subsections above. Read those first for the four accepted child forms and `:id` uniqueness.

### The child specification map

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-child-specification):

> "The child specification describes how the supervisor starts, shuts down, and restarts child processes.
>
> The child specification is a map containing up to 6 elements. The first two keys in the following list are required, and the remaining ones are optional:
>
> - `:id` - any term used to identify the child specification internally by the supervisor; defaults to the given module. This key is required. For supervisors, in the case of conflicting `:id` values, the supervisor will refuse to initialize and require explicit IDs. This is not the case for dynamic supervisors though.
> - `:start` - a tuple with the module-function-args to be invoked to start the child process. This key is required.
> - `:restart` - an atom that defines when a terminated child process should be restarted (see the 'Restart values' section below). This key is optional and defaults to `:permanent`.
> - `:shutdown` - an integer or atom that defines how a child process should be terminated (see the 'Shutdown values' section below). This key is optional and defaults to `5_000` if the type is `:worker` or `:infinity` if the type is `:supervisor`.
> - `:type` - specifies that the child process is a `:worker` or a `:supervisor`. This key is optional and defaults to `:worker`.
> - `:modules` - a list of modules used by hot code upgrade mechanisms to determine which processes are using certain modules. It is typically set to the callback module of behaviours like `GenServer`, `Supervisor`, and such. It is set automatically based on the `:start` value and it is rarely changed in practice.
> - `:significant` - a boolean indicating if the child process should be considered significant with regard to automatic shutdown. Only `:transient` and `:temporary` child processes can be marked as significant. This key is optional and defaults to `false`. See section 'Automatic shutdown' below for more details."

The Elixir typespec (cross-reference the Supervisor section above):

```elixir
@type child_spec :: %{
        :id => atom() | term(),
        :start => {module(), function_name :: atom(), args :: [term()]},
        optional(:restart) => restart(),
        optional(:shutdown) => shutdown(),
        optional(:type) => type(),
        optional(:modules) => [module()] | :dynamic,
        optional(:significant) => boolean()
      }

@type restart :: :permanent | :transient | :temporary
@type shutdown :: timeout() | :brutal_kill
@type type :: :worker | :supervisor
```

| Key | Required | Default | Notes |
|---|---|---|---|
| `:id` | yes | the module | Must be unique among a static supervisor's children; ignored by `DynamicSupervisor`. |
| `:start` | yes | — | `{module, function, args}` MFA tuple. |
| `:restart` | no | `:permanent` | `:permanent` / `:transient` / `:temporary` (see Supervisor section). |
| `:shutdown` | no | `5_000` (worker) / `:infinity` (supervisor) | |
| `:type` | no | `:worker` | `:worker` / `:supervisor`; controls the `:shutdown` default. |
| `:modules` | no | inferred from `:start` | Usually `[callback_module]`; `:dynamic` for `gen_event`. |
| `:significant` | no | `false` | Only valid for `:transient`/`:temporary` children (OTP 24+). |

### The child_spec/1 convention

Rather than hand-writing maps, a module should expose its own spec via a `child_spec/1` function. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-child_spec-1-function):

> "However, defining the child specification for each child as a map can be quite error prone, as we may change the `Counter` implementation and forget to update its specification. That's why Elixir allows you to pass a tuple with the module name and the `start_link` argument instead of the specification:
>
> ```
> children = [
>   {Counter, 0}
> ]
> ```
>
> The supervisor will then invoke `Counter.child_spec(0)` to retrieve a child specification. Now the `Counter` module is responsible for building its own specification, for example, we could write:
>
> ```
> def child_spec(arg) do
>   %{
>     id: Counter,
>     start: {Counter, :start_link, [arg]}
>   }
> end
> ```"

This is why `{Module, arg}` (which calls `Module.child_spec(arg)`) and a bare `Module` (which calls `Module.child_spec([])`) work in a children list — cross-reference the Supervisor section's "Child specifications and child_spec/1" table.

### How use GenServer / use Supervisor / use DynamicSupervisor generate child_spec/1

You usually do not write `child_spec/1` by hand. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-child_spec-1-function):

> "Luckily for us, `use GenServer` already defines a `Counter.child_spec/1` exactly like above, so you don't need to write the definition above yourself. If you want to customize the automatically generated `child_spec/1` function, you can pass the options directly to `use GenServer`:
>
> ```
> use GenServer, restart: :transient
> ```"

From [GenServer.html](https://hexdocs.pm/elixir/GenServer.html#module-how-to-supervise):

> "When you `use GenServer`, the `GenServer` module will set `@behaviour GenServer` and define a `child_spec/1` function, so your module can be used as a child in a supervision tree.
>
> `use GenServer` also accepts a list of options which configures the child specification and therefore how it runs under a supervisor. The generated `child_spec/1` can be customized with the following options:
>
> - `:id` - the child specification identifier, defaults to the current module
> - `:restart` - when the child should be restarted, defaults to `:permanent`
> - `:shutdown` - how to shut down the child, either immediately or by giving it time to shut down, defaults to `5_000`
>
> For example:
>
> ```
> use GenServer, restart: :transient, shutdown: 10_000
> ```"

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-module-based-supervisors):

> "When you `use Supervisor`, the `Supervisor` module will set `@behaviour Supervisor` and define a `child_spec/1` function, so your module can be used as a child in a supervision tree."

`use Supervisor` accepts `:id` and `:restart` (defaults `__MODULE__` and `:permanent`). `use DynamicSupervisor` likewise auto-defines `child_spec/1` (cross-reference the DynamicSupervisor section's "Starting a DynamicSupervisor" subsection). Defaults for `use GenServer`: `id: __MODULE__`, `restart: :permanent`, `shutdown: 5000`.

### When to define child_spec/1 manually

Define `child_spec/1` yourself when the init argument must influence the spec — the `use GenServer` default cannot, because it does not see `arg`. Typical reasons:

- the `:start` MFA args depend on `arg` (beyond the trivial `[arg]`),
- you want a computed `:id` (e.g. distinct ids per arg),
- you want `:type`, `:modules`, or `:significant` that the `use` options do not expose,
- the same module must run as both a `:worker` and a `:supervisor` in different trees.

```elixir
defmodule MyApp.ConnectionWorker do
  use GenServer

  # Override the generated child_spec/1 to compute the :id from the arg.
  def child_spec(connection_id) do
    %{
      id: {__MODULE__, connection_id},
      start: {__MODULE__, :start_link, [connection_id]},
      restart: :transient,
      shutdown: 10_000
    }
  end

  def start_link(connection_id), do: GenServer.start_link(__MODULE__, connection_id)
  # ...callbacks...
end
```

When overriding, return the full map — `use GenServer` will NOT merge into a hand-written `child_spec/1`. For a lighter override (one or two keys), use `Supervisor.child_spec/2` instead.

### Supervisor.child_spec/2 — overriding without editing the module

```elixir
@spec child_spec(child_spec() | module_spec(), child_spec_overrides()) :: child_spec()
```

From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#child_spec/2):

> "Builds and overrides a child specification.
>
> Similar to `start_link/2` and `init/2`, it expects a module, `{module, arg}`, or a child specification.
>
> If a two-element tuple in the shape of `{module, arg}` is given, the child specification is retrieved by calling `module.child_spec(arg)`.
>
> If a module is given, the child specification is retrieved by calling `module.child_spec([])`.
>
> After the child specification is retrieved, the fields on `overrides` are directly applied to the child spec. If `overrides` has keys that do not map to any child specification field, an error is raised."

> "This function is often used to set an `:id` option when the same module needs to be started multiple times in the supervision tree:
>
> ```
> Supervisor.child_spec({Agent, fn -> :ok end}, id: {Agent, 1})
> #=> %{id: {Agent, 1}, start: {Agent, :start_link, [fn -> :ok end]}}
> ```"

Use `Supervisor.child_spec/2` to start several children of the same module under one static supervisor (distinct `:id` each) or to override `:restart`/`:shutdown` per-child without touching the module. Cross-reference the Supervisor section's "Supervisor.child_spec/2 — overriding defaults" subsection above.

### :type and :shutdown defaults

The `:type` field is `:worker | :supervisor` and determines the default `:shutdown`. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#t:type/0):

> "`@type type() :: :worker | :supervisor` — Type of a supervised child. Whether the supervised child is a worker or a supervisor."

A child supervisor must be given `:infinity` shutdown so its subtree can terminate cleanly (cross-reference the Supervisor section's "Shutdown values" subsection). Setting a finite shutdown on a child supervisor is a race risk.

### :significant and automatic shutdown

`:significant` (OTP 24+, Elixir 1.12+) marks children that should count toward an `:auto_shutdown` supervisor. As quoted above, only `:transient` and `:temporary` children may be `:significant`. Cross-reference the Supervisor section's "Automatic shutdown" subsection, including the rule that an auto-shutdown supervisor must not be a `:permanent` child of another supervisor.

### Common mistakes

1. **Hand-writing the full child map when `{Module, arg}` suffices** — fix: rely on the module's `child_spec/1`; only write the map when the module cannot express the spec.
2. **Overriding `child_spec/1` and forgetting a required key** — fix: a manual `child_spec/1` must return at least `:id` and `:start`; `use GenServer` does not merge into it.
3. **Setting a finite `:shutdown` on a child supervisor** — fix: child supervisors need `:infinity` (or rely on the `:supervisor` type default).
4. **Marking a `:permanent` child `:significant`** — fix: only `:transient`/`:temporary` children may be significant.
5. **Passing unknown keys to `Supervisor.child_spec/2`** — fix: only child-spec fields are valid overrides; unknown keys raise.
6. **Expecting a custom `child_spec/1` to inherit `use GenServer` options** — fix: a hand-written `child_spec/1` replaces the generated one entirely.

## Supervision Trees

This section expands on the Supervisor section's "Building supervision trees" and "Restart intensity" subsections above. It covers tree design, nested supervisors, intensity across levels, scaling, and the application/release lifecycle.

### What a supervision tree is

From the Erlang [design_principles.html](https://www.erlang.org/doc/system/design_principles.html):

> "A basic concept in Erlang/OTP is the *supervision tree*. This is a process structuring model based on the idea of *workers* and *supervisors*:
>
> - Workers are processes that perform computations and other actual work.
> - Supervisors are processes that monitor workers. A supervisor can restart a worker if something goes wrong.
> - The supervision tree is a hierarchical arrangement of code into supervisors and workers, which makes it possible to design and program fault-tolerant software."

From the Erlang [sup_princ.html](https://www.erlang.org/doc/system/sup_princ.html):

> "A supervisor is responsible for starting, stopping, and monitoring its child processes. The basic idea of a supervisor is that it is to keep its child processes alive by restarting them when necessary.
>
> Which child processes to start and monitor is specified by a list of child specifications. The child processes are started in the order specified by this list, and are terminated in the reverse order."

So a tree is a hierarchy: one application → one top supervisor → nested supervisors and workers. Fault tolerance comes from isolating restarts to the smallest subtree that can recover.

### The top-level application supervisor

The top of the tree lives in `Application.start/2` (cross-reference the `## Application` section above). From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-module-based-supervisors):

> "A general guideline is to use the supervisor without a callback module only at the top of your supervision tree, generally in the `Application.start/2` callback. We recommend using module-based supervisors for any other supervisor in your application, so they can run as a child of another supervisor in the tree."

Canonical top-level tree (direct form, only at the top):

```elixir
defmodule MyApp.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      MyApp.WorkerSupervisor,
      {Registry, keys: :unique, name: MyApp.Registry},
      {DynamicSupervisor, name: MyApp.ConnectionSup, strategy: :one_for_one}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: MyApp.Supervisor)
  end
end
```

The PID returned by `Supervisor.start_link/2` here is the top supervisor PID returned from `start/2`.

### Nested (module-based) supervisors

Every supervisor below the top should be module-based (`use Supervisor`) so it can itself be a child. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-module-based-supervisors):

> "When you `use Supervisor`, the `Supervisor` module will set `@behaviour Supervisor` and define a `child_spec/1` function, so your module can be used as a child in a supervision tree."

```elixir
defmodule MyApp.WorkerSupervisor do
  use Supervisor

  def start_link(init_arg) do
    Supervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    children = [
      MyApp.DatabaseWorker,
      MyApp.CacheWorker
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end
```

Cross-reference the Supervisor section's "Starting a supervisor" subsection and the `## Child Specs` section above (module-based supervisors auto-define `child_spec/1`).

### Start and shutdown order

Children start left-to-right (in listed order); the whole tree terminates in reverse order. From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-start-and-shutdown):

> "When the supervisor starts, it traverses all child specifications and then starts each child in the order they are defined. This is done by calling the function defined under the `:start` key in the child specification and typically defaults to `start_link/1`."
>
> "The shutdown process happens in reverse order. When a supervisor shuts down, it terminates all children in the opposite order they are listed. The termination happens by sending a shutdown exit signal, via `Process.exit(child_pid, :shutdown)`, to the child process and then awaiting for a time interval for the child process to terminate. This interval defaults to 5000 milliseconds."

This reverse-order termination is why dependencies must be listed before their dependents, and why `:rest_for_one` terminates later children before earlier ones (cross-reference the Supervisor section's strategies table). At application shutdown, the Erlang application master drives this for the whole tree:

> "The application master stops the application by telling the top supervisor to shut down. The top supervisor tells all its child processes to shut down, and so on; the entire tree is terminated in reverse start order. The application master then calls the application callback function `stop/1`..." — https://www.erlang.org/doc/system/applications.html

### Restart intensity across tree levels

Restart intensity compounds down the tree: a child that crash-loops is restarted up to its own supervisor's limit, and if that supervisor gives up, its parent may restart it, multiplying the effective restart budget. Cross-reference the Supervisor section's "Restart intensity" subsection for `:max_restarts` (default `3`) and `:max_seconds` (default `5`). The Erlang defaults are stricter (`intensity` `1`, `period` `5` seconds).

From the Erlang [sup_princ.html](https://www.erlang.org/doc/system/sup_princ.html):

> "If more than `MaxR` number of restarts occur in the last `MaxT` seconds, the supervisor terminates all the child processes and then itself. The termination reason for the supervisor itself in that case will be `shutdown`.
>
> When the supervisor terminates, then the next higher-level supervisor takes some action. It either restarts the terminated supervisor or terminates itself.
>
> The intention of the restart mechanism is to prevent a situation where a process repeatedly dies for the same reason, only to be restarted again."

The key multi-level warning:

> "If your application has multiple levels of supervision, do not set the restart intensities to the same values on all levels. Keep in mind that the total number of restarts (before the top level supervisor gives up and terminates the application) will be the product of the intensity values of all the supervisors above the failing child process.
>
> For example, if the top level allows 10 restarts, and the next level also allows 10, a crashing child below that level will be restarted 100 times, which is probably excessive. Allowing at most 3 restarts for the top level supervisor might be a better choice in this case." — https://www.erlang.org/doc/system/sup_princ.html

Rule: lower the intensity as you go UP the tree, so the root is the most conservative and a runaway subtree takes down only itself, not the whole application. Keep `:max_restarts` conservative at every level by default; raise it only for a specific, understood failure mode.

### Strategy choices and tree depth

Strategy is per-supervisor (cross-reference the Supervisor section's "Supervision strategies"). Use the structure of the tree to bound blast radius:

| Pattern | Tree shape & strategy | Why |
|---|---|---|
| Independent workers | One supervisor, `:one_for_one` | A crash restarts only that worker. |
| Tightly coupled unit | One supervisor, `:one_for_all` | All peers restart together to re-establish invariants. |
| Dependency chain | `:rest_for_one`, least-dependent first | A failure restarts only what depends on it. |
| Many same-template workers | `DynamicSupervisor` (children on demand) | Unknown count; `:one_for_one` only. |
| Hot path / bottleneck | `PartitionSupervisor` over a `DynamicSupervisor` or `Registry` | Spread work across cores. |

Introduce a new nested supervisor (rather than a flat list) when:

- a subtree needs a different strategy or restart intensity from its siblings;
- a group of children should fail and restart as a unit independently of the rest;
- restart intensity must be bounded locally to avoid the multi-level product problem.

Keep trees shallow where possible: each extra level multiplies effective restart budgets and adds shutdown latency. But do not flatten a coupled group into `:one_for_all` of an unrelated supervisor just to save a level — prefer a dedicated nested supervisor so coupling is explicit.

### Transient/temporary workers and fault isolation

Not everything should be `:permanent` (the default). From [Supervisor.html](https://hexdocs.pm/elixir/Supervisor.html#module-restart-values-restart):

> "`:permanent` - the child process is always restarted.
> `:temporary` - the child process is never restarted, regardless of the supervision strategy: any termination (even abnormal) is considered successful.
> `:transient` - the child process is restarted only if it terminates abnormally, i.e., with an exit reason other than `:normal`, `:shutdown`, or `{:shutdown, term}`."

Use `:transient` for workers with a deliberate `{:stop, :normal, state}` or `{:stop, {:shutdown, _}, state}` finish — they complete and stay down instead of being auto-restarted. Use `:temporary` for workers that should never be restarted even on crash (e.g. one-shot jobs). Cross-reference the DynamicSupervisor section's "Dynamic child specs" subsection, which pairs `:transient`/`:temporary` with `terminate_child/2` or self-shutdown.

For demand-driven work units, `:auto_shutdown` supervisors with `:significant` children let a whole subtree retire cleanly when its work is done (cross-reference the `## Child Specs` `:significant` subsection and the Supervisor section's "Automatic shutdown"). From the Erlang [sup_princ.html](https://www.erlang.org/doc/system/sup_princ.html):

> "A supervisor can be configured to automatically shut itself down when significant children terminate... This is useful when a supervisor represents a work unit of cooperating children, as opposed to independent workers."

### PartitionSupervisor for horizontal scaling

A single `Registry`, `DynamicSupervisor`, or `GenServer` can become a bottleneck. `PartitionSupervisor` (Elixir 1.14+) starts N partitions of the same child and routes by key. From [PartitionSupervisor.html](https://hexdocs.pm/elixir/PartitionSupervisor.html):

> "A supervisor that starts multiple partitions of the same child.
>
> Certain processes may become bottlenecks in large systems. If those processes can have their state trivially partitioned, in a way there is no dependency between them, then they can use the `PartitionSupervisor` to create multiple isolated and independent partitions.
>
> Once the `PartitionSupervisor` starts, you can dispatch to its children using `{:via, PartitionSupervisor, {name, key}}`, where `name` is the name of the `PartitionSupervisor` and key is used for routing.
>
> This module was introduced in Elixir v1.14.0."

Partitioning a `DynamicSupervisor` across cores (cross-reference the DynamicSupervisor section's "Scalability and partitioning"):

```elixir
children = [
  {PartitionSupervisor,
   child_spec: DynamicSupervisor,
   name: MyApp.DynamicSupervisors}
]

Supervisor.start_link(children, strategy: :one_for_one)
```

```elixir
DynamicSupervisor.start_child(
  {:via, PartitionSupervisor, {MyApp.DynamicSupervisors, self()}},
  {Agent, fn -> %{} end}
)
```

> "In the code above, we start a partition supervisor that will by default start a dynamic supervisor for each core in your machine." — https://hexdocs.pm/elixir/PartitionSupervisor.html

`:partitions` defaults to `System.schedulers_online/0`; `:strategy` defaults to `:one_for_one`. Routing is `rem(abs(key), partitions)` for integers, else `:erlang.phash2(key, partitions)`:

> "The particular routing may change in the future, and therefore must not be relied on. If you want to retrieve a particular PID for a certain key, you can use `GenServer.whereis({:via, PartitionSupervisor, {name, key}})`." — https://hexdocs.pm/elixir/PartitionSupervisor.html

Address partitions through the `:via` tuple, never by reaching into the internal child ids.

### Releases vs applications (tree lifecycle)

A supervision tree's definition is identical whether the app is started by `iex -S mix` or by a release boot script; what changes is packaging and startup ordering. From the Erlang [release_structure.html](https://www.erlang.org/doc/system/release_structure.html):

> "When starting Erlang/OTP using the boot script, all applications from the `.rel` file are automatically loaded and started."

`mix release` packages applications and the Erlang VM into a self-contained unit and gives explicit control over application start order:

> "Releases also give you explicit control over how each application is started and in which order." — https://hexdocs.pm/elixir/Application.html#module-tooling

In practice: design each application's tree to be self-contained (a well-named top supervisor), and let the release's dependency order compose them at the system level.

### Common mistakes

1. **Flat `:one_for_all` over unrelated workers** — fix: split into a nested supervisor with `:one_for_one`, or use `:rest_for_one`, so coupling is explicit and blast radius is bounded.
2. **Same `:max_restarts` on every level** — fix: the effective budget is the product across levels; lower intensity toward the root.
3. **Heavy work in `Application.start/2`** — fix: build the tree only; do work in child `init/1`/`handle_continue/2`.
4. **Direct `Supervisor.start_link/2` form below the top** — fix: use module-based (`use Supervisor`) supervisors for any supervisor that is itself a child.
5. **Treating the root supervisor as infinitely tolerant** — fix: when the root supervisor hits its restart limit it terminates with `:shutdown`; the whole application goes down.
6. **Listing a dependency after its dependent** — fix: children start left-to-right and terminate right-to-left; order dependencies first.
7. **Reaching into a `PartitionSupervisor`'s internal child ids** — fix: address partitions only via `{:via, PartitionSupervisor, {name, key}}`; routing is an implementation detail.
8. **Defaulting every worker to `:permanent`** — fix: use `:transient`/`:temporary` for workers with a deliberate end or one-shot semantics.

## Review checklist

- [ ] GenServer is used to model runtime properties (state, concurrency, failure), not code organization.
- [ ] `@impl true` is present on every callback.
- [ ] Callback return tuples match the exact contract (especially `:continue`, `:noreply`, and stop forms).
- [ ] `init/1` does not perform slow/blocking work unless the supervisor must wait.
- [ ] Slow init work is moved to `handle_continue/2` via `{:ok, state, {:continue, term}}`.
- [ ] `handle_info/2` is implemented explicitly when the process receives raw messages.
- [ ] `{:noreply, state}` from `handle_call/3` is paired with a guaranteed `GenServer.reply/2` path.
- [ ] `timeout: 0` is not used as a substitute for `{:continue, term}`.
- [ ] Dynamic names use `Registry` or another `:via` module, not `String.to_atom/1`.
- [ ] `start_link/3` is used for supervised processes; `start/3` is only for unlinked/ad-hoc use.
- [ ] Client API is separated from callbacks, all in one module.
- [ ] `terminate/2` is not relied on for critical cleanup.
- [ ] `:sys` debugging handlers are disabled in production (`:sys.no_debug/1`).

## Implementation checklist

- [ ] Decide whether a process is actually needed; prefer pure functions when possible.
- [ ] Choose `GenServer.start_link/3` for supervised workers.
- [ ] Define the client API (functions calling `call/3`, `cast/2`, etc.) at the top of the module.
- [ ] Define `init/1` to return `{:ok, state}` or `{:ok, state, {:continue, term}}` for slow setup.
- [ ] Implement `handle_call/3`, `handle_cast/2`, and/or `handle_info/2` as required.
- [ ] Implement `handle_continue/2` whenever `{:continue, _}` is returned.
- [ ] Add `@spec` for public client functions and callbacks where helpful.
- [ ] Use `{:via, Registry, ...}` for dynamic process names.
- [ ] Add `format_status/1` if the state contains secrets.
- [ ] Disable `:sys` tracing/statistics before shipping.

## Validation hooks

- `mix compile` — catches missing `@impl true` warnings and bad callback arities.
- `mix dialyzer` — verifies `@spec` contracts and GenServer callback return types.
- `mix credo` — flags common process anti-patterns (community tool, not official).
- Unit tests with `ExUnit` — start the GenServer under a `Supervisor` in tests or use `start_supervised!/1`.
- Note explicitly: markdown documentation cannot be compiled; validate code blocks by reading them against the official callback contracts.

## Examples

### Canonical Stack GenServer

```elixir
defmodule Stack do
  use GenServer

  # Client API
  def start_link(default) when is_list(default) do
    GenServer.start_link(__MODULE__, default, name: __MODULE__)
  end

  def push(element) do
    GenServer.cast(__MODULE__, {:push, element})
  end

  def pop do
    GenServer.call(__MODULE__, :pop)
  end

  # Server callbacks
  @impl true
  def init(stack) do
    {:ok, stack}
  end

  @impl true
  def handle_call(:pop, _from, [head | tail]) do
    {:reply, head, tail}
  end

  @impl true
  def handle_cast({:push, element}, state) do
    {:noreply, [element | state]}
  end
end
```

### handle_continue for slow init

```elixir
defmodule ConnectionWorker do
  use GenServer

  def start_link(arg), do: GenServer.start_link(__MODULE__, arg)

  @impl true
  def init(arg) do
    # Return immediately so the supervisor can continue.
    {:ok, %{arg: arg, conn: nil}, {:continue, :connect}}
  end

  @impl true
  def handle_continue(:connect, %{arg: arg} = state) do
    case SomeDatabase.connect(arg) do
      {:ok, conn} ->
        {:noreply, %{state | conn: conn}}

      {:error, reason} ->
        {:stop, reason, state}
    end
  end
end
```

### Timeout/idle-driven shutdown

Note: a simple counter is not a real reason to use a GenServer; this only demonstrates the timeout mechanism.

```elixir
defmodule IdleCounter do
  use GenServer

  @idle_timeout 30_000

  def start_link(_), do: GenServer.start_link(__MODULE__, 0)

  def increment(pid), do: GenServer.cast(pid, :increment)
  def count(pid), do: GenServer.call(pid, :count)

  @impl true
  def init(count) do
    {:ok, count, @idle_timeout}
  end

  @impl true
  def handle_cast(:increment, count) do
    {:noreply, count + 1, @idle_timeout}
  end

  @impl true
  def handle_call(:count, _from, count) do
    {:reply, count, count, @idle_timeout}
  end

  @impl true
  def handle_info(:timeout, state) do
    {:stop, :normal, state}
  end
end
```

### Periodic timer with handle_info

```elixir
defmodule Ticker do
  use GenServer

  @interval 1_000

  def start_link(_), do: GenServer.start_link(__MODULE__, nil)

  @impl true
  def init(nil) do
    schedule_tick()
    {:ok, 0}
  end

  @impl true
  def handle_info(:tick, count) do
    schedule_tick()
    {:noreply, count + 1}
  end

  defp schedule_tick do
    Process.send_after(self(), :tick, @interval)
  end
end
```

### Registry :via naming

```elixir
{:ok, _} = Registry.start_link(keys: :unique, name: MyApp.Registry)

name = {:via, Registry, {MyApp.Registry, "user:42"}}
{:ok, pid} = GenServer.start_link(MyServer, arg, name: name)

# Later, any process can send to the same logical name:
GenServer.cast({:via, Registry, {MyApp.Registry, "user:42"}}, :poke)
```

### noreply + GenServer.reply/2

```elixir
defmodule AsyncWorker do
  use GenServer

  def start_link(_), do: GenServer.start_link(__MODULE__, nil)

  def submit(pid, job) do
    GenServer.call(pid, {:submit, job}, :infinity)
  end

  @impl true
  def handle_call({:submit, job}, from, state) do
    Task.start(fn ->
      result = do_work(job)
      GenServer.reply(from, result)
    end)

    {:noreply, state}
  end

  defp do_work(job) do
    # ...
  end
end
```

## Common mistakes

1. **GenServer for pure functions/code organization** (calculator anti-pattern) — fix: just write functions.
2. **Slow/blocking work in `init/1`** — fix: `{:ok, state, {:continue, ...}}` + `handle_continue/2`.
3. **Blocking work in `handle_call/3`** (serializes all messages, hits caller timeout) — fix: `{:noreply, state}` + spawn `Task` + `GenServer.reply/2`, or move to `cast`.
4. **Forgetting `handle_info/2`** (default silently logs and drops `:DOWN`/timers/`:EXIT`) — fix: implement explicitly.
5. **Forgetting to return the new state** (state is immutable; `state = Map.put(...)` without returning it silently keeps old state).
6. **Returning `{:noreply, state}` without ever replying** — caller blocks forever. Fix: `{:reply, ...}` or `GenServer.reply(from, reply)`.
7. **Dynamic atom names via `String.to_atom/1`** (atom leak) — fix: `Registry` via `:via`.
8. **Calling your own `receive` inside a callback** — from [GenServer.html](https://hexdocs.pm/elixir/GenServer.html): "you should never call your own 'receive' inside the GenServer callbacks as doing so will cause the GenServer to misbehave." Fix: route messages through `handle_info/2`.
9. **Process dictionary (`Process.put/get`) for state** — invisible to `:sys`, bypasses the callback contract. Fix: use GenServer state.
10. **Raw `Process.spawn` for stateful workers** — no OTP special-process protocol, no `:sys`/debug/`code_change`, poor supervision fit. Fix: GenServer or another behaviour.
11. **Wrong stop reason** — anything other than `:normal`/`:shutdown`/`{:shutdown, term}` implies error (logger report, transient restart, linked exit).
12. **Relying on `terminate/2` for critical cleanup** — not guaranteed. Fix: monitors for cleanup; ports/sockets auto-close on exit.
13. **Using `timeout: 0` as "do this immediately"** — not guaranteed. Fix: `{:continue, ...}`.
14. **Aggressive hibernation** — from [GenServer.html](https://hexdocs.pm/elixir/GenServer.html): "Hibernating should not be used aggressively as too much time could be spent garbage collecting." Use only for long-idle large-heap processes.
15. **Leaving `:debug` options on in production** — performance damage. Fix: `:sys.no_debug/1`.

## Strict vs contextual guidance

### Strict

- A GenServer MUST be used to model runtime properties (mutable state, concurrency, failures), never for code organization.
- Callback return tuples MUST match the exact contracts documented above.
- `@impl true` MUST be used on every callback implementation.
- You MUST NOT call your own `receive` inside GenServer callbacks.
- `String.to_atom/1` MUST NOT be used to generate dynamic process names.
- `terminate/2` MUST NOT be relied on for critical cleanup.
- `timeout: 0` MUST NOT be treated as guaranteed immediate execution.

### Conventions (not enforced by the compiler/OTP)

- Keep client API functions and callbacks in the same module.
- Prefer `start_link/3` for supervised workers and `start/3` only for unlinked/ad-hoc processes.
- Prefer `{:continue, term}` + `handle_continue/2` over blocking `init/1`.
- Prefer `{:via, Registry, ...}` for dynamic local names.
- Use `format_status/1` to redact secrets from `:sys.get_status/1` and crash logs.
- Disable `:sys` debug handlers once debugging is complete.

### Contextual tradeoffs

- Use `handle_call/3` for operations whose outcome the caller needs synchronously; use `handle_cast/2` for fire-and-forget side effects; use `handle_info/2` for out-of-band messages and timers.
- Use `:hibernate` only for processes with large heaps that will be idle for long periods.
- Use `:sys` tracing in development and tests; remove or disable it in production.

## Policy decisions for individual repos

- Whether to require `@spec` on all public client functions and callbacks.
- Whether to require `handle_info/2` even when no raw messages are expected (defensive default).
- Whether dynamic process names must use `Registry` or a specific `:via` module.
- Lint stack: enable `mix format --check-formatted`, `mix credo`, and/or `mix dialyzer`.
- Whether `:sys` debug options are permitted in production code at all.
- Naming policy for supervised GenServers (`__MODULE__` name vs explicit atoms).
- Restart/shutdown policy for GenServer workers (`:permanent` vs `:transient` vs `:temporary`).

## Related docs

- `docs/elixir/naming-conventions.md` — identifier casing and conventions.
- Cross-reference the repo's existing `docs/rust/style-formatting.md` for formatting conventions and document tone.

## Related skills

- None defined yet.
