# Crawl: runtime_tools/dbg.html
- seed_url: https://www.erlang.org/doc/apps/runtime_tools/dbg.html
- canonical_url: https://www.erlang.org/doc/apps/runtime_tools/dbg.html
- family: Erlang/OTP runtime_tools module docs
- fetch: 200
- otp_version: OTP 29.0.2 (runtime_tools v2.4)
- feeds_docs: runtime-debugging.md

## Purpose
`dbg` is the Text Based Trace Facility — a text/REPL-friendly front-end over the
`trace:process/4`, `trace:port/4`, and `trace:function/4` BIFs. It simplifies
tracing of functions, processes, ports, and messages by managing a tracer
process/port/module, a list of traced nodes, trace flags on processes/ports,
and trace patterns (match specifications) on functions and send/'receive'
events. Intended for interactive debugging from the Erlang shell; the companion
`ttb` (Trace Tool Builder) builds on the same primitives for scripted tracing.

## Key functions (exact arities)
Tracer / output handler:
- `tracer/0` — starts a local tracer server (process) that prints formatted
  trace messages to the shell via `io:format/2`. Returns `{ok, Pid} | {error, already_started}`.
- `tracer/2` — `tracer(Type, Data)` starts a tracer with a custom sink.
  `Type :: process | port | module | file`.
  - `process` → `Data = {HandlerFun, InitialData}` where
    `HandlerFun :: fun((Event, Data) -> NewData)`. This is the **output handler**.
  - `port` → `Data` is a 0-arity fun returning a port (from `trace_port/2`).
  - `module` → `Data` is `{TracerModule, TracerState}` or a fun returning it
    (uses the `erl_tracer` callback module).
  - `file` → `Data` is a filename to print traces to.
- `tracer/3` — `tracer(Nodename, Type, Data)` — same as `tracer/2` but on a
  remote node; adds the node to the traced-node list. NOT equivalent to `n/1`
  (which only starts a process tracer forwarding to the local control node).
- `get_tracer/0,1` — returns the process/port/module receiving trace messages.
- `trace_port/2` — `trace_port(Type, Parameters)` builds a port-generating fun
  for `tracer/2`. `Type :: file | ip`. `file` writes binary trace files
  (supports wrap specs); `ip` opens a TCP listener.
- `trace_client/2,3` — reads output from a trace port (file/follow_file/ip) and
  formats it; `/3` accepts a custom `handler_spec()`.
- `trace_port_control/1,2` — control ops on the active trace port driver
  (`flush`, `get_listen_port`).
- `flush_trace_port/0,1` — `= trace_port_control(Nodename, flush)`.

Note: there is **no `handler/0,2` function** in this module. The "output
handler" is the `HandlerSpec = {HandlerFun, InitialData}` argument to
`tracer/2` (process) and `trace_client/3`. The handler fun receives each trace
event plus its own rolling state.

Trace process/port with flags:
- `p/1` — `p(Item)` ≡ `p(Item, [m])`.
- `p/2` — `p(Item, Flags)` — enables trace flags on a process/port/group.
  Returns `{ok, [{matched, Node, N} | {matched, Node, 0, RPCError}]} | {error, _}`.

Trace on nodes:
- `n/1` — `n(Nodename)` — adds a remote node to the traced-node list; starts a
  process tracer on it forwarding to the local tracer. Errors:
  `no_local_tracer`, `cant_add_local_node`, `cant_trace_remote_pid_to_local_port`.
- `cn/1` — `cn(Nodename)` — clears a node from the list (already-active
  tracing on it continues). Always returns `ok`.
- `ln/0` — lists traced nodes on the console.

Trace patterns (function call tracing):
- `tp/2` — `tp(ModuleOrMFA, MatchSpec)` — enables **global** (exported) call
  tracing. `ModuleOrMFA :: Module | {Module, Function, Arity}` with `'_'`
  wildcards (Module `'_'` ⇒ Function/Arity must also be `'_'`). Returns
  `{ok, match_desc()} | {error, _}`; may include `{saved, N}` alias.
- `tp/3` — `tp(Module, Function, MatchSpec)` ≡ `tp({M,F,'_'}, MatchSpec)`.
- `tp/4` — `tp(Module, Function, Arity, MatchSpec)` ≡ `tp({M,F,A}, MatchSpec)`.
- `tpl/2` — `tpl({Module, Function, Arity}, MatchSpec)` — **local** call
  tracing: traces both local and exported functions, local or remote calls.
- `tpl/3` — `tpl(Module, Function, MatchSpec)`.
- `tpl/4` — `tpl(Module, Function, Arity, MatchSpec)`.
- `tpe/2` — `tpe(Event, MatchSpec)` (since OTP 19.0) — match spec for
  `send` / `'receive'` events. `send` matches `[Receiver, Msg]`; `'receive'`
  matches `[Node, Sender, Msg]`; `self/0` available in guards.
- `ctp/0,1,2,3` — clear trace pattern (both local and global).
- `ctpg/0,1,2,3` — clear global only (set by `tp/2`).
- `ctpl/0,1,2,3` — clear local only (set by `tpl/2`).
- `ctpe/1` (since OTP 19.0) — clear match spec for send/'receive', reverting
  to tracing all triggered events.

Trace pattern log (saved match specs):
- `ltp/0` — lists all saved match specs + built-in aliases.
- `dtp/0` — forgets all saved match specs.
- `dtp/1` — `dtp(N)` forgets a specific saved spec by `tp_id()`.
- `wtp/1` — `wtp(Name)` writes saved specs (+ built-ins) to a text file.
- `rtp/1` — `rtp(Name)` reads/merges specs from a file (syntax-verified;
  atomic — none added if any error).

Built-in match spec aliases (`built_in_alias() :: x | c | cx`):
- `x` = `exception_trace` — function name, params, return value, exceptions.
- `c` = `caller_trace` — function name, params, and caller info.
- `cx` = `caller_exception_trace` — combines `x` and `c`.

Sessions (since OTP 27.0, experimental):
- `session/2` — `session(Session, Fun)` runs dbg calls in an isolated session.
- `session_create/1` — `session_create(Name)` creates a named session.
- `session_destroy/1` — destroys a session and its `trace:session/0`.

Control / introspection:
- `stop/0` — stops the dbg server, clears all trace flags, all trace patterns,
  send/'receive' patterns, all trace clients, and all trace ports.
- `stop_trace_client/1` — shuts down a trace client by pid.
- `i/0` — displays info about all traced processes and ports.
- `h/0,1` — online help.
- `c/3,4` — `c(Mod, Fun, Args[, Flags])` — `apply(Mod, Fun, Args)` with trace
  flags set; convenience for shell tracing.
- `fun2ms/1` — parse-transform pseudo-function converting a literal fun into a
  match spec (requires `ms_transform.hrl`).

## p/2 flags list
`Flags` may be a single atom or a list. Available flags:
- `s` (send) — trace messages the process/port sends.
- `r` (receive) — trace messages the process/port receives.
- `m` (messages) — trace both send and receive.
- `c` (call) — trace global function calls per the system's trace patterns (`tp/2`).
- `p` (procs) — trace process-related events.
- `ports` — trace port-related events.
- `sos` (set on spawn) — children inherit the traced process's flags.
- `sol` (set on link) — a linked process inherits the flags.
- `sofs` (set on first spawn) — like `sos` but only the first spawn.
- `sofl` (set on first link) — like `sol` but only the first `link/1`.
- `all` — sets all flags except `silent`.
- `clear` — clears all flags.
- Plus any flags allowed in `trace:process/4` and `trace:port/4`.

`Item` for `p/2` may be: `pid()|port()`; `all`; `processes`; `ports`; `new`;
`new_processes`; `new_ports`; `existing`; `existing_processes`; `existing_ports`;
a registered name (`atom()`); an `integer()` (→ `<0.Item.0>`); `{X,Y,Z}`;
or a `string()` from `pid_to_list/1`. Group items are enabled on all nodes
added via `n/1`/`tracer/3`.

## tp/ctp trace patterns + MatchSpec form
- `tp` sets **global** call tracing (exported functions only); `tpl` sets
  **local** call tracing (local + exported functions, local + remote calls).
- MFA wildcards: `'_'` for Module/Function/Arity; if Module is `'_'`, Function
  and Arity must also be `'_'`; same constraint for Function→Arity.
- `MatchSpec :: tp_match_spec() :: tp_id() | built_in_alias() | [] | match_spec()`.
- `match_spec() :: [{match_pattern(), [_], [_]}]` — i.e. the dbg/trace form is
  `[{Arguments, Guard, Body}]`:
  - `Arguments` (`match_pattern() :: atom() | list()`) — pattern matched
    against the function's argument list.
  - `Guard` — list of guard tests.
  - `Body` — list of actions (e.g. `[{return_trace}]`, `[{message, ...}]`).
- Saved specs get a `{saved, N}` alias (`tp_id() :: pos_integer()`) reusable in
  later `tp`/`tpl` calls.
- Invalid match specs return `{error, [{error, string()}]}` with textual
  compile errors (e.g. wrong arity of special form, unknown function).
- `fun2ms/1` translates a literal fun into a match spec via the `ms_transform`
  parse transform (`-include_lib("stdlib/include/ms_transform.hrl").`). Fun
  head must be a single pattern matching a list; only guard expressions and the
  special tracing functions are allowed in the body. Bit-syntax matching and
  map updates in guards are NOT supported.

## Relationship to trace/trace_pattern BIFs + ttb
- dbg is a **wrapper** over the `trace` and `trace_pattern` BIFs (now exposed
  as `trace:process/4`, `trace:port/4`, `trace:function/4`). It manages:
  - a tracer process/port/module that receives `trace` messages;
  - the list of traced nodes (forwarding via the Erlang distribution);
  - per-process/port trace flags (via `p/2` → `trace:process/4`/`trace:port/4`);
  - function trace patterns with match specs (via `tp/tpl` →
    `trace:function/4` / `trace_pattern`), including saved-spec aliases and
    built-in aliases (`x`/`c`/`cx`);
  - send/'receive' match specs (via `tpe/2`).
- `tracer/2` `Type=module` delegates to an `erl_tracer` callback module.
- `trace_port/2` produces a port-based tracer that bypasses Erlang-message
  overhead by handing trace messages to a linked-in driver (file/ip).
- `ttb` (Trace Tool Builder, same `runtime_tools` app) is the higher-level
  scripted/automated layer over the same tracing primitives; dbg is the
  interactive layer. (dbg_guide.html is the "Tracing in Erlang with dbg" users
  guide.)

## Performance caveats
- **Untargeted tracing is expensive.** Tracing `all` processes/ports, or
  setting broad patterns (`tp({'_','_','_'}, ...)`) with `c` (call) flags,
  generates a trace message per matching event across the whole system. Each
  traced call/send/receive produces a message to the tracer process, which
  serializes formatting and I/O — this can dominate runtime and, in the
  process-tracer case, the tracer process becomes a bottleneck.
- A **trace port** (`tracer(port, dbg:trace_port(...))`) "significantly lowers
  the overhead imposed by tracing" because trace messages go directly to a
  linked-in driver instead of being sent as Erlang messages to a process.
- The **ip** trace driver has a fixed queue (`QueSize`, default 200); under
  heavy tracing or with no client reading, messages are **dropped** and a
  `{drop, N}` pseudo-message reports the count. Drops are "likely" under heavy
  tracing.
- The **file** trace driver is highly buffered, so on a system crash there is
  "no guarantee that all are saved in the file".
- Match specs narrow which calls generate trace messages; using them (and
  targeting specific MFAs / specific pids) is the primary mitigation.
- Remote-node tracing via `n/1` ships all trace messages over the Erlang
  distribution to the local tracer — additional network/serialization cost.

## Strict rules
- `tp/2` traces **exported** functions only (global); `tpl/2` traces local
  **and** exported functions. `ctp` clears both; `ctpg` clears only `tp`;
  `ctpl` clears only `tpl`.
- MFA wildcard rule: if Module is `'_'`, Function and Arity must be `'_'`;
  if Function is `'_'`, Arity must be `'_'`.
- `cn/1` cannot fail and always returns `ok`; it does NOT stop already-active
  tracing on the cleared node.
- `n/1` errors: `no_local_tracer` (no local tracer running), `cant_add_local_node`
  (Nodename is local), `cant_trace_remote_pid_to_local_port` (a trace port is
  running locally — use `tracer/3` to start a port on the remote node instead),
  and unreachable-node errors.
- `tracer/3` is NOT equivalent to `n/1`: `n/1` starts a process tracer that
  forwards to the local control-node tracer; `tracer/3` starts any tracer type
  on the remote node independently of the local tracer type.
- `p/2` with a specific `pid()`/`port()` runs only on the node where that
  process/port resides; group items (`all`, `processes`, ...) run on all traced
  nodes.
- `tpe/2` Event has no wildcards; matched count is always 1.
- `rtp/1` is atomic: if any match spec in the file is syntactically invalid,
  none are added.
- `fun2ms/1` requires the `ms_transform` parse transform; without it the call
  fails at **runtime** with `badarg`, not at compile time. The fun must be
  written literally as the argument (not via a variable).
- `stop/0` clears everything (flags, patterns, send/'receive' patterns, trace
  clients, trace ports). Saved match specs are lost on `stop/0`.
- `session/*` (OTP 27.0) is **experimental** and may change without notice.

## Verbatim quotes
- "This module implements a text based interface to the `trace:process/4`,
  `trace:port/4`, and `trace:function/4` BIFs, simplifying tracing of
  functions, processes, ports, and messages."
- `tracer/0`: "Starts a server on the local node that will be the recipient of
  all trace messages."
- `p/2` `c` flag: "Traces global function calls for the process according to
  the trace patterns set in the system (see `tp/2`)."
- `tp/2`: "tp stands for trace pattern." / "All exported functions matching the
  `{Module, Function, Arity}` argument will be concerned, but the match
  specification may further narrow down the set of function calls generating
  trace messages."
- `tpl/2`: "tpl stands for trace pattern local." / "This function works as
  `tp/2`, but enables tracing for local or remote calls to both local and
  exported functions."
- `trace_port/2`: "A trace port is an Erlang port to a dynamically linked-in
  driver that handles trace messages directly, without the overhead of sending
  them as messages to an Erlang process. Using a trace port significantly
  lowers the overhead imposed by tracing."
- ip driver: "In case of heavy tracing, drops are likely to occur, and they
  surely occur if no client is reading the trace messages."
- file driver: "A file is written with a high degree of buffering, which is why
  there is no guarantee that all are saved in the file in case of a system
  crash."
- `tracer/3`: "This function is not equivalent to `n/1`. While `n/1` starts a
  process tracer which redirects all trace information to a process tracer on
  the local node ... `tracer/3` starts any type of tracer, independent of the
  type of tracer on the trace control node."
- `match_spec()` type: `[{match_pattern(), [_], [_]}]`.
- `fun2ms/1`: "Pseudo function that by means of a parse transform translates
  the literal fun typed as parameter in the function call to a match
  specification."

## Version notes
- Page metadata: Erlang/OTP 29.0.2; module `dbg` from `runtime_tools v2.4`.
- `ctpe/1`, `tpe/2`: since OTP 19.0.
- `session/2`, `session_create/1`, `session_destroy/1`: since OTP 27.0
  (experimental — "may change in future releases without notice").
- `session_create/1` links the session to the calling process.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/runtime_tools/dbg_guide.html — "Tracing in
  Erlang with dbg" users guide (quick-start for function-call tracing).
- https://www.erlang.org/doc/apps/erts/match_spec.html — Match Specifications
  in Erlang (the match spec language referenced by `tp/2`/`tpe/2`).
- https://www.erlang.org/doc/apps/erts/match_spec.html#functions-allowed-only-for-tracing
  — special tracing-only functions usable in match spec bodies.
- https://www.erlang.org/doc/apps/erts/erl_tracer.html — `erl_tracer` callback
  module (used by `tracer/2` `Type=module`).
- https://www.erlang.org/doc/apps/stdlib/ms_transform.html — `ms_transform`
  parse transform (backs `fun2ms/1`).
- https://www.erlang.org/doc/apps/kernel/trace.html#process/4 — `trace:process/4` BIF.
- https://www.erlang.org/doc/apps/kernel/trace.html#port/4 — `trace:port/4` BIF.
- https://www.erlang.org/doc/apps/kernel/trace.html#function/4 — `trace:function/4` BIF.
- https://www.erlang.org/doc/apps/kernel/trace.html#t:session/0 — `trace:session/0`
  (underlying session type for `dbg:session/0`).
- https://www.erlang.org/doc/apps/kernel/rpc.html — `rpc` (used for remote-node
  `tp`/`p` calls; surfaces as `{matched, Node, 0, RPCError}`).

### Skipped
- https://www.erlang.org/doc/apps/runtime_tools/dbg.md (raw markdown mirror).
- https://www.erlang.org/doc/apps/runtime_tools/runtime_tools.epub
- https://www.erlang.org/doc/apps/runtime_tools/llms.txt
- https://www.erlang.org/doc/index.html
- https://erlang.org , https://www.ericsson.com , https://github.com/elixir-lang/ex_doc
- CSS/asset links.
