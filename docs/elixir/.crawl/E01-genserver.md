# Crawl: hexdocs.pm/elixir/GenServer.html
- seed_url: https://hexdocs.pm/elixir/GenServer.html
- canonical_url: https://elixir.hexdocs.pm/GenServer.html
- family: Elixir core module (hexdocs)
- fetch: 200
- elixir_version: v1.20.2
- feeds_docs: otp-supervision.md, beam-otp-internals.md

## Purpose

`GenServer` is an Elixir behaviour module for implementing the server side of a
client-server relation. A GenServer is a plain Elixir process used to keep state,
execute code asynchronously, and fit into a supervision tree. It abstracts the
"receive" loop, message passing, startup, tracing/error reporting, and code
change, leaving the developer to implement only the callbacks of interest
(`init/1` is the only required callback; 8 callbacks total).

Verbatim: "A behaviour module for implementing the server of a client-server
relation. A GenServer is a process like any other Elixir process and it can be
used to keep state, execute code asynchronously and so on."

`use GenServer` sets `@behaviour GenServer` and defines a `child_spec/1`
function so the module can be used directly as a supervisor child.

## Callback contract (Elixir) + return-value conventions

Elixir `GenServer` exposes 8 callbacks. All are optional except `init/1`.

### init/1  (required)
```
@callback init(init_arg :: term()) ::
  {:ok, state}
  | {:ok, state, timeout() | :hibernate | {:continue, continue_arg :: term()}}
  | :ignore
  | {:stop, reason :: term()}
```
- `{:ok, state}` → `start_link/3` returns `{:ok, pid}`, process enters loop.
- `{:ok, state, timeout}` → also sets a timeout (see "Timeouts").
- `{:ok, state, :hibernate}` → hibernate before entering loop.
- `{:ok, state, {:continue, arg}}` → `handle_continue/2` invoked immediately
  after entering the loop.
- `:ignore` → `start_link/3` returns `:ignore`; process exits normally without
  entering loop or calling `terminate/2`. Supervisor does not fail/restart.
- `{:error, reason}` → `start_link/3` returns `{:error, reason}`.
- `{:stop, reason}` → process exits with `reason` without entering loop or
  calling `terminate/2`; `start_link/3` returns `{:error, reason}` (only if
  caller traps exits).

### handle_call/3  (optional)
```
@callback handle_call(request :: term(), from(), state :: term()) ::
  {:reply, reply, new_state}
  | {:reply, reply, new_state, timeout() | :hibernate | {:continue, continue_arg}}
  | {:noreply, new_state}
  | {:noreply, new_state, timeout() | :hibernate | {:continue, continue_arg}}
  | {:stop, reason, reply, new_state}
  | {:stop, reason, new_state}
```
- `{:reply, reply, new_state}` → send `reply` to caller, continue with `new_state`.
- `{:noreply, new_state}` → do NOT reply via return value; reply must be sent
  with `GenServer.reply/2`. Use cases: reply before slow work, reply after work,
  reply from another process.
- `{:stop, reason, reply, new_state}` → `terminate/2` called, reply sent,
  process exits with `reason`.
- `:hibernate` → GC + minimal memory; do not overuse.
- `{:continue, continue_arg}` → `handle_continue/2` invoked immediately after.
- If not implemented, server fails when a `call` is performed against it.

### handle_cast/2  (optional)
```
@callback handle_cast(request :: term(), state :: term()) ::
  {:noreply, new_state}
  | {:noreply, new_state, timeout() | :hibernate | {:continue, continue_arg}}
  | {:stop, reason :: term(), new_state}
```
Return values mirror `handle_call/3` minus the `:reply` variants. If not
implemented, server fails when a `cast` is performed against it.

### handle_continue/2  (optional)
```
@callback handle_continue(continue_arg, state :: term()) ::
  {:noreply, new_state}
  | {:noreply, new_state, timeout() | :hibernate | {:continue, continue_arg}}
  | {:stop, reason :: term(), new_state}
```
Useful for work after init or splitting callback work into multiple steps.
Return values same as `handle_cast/2`. If not implemented, server fails when a
continue instruction is used.

### handle_info/2  (optional)
```
@callback handle_info(msg :: :timeout | term(), state :: term()) ::
  {:noreply, new_state}
  | {:noreply, new_state, timeout() | :hibernate | {:continue, continue_arg}}
  | {:stop, reason :: term(), new_state}
```
Handles all other messages (e.g. `send/2`, `Process.send_after/4`, monitor
`DOWN`). On timeout the message is `:timeout`. Return values same as
`handle_cast/2`. If not implemented, the received message is logged.

### terminate/2  (optional)
```
@callback terminate(reason, state :: term()) :: term()
  when reason: :normal | :shutdown | {:shutdown, term()} | term()
```
Return value is ignored. Called for cleanup requiring access to state. NOT
guaranteed to be called on exit — important cleanup should use links/monitors.
Called when: process traps exits and parent sends exit signal; a callback
(except `init/1`) returns a `:stop` tuple, raises, exits, or returns an invalid
value. If reason is not `:normal`/`:shutdown`/`{:shutdown,_}`, an error is
logged. Ports/IO devices are auto-closed on exit signal — no manual close needed.

### code_change/3  (optional)
```
@callback code_change(old_vsn, state :: term(), extra :: term()) ::
  {:ok, new_state :: term()} | {:error, reason :: term()}
  when old_vsn: term() | {:down, term()}
```
Hot code swapping. `old_vsn` is previous module version (from `@vsn`); on
downgrade wrapped in `{:down, term()}`. `{:ok, new_state}` changes state;
`{:error, reason}` fails code change, state unchanged. If it raises, code change
fails and loop continues with previous state — so avoid side effects here.

### format_status/1  (optional, since Elixir 1.17.0)
```
@callback format_status(status :: :gen_server.format_status()) ::
  new_status :: :gen_server.format_status()
```
Called by `:sys.get_status/1,2` and on abnormal termination logging. Receives a
map `status`, returns a map `new_status` with same keys (values may transform).
Use cases: redact sensitive state, compact large status items.

### format_status/2  (DEPRECATED)
```
@callback format_status(reason, pdict_and_state :: list()) :: term()
  when reason: :normal | :terminate
```
Deprecated — use `format_status/1` instead.

### Return-value conventions summary (Elixir idioms)
- Elixir uses atoms `:ok`/`:noreply`/`:reply`/`:stop`/`:ignore`/`:error`/
  `:hibernate`/`:continue` as tuple tags (vs Erlang's identical atoms — the
  shapes are the same; Elixir just surfaces them through the `@callback` types).
- The 4th tuple element for `:reply`/`:noreply` is a single `action()`:
  `timeout() | :hibernate | {:continue, continue_arg}` — never a list (Elixir
  GenServer does not expose the OTP 25+ multi-action list form directly; it
  models one action per return).
- `from()` is `{pid, tag}` — opaque; pass to `GenServer.reply/2`.

## Key functions (signatures)

```
start_link(module, init_arg, options \\ []) :: on_start()
start(module, init_arg, options \\ [])       :: on_start()
call(server, request, timeout \\ 5000)       :: term()
cast(server, request)                        :: :ok
reply(client, reply)                         :: :ok
abcast(nodes \\ [node() | Node.list()], name, request) :: :abcast
multi_call(nodes \\ [node() | Node.list()], name, request, timeout \\ :infinity)
  :: {replies :: [{node(), term()}], bad_nodes :: [node()]}
stop(server, reason \\ :normal, timeout \\ :infinity) :: :ok
whereis(server) :: pid() | {atom(), node()} | nil
```

Types:
```
@type on_start() :: {:ok, pid()} | :ignore | {:error, {:already_started, pid()} | term()}
@type from()     :: {pid(), tag :: term()}
@type name()     :: nil | atom() | {:global, term()} | {:via, module(), term()}
@type server()   :: pid() | name() | {atom(), node()}
@type option()   :: {:debug, debug()} | {:name, name()} | {:timeout, timeout()}
                  | {:spawn_opt, [Process.spawn_opt()]} | {:hibernate_after, timeout()}
@type debug()    :: [:trace | :log | :statistics | {:log_to_file, Path.t()}]
```

`start_link/3` options: `:name`, `:timeout`, `:debug`, `:spawn_opt`,
`:hibernate_after`. `start_link/3` blocks until `init/1` returns (synchronized
start). `start/3` is the unlinked variant (outside supervision tree).

`call/3` default timeout is `5000` ms; `:infinity` waits indefinitely. On
timeout the caller exits; late replies may arrive later as 2-tuples with a
reference first element — caller must discard them.

`cast/2` always returns `:ok` regardless of whether destination exists.

`reply/2` can be called from any process (not just the GenServer) as long as
the `from` argument was communicated.

## Elixir GenServer ↔ BEAM gen_server mapping (→ docs/beam/gen-server.md)

Elixir's `GenServer` is a thin Elixir wrapper over the Erlang/OTP `:gen_server`
module. The callback contract is identical; Elixir adds `@behaviour GenServer`,
a generated `child_spec/1`, and Elixir-flavoured typespecs. The mapping below
aligns with `docs/beam/gen-server.md` "Mapping (gen_server module -> callback
module)" and "Callback signatures" sections.

| Elixir `GenServer`                | BEAM `:gen_server`                  | docs/beam/gen-server.md anchor |
|-----------------------------------|-------------------------------------|---------------------------------|
| `GenServer.start_link/3`          | `:gen_server.start_link/3`          | start/start_link → init/1       |
| `GenServer.start/3`              | `:gen_server.start/3`              | start/start_monitor/start_link  |
| `GenServer.call/3`               | `:gen_server.call/3`               | call/send_request/multi_call → handle_call/3 |
| `GenServer.cast/2`               | `:gen_server.cast/2`               | cast/abcast → handle_cast/2      |
| `GenServer.abcast/3`            | `:gen_server.abcast/3`            | cast/abcast → handle_cast/2      |
| `GenServer.multi_call/4`        | `:gen_server.multi_call/4`        | call/multi_call → handle_call/3  |
| `GenServer.reply/2`             | `:gen_server.reply/2`             | (reply to `from`)               |
| `GenServer.stop/3`              | `:gen_server.stop/3`              | stop → terminate/2              |
| `GenServer.whereis/1`           | `:gen_server.whereis/1` (via `:global`/registry) | (name lookup)        |
| `init/1`                         | `Module:init/1`                    | init/1 callback                 |
| `handle_call/3`                  | `Module:handle_call/3`             | handle_call/3 callback           |
| `handle_cast/2`                  | `Module:handle_cast/2`             | handle_cast/2 callback           |
| `handle_info/2`                  | `Module:handle_info/2`             | handle_info/2 callback           |
| `handle_continue/2`             | `Module:handle_continue/2`         | handle_continue/2 (OTP 21+)      |
| `terminate/2`                    | `Module:terminate/2`              | terminate/2 callback             |
| `code_change/3`                  | `Module:code_change/3`            | code_change/3 callback           |
| `format_status/1` (1.17+)       | `Module:format_status/1` (OTP 25+) | format_status/1 callback         |
| `format_status/2` (deprecated)  | `Module:format_status/2` (deprecated) | format_status/2 (DEPRECATED)  |
| 4th-element action `timeout \| :hibernate \| {:continue,_}` | `action()` single-action form | action/0 type |
| `from()` = `{pid, tag}`          | `from()` = `{pid, tag}`            | from() type                     |
| `:sys.get_state/2`, `:sys.get_status/2`, `:sys.trace/3`, `:sys.statistics/3`, `:sys.no_debug/2`, `:sys.suspend/2`, `:sys.resume/2` | same `:sys` API | (special process / sys debugging) |

Notes on divergence:
- Elixir surfaces only the single-action 4th-tuple-element form
  (`timeout | :hibernate | {:continue, _}`), not the OTP 25+ multi-action list.
  The underlying `:gen_server` still accepts action lists, but the Elixir
  `@callback` types do not advertise them.
- `use GenServer` auto-generates `child_spec/1` (configurable via
  `:id`/`:restart`/`:shutdown` options to `use`) — this is an Elixir convenience
  layer on top of OTP child specs (see `docs/beam/supervision.md` /
  `docs/elixir/otp-supervision.md`).
- Elixir `name()` adds `{:via, module, term}` first-class (Registry, :global)
  and `nil`; BEAM `:gen_server` accepts the same `:via` tuples.
- `handle_info/2` first arg is typed `:timeout | term()` matching BEAM's
  `Info :: timeout | term()`.

## Strict rules / @impl true usage

- `@impl true` (or `@impl GenServer`) annotates a callback clause so the
  compiler verifies it implements the behaviour contract and warns on
  mismatches/typos. Use it on every callback head (`init/1`, `handle_call/3`,
  `handle_cast/2`, `handle_continue/2`, `handle_info/2`, `terminate/2`,
  `code_change/3`, `format_status/1`).
- Never call your own `receive` inside GenServer callbacks — it breaks the
  abstraction and causes misbehaviour. Use `handle_info/2` for non-GenServer
  messages.
- Do NOT use a GenServer for code organization. Use modules/functions for code
  organization; use processes only to model runtime properties (mutable state,
  concurrency, failures). Putting pure logic (e.g. a calculator) behind a
  GenServer is an anti-pattern and a bottleneck.
- `:ignore` from `init/1` does not call `terminate/2` and does not fail/restart
  the supervisor; the child spec is retained for `Supervisor.restart_child/2`.
- `terminate/2` is not guaranteed — do critical cleanup via links/monitors.
- Turn off `:sys` debug handlers when done (`:sys.no_debug/2`); excessive
  handlers damage performance.
- For dynamic local names use `Registry` (`{:via, Registry, {...}}`), not atoms
  (atoms are never GC'd).
- `@doc` immediately preceding `use GenServer` is attached to the generated
  `child_spec/1`.

## Verbatim quotes

- "A behaviour module for implementing the server of a client-server relation."
- "GenServer supports 8 callbacks, but only init/1 is required."
- "When you use GenServer , the GenServer module will set @behaviour GenServer
  and define a child_spec/1 function, so your module can be used as a child in
  a supervision tree."
- "you should never call your own 'receive' inside the GenServer callbacks as
  doing so will cause the GenServer to misbehave."
- "A GenServer ... must be used to model runtime characteristics of your system.
  A GenServer must never be used for code organization purposes."
- "If you don't need a process, then you don't need a process. Use processes
  only to model runtime properties, such as mutable state, concurrency and
  failures, never for code organization."
- "Returning {:noreply, new_state} does not send a response to the caller and
  continues the loop with new state new_state . The response must be sent with
  reply/2 ."
- "Note that reply/2 can be called from any process, not just the GenServer
  that originally received the call (as long as that GenServer communicated the
  from argument somehow)."
- "terminate/2 is useful for cleanup that requires access to the GenServer's
  state. However, it is not guaranteed that terminate/2 is called when a
  GenServer exits."
- "If reason is neither :normal , :shutdown , nor {:shutdown, term} an error is
  logged."
- "The :via option expects a module that exports register_name/2 ,
  unregister_name/1 , whereis_name/1 and send/2 ."
- "If there is an interest to register dynamic names locally, do not use atoms,
  as atoms are never garbage-collected..."
- "Hibernating should not be used aggressively as too much time could be spent
  garbage collecting, which would delay the processing of incoming messages."
- "It's very important to switch off debugging once we're done. Excessive debug
  handlers or those that should be turned off, but weren't, can seriously
  damage the performance of the system."

## Version notes

- Page built with ExDoc v0.40.3 for Elixir v1.20.2.
- `format_status/1` callback: since Elixir 1.17.0 (maps to OTP 25.0
  `format_status/1`).
- `handle_continue/2`: present (maps to OTP 21.0+).
- `format_status/2`: deprecated; use `format_status/1`.
- `to_timeout(second: 5)` used in timeout example (Elixir 1.17+ `to_timeout/1`).
- `@impl true` / `@impl GenServer` both supported.
- Default `call/3` timeout: 5000 ms.
- `use GenServer` child_spec options: `:id` (default: module), `:restart`
  (default: `:permanent`), `:shutdown` (default: `5_000`).

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/elixir/Supervisor.html  (child_spec, shutdown values, exit reasons/restarts) → feeds otp-supervision.md
- https://hexdocs.pm/elixir/Registry.html    (:via registration, dynamic names)
- https://hexdocs.pm/elixir/Process.html     (send, monitor, send_after, spawn_opt, flag)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html  (BEAM source — already crawled as beam gen-server.md crawl 11)
- https://www.erlang.org/doc/system/gen_server_concepts.html (OTP Design Principles)
- https://hexdocs.pm/elixir/Application.html (supervision tree root)

### Skipped
- https://hexdocs.pm/elixir/Kernel.html  (send/2, self/0 — language primitives)
- https://hexdocs.pm/elixir/String.html  (split/3 — used only in example)
- https://hexdocs.pm/elixir/Path.html     (type reference only)
- ExDoc UI links (search, settings, package docs, llms.txt, ePub download)
