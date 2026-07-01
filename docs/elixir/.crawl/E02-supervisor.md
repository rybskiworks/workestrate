# Crawl: hexdocs.pm/elixir/Supervisor.html
- seed_url: https://hexdocs.pm/elixir/Supervisor.html
- canonical_url: https://elixir.hexdocs.pm/Supervisor.html
- family: Elixir core module
- fetch: HTTP 200
- elixir_version: v1.20.2
- feeds_docs: otp-supervision.md (→ docs/beam/supervision.md)

## Purpose
A behaviour module for implementing supervisors. A supervisor is a process which
supervises other processes (child processes). Supervisors build a hierarchical
process structure called a supervision tree, providing fault-tolerance and
encapsulating how applications start and shutdown.

A supervisor may be started directly with a list of child specifications via
`start_link/2`, or as a module-based supervisor implementing the `init/1`
callback (via `use Supervisor`).

## init/1 + sup_flags

### init/1 callback (verbatim @callback)
```elixir
@callback init(init_arg :: term()) ::
  {:ok,
   {sup_flags(),
    [child_spec() | (old_erlang_child_spec :: :supervisor.child_spec())]}}
  | :ignore
```
Developers typically invoke `Supervisor.init/2` at the end of their `init`
callback to return the proper supervision flags. The callback is also invoked
during hot code upgrades.

### sup_flags type (verbatim @type — ACTUAL Elixir key names)
```elixir
@type sup_flags() :: %{
  strategy: strategy(),
  intensity: non_neg_integer(),
  period: pos_integer(),
  auto_shutdown: auto_shutdown()
}
```

### CRITICAL naming distinction (verbatim)
Elixir uses TWO different key namespaces:

1. **Returned `sup_flags()` map keys** (what `init/1` returns and what the
   supervisor stores internally) — verbatim:
   - `:strategy`
   - `:intensity`
   - `:period`
   - `:auto_shutdown`

2. **Input `init_option()` keys** (what you pass to `start_link/2` and
   `Supervisor.init/2`) — verbatim `@type init_option()`:
   - `{:strategy, strategy()}`
   - `{:max_restarts, non_neg_integer()}`   # → mapped to `intensity`
   - `{:max_seconds, pos_integer()}`          # → mapped to `period`
   - `{:auto_shutdown, auto_shutdown()}`

So Elixir accepts `max_restarts` / `max_seconds` as input options but stores
them internally (and in the `sup_flags()` type) as `intensity` / `period`.
This is a 1:1 rename: `max_restarts == intensity`, `max_seconds == period`.

Defaults (verbatim from "Supervisor strategies and options"):
- `:max_restarts` defaults to **3**
- `:max_seconds` defaults to **5**
- `:strategy` is **required**
- `:auto_shutdown` is optional, defaults to `:never`

### Supervisor.init/2 (since 1.5.0)
```elixir
@spec init(
  [child_spec() | module_spec() | :supervisor.child_spec()],
  [init_option()]
) ::
  {:ok, {sup_flags(), [child_spec() | :supervisor.child_spec()]}}
```
Typically invoked at the end of the `init/1` callback of module-based
supervisors. Returns `{sup_flags, child_specs}`.

## child_spec keys

### Verbatim @type child_spec()
```elixir
@type child_spec() :: %{
  :id => atom() | term(),
  :start => {module(), function_name :: atom(), args :: [term()]},
  optional(:restart) => restart(),
  optional(:shutdown) => shutdown(),
  optional(:type) => type(),
  optional(:modules) => [module()] | :dynamic,
  optional(:significant) => boolean()
}
```

### Key-by-key (verbatim descriptions)
- `:id` — any term used to identify the child specification internally by the
  supervisor; defaults to the given module. **Required.** For supervisors,
  conflicting `:id` values cause the supervisor to refuse initialization and
  require explicit IDs (NOT the case for dynamic supervisors).
- `:start` — a tuple `{module, function, args}` invoked to start the child
  process. **Required.**
- `:restart` — when a terminated child should be restarted. Optional, defaults
  to `:permanent`. Values: `:permanent | :transient | :temporary`.
- `:shutdown` — how a child process should be terminated. Optional; defaults to
  `5_000` if type is `:worker`, or `:infinity` if type is `:supervisor`.
  Values: `:brutal_kill | non_neg_integer() | :infinity`.
- `:type` — `:worker` or `:supervisor`. Optional, defaults to `:worker`.
- `:modules` — list of modules used by hot code upgrade mechanisms. Set
  automatically based on `:start`; rarely changed. `[module()] | :dynamic`.
- `:significant` — boolean; whether the child is significant for automatic
  shutdown. Only `:transient` and `:temporary` children can be significant.
  Optional, defaults to `false`.

### child_spec/1 (module callback)
When a child is given as `{module, arg}` or just `module`, the supervisor calls
`module.child_spec(arg)` (or `module.child_spec([])` for bare module) to
retrieve the child spec map. `use GenServer` / `use Supervisor` auto-defines a
`child_spec/1`; customize via options e.g. `use GenServer, restart: :transient`.

### Supervisor.child_spec/2 (builds + overrides)
```elixir
@spec child_spec(child_spec() | module_spec(), child_spec_overrides()) :: child_spec()
```
Retrieves the child spec (from map, `{module,arg}`, or module) then applies
`overrides` keys directly. Raises if overrides has keys not mapping to a child
spec field. Used to start the same module multiple times with distinct `:id`:
```elixir
Supervisor.child_spec({Agent, fn -> :ok end}, id: {Agent, 1})
#=> %{id: {Agent, 1}, start: {Agent, :start_link, [fn -> :ok end]}}
```

## Functions

| Function | Arity | Signature / Return |
|---|---|---|
| `start_link/2` | 2 | `start_link(children, options)` → `{:ok, pid} \| {:error, {:already_started, pid} \| {:shutdown, term} \| term}`. Starts a supervisor with a list of child specs. `:strategy` required. |
| `start_link/3` | 3 | `start_link(module, init_arg, options \\ [])` → `on_start()`. Module-based supervisor; invokes `module.init(init_arg)`. |
| `start_child/2` | 2 | `start_child(supervisor, child_spec)` → `on_start_child()`. Appends child spec; preserves `:rest_for_one` ordering. Returns `{:ok, child} \| {:ok, child, info} \| {:error, {:already_started, child} \| :already_present \| term}`. |
| `terminate_child/2` | 2 | `terminate_child(supervisor, child_id)` → `:ok \| {:error, :not_found}`. Terminates process; keeps child spec unless child is `:temporary`. |
| `delete_child/2` | 2 | `delete_child(supervisor, child_id)` → `:ok \| {:error, :not_found \| :running \| :restarting}`. Child must not be running. |
| `restart_child/2` | 2 | `restart_child(supervisor, child_id)` → `{:ok, child} \| {:ok, child, term} \| {:error, :not_found \| :running \| :restarting \| term}`. Invalid for `:temporary` children (spec auto-deleted on termination). |
| `which_children/1` | 1 | `which_children(supervisor)` → `[{id, child, type, modules}]`. `child` = pid \| `:restarting` \| `:undefined`. WARNING: under low memory with many children can OOM the system. |
| `count_children/1` | 1 | `count_children(supervisor)` → `%{specs, active, supervisors, workers}`. `specs` = total (dead or alive); `active` = running; `supervisors`/`workers` = counts by type. |
| `child_spec/2` | 2 | `child_spec(module_or_map, overrides)` → `child_spec()`. See above. |
| `init/2` | 2 | `init(children, options)` → `{:ok, {sup_flags, child_specs}}`. (since 1.5.0) Called inside module-based `init/1`. |
| `stop/3` | 3 | `stop(supervisor, reason \\ :normal, timeout \\ :infinity)` → `:ok`. Synchronous. If reason ≠ `:normal`/`:shutdown`/`{:shutdown,_}`, an error report is logged. |

### NOTE on `start_link_with_flags`
There is **no** `start_link_with_flags` function in Elixir's `Supervisor` module.
This is not an Elixir API. (Erlang/OTP's `supervisor` module likewise has no
function by that exact name; the closest concept is passing `sup_flags()` via
`init/1`'s return value, which Elixir exposes through `Supervisor.init/2`.)

## Strategies (verbatim @type strategy)
```elixir
@type strategy() :: :one_for_one | :one_for_all | :rest_for_one
```
- `:one_for_one` — if a child terminates, only that process is restarted.
- `:one_for_all` — if a child terminates, all other children are terminated
  and then all children (including the terminated one) are restarted.
- `:rest_for_one` — if a child terminates, the terminated child and all
  children started after it are terminated and restarted.

"Process termination" above refers to **unsuccessful** termination, as
determined by the `:restart` option.

### NOTE on `:simple_one_for_one`
Elixir's `Supervisor` does **NOT** support `:simple_one_for_one`. The
`@type strategy()` lists only three values. Dynamic, same-type children are
handled by `DynamicSupervisor` instead (the docs explicitly direct readers
there: "To efficiently supervise children started dynamically, see
DynamicSupervisor").

## Restart values (`:restart`)
```elixir
@type restart() :: :permanent | :transient | :temporary
```
- `:permanent` — always restarted.
- `:temporary` — never restarted; any termination (even abnormal) is
  considered successful.
- `:transient` — restarted only if it terminates abnormally, i.e. with an
  exit reason other than `:normal`, `:shutdown`, or `{:shutdown, term}`.

## Shutdown values (`:shutdown`)
```elixir
@type shutdown() :: timeout() | :brutal_kill
```
- `:brutal_kill` — child unconditionally and immediately terminated via
  `Process.exit(child, :kill)`.
- integer `>= 0` — ms the supervisor waits after `Process.exit(child, :shutdown)`
  before `Process.exit(child, :kill)`. If child isn't trapping exits, the
  initial `:shutdown` signal terminates it immediately.
- `:infinity` — wait indefinitely. Recommended for `:supervisor` children.
  Discouraged for workers (can block application termination).

## Automatic shutdown (`:auto_shutdown`)
```elixir
@type auto_shutdown() :: :never | :any_significant | :all_significant
```
- `:never` — default; disabled.
- `:any_significant` — if any significant child exits, supervisor shuts down
  its children then itself.
- `:all_significant` — when all significant children have exited, supervisor
  shuts down.
Only `:transient` and `:temporary` children can be marked `:significant`.
Significant `:transient` children must exit **normally** for auto-shutdown to
be considered; `:temporary` children may exit for any reason.

## Exit reasons and restarts (verbatim summary)
- `:normal` — exit not logged; no restart in `:transient` mode; linked
  processes do NOT exit.
- `:shutdown` / `{:shutdown, term}` — exit not logged; no restart in
  `:transient` mode; linked processes exit with same reason unless trapping.
- any other term — exit IS logged; restarts happen in `:transient` mode;
  linked processes exit with same reason unless trapping.

A supervisor that reaches maximum restart intensity exits with reason
`:shutdown`. It will only be restarted if its own child spec has
`:restart => :permanent` (the default).

## Start and shutdown ordering
- Start: traverses child specs in order, starting each child via the `:start`
  MFA (typically `start_link/1`). `start_link/1` must return `{:ok, pid}`
  with pid linked to the supervisor.
- Shutdown: terminates children in **reverse** order, sending
  `Process.exit(child_pid, :shutdown)` and awaiting the configured `:shutdown`
  interval (default 5000 ms) before `Process.exit(child_pid, :kill)`.
- If a child is trapping exits, its `terminate` callback is invoked; otherwise
  it shuts down immediately on the first signal.

## Module-based supervisors (`use Supervisor`)
`use Supervisor` sets `@behaviour Supervisor` and defines a `child_spec/1`
function so the module can itself be a child. Customizable options:
- `:id` — defaults to the current module
- `:restart` — defaults to `:permanent`
The `@doc` immediately preceding `use Supervisor` is attached to the
generated `child_spec/1`.

Guideline: use the no-callback form (`start_link/2` with a child list) only
at the top of the supervision tree (typically in `Application.start/2`); use
module-based supervisors everywhere else so they can be children of other
supervisors.

## Elixir Supervisor ↔ BEAM supervisor mapping (→ docs/beam/supervision.md)

| Elixir (this page, v1.20.2) | BEAM `:supervisor` (docs/beam/supervision.md) | Notes |
|---|---|---|
| `Supervisor.init/1` callback returns `{:ok, {sup_flags(), [child_spec()]}} \| :ignore` | `Module:init/1` returns `{ok, {SupFlags :: sup_flags(), [ChildSpec :: child_spec()]}} \| ignore` | Identical shape; Elixir uses map/atom-keyed structs, Erlang uses maps or tuples. |
| `sup_flags()` keys: `:strategy`, `:intensity`, `:period`, `:auto_shutdown` | `sup_flags()` keys: `strategy`, `intensity`, `period`, `auto_shutdown` (+ `hibernate_after` in OTP 28.0+) | Same internal key names. Elixir does NOT expose `hibernate_after`. |
| Input options `:max_restarts` / `:max_seconds` | (no equivalent input) — Erlang sets `intensity`/`period` directly in `sup_flags` | Elixir renames `max_restarts→intensity`, `max_seconds→period` for ergonomic input. |
| `:strategy` ∈ `{:one_for_one, :one_for_all, :rest_for_one}` | `strategy() :: one_for_all \| one_for_one \| rest_for_one \| simple_one_for_one` | Elixir DROPS `simple_one_for_one` → use `DynamicSupervisor`. |
| `child_spec()` map with `:id/:start/:restart/:shutdown/:type/:modules/:significant` | `child_spec()` map with `id/start/restart/shutdown/type/modules` (+ `significant`) | Same keys; Elixir atoms, Erlang atoms. |
| `Supervisor.start_link/2` (children, options) | `supervisor:start_link/2,3` | Elixir /2 takes child list + keyword opts; Erlang /2 is `start_link(Module, Args)`, /3 is `start_link(SupName, Module, Args)`. Elixir folds name registration into the options keyword list (`:name`). |
| `Supervisor.start_link/3` (module, init_arg, options) | `supervisor:start_link/3` (`SupName, Module, Args`) | Elixir /3 is the module-based form; name passed via `:name` option, not a separate `SupName` arg. |
| `start_child/2`, `terminate_child/2`, `delete_child/2`, `restart_child/2`, `which_children/1`, `count_children/1` | `supervisor:start_child/2`, `terminate_child/2`, `delete_child/2`, `restart_child/2`, `which_children/1`, `count_children/1` | 1:1 correspondence; Elixir returns maps/atoms, Erlang returns tuples/atoms. |
| `Supervisor.child_spec/2` | (no direct equivalent; Erlang has `supervisor:get_childspec/2` since OTP 18.0 for retrieval, not building) | Elixir's `child_spec/2` BUILDS+OVERRIDES a spec from a module/tuple/map; Erlang's `get_childspec/2` RETRIEVES an existing spec. |
| `Supervisor.init/2` | (no direct equivalent; Erland builds `sup_flags()` map directly in `init/1`) | Elixir helper that constructs the `{sup_flags, child_specs}` tuple from option keywords. |
| `Supervisor.stop/3` | `supervisor:stop/1,3` (`stop(SupRef)` / `stop(SupRef, Reason, Timeout)`) | Elixir exposes the 3-arg form with defaults. |
| `:auto_shutdown` ∈ `{:never, :any_significant, :all_significant}` | `auto_shutdown :: never \| any_significant \| all_significant` | Identical values; OTP feature. |
| `:significant` child_spec key | `significant` child_spec key | Only `:transient`/`:temporary` children. |
| (no `start_link_with_flags`) | (no `start_link_with_flags`) | Not a real function in either Elixir or Erlang. |

### Key divergences to record in docs/beam/supervision.md
1. **`simple_one_for_one` is Erlang-only.** Elixir removed it; direct readers to
   `DynamicSupervisor`. The BEAM doc already notes `simple_one_for_one` caveats
   (lines 116–121, 333, 352, 361) — these do NOT apply to Elixir users.
2. **Input option rename.** Elixir users pass `max_restarts`/`max_seconds`;
   these become `intensity`/`period` in `sup_flags()`. The BEAM doc's
   `intensity`/`period` discussion (lines 47–63, 206–220) maps directly.
3. **`hibernate_after`** (OTP 28.0+, BEAM doc line 63) is NOT exposed by Elixir.
4. **Name registration** is folded into Elixir's options keyword list (`:name`)
   rather than a separate `SupName` positional argument as in Erlang `/3`.
5. **`get_childspec/2`** (OTP 18.0, BEAM doc line 283) has no Elixir counterpart
   for retrieval; Elixir's `child_spec/2` is build/override only.

## Strict rules
1. `:strategy` is REQUIRED in both `start_link/2` options and `Supervisor.init/2`.
2. `:id` and `:start` are REQUIRED child_spec keys; the rest are optional.
3. For supervisors (not dynamic), conflicting `:id` values cause initialization
   failure — explicit IDs required.
4. `:significant` may only be set on `:transient` or `:temporary` children.
5. `:temporary` children: child spec is auto-deleted on termination →
   `restart_child/2` cannot restart them.
6. `delete_child/2` requires the child NOT be running (terminate first).
7. `restart_child/2` requires the child spec exist and the child NOT be running.
8. `:shutdown` defaults to `5_000` for `:worker`, `:infinity` for `:supervisor`.
9. `:infinity` shutdown for workers is discouraged — can block app termination.
10. A supervisor exceeding max restart intensity exits with reason `:shutdown`
    and is restarted only if its own spec is `:permanent` (default).
11. `which_children/1` under low memory + many children can OOM the system.
12. `Supervisor.child_spec/2` raises if `overrides` contains keys not mapping
    to a child_spec field.
13. `start_link/1` of a child must return `{:ok, pid}` with pid linked to the
    supervisor (or `:ignore` / `{:error, reason}`).
14. Children start in listed order; shutdown is in REVERSE order.
15. `use Supervisor` auto-generates `child_spec/1`; customize via `:id`/`:restart`
    options to `use Supervisor`.

## Verbatim quotes
- "A behaviour module for implementing supervisors."
- "A supervisor is a process which supervises other processes, which we refer to
  as child processes."
- "Supervisors are used to build a hierarchical process structure called a
  supervision tree."
- "`:id` - any term used to identify the child specification internally by the
  supervisor; defaults to the given module. This key is required."
- "`:start` - a tuple with the module-function-args to be invoked to start the
  child process. This key is required."
- "`:max_restarts` - the maximum number of restarts allowed in a time frame.
  Defaults to 3."
- "`:max_seconds` - the time frame in which `:max_restarts` applies. Defaults
  to 5."
- "`:auto_shutdown` - the automatic shutdown option. It can be `:never`,
  `:any_significant`, or `:all_significant`. Optional."
- "`:strategy` - the supervision strategy option. It can be either
  `:one_for_one`, `:rest_for_one` or `:one_for_all`. Required."
- "To efficiently supervise children started dynamically, see DynamicSupervisor."
- "The shutdown process happens in reverse order."
- "Note that the supervisor that reaches maximum restart intensity will exit
  with `:shutdown` reason."
- "Only `:transient` and `:temporary` child processes can be marked as
  significant."
- "Note that calling this function [`which_children/1`] when supervising a
  large number of children under low memory conditions can bring the system
  down due to an out of memory error."
- `@type strategy() :: :one_for_one | :one_for_all | :rest_for_one`
- `@type sup_flags() :: %{strategy: strategy(), intensity: non_neg_integer(),
  period: pos_integer(), auto_shutdown: auto_shutdown()}`
- `@type init_option() :: {:strategy, strategy()} | {:max_restarts,
  non_neg_integer()} | {:max_seconds, pos_integer()} | {:auto_shutdown,
  auto_shutdown()}`

## Version notes
- Page reports **Elixir v1.20.2**.
- `Supervisor.init/2` available since **1.5.0**.
- `module_spec()` type available since **1.16.0**.
- `:simple_one_for_one` is NOT in Elixir's `strategy()` type — removed in favor
  of `DynamicSupervisor` (long-standing Elixir design; confirmed still absent in
  v1.20.2).
- `:auto_shutdown` and `:significant` are present (OTP 26+ era features).
- `hibernate_after` (OTP 28.0+ in Erlang) is NOT exposed by Elixir v1.20.2.

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/elixir/DynamicSupervisor.html — replaces `simple_one_for_one`;
  essential companion for dynamic child supervision.
- https://hexdocs.pm/elixir/GenServer.html — child process implementation;
  `use GenServer` auto-defines `child_spec/1`; name registration rules shared.
- https://hexdocs.pm/elixir/Application.html — `Application.start/2` is the
  recommended top-of-tree supervisor start point.
- https://hexdocs.pm/elixir/Supervisor.html (Erlang `:supervisor` module) —
  for the raw BEAM API and `get_childspec/2` (OTP 18.0).
- https://hexdocs.pm/elixir/Registry.html — often supervised; uses
  `:partition` child specs.

### Skipped
- GenServer name registration section (cross-reference only, not a separate
  crawl target for this step).
- "View Source" / "Copy Markdown" UI links.
- Search box / settings UI.
- ExDoc sidebar navigation to unrelated modules.
