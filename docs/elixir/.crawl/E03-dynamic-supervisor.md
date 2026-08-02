# Crawl: hexdocs.pm/elixir/DynamicSupervisor.html
- seed_url: https://hexdocs.pm/elixir/DynamicSupervisor.html
- canonical_url: https://elixir.hexdocs.pm/DynamicSupervisor.html
- family: Elixir core module
- fetch: 200
- elixir_version: Elixir v1.20.2 (ExDoc v0.40.3)
- feeds_docs: otp-supervision.md

## Purpose

A supervisor optimized to only start children dynamically. The `Supervisor`
module was designed to handle mostly static children that are started in the
given order when the supervisor starts. A `DynamicSupervisor` starts with no
children. Instead, children are started on demand via `start_child/2` and there
is no ordering between children. This allows the `DynamicSupervisor` to hold
millions of children by using efficient data structures and to execute certain
operations, such as shutting down, concurrently.

Started with no children and often with a name:

```elixir
children = [
  {DynamicSupervisor, name: MyApp.DynamicSupervisor, strategy: :one_for_one}
]

Supervisor.start_link(children, strategy: :one_for_one)
```

Once running, children are started on demand via `start_child/2` with a child
specification. `count_children/1` returns
`%{active: 2, specs: 2, supervisors: 0, workers: 2}`.

### Scalability and partitioning

The `DynamicSupervisor` is a single process responsible for starting other
processes. In some applications it may become a bottleneck. To address this,
start multiple instances via `PartitionSupervisor` and pick a "random" instance
to start the child on:

```elixir
children = [
  {PartitionSupervisor, child_spec: DynamicSupervisor, name: MyApp.DynamicSupervisors}
]
```

Then call through the partition supervisor, using `self()` as the routing key:

```elixir
DynamicSupervisor.start_child(
  {:via, PartitionSupervisor, {MyApp.DynamicSupervisors, self()}},
  {Counter, 0}
)
```

`PartitionSupervisor` by default starts a dynamic supervisor for each core.

### Module-based supervisors

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

A `@doc` annotation immediately preceding `use DynamicSupervisor` will be
attached to the generated `child_spec/1` function. `use DynamicSupervisor`
sets `@behaviour DynamicSupervisor` and defines a `child_spec/1` function so
the module can be used as a child in a supervision tree.

### Name registration

A supervisor is bound to the same name registration rules as a `GenServer`.

## Key functions + :extra_arguments

### Types

```elixir
@type init_option() ::
  {:strategy, strategy()}
  | {:max_restarts, non_neg_integer()}
  | {:max_seconds, pos_integer()}
  | {:max_children, non_neg_integer() | :infinity}
  | {:extra_arguments, [term()]}

@type on_start_child() ::
  {:ok, pid()}
  | {:ok, pid(), info :: term()}
  | :ignore
  | {:error, {:already_started, pid()} | :max_children | term()}

@type strategy() :: :one_for_one

@type sup_flags() :: %{
  strategy: strategy(),
  intensity: non_neg_integer(),
  period: pos_integer(),
  max_children: non_neg_integer() | :infinity,
  extra_arguments: [term()]
}
```

### Callback

```elixir
@callback init(init_arg :: term()) :: {:ok, sup_flags()} | :ignore
```

Callback invoked to start the supervisor and during hot code upgrades.
Developers typically invoke `DynamicSupervisor.init/1` at the end of their
`init` callback to return the proper supervision flags.

### Functions

| function | since | spec / signature | notes |
|----------|-------|------------------|-------|
| `child_spec(options)` | 1.6.1 | `@spec child_spec([init_option() \| GenServer.option()]) :: Supervisor.child_spec()` | Returns a spec to start a dynamic supervisor under a supervisor. Accepts same options as `start_link/1`. |
| `count_children(supervisor)` | 1.6.0 | `@spec count_children(Supervisor.supervisor()) :: %{specs: non_neg_integer(), active: non_neg_integer(), supervisors: non_neg_integer(), workers: non_neg_integer()}` | Map keys: `:specs` (number of children), `:active` (running children), `:supervisors` (count of all supervisors whether or not alive), `:workers` (count of all workers whether or not alive). |
| `init(options)` | 1.6.0 | `@spec init([init_option()]) :: {:ok, sup_flags()}` | Receives options that initialize a dynamic supervisor. Typically invoked at end of module-based `init/1` callback. Accepts same options as `start_link/1` except `:name`. |
| `start_child(supervisor, child_spec)` | 1.6.0 | `@spec start_child(Supervisor.supervisor(), Supervisor.child_spec() \| {module(), term()} \| module() \| :supervisor.child_spec()) :: on_start_child()` | Dynamically adds a child spec and starts that child. Blocks the DynamicSupervisor until the child initializes. Returns `{:error, :max_children}` if `:max_children` exceeded. |
| `start_link(options)` | 1.6.0 | `@spec start_link([init_option() \| GenServer.option()]) :: Supervisor.on_start()` | Starts a supervisor with the given options. Typically invoked when using a `DynamicSupervisor` as a child of another supervisor. |
| `start_link(module, init_arg, opts \\ [])` | 1.6.0 | `@spec start_link(module(), term(), [GenServer.option()]) :: Supervisor.on_start()` | Starts a module-based supervisor. The `init/1` callback is invoked with `init_arg`; must return a supervisor spec (use `init/1`). |
| `stop(supervisor, reason \\ :normal, timeout \\ :infinity)` | 1.7.0 | `@spec stop(Supervisor.supervisor(), reason :: term(), timeout()) :: :ok` | Synchronously stops the supervisor. Returns `:ok` if it terminates with the given reason; otherwise the call exits. Keeps OTP error-reporting semantics. |
| `terminate_child(supervisor, pid)` | 1.6.0 | `@spec terminate_child(Supervisor.supervisor(), pid()) :: :ok \| {:error, :not_found}` | Terminates the given child identified by **pid**. Blocks until the child terminates. Returns `{:error, :not_found}` if no process with the given PID. |
| `which_children(supervisor)` | 1.6.0 | `@spec which_children(Supervisor.supervisor()) :: [{:undefined, pid() \| :restarting, :worker \| :supervisor, [module()] \| :dynamic}]` | Returns a list of tuples. `id` is always `:undefined` for dynamic supervisors. `child` is the PID or `:restarting`. Calling with a large number of children under low memory can bring the system down (OOM). |

### `start_link/1` options (verbatim semantics)

| option | default | notes |
|--------|---------|-------|
| `:name` | — | Registers the supervisor under the given name (same rules as GenServer "Name registration"). |
| `:strategy` | — | The only supported value is `:one_for_one` (no other child terminated if a child terminates). |
| `:max_restarts` | `3` | Maximum number of restarts allowed in a time frame. |
| `:max_seconds` | `5` | The time frame in which `:max_restarts` applies. |
| `:max_children` | `:infinity` | Maximum amount of children running under this supervisor at the same time. When exceeded, `start_child/2` returns `{:error, :max_children}`. |
| `:extra_arguments` | `[]` (empty list) | Arguments that are **prepended** to the arguments specified in the child spec given to `start_child/2`. |

### `:extra_arguments` detail

`:extra_arguments` are prepended to the arguments specified in the child spec
given to `start_child/2`. This is the DynamicSupervisor analogue of the
`simple_one_for_one` "append ExtraArgs to start args via `apply(M,F,A++ExtraArgs)`"
mechanism in the Erlang `supervisor` module — but expressed as a supervisor
flag rather than a `start_child/2` argument-list convention.

## one_for_one only; :id ignored

### `:one_for_one` is the only supported strategy

```elixir
@type strategy() :: :one_for_one
```

> `:strategy` — the restart strategy option. The only supported value is
> `:one_for_one` which means that no other child is terminated if a child
> process terminates.

There is no `:one_for_all`, `:rest_for_one`, or `:simple_one_for_one` option.
The `:one_for_one` semantics (only the terminated child is restarted) are
inherent because all children are dynamically added instances with no ordering.

### `:id` ignored semantics

> Note that while the `:id` field is still required in the spec, the value is
> ignored and therefore does not need to be unique. Unlike `Supervisor`, this
> module does not return `{:error, {:already_started, pid}}` for child specs
> given with the same id. `{:error, {:already_started, pid}}` is returned
> however if a duplicate name is used when using name registration.

Consequences:
- The `:id` field is **required** in the child spec but its value is **ignored**.
- Multiple children may share the same `:id` value without conflict.
- `{:error, {:already_started, pid}}` is NOT returned for duplicate child-spec
  ids (unlike `Supervisor`).
- `{:error, {:already_started, pid}}` IS returned if a duplicate **name** is
  used when using name registration (i.e. the child registers a name that
  already exists).
- `which_children/1` always returns `id = :undefined` for dynamic supervisors.

## Migration from simple_one_for_one

`DynamicSupervisor` is the Elixir-native replacement for the Erlang
`supervisor` `:simple_one_for_one` strategy. The migration mapping:

| Erlang `:simple_one_for_one` | Elixir `DynamicSupervisor` |
|------------------------------|----------------------------|
| `SupFlags = #{strategy => simple_one_for_one, ...}` | `DynamicSupervisor.init(strategy: :one_for_one, ...)` (strategy is always `:one_for_one`) |
| `init/1` returns exactly one child spec; no child started during init | `DynamicSupervisor` starts with no children; children started via `start_child/2` |
| `start_child(SupRef, ExtraArgs)` — appends `ExtraArgs` to start args via `apply(M,F,A++ExtraArgs)` | `start_child(supervisor, child_spec)` — child spec given explicitly; `:extra_arguments` supervisor flag prepends args |
| `terminate_child(Sup, Pid)` — Id must be pid (using child spec id returns `{error,simple_one_for_one}`) | `terminate_child(supervisor, pid)` — takes pid; returns `:ok \| {:error, :not_found}` |
| `delete_child/2` returns `{error,simple_one_for_one}` (invalid) | No `delete_child` API (children are not identified by id) |
| `restart_child/2` returns `{error,simple_one_for_one}` (invalid) | No `restart_child` API (restart is automatic per `:one_for_one`) |
| `which_children/1` returns `Id = undefined` | `which_children/1` returns `id = :undefined` |
| `:max_children` not a native simple_one_for_one flag | `:max_children` is a first-class `init_option()` (default `:infinity`) |

Key migration notes:
- The `:extra_arguments` flag replaces the `start_child/2` "append ExtraArgs"
  convention: instead of passing extra args at each `start_child` call, they
  are configured once on the supervisor and prepended to every child's start
  args.
- `:simple_one_for_one` is NOT deprecated in OTP 29.0.2 (per
  `docs/beam/supervision.md` crawl 13), but `DynamicSupervisor` is the
  preferred Elixir API.
- `DynamicSupervisor` can hold millions of children via efficient data
  structures and concurrent shutdown — a scalability advantage over the
  Erlang `simple_one_for_one` supervisor.

## DynamicSupervisor ↔ BEAM simple_one_for_one mapping (→ docs/beam/supervision.md)

`docs/beam/supervision.md` (crawl 13: `docs/beam/.crawl/13-supervisor-module.md`)
documents the Erlang `supervisor` module including `:simple_one_for_one`.
Cross-reference mapping:

| `docs/beam/supervision.md` concept | `DynamicSupervisor` equivalent |
|------------------------------------|--------------------------------|
| `strategy() :: one_for_all \| one_for_one \| rest_for_one \| simple_one_for_one` (line 109) | `@type strategy() :: :one_for_one` — only `:one_for_one` exposed |
| `simple_one_for_one` caveats (lines 116–123): `delete_child/2` and `restart_child/2` return `{error,simple_one_for_one}`; `terminate_child/2` requires pid; `init/1` one child spec; all started via `start_child/2` | `DynamicSupervisor` has no `delete_child`/`restart_child` API at all; `terminate_child/2` takes pid; starts with no children; all via `start_child/2` |
| `sup_flags()` map: `strategy`, `intensity`, `period`, `hibernate_after`, `auto_shutdown` (lines 25–32) | `sup_flags()`: `strategy`, `intensity`, `period`, `max_children`, `extra_arguments` — no `hibernate_after`, no `auto_shutdown` |
| `start_child/2` for simple_one_for_one: `start_child(SupRef, ExtraArgs)` appends to start args (line 275) | `start_child(supervisor, child_spec)` — full child spec; `:extra_arguments` prepended |
| `terminate_child/2`: `Id :: pid() \| child_id()`, `Error :: not_found \| simple_one_for_one` (line 276) | `terminate_child(supervisor, pid) :: :ok \| {:error, :not_found}` — pid only |
| `which_children/1`: `Id=undefined` for simple_one_for_one (line 279) | `which_children/1`: `id` always `:undefined` |
| "Dynamic children lost on supervisor restart" (line 246) | Same semantics apply: dynamically added children are lost if the DynamicSupervisor dies and is recreated |
| `simple_one_for_one` never hibernates by default (line 226) | No `hibernate_after` option exposed |

**Net:** `DynamicSupervisor` is a constrained, Elixir-ergonomic wrapper over
the `:simple_one_for_one` capability. It drops `delete_child`/`restart_child`
(which were already errors under `simple_one_for_one`), promotes
`:max_children` and `:extra_arguments` to first-class options, and exposes only
the `:one_for_one` strategy. The underlying BEAM supervision semantics
(restart intensity/period, child termination, dynamic children lost on
supervisor restart) are identical and documented in `docs/beam/supervision.md`.

## Strict rules

1. **Strategy is always `:one_for_one`.** No other strategy is accepted. The
   `:one_for_one` semantics are inherent because all children are dynamic with
   no ordering.
2. **`:id` is required but ignored.** The `:id` field must be present in the
   child spec but its value is ignored and need not be unique. Duplicate ids do
   not produce `{:error, {:already_started, pid}}`.
3. **`{:error, {:already_started, pid}}` is returned only for duplicate
   names** (when using name registration), never for duplicate child-spec ids.
4. **`terminate_child/2` takes a pid, not an id.** Returns `:ok` or
   `{:error, :not_found}`. There is no `delete_child` or `restart_child` API.
5. **`start_child/2` blocks** the DynamicSupervisor until the child
   initializes. For many dynamic processes, use `PartitionSupervisor` to split
   work across multiple supervisor instances.
6. **`:max_children` enforces a cap.** When exceeded, `start_child/2` returns
   `{:error, :max_children}`. Default `:infinity`.
7. **`:extra_arguments` are prepended** to the child spec's start args. They
   are configured once on the supervisor, not per `start_child` call.
8. **`which_children/1` can OOM** under low memory with many children — use
   `count_children/1` when only counts are needed.
9. **Dynamic children are lost on supervisor restart** — same as BEAM
   `simple_one_for_one`.
10. **`init/1` callback** must return `{:ok, sup_flags()}` or `:ignore`. Use
    `DynamicSupervisor.init/1` to build the flags. `:name` is NOT accepted by
    `init/1` (only by `start_link`).

## Verbatim quotes

1. > "A supervisor optimized to only start children dynamically. The Supervisor
   > module was designed to handle mostly static children that are started in
   > the given order when the supervisor starts. A DynamicSupervisor starts
   > with no children. Instead, children are started on demand via start_child/2
   > and there is no ordering between children. This allows the
   > DynamicSupervisor to hold millions of children by using efficient data
   > structures and to execute certain operations, such as shutting down,
   > concurrently."

2. > "Unlike Supervisor, this module ignores the child spec ids, so
   > {:error, {:already_started, pid}} is not returned for child specs given
   > with the same id. {:error, {:already_started, pid}} is returned however if
   > a duplicate name is used when using name registration."

3. > "Note that while the :id field is still required in the spec, the value is
   > ignored and therefore does not need to be unique. Unlike Supervisor, this
   > module does not return {:error, {:already_started, pid}} for child specs
   > given with the same id. {:error, {:already_started, pid}} is returned
   > however if a duplicate name is used when using name registration."

4. > ":strategy - the restart strategy option. The only supported value is
   > :one_for_one which means that no other child is terminated if a child
   > process terminates."

5. > ":extra_arguments - arguments that are prepended to the arguments
   > specified in the child spec given to start_child/2. Defaults to an empty
   > list."

6. > "This function will block the DynamicSupervisor until the child
   > initializes. When starting too many processes dynamically, you may want to
   > use a PartitionSupervisor to split the work across multiple processes."

7. > "If the supervisor already has N children in a way that N exceeds the
   > amount of :max_children set on the supervisor initialization (see init/1),
   > then this function returns {:error, :max_children}."

8. > "Terminates the given child identified by pid. This function will block
   > the DynamicSupervisor until the child terminates, which may take an
   > arbitrary amount of time if the child is trapping exits and implements its
   > own terminate callback. For this reason, it is often better to ask the
   > child process itself to terminate, often by declaring in its child spec it
   > has a restart strategy of :transient (or :temporary) and then sending it a
   > message to stop with reason :shutdown. If successful, this function returns
   > :ok. If there is no process with the given PID, this function returns
   > {:error, :not_found}."

9. > "Note that calling this function when supervising a large number of
   > children under low memory conditions can bring the system down due to an
   > out of memory error." (re `which_children/1`)

10. > "id - it is always :undefined for dynamic supervisors" (re
    `which_children/1` return tuples)

11. > "Callback invoked to start the supervisor and during hot code upgrades.
    > Developers typically invoke DynamicSupervisor.init/1 at the end of their
    > init callback to return the proper supervision flags."

12. > "It accepts the same options as start_link/1 (except for :name) and it
    > returns a tuple containing the supervisor options." (re `init/1`)

## Version notes

- Page reports **Elixir v1.20.2**, built with **ExDoc v0.40.3**.
- `child_spec/1` since 1.6.1; all other public functions since 1.6.0;
  `stop/3` since 1.7.0.
- `DynamicSupervisor` was introduced in Elixir 1.6.0 as the Elixir-native
  successor to the `:simple_one_for_one` supervisor strategy.
- No deprecation notices on this page as of v1.20.2.
- Canonical URL redirects: `hexdocs.pm/elixir/DynamicSupervisor.html` →
  `elixir.hexdocs.pm/DynamicSupervisor.html` (HTTP 200, no further redirect).
- Corpus pins Elixir v1.20.2 / OTP 29; this crawl confirms the page is current
  with no post-v1.20.2 drift detected in the extracted API surface.

## Discovered links

### Relevant (crawl later)

- `https://hexdocs.pm/elixir/Supervisor.html` — parent `Supervisor` module;
  child specs, strategies, `child_spec/2`, auto_shutdown. (P0 item 2; crawl as
  E02 if not already done.)
- `https://hexdocs.pm/elixir/PartitionSupervisor.html` — partitioning for
  scalable dynamic supervision. (P0 item 5.)
- `https://hexdocs.pm/elixir/GenServer.html` — name registration rules shared
  with DynamicSupervisor; `:via` registration. (P0 item 1; crawl as E01 if not
  already done.)
- `https://hexdocs.pm/elixir/Registry.html` — `:via` registration backing for
  named dynamic children. (P0 item 4.)

### Skipped

- `https://hexdocs.pm/elixir/Kernel.html` — general-purpose, not supervision-specific.
- ExDoc UI / search / settings / "View llms.txt" / "Download ePub version" /
  "Go to package docs" — navigational chrome, no content.
- "View Source" link to GitHub source — source code, not docs.
