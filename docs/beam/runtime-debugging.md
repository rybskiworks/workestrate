# Runtime Debugging

## Purpose

Runtime debugging on the BEAM spans three layers: the `sys` protocol (inspect,
suspend, resume, code-change, and debug-log OTP behaviours and special
processes), the `trace` module (low-level trace points for calls, messages,
scheduling, GC), and the introspection BIFs (`process_info`, `system_info`,
`statistics`, `memory`) that power tools like Observer. This doc consolidates
the source-verified API surface and the strict rules that govern safe use. It
also covers the `dbg` text-based trace facility (interactive tracing) and the
`ttb` Trace Tool Builder (scripted/distributed tracing-to-disk), which are
the higher-level layers over the `trace` BIFs for interactive and distributed
use respectively.

## Sources used

- Crawl `15-sys.md` — stdlib `sys.html` — https://www.erlang.org/doc/apps/stdlib/sys.html
- Crawl `17-erlang-bifs.md` — erts `erlang.html` — https://www.erlang.org/doc/apps/erts/erlang.html
- Crawl `31-trace.md` — kernel `trace.html` — https://www.erlang.org/doc/apps/kernel/trace.html
- Crawl `37-dbg.md` — runtime_tools `dbg.html` — https://www.erlang.org/doc/apps/runtime_tools/dbg.html
- Crawl `38-ttb.md` — observer `ttb.html` — https://www.erlang.org/doc/apps/observer/ttb.html

## Core guidance

### The `sys` module — system message protocol

`sys` is "a functional interface to system messages." Standard behaviours
(`gen_server`, `gen_statem`, `gen_event`) already understand the system message
protocol; special processes / user-defined behaviours must implement the `sys`
callbacks and route `{system, From, Msg}` to `handle_system_msg/6`.

Default timeout is 5000 ms; on timeout the caller exits with
`exit({timeout, {M, F, A}})`.

Inspection / state:
- `get_state(Name) | get_state(Name, Timeout) -> State` — returns callback state.
  `gen_server`: state term. `gen_statem`: `{CurrentState,CurrentData}`.
  `gen_event`: list of `{Module, Id, HandlerState}`. Calls
  `Module:system_get_state/1` if exported. "These functions are intended only to
  help with debugging."
- `get_status(Name) | get_status(Name, Timeout) -> Status` — full status tuple:
  `{status, Pid, {module, Module}, [SItem]}` where `SItem` is process dictionary,
  `running | suspended`, parent pid, `[dbg_opt()]`, and module-specific `Misc`.
- `replace_state(Name, StateFun) | replace_state(Name, StateFun, Timeout)` —
  replaces state via `StateFun(State)`. For `gen_event`, `StateFun` runs once per
  handler; a failing handler only leaves that handler unchanged. "Not to be
  called from normal code."

Lifecycle control:
- `suspend(Name) | suspend(Name, Timeout) -> ok` — "When the process is
  suspended, it only responds to other system messages, but not other messages."
- `resume(Name) | resume(Name, Timeout) -> ok`.
- `terminate(Name, Reason) | terminate(Name, Reason, Timeout) -> ok` (OTP 18.0) —
  "The termination is done asynchronously, so it is not guaranteed that the
  process is terminated when the function returns."
- `change_code(Name, Module, OldVsn, Extra) | /5` — "The process must be
  suspended to handle this message." Calls
  `Module:system_code_change(Misc, Module, OldVsn, Extra)`.

Debug control:
- `log(Name, Flag) | log(Name, Flag, Timeout)` — `Flag :: true | {true, N} |
  false | get | print`. Keeps max `N` events (default 10).
- `log_to_file(Name, Flag) | /3` — `Flag :: FileName | false`; UTF-8 text log.
- `trace(Name, Flag) | /3 -> ok` — `Flag :: boolean()`; prints all system events
  to `standard_io`.
- `statistics(Name, Flag) | /3` — `Flag :: true | false | get`. Returns
  `{start_time, _}`, `{current_time, _}`, `{reductions, non_neg_integer()}`,
  `{messages_in, _}`, `{messages_out, _}` (or `no_statistics`).
- `install(Name, FuncSpec) | /3` — install a `dbg_fun/0`; unique `FuncId` needed
  to install the same fun multiple times.
- `remove(Name, FuncOrFuncId) | /3`.
- `no_debug(Name) | /3 -> ok` — turns off ALL debugging including `install/2,3`
  triggers.

Debug options (option-list form, e.g. `debug_options/1` and behaviour start opts):
```
trace
log
{log, N :: pos_integer()}
statistics
{log_to_file, FileName :: file:name()}
{install, {Func :: dbg_fun(), FuncState :: term()}
        | {FuncId :: term(), Func :: dbg_fun(), FuncState :: term()}}
```

Process-implementation functions (special processes call these):
- `handle_system_msg/6` — "This function never returns." `Module` must export
  `system_continue/3`, `system_terminate/4`, `system_code_change/4`,
  `system_get_state/1`, `system_replace_state/2`.
- `handle_debug/4`, `get_log/1` (OTP 22.0), `print_log/1`.
- `get_debug/3` — DEPRECATED; use `sys:get_log/1`.

### The `trace` module (OTP 27.0)

The `trace` module is the session-isolated successor to the older
`erlang:trace/3`, `erlang:trace_pattern/3`, `erlang:trace_info/2` BIFs (which
operated on a single static trace session per node, so different tools
interfered). "For debugging Erlang code it is recommended to use dbg and for
profiling to use tprof."

Session lifecycle:
- `session_create(Name, Tracer, Opts) -> session()` — `Opts` must be `[]`.
  Tracer fixed at creation; default tracer is never the calling process.
- `session_destroy/1` — cleans up settings; does NOT retract already-sent trace
  messages.
- `session_info/1`.

Process/port/function tracing:
- `process(Session, Procs, How, FlagList) -> integer()` — `Procs :: pid() | all
  | existing | new`.
- `port(Session, Ports, How, FlagList) -> integer()`.
- `function(Session, MFA, MatchSpec, FlagList) -> non_neg_integer()` — `MFA ::
  trace_pattern_mfa() | on_load`.
- `send(Session, MatchSpec, FlagList)` — matches on `[Receiver, Msg]`.
- `recv(Session, MatchSpec, FlagList)` — matches on `[Node, Sender, Msg]`.

`trace_info_flag()` (process/port flags):
```
arity | call | exiting | garbage_collection | monotonic_timestamp | procs |
ports | 'receive' | return_to | running | running_procs | running_ports |
send | set_on_first_link | set_on_first_spawn | set_on_link | set_on_spawn |
silent | strict_monotonic_timestamp | timestamp
```
Also `all` and `cpu_timestamp` (global, `Procs == all` only, not all platforms,
"time can seem to go backwards").

`trace_pattern_flag()`:
```
global | local | meta | call_count | call_time | call_memory
```
`global` (default) cannot combine with the others. `local` enables `return_to`.
`meta` traces ALL processes (fixed `[call, timestamp]`). `call_count`/`call_time`/
`call_memory` are counter-based profiling (no process trace flags needed).

Key rules:
- `function/4` call tracing requires ALSO setting the `call` flag via `process/4`.
- `send/3`/`recv/3` require `send`/`'receive'` flag; their `FlagList` MUST be `[]`.
- `return_to` only with `local`-traced functions + `call`.
- `silent` — trace many/all processes cheaply, activate selectively via
  `{silent, Bool}` match-spec action.
- Wildcard `{Module,'_',Arity}` is NOT allowed in `function/4`.
- The `trace` module is LOCAL-node only; for remote nodes use `dbg` or `ttb`.

System monitoring (`system/3`, OTP 28.0): `long_gc`, `long_message_queue`,
`long_schedule`, `large_heap`, `busy_port`, `busy_dist_port`. Session tracer
MUST be a process. "1 ms is considered a good maximum time for a driver callback
or a NIF."

### The `dbg` module — text-based trace facility (crawl 37)

`dbg` is the Text Based Trace Facility — a text/REPL-friendly front-end over
the `trace:process/4`, `trace:port/4`, and `trace:function/4` BIFs. It is
intended for interactive debugging from the Erlang shell; `ttb` is the
scripted/distributed companion. "This module implements a text based interface
to the `trace:process/4`, `trace:port/4`, and `trace:function/4` BIFs,
simplifying tracing of functions, processes, ports, and messages."

Tracer / output handler functions:
- `tracer/0` — starts a local tracer server (process) printing formatted trace
  messages to the shell. "Starts a server on the local node that will be the
  recipient of all trace messages."
- `tracer/2` — `tracer(Type, Data)`; `Type :: process | port | module | file`.
  For `process`, `Data = {HandlerFun, InitialData}` (the output handler). For
  `port`, a 0-arity fun from `trace_port/2`. For `module`,
  `{TracerModule, TracerState}` (uses `erl_tracer`). For `file`, a filename.
- `tracer/3` — `tracer(Nodename, Type, Data)` — remote node; NOT equivalent
  to `n/1`.
- `get_tracer/0,1`, `trace_port/2`, `trace_client/2,3`,
  `trace_port_control/1,2`, `flush_trace_port/0,1`.
- **CORRECTION**: there is NO `handler/0,2` function in this module. The
  "output handler" is the `HandlerSpec = {HandlerFun, InitialData}` argument
  to `tracer/2` (process type) and `trace_client/3`.

Trace process/port with flags:
- `p/1` — `p(Item)` ≡ `p(Item, [m])`.
- `p/2` — `p(Item, Flags)`. For the `c` flag: "Traces global function calls
  for the process according to the trace patterns set in the system (see
  `tp/2`)."

Trace on nodes: `n/1`, `cn/1` (always returns `ok`; does NOT stop
already-active tracing), `ln/0`.

Trace patterns (function call tracing):
- `tp/2,3,4` — GLOBAL (exported) call tracing. "tp stands for trace pattern."
  "All exported functions matching the `{Module, Function, Arity}` argument
  will be concerned, but the match specification may further narrow down the
  set of function calls generating trace messages."
- `tpl/2,3,4` — LOCAL + exported call tracing. "tpl stands for trace pattern
  local." "This function works as `tp/2`, but enables tracing for local or
  remote calls to both local and exported functions."
- `tpe/2` (since OTP 19.0) — match spec for `send`/`'receive'` events.
- `ctp/0,1,2,3` (clear both), `ctpg/0,1,2,3` (clear global only),
  `ctpl/0,1,2,3` (clear local only), `ctpe/1` (since OTP 19.0).

The `p/2` flags list: `s` (send), `r` (receive), `m` (messages), `c` (call),
`p` (procs), `ports`, `sos` (set on spawn), `sol` (set on link), `sofs`,
`sofl`, `all` (all except `silent`), `clear`. `Item` may be a pid/port, `all`,
`processes`, `ports`, `new`, `new_processes`, `new_ports`, `existing`, a
registered name, an integer, `{X,Y,Z}`, or a string.

MatchSpec form: `match_spec() :: [{match_pattern(), [_], [_]}]` — i.e.
`[{Arguments, Guard, Body}]`. Saved specs get a `{saved, N}` alias. Built-in
aliases: `x` (exception_trace), `c` (caller_trace), `cx`
(caller_exception_trace). `fun2ms/1` translates a literal fun into a match
spec via the `ms_transform` parse transform (requires
`-include_lib("stdlib/include/ms_transform.hrl").`).

Performance caveats: untargeted tracing is expensive; a trace port
"significantly lowers the overhead imposed by tracing" — "A trace port is an
Erlang port to a dynamically linked-in driver that handles trace messages
directly, without the overhead of sending them as messages to an Erlang
process. Using a trace port significantly lowers the overhead imposed by
tracing." The ip driver drops messages under heavy tracing: "In case of heavy
tracing, drops are likely to occur, and they surely occur if no client is
reading the trace messages." The file driver is highly buffered so on crash
"there is no guarantee that all are saved in the file" — "A file is written
with a high degree of buffering, which is why there is no guarantee that all
are saved in the file in case of a system crash."

Sessions (OTP 27.0, experimental): `session/2`, `session_create/1`,
`session_destroy/1`.

Strict rules: `tp/2` traces exported only; `tpl/2` traces local + exported;
`ctp` clears both, `ctpg` only `tp`, `ctpl` only `tpl`. MFA wildcard rule: if
Module is `'_'`, Function and Arity must be `'_'`. `cn/1` always returns
`ok`. `tracer/3` is NOT equivalent to `n/1`. `fun2ms/1` requires the parse
transform; fails at runtime with `badarg` without it. `stop/0` clears
everything; saved specs are lost. `session/*` is experimental.

### The `ttb` module — Trace Tool Builder (crawl 38)

ttb is "a base for building trace tools for distributed systems." It wraps
`dbg` with conveniences for distributed tracing: a file trace port on every
node, automatic log collection ("fetch") to a trace control node, a history
buffer, config save/restore, sequential-trace integration, and offline
formatting.

**CRITICAL RULE** (verbatim): "When using ttb, do not use module dbg in
application Runtime_Tools in parallel."

Key functions (exact arities — corrections applied):
- `tracer/0` ≡ `tracer(node())`; `tracer/1` (`shell | dbg | nodes()`);
  `tracer/2` (`tracer(Nodes, Opts)`) — starts a file trace port on all nodes
  and points the system tracer for sequential tracing to the same port.
- `p/2` — sets trace flags; `timestamp` is ALWAYS forced on. "Sets the
  specified trace flags on the specified processes or ports. Flag timestamp
  is always turned on."
- `tp/2,3,4` (global), `tpl/2,3,4` (local+global), `tpe/2` (messages, since
  OTP 19.0), `ctp/0,1,2,3`, `ctpl/0,1,2,3`, `ctpg/0,1,2,3`, `ctpe/1`.
- **`start_trace/4`** — `start_trace(Nodes, Patterns, FlagSpec, TracerOpts)`
  (since OTP R15B). One-command shortcut. **CORRECTION**: only `start_trace/4`
  is exported — there is NO `start_trace/3`.
- **`stop/0`** ≡ `stop([])`; **`stop/1`** — `stop(Opts)`. **CORRECTION**: only
  `stop/0` and `stop/1` are exported — there is NO `stop/2`. The fetch/return
  behavior is via `stop/1` options: `nofetch`, `{fetch, Dir}`, `format`,
  `return_fetch_dir`.
- **CORRECTION**: there is NO `trace_pattern/2` function in ttb; pattern
  setting is via `tp/tpl/tpe`.
- `format/1` ≡ `format(Files, [])`; `format/2` — `format(Files, Options)`.
  "Reads the specified binary trace log(s). The logs are processed in the
  order of their time stamps as long as option disable_sort is not specified."
- `get_et_handler/0`, `write_trace_info/2`, `seq_trigger_ms/0,1`,
  `list_history/0`, `run_history/1`, `list_config/1`, `run_config/1,2`,
  `write_config/2,3`.
- Match-spec shortcuts: `return` (for `[{'_',[],[{return_trace}]}]`), `caller`
  (for `[{'_',[],[{message,{caller}}]}]`), `{codestr, Str}`.

Tracing-to-disk + offline formatting flow (5 steps): (1) `tracer/2` starts
file trace port on all nodes; (2) `tp/tpl/tpe` set patterns (stored in
history); (3) `p/2` selects targets (timestamp forced); (4) optional
`start_trace/4` one-shot; (5) `stop/1` stops + fetches logs/`.ti` to
`ttb_upload_FileName-Timestamp`. "Stops tracing on all nodes. Logs and trace
information files are sent to the trace control node and stored in a directory
named ttb_upload_FileName-Timestamp." `stop/1` options: `nofetch`,
`{fetch, Dir}` (must not already exist), `format` (format after fetch),
`return_fetch_dir`.

Formatting: `format/1,2` options: `{out, standard_io | file:filename()}`,
`{handler, {format_fun(), InitialState}}`, `disable_sort`. Handler choices:
default (text lines), `ttb:get_et_handler()` (graphical via `et_viewer`),
custom `{Function, InitialState}`.

Relationship to dbg/seq_trace: ttb's `tp/tpl/tpe/ctp/ctpl/ctpg/ctpe` are
"equivalent to the corresponding functions in module dbg, but all calls are
stored in the history." `p/2`'s `MatchDesc` is the same as `dbg:p/2`. The
system tracer for sequential tracing is automatically initiated by ttb when a
trace port is started.

Strict rules: never use `dbg` in parallel with ttb; `p/2` always forces
`timestamp` on; `{fetch, Dir}` must not already exist; `overload_check`
disables further `p/2`/`tp` once a node is overloaded; `flush` not allowed
with `{file, {local, File}}`; `resume` requires Runtime_Tools started on
traced nodes.

### Introspection BIFs (from crawl 17)

`process_info/1` — "intended for debugging only. For all other purposes, use
process_info/2." Returns `undefined` if not alive. `process_info/2` items
(`process_info_item()`):
```
async_dist | backtrace | binary | catchlevel | current_function |
current_location | current_stacktrace | dictionary | {dictionary, Key} |
error_handler | garbage_collection | garbage_collection_info | group_leader |
heap_size | initial_call | links | label | last_calls | memory |
message_queue_len | messages | min_heap_size | min_bin_vheap_size |
monitored_by | monitors | message_queue_data | parent | priority |
priority_messages | reductions | registered_name | sequential_trace_token |
stack_size | status | suspending | total_heap_size | trace | trap_exit
```
`is_process_alive/1` — local process only; guarantees prior signals from caller
are delivered before aliveness is checked.

`system_info/1` items (subset relevant to debugging): `atom_count`,
`atom_limit`, `dirty_cpu_schedulers`, `dirty_cpu_schedulers_online`,
`dirty_io_schedulers`, `dist`, `dist_buf_busy_limit`, `ets_count`, `ets_limit`,
`process_count`, `process_limit`, `port_count`, `port_limit`, `schedulers`,
`schedulers_online`, `threads`, `thread_pool_size`, `wordsize`, plus
`allocated_areas`, `allocator`, `garbage_collection`, `start_time`, `end_time`.

`statistics/1` items: `active_tasks`, `active_tasks_all`, `context_switches`,
`exact_reductions`, `garbage_collection`, `io`, `microstate_accounting`,
`reductions`, `run_queue`, `run_queue_lengths`, `run_queue_lengths_all`,
`runtime`, `scheduler_wall_time`, `total_active_tasks`, `total_run_queue_lengths`,
`total_run_queue_lengths_all`, `wall_clock`.

`memory/0` types: `total`, `processes`, `processes_used`, `system`, `atom`,
`atom_used`, `binary`, `code`, `ets`, `maximum` (instrumented mode only).

`processes/0`, `ports/0` — list all local pids/ports (exiting ones included but
not alive).

### Reduction counting

Reductions are the BEAM's unit of fair scheduling and the primary cost metric.
- `statistics(reductions) -> {Total, Since_Last_Call}`.
- `statistics(exact_reductions)` — exact (vs estimated) counts.
- `process_info(Pid, reductions) -> {reductions, N}`.
- `sys:statistics(Name, get)` returns `{reductions, non_neg_integer()}` per
  process (plus messages_in/out and start/current time).
- `trace` `call_count`/`call_time`/`call_memory` pattern flags give per-function
  profiling without process trace flags.

### Observer and crash dumps (coverage gap)

The Observer GUI and the crash dump (`erl_crash.dump`) format are NOT described
in the crawled corpus (15/17/31). Observer consumes the BIFs above
(`process_info`, `system_info`, `statistics`, `memory`) plus ETS/app/supervisor
introspection. Crash dumps are emitted by the runtime on fatal conditions
(`+S`, `init:stop/1` with reason, system limits). Revisit: crawl the
`observer` app and ERTS "Crash Dump" chapter for source-verified detail.

## Practical rules

- `sys` default timeout 5000 ms; timeout -> `exit({timeout, {M,F,A}})`.
- `change_code/4,5` requires the process to be **suspended** first.
- A suspended process responds ONLY to system messages.
- `terminate/2,3` is asynchronous — not guaranteed terminated on return.
- `get_state`/`replace_state` are debugging-only, never normal code paths.
- `no_debug/1,2` removes ALL debugging including installed triggers.
- `trace` module is local-node only; use `dbg`/`ttb` for remote nodes.
- `function/4` call tracing needs the `call` flag set via `process/4` too.
- `cpu_timestamp` only with `Procs == all`; may go backwards across cores.
- Keep the system-monitor tracer process small; do not set limits too tight.
- `process_info/1` is debugging-only; use `process_info/2` otherwise.
- `dbg:tp/2` traces exported functions only; `dbg:tpl/2` traces local +
  exported. `ctp` clears both; `ctpg` only `tp`; `ctpl` only `tpl`.
- `dbg` has NO `handler/0,2` function — the output handler is the
  `{HandlerFun, InitialData}` arg to `tracer/2`/`trace_client/3`.
- `dbg:tracer/3` is NOT equivalent to `n/1`.
- `dbg:fun2ms/1` requires the `ms_transform` parse transform; without it,
  fails at runtime with `badarg`.
- Never use `dbg` in parallel with `ttb` — ttb owns the tracer state.
- `ttb:start_trace/4` only (no `/3`); `ttb:stop/0,1` only (no `/2`); no
  `ttb:trace_pattern/2`.
- `ttb:p/2` always forces the `timestamp` flag on.
- `ttb:stop/1` `{fetch, Dir}` — Dir must not already exist.

## Review checklist

- [ ] Are `sys` calls given an explicit timeout where 5000 ms is too short?
- [ ] Is `change_code` always preceded by `suspend`?
- [ ] Are trace sessions destroyed (`session_destroy/1`) to avoid leaks?
- [ ] Is call tracing paired with the `call` process flag?
- [ ] Are match specs replaced wholesale (not partially edited)?
- [ ] Is `process_info/2` (not `/1`) used in non-debug code?
- [ ] Is `dbg` used (not `ttb`) for interactive shell tracing, and `ttb` for
      scripted/distributed tracing?
- [ ] Is `dbg` NOT used in parallel with `ttb`?
- [ ] Are `dbg` match specs narrowed (not `tp({'_','_','_'}, ...)`) to avoid
      trace-message flooding?
- [ ] Are `ttb` arities correct (`start_trace/4`, `stop/0,1`, no
      `trace_pattern/2`)?

## Implementation checklist

- [ ] Special processes route `{system, From, Msg}` to `handle_system_msg/6`.
- [ ] `Module` exports all five `system_*` callbacks.
- [ ] Trace sessions created with `session_create/3` before any other call.
- [ ] `silent` flag used when tracing many processes, activated via match spec.
- [ ] System-monitor tracer process kept small; limits not too tight.
- [ ] `dbg:tracer/2` with a trace port (`trace_port/2`) for high-volume
      tracing (lowers overhead).
- [ ] `dbg:fun2ms/1` used with
      `-include_lib("stdlib/include/ms_transform.hrl").`.
- [ ] `ttb:tracer/2` + `ttb:tpl/*` + `ttb:p/2` + `ttb:stop/1` for distributed
      trace-to-disk.
- [ ] `ttb:format/2` with a custom handler for offline log analysis.

## Runtime / debugging checklist

- [ ] `sys:get_state/1,2` to inspect a behaviour's state.
- [ ] `sys:get_status/1,2` for full status incl. parent/debug/Misc.
- [ ] `sys:statistics(Name, get)` for per-process reductions + message counts.
- [ ] `process_info(Pid, backtrace)` for the call stack.
- [ ] `process_info(Pid, message_queue_len)` to spot overloaded mailboxes.
- [ ] `statistics(reductions)` / `scheduler_wall_time` for load balancing.
- [ ] `memory(total)` / `memory(ets)` / `memory(binary)` for memory pressure.
- [ ] `trace:system/3` for `long_schedule` / `large_heap` / `busy_port`.
- [ ] `dbg:i/0` to display all traced processes/ports.
- [ ] `dbg:stop/0` to clear ALL dbg tracing (flags, patterns, clients, ports).
- [ ] `ttb:stop/1` with `return_fetch_dir` to get the fetch directory for
      offline formatting.
- [ ] `ttb:format/1,2` to read binary trace logs (timestamp order unless
      `disable_sort`).

## Validation hooks

- `sys:get_state/1,2` — assert a behaviour's state after a transition.
- `sys:get_status/1,2` — assert `SysState` is `running` (not stuck `suspended`).
- `sys:statistics/2` — assert reductions/messages are within expected bounds.
- `process_info(Pid, status)` — assert `running`/`waiting`/`garbage_collecting`.
- `is_process_alive/1` — assert a process is up (with signal-ordering guarantee).
- `statistics(run_queue)` — assert the run queue is not backing up.
- `trace:system/3` — fail-fast on `long_schedule` > threshold in tests.
- `dbg:p/2` returns `{ok, [{matched, Node, N} | ...]}` — assert matched count.
- `dbg:tp/2` returns `{ok, match_desc()}` or `{error, _}` — assert invalid
  match specs return `{error, [{error, string()}]}`.
- `ttb:stop/1` with `return_fetch_dir` returns `{stopped, Dir}` — assert the
  fetch directory exists.
- `ttb:format/2` processes logs in timestamp order — assert ordering unless
  `disable_sort`.

## Examples

Inspect and patch a gen_server's state for debugging:
```erlang
{ok, State} = sys:get_state(my_server),
NewState = State#{debug => true},
sys:replace_state(my_server, fun(_) -> NewState end).
```

Trace all calls to a module, then activate selectively:
```erlang
{ok, S} = trace:session_create(dbg_sess, self(), []),
trace:process(S, all, true, [call, silent]),
trace:function(S, {mymod, '_', '_'}, [{['_'], [], [{silent, false}]}], [local]).
```

Suspend, code-change, resume:
```erlang
sys:suspend(my_server),
sys:change_code(my_server, my_server_mod, "1.0", []),
sys:resume(my_server).
```

dbg interactive call tracing:
```erlang
dbg:tracer(),
dbg:tp(mymod, '_', cx),
dbg:p(all, c).
```

ttb distributed trace-to-disk + offline format:
```erlang
ttb:start_trace([node(), other@host],
    [{mymod, '_', []}],
    {all, call},
    [{file, "ttb_log"}, {handler, {fun myhandler/4, []}}]),
%% ... exercise the system ...
ttb:stop([return_fetch_dir]),
{ok, Dir} = ttb:stop([return_fetch_dir]),
ttb:format(Dir, []).
```

## Common mistakes

- Calling `change_code` without `suspend` first (the process ignores it).
- Using `sys:get_state` in production logic (debugging-only).
- Forgetting `session_destroy/1` — trace settings persist on processes/ports.
- Setting `call` flag without `function/4` patterns (or vice versa).
- Using `cpu_timestamp` on multi-core without `Procs == all`.
- Relying on `process_info/1` ordering of tuples (undefined order).
- Using `dbg:handler/0,2` (does not exist — the handler is the
  `{HandlerFun, InitialData}` arg to `tracer/2`).
- Using `dbg` in parallel with `ttb` (inconsistent tracer state).
- Calling `ttb:start_trace/3` (only `/4` exists).
- Calling `ttb:stop/2` (only `/0,1` exist; fetch is via `stop/1` options).
- Calling `ttb:trace_pattern/2` (does not exist; use `tp/tpl/tpe`).
- Forgetting `dbg:fun2ms/1` needs the `ms_transform` parse transform
  (runtime `badarg` without it).
- Broad `dbg:tp({'_','_','_'}, ...)` patterns with `c` flags (trace-message
  flooding).
- Expecting the `ttb` file trace driver to retain all data on a crash (highly
  buffered; no guarantee).

## Strict vs contextual guidance

Strict:
- `change_code` requires prior `suspend`.
- `trace` module is local-node only.
- `function/4` needs `call` flag; `send/3`/`recv/3` need `send`/`'receive'`.
- `get_state`/`replace_state` are debugging-only.
- `no_debug` removes all debugging.
- `dbg:tp/2` = exported only; `dbg:tpl/2` = local + exported; `ctp`/`ctpg`/
  `ctpl` clear accordingly.
- `dbg` has no `handler/0,2`; the output handler is the `tracer/2`/
  `trace_client/3` arg.
- Never use `dbg` in parallel with `ttb`.
- `ttb` exports `start_trace/4` only (no `/3`); `stop/0,1` only (no `/2`); no
  `trace_pattern/2`.
- `ttb:p/2` always forces `timestamp` on.
- `dbg:fun2ms/1` requires the `ms_transform` parse transform.

Contextual:
- `sys` timeouts: 5000 ms default; raise for slow/remote targets.
- `silent` tracing vs per-process flag tracing — choose by blast radius.
- `statistics(exact_reductions)` vs `statistics(reductions)` — exact is costlier.
- `dbg` (interactive) vs `ttb` (scripted/distributed) — choose by use case.
- `dbg` process tracer vs trace port (port lowers overhead for high-volume
  tracing).
- `ttb` default text handler vs `get_et_handler()` (graphical) vs custom
  handler.

## Policy decisions for individual repos

- Default `sys` timeout for production debug probes (e.g. 30000 ms).
- Whether to enable `trace:system/3` monitors in staging always-on.
- Thresholds for `long_schedule` / `large_heap` monitors per workload.
- Whether `process_info` is permitted in non-debug modules.
- Whether `dbg` is permitted in production (interactive tracing can flood).
- Default `ttb` trace-port wrap settings (`{wrap, Filename, Size, Count}`).
- Whether `ttb:format` output goes to `standard_io` or a file.

## Related docs

- `proc-lib-and-sys.md` — special-process `sys` integration.
- `processes-and-messages.md` — process lifecycle BIFs.
- `links-monitors-and-exits.md` — failure signals.
- `validation.md` — cross-cutting validation hooks.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
