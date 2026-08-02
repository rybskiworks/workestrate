# Crawl: hexdocs.pm/elixir/Registry.html
- seed_url: https://hexdocs.pm/elixir/Registry.html
- canonical_url: https://elixir.hexdocs.pm/Registry.html
- family: Elixir core module
- fetch: 200 (HTTP/2 via curl -sL; redirected hexdocs.pm/elixir/Registry.html -> elixir.hexdocs.pm/Registry.html)
- elixir_version: Elixir v1.20.2 (page title: "Registry — Elixir v1.20.2")
- feeds_docs: otp-supervision.md, concurrency-processes.md

## Purpose
Registry is a local, decentralized and scalable key-value process storage. It
allows developers to look up one or more processes with a given key. Each entry
is associated to the process that registered the key; if that process crashes,
its keys are automatically removed from the registry. All key comparisons use
the match operation (`===/2`).

It is used for: name lookups (via the `:via` option), storing properties, custom
dispatching rules, or a local non-distributed PubSub. It may be transparently
partitioned for highly concurrent environments with thousands/millions of
entries.

> "A local, decentralized and scalable key-value process storage. It allows
> developers to lookup one or more processes with a given key."

## :unique / :duplicate + partitioning
The `:keys` start option selects the registry kind:
- `:unique` — a key points to 0 or 1 process. Required for `:via` registration.
- `:duplicate` — a single key may point to any number of processes; multiple
  registrations from the same process under the same key are allowed.
- `{:duplicate, :pid}` (default duplicate) — partition by PID; groups all
  entries from the same process together. Best when keys have many entries
  (e.g. one topic with many subscribers).
- `{:duplicate, :key}` — partition by key; key-based lookups check only a
  single partition. Best when entries are spread across many keys (e.g. many
  topics with few subscribers each). Uses `ordered_set` ETS internally instead
  of `duplicate_bag`, so match specs referencing `:"$_"` behave differently;
  use named variables (`:"$1"`, `:"$2"`, ...).

Partitioning is set via `:partitions` (default 1). A good default for intensive
workloads is `System.schedulers_online()`.

Type: `@type keys() :: :unique | :duplicate | {:duplicate, :key} | {:duplicate, :pid}`

ETS note: the registry uses one ETS table plus two ETS tables per partition.

## Key functions
- `start_link(options)` (since 1.5.0) — `@spec start_link([start_option()]) :: {:ok, pid()} | {:error, term()}`.
  Starts the registry as a supervisor process. Required options: `:keys`, `:name`.
  Optional: `:partitions` (default 1), `:listeners` (named processes notified of
  register/unregister events via `listener_message/0`), `:meta` (keyword list of
  metadata). `child_spec/1` returns a `Supervisor.child_spec()` for supervision
  trees: `{Registry, keys: :unique, name: MyApp.Registry}`.
- `register(registry, key, value)` (since 1.4.0) —
  `@spec register(registry(), key(), value()) :: {:ok, pid()} | {:error, {:already_registered, pid()}}`.
  Registers the *current* process under `key` with associated `value`. Returns
  `{:ok, owner}` where `owner` is the PID in the registry partition responsible
  for the PID; the owner is automatically linked to the caller. For `:unique`,
  returns `{:error, {:already_registered, pid}}` if the key is taken. For
  `:duplicate`, multiple registrations from the same process under the same key
  are allowed.
- `unregister(registry, key)` (since 1.4.0) — `@spec unregister(registry(), key()) :: :ok`.
  Unregisters all entries for `key` associated to the current process. Always
  returns `:ok`; automatically unlinks the current process from the owner when
  no more keys remain. `unregister_match/4` removes only entries matching a
  pattern.
- `lookup(registry, key)` (since 1.4.0) —
  `@spec lookup(registry(), key()) :: [{pid(), value()}]`.
  Finds the `{pid, value}` pair(s) for `key` in no particular order; `[]` if no
  match. For `:unique`, a single partition lookup is necessary; for `:duplicate`,
  all partitions must be looked up.
- `dispatch(registry, key, mfa_or_fun, opts \\ [])` (since 1.4.0) —
  `@spec dispatch(registry(), key(), dispatcher, dispatch_opts()) :: :ok when
  dispatcher: (entries :: [{pid(), value()}] -> term()) | {module(), atom(), [term()]}`.
  Invokes the callback with all entries under `key` in each partition. The
  callback receives a non-empty list of `{pid, value}` tuples; it is never
  invoked if there are no entries. Dispatching happens in the caller process
  (serially, or concurrently via spawned tasks when `parallel: true` and the
  registry is partitioned with `{:duplicate, :pid}`). Registered processes are
  not involved unless the callback explicitly messages them. `:parallel` has no
  effect for `{:duplicate, :key}` registries (all entries for a key are in one
  partition). Core of the dispatcher/PubSub use cases.
- `select(registry, spec)` (since 1.9.0) — `@spec select(registry(), spec()) :: [term()]`.
  Selects key, pid, and values using full match specs. `spec` is a list of
  `{match_pattern, guards, body}` tuples. The match pattern matches the stored
  structure `{key, pid, value}`. Use named variables (`:"$1"`, `:"$2"`, ...);
  avoid `:"$_"` / `:"$$"`. Costly for large multi-partition registries (builds
  result by concatenating partitions).
- `count(registry)` (since 1.7.0) — `@spec count(registry()) :: non_neg_integer()`.
  Number of registered keys; runs in constant time.
- `count_match(registry, key, pattern, guards \\ [])` (since 1.7.0) — number of
  `{pid, value}` pairs under `key` matching `pattern`.
- `count_select(registry, spec)` (since 1.14.0) — like `select/2` but returns
  only the count of matching records.
- `match(registry, key, pattern, guards \\ [])` (since 1.4.0) —
  `@spec match(registry(), key(), match_pattern(), guards()) :: [{pid(), term()}]`.
  Returns `{pid, value}` pairs under `key` matching `pattern`. Pattern matches
  the value structure; `:_` ignores, `:"$1"` binds for guard comparisons (e.g.
  `{:>, :"$1", 1}`). `[]` if no match.
- `keys(registry, pid)` (since 1.4.0) — known keys for a `pid` (unique for
  `:unique`; may contain duplicates for `:duplicate`); `[]` if the process is
  dead or has no keys.
- `values(registry, key, pid)` — values for `key` registered by `pid`.
- `update_value(registry, key, callback)` — updates the value for `key` for the
  current process in a `:unique` registry.
- `lock(registry, lock_key, function)` — out-of-band locking of `lock_key` for
  the duration of `function`.
- Metadata: `meta/2`, `put_meta/3`, `delete_meta/2` (registry-level metadata,
  separate from per-entry values).

## :via registration
Once a registry is started with a name, it can register and access named
processes via the `:via` tuple. Two shapes:
- `{:via, Registry, {registry, key}}` — no associated value; `lookup` returns
  `[{self(), nil}]`.
- `{:via, Registry, {registry, key, value}}` — associates a `value` with the
  process; `lookup` returns `[{pid, value}]`. The metadata-less form can still
  be used to look up a process registered with metadata.

Only `:unique` registries can be used in `:via`. If the name is already taken,
the case-specific `start_link` (e.g. `Agent.start_link/2`) returns
`{:error, {:already_started, current_pid}}`.

Example (supervision tree child):
```elixir
{Registry, keys: :unique, name: MyApp.Registry}
```
Then:
```elixir
name = {:via, Registry, {MyApp.Registry, "agent"}}
{:ok, _} = Agent.start_link(fn -> 0 end, name: name)
Agent.get(name, & &1)  #=> 0
```
With metadata:
```elixir
name = {:via, Registry, {MyApp.Registry, "agent", :hello}}
{:ok, agent_pid} = Agent.start_link(fn -> 0 end, name: name)
Registry.lookup(MyApp.Registry, "agent")  #=> [{agent_pid, :hello}]
```

This is how Registry provides typed/dynamic name registration over BEAM: instead
of the built-in `register/2` (which only accepts atoms and one process per
name), `:via` lets any term be a name, scoped to a named Registry instance, with
an optional typed value attached and automatic cleanup on process crash.

## Registry relationship to BEAM (process registration; → docs/beam/processes-and-messages.md)
Registry is a user-space, ETS-backed layer on top of BEAM process registration
semantics. Key relationships:
- BEAM's built-in `register/2` registers a process under a single atom name and
  links name lifetime to the process; `Registry` generalizes this to arbitrary
  term keys, multiple registries, partitioning, and per-entry values.
- Automatic cleanup relies on BEAM links/monitors: `register/3` links the caller
  to the partition "owner" PID, so when the registering process crashes the
  owner is notified and removes the keys. Cleanup is eventual, not immediate
  (see Strict rules).
- `:via` is the OTP-idiomatic bridge: `GenServer`/`Agent`/`Supervisor`
  `:name` options accept `{:via, module, term}` and call the module's
  `register_name/2`, `unregister_name/2`, `whereis_name/1`, and `send/2`
  callbacks. Registry implements these, so any `:via`-aware OTP process can be
  named through it.
- Dispatch/PubSub builds on BEAM message passing (`send/2`): the dispatch
  callback runs in the caller and explicitly sends messages to registered PIDs;
  `send/2` is a no-op for already-dead processes, and `Process.monitor/1`
  delivers `:DOWN` immediately if the target is dead — both cope with the
  eventual-consistency window.
- Partitioning maps to BEAM schedulers: `System.schedulers_online()` is the
  recommended partition count, distributing ETS ownership across scheduler
  partitions.

See `docs/beam/processes-and-messages.md` for the underlying process
registration, linking, and message-passing primitives.

## Strict rules
- Only `:unique` registries may be used with `:via`.
- `register/3` registers the *current* process only; it cannot register an
  arbitrary PID.
- For `:unique`, a key maps to at most one process; re-registering the same key
  returns `{:error, {:already_registered, pid}}`.
- For `:duplicate`, the same process may register the same key multiple times;
  `keys/2` then returns duplicates.
- Cleanup is eventual: a crashed process's keys are removed automatically but
  not immediately; some operations may return dead PIDs. Functions that can
  return dead PIDs state this explicitly. This is generally safe because
  `send/2` is a no-op for dead processes and `Process.monitor/1` delivers
  `:DOWN` immediately for already-dead targets.
- All key comparisons use `===/2` (strict equality).
- Avoid special match variables `:"$_"` and `:"$$"` in `select/2`,
  `count_select/2`, and `match/4`; they may not work as expected, especially
  for `{:duplicate, :key}` registries which use a different internal ETS
  layout. Use named variables `:"$1"`, `:"$2"`, ...
- Guard conditions in `match/4`/`count_match/4`/`select/2` work only for
  assigned variables (e.g. `{:>, :"$1", 1}`).
- `dispatch/3` runs in the caller process; failures during dispatch do not
  notify registered processes — wrap and report errors explicitly.
- `:parallel` dispatch option is only meaningful for `{:duplicate, :pid}`
  registries; it has no effect for `{:duplicate, :key}` (single partition per
  key).

## Verbatim quotes
- "A local, decentralized and scalable key-value process storage. It allows
  developers to lookup one or more processes with a given key."
- "If the registry has `:unique` keys, a key points to 0 or 1 process. If the
  registry allows `:duplicate` keys, a single key may point to any number of
  processes. In both cases, different keys could identify the same process."
- "Each entry in the registry is associated to the process that has registered
  the key. If the process crashes, the keys associated to that process are
  automatically removed. All key comparisons in the registry are done using the
  match operation (`===/2`)."
- "Only registries with unique keys can be used in `:via`. If the name is
  already taken, the case-specific `start_link` function ... will return
  `{:error, {:already_started, current_pid}}`."
- "The `owner` is the PID in the registry partition responsible for the PID. The
  owner is automatically linked to the caller."
- "Looking up, dispatching and registering are efficient and immediate at the
  cost of delayed unsubscription. ... certain operations may return processes
  that are already dead."
- "Note that the registry uses one ETS table plus two ETS tables per
  partition."
- "Dispatching happens in the process that calls `dispatch/3` either serially or
  concurrently in case of multiple partitions (via spawned tasks). The
  registered processes are not involved in dispatching unless involving them is
  done explicitly."

## Version notes
- Page documents Elixir v1.20.2.
- `child_spec/1`, `start_link/1`: since 1.5.0.
- `register/3`, `unregister/3`, `lookup/2`, `dispatch/3`, `match/4`, `keys/2`:
  since 1.4.0 (Registry was introduced in Elixir 1.4).
- `count/1`, `count_match/4`: since 1.7.0.
- `select/2`: since 1.9.0.
- `delete_meta/2`: since 1.11.0.
- `count_select/2`: since 1.14.0.
- `listener_message/0` type and `:listeners` option: since 1.15.0.
- `{:duplicate, :key}` / `{:duplicate, :pid}` partitioning strategies and the
  `keys()` type extension are present in v1.20.2.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/elixir/Supervisor.html (linked as Supervisor.html) —
  supervision tree child specs; Registry is started as a supervisor process.
- Text-referenced (not hrefs, but core to understanding): Agent, Task,
  GenServer (`:via` naming), Process (`monitor/1`, `send/2`),
  `:elixir.register/2` (built-in BEAM process registration).

### Skipped
- https://elixir-lang.org/docs.html (top-level docs index; not a module page).
- https://elixir.hexdocs.pm/Registry.html (self/canonical).
