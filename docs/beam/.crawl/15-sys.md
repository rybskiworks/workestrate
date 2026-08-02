# Crawl: stdlib/sys.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/sys.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/sys.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib 8.0.1)
- feeds_docs: proc-lib-and-sys.md, runtime-debugging.md
## Purpose
`sys` is a functional interface to **system messages** in Erlang/OTP. It provides
the API used by programs/operators to inspect, debug, suspend, resume,
code-change, and terminate processes that understand the system message protocol.
Processes implemented as standard behaviours (`gen_server`, `gen_statem`,
`gen_event`) already understand these messages. Special processes (OTP special
processes / user-defined behaviours) must implement the `sys` callback functions
themselves and route `{system, From, Msg}` to `handle_system_msg/6`.

The module also contains the **process implementation functions** a special
process calls from its receive loop (`handle_system_msg/6`, `handle_debug/4`,
`debug_options/1`, `get_log/1`, `print_log/1`, deprecated `get_debug/3`).

Default timeout is 5000 ms unless otherwise specified. On timeout the caller
exits with `exit({timeout, {M, F, A}})`.

The debug structure is a list of `dbg_opt/0` (opaque); an empty list means no
debugging is performed.
## Key functions (exact arities + what each does)
### Inspection / state
- `get_state(Name) -> State` — equiv `get_state(Name, 5000)`. (since OTP R16B01)
- `get_state(Name, Timeout) -> State` — returns the callback module state.
  For `gen_server`: the state term. For `gen_statem`: `{CurrentState,CurrentData}`.
  For `gen_event`: list of `{Module, Id, HandlerState}` per handler (Id is `false`
  if registered without an id). Calls `Module:system_get_state/1` if exported;
  otherwise assumes `Misc` is the state. On callback crash/throw the caller exits
  with `{callback_failed, {Module, system_get_state}, {Class, Reason}}`.
- `get_status(Name) -> Status` — equiv `get_status(Name, 5000)`.
- `get_status(Name, Timeout) -> Status` — full status (see shape below).
- `replace_state(Name, StateFun) -> NewState` — equiv `replace_state(Name, StateFun, 5000)`. (since OTP R16B01)
- `replace_state(Name, StateFun, Timeout) -> NewState` — replaces state via
  `StateFun(State)`. For `gen_event`, `StateFun` is called once per handler.
  Returning `State` unchanged leaves state intact. Crashes leave original state
  unchanged (gen_server/gen_statem); for gen_event only the failing handler is
  unchanged. Calls `Module:system_replace_state/2` if exported.

### Lifecycle control
- `suspend(Name)` / `suspend(Name, Timeout) -> ok` — suspends the process. While
  suspended it only responds to other system messages, not normal messages.
- `resume(Name)` / `resume(Name, Timeout) -> ok` — resumes a suspended process.
- `terminate(Name, Reason)` / `terminate(Name, Reason, Timeout) -> ok` — (since
  OTP 18.0) orders the process to terminate with `Reason`. Termination is
  **asynchronous**; not guaranteed terminated when the function returns.
- `change_code(Name, Module, OldVsn, Extra)` — equiv `change_code(Name, Module, OldVsn, Extra, 5000)`.
- `change_code(Name, Module, OldVsn, Extra, Timeout) -> ok | {error, Reason}` —
  tells the process to change code. **The process must be suspended first.**
  Calls `Module:system_code_change(Misc, Module, OldVsn, Extra)`. `OldVsn` is the
  old version of `Module` (atom `undefined` if no vsn attribute).

### Debug control
- `log(Name, Flag)` / `log(Name, Flag, Timeout) -> ok | {ok, [system_event()]}`
  — `Flag :: true | {true, N :: pos_integer()} | false | get | print`. Turns
  logging of system events on/off; keeps max `N` events (default 10). `get`
  returns the logged events; `print` prints them to `standard_io`.
- `log_to_file(Name, Flag)` / `log_to_file(Name, Flag, Timeout) -> ok | {error, open_file}`
  — `Flag :: (FileName :: string()) | false`. Logs all system events in text
  format to file (UTF-8). Uses the process-defined format function from
  `handle_debug/4`.
- `trace(Name, Flag)` / `trace(Name, Flag, Timeout) -> ok` — `Flag :: boolean()`.
  Prints all system events on `standard_io` (formatted via `handle_debug/4`).
- `statistics(Name, Flag)` / `statistics(Name, Flag, Timeout) -> ok | {ok, Statistics}`
  — `Flag :: true | false | get`. Enables/disables statistics collection; `get`
  returns it. `Statistics :: [StatisticsTuple] | no_statistics` where tuples are
  `{start_time, DateTime1}`, `{current_time, DateTime2}`, `{reductions, non_neg_integer()}`,
  `{messages_in, non_neg_integer()}`, `{messages_out, non_neg_integer()}`.
- `install(Name, FuncSpec)` / `install(Name, FuncSpec, Timeout) -> ok` — installs
  an alternative debug function. `FuncSpec :: {Func, FuncState} | {FuncId, Func, FuncState}`.
  `Func` (a `dbg_fun/0`) is called on every system event; returns `done` (removed)
  or a new `FuncState`. To install the same function multiple times, supply a
  unique `FuncId` per installation. Removed if it fails.
- `remove(Name, FuncOrFuncId)` / `remove(Name, FuncOrFuncId, Timeout) -> ok` —
  removes an installed debug function. `Func` or `FuncId` must match a prior install.
- `no_debug(Name)` / `no_debug(Name, Timeout) -> ok` — turns off ALL debugging,
  including functions installed via `install/2,3` (e.g. triggers).

### Process implementation functions (called by special processes)
- `debug_options([Opt :: debug_option()]) -> [dbg_opt()]` — builds a debug
  structure from option list (same option values as the corresponding functions).
- `handle_system_msg(Msg, From, Parent, Module, Debug, Misc) -> no_return()` —
  called by a process module when it receives `{system, From, Msg}`. Never
  returns; calls `Module:system_continue/3` (continue) or
  `Module:system_terminate/4` (terminate). `Module` must export
  `system_continue/3`, `system_terminate/4`, `system_code_change/4`,
  `system_get_state/1`, `system_replace_state/2`. `Misc` carries internal data
  (e.g. the state) and is forwarded to continue/terminate.
- `handle_debug(Debug, FormFunc, Extra, Event) -> [dbg_opt()]` — called by a
  process when it generates a system event. `FormFunc` is `format_fun/0`, called
  as `FormFunc(Device, Event, Extra)` to print when tracing is active. `Extra`
  holds extra info for the format function (e.g. process name).
- `get_log(Debug) -> [system_event()]` — (since OTP 22.0) returns the logged
  system events in the debug structure (the last argument to `handle_debug/4`).
- `print_log(Debug) -> ok` — prints logged events using the `FormFunc` defined
  at `handle_debug/4` time.
- `get_debug(Item, Debug, Default) -> term()` — **DEPRECATED**. `Item :: log |
  statistics`. Use `sys:get_log/1` instead. Incorrectly documented, internal use.
## The get_status result shape
```
Status :: {status, Pid :: pid(), {module, Module :: module()}, [SItem]}
SItem ::
    (PDict :: [{Key :: term(), Value :: term()}]) |   % process dictionary
    (SysState :: running | suspended) |                % current sys state
    (Parent :: pid()) |                                % parent pid
    (Dbg :: [dbg_opt()]) |                             % active debug opts
    (Misc :: term())                                   % module-specific
```
The `Misc` value varies by process type:
- `gen_server` — the callback module state.
- `gen_statem` — current state name and state data.
- `gen_event` — info about each registered handler.
- bare `sys` process — the `Misc` value passed to `handle_system_msg/6`.

Callback modules for `gen_server`/`gen_statem`/`gen_event` can override `Misc`
by exporting `format_status/1` (see `gen_server:format_status/1`, etc.).

`get_state/1,2` is a convenience wrapper that extracts the state from the
`Misc` portion without forcing the caller to parse the full `get_status` tuple.
## System message/event/callback protocol
### System Messages (must be understood by any non-behaviour process)
1. **Plain system messages** — received as `{system, From, Msg}`. Content is not
   interpreted by the receiving process module; `Msg` and `From` are passed to
   `handle_system_msg/6`.
2. **Shutdown messages** — if the process traps exits, it must handle
   `{'EXIT', Parent, Reason}` from its parent (supervisor) as an order to
   terminate, normally with the same `Reason`.
3. **get_modules message** — only if modules change dynamically at runtime
   (e.g. `gen_event`). Message: `{_Label, {From, Ref}, get_modules}`. Reply:
   `From ! {Ref, Modules}` where `Modules` is the list of currently active
   modules. Used by the release handler to find processes running a module so
   they can be suspended and code-changed.

### System Events (generated by the process while debugging)
The process generates `system_event()`s which the debug functions consume
(`trace` formats them to the terminal). Four predefined events cover send/receive;
processes may define their own. The process itself always formats its own events.

`system_event()` type (events produced by `gen_server`, `gen_statem`, `gen_event`):
```
{in, Msg}
{in, Msg, State}
{out, Msg, To}
{out, Msg, To, State}
{noreply, State}
{continue, Continuation}
{postpone, Event, State, NextState}
{consume, Event, State, NextState}
{start_timer, Action, State}
{insert_timeout, Event, State}
{enter, Module, State}
{module, Module, State}
{terminate, Reason, State}
term()   % user-defined events
```
Per-event semantics:
- `{in,Msg}` — `gen_server`/`gen_event` when `Msg` arrives.
- `{in,Msg,State}` — `gen_statem` when `Msg` arrives in `State` (`Msg` is
  `{EventType,EventContent}`).
- `{out,Msg,To}` — `gen_statem` reply via `{reply,To,Msg}` action.
- `{out,Msg,To,State}` — `gen_server` reply via `{reply,...}`; `State` is new state.
- `{noreply,State}` — `gen_server` `{noreply,...}` return; new state.
- `{continue,Continuation}` — `gen_server` `{continue,...}` return.
- `{postpone,Event,State,NextState}` — `gen_statem` postpones `Event`.
- `{consume,Event,State,NextState}` — `gen_statem` consumes `Event`.
- `{start_timer,Action,State}` — `gen_statem` action starts a timer.
- `{insert_timeout,Event,State}` — `gen_statem` timeout-zero inserts `Event`.
- `{enter,Module,State}` — `gen_statem` enters first state.
- `{module,Module,State}` — `gen_statem` sets module.
- `{terminate,Reason,State}` — `gen_statem` terminates.

### Callbacks (Process Implementation Functions) — `Module` must export
- `system_code_change(Misc, Module, OldVsn, Extra) -> {ok, NMisc}` — called from
  `handle_system_msg/6` to perform code change; converts `Misc` to new structure.
  `OldVsn` is the `vsn` attribute of old `Module` (atom `undefined` if none).
- `system_continue(Parent, Debug, Misc) -> no_return()` — continue execution
  (e.g. after suspend). Never returns.
- `system_get_state(Misc) -> {ok, State}` — (since OTP 17.0) return a term
  reflecting current state. `State` is what `get_state/2` returns.
- `system_replace_state(StateFun, Misc) -> {ok, NState, NMisc}` — (since OTP
  17.0) replace state using `StateFun`; `NState` is what `replace_state/3` returns.
- `system_terminate(Reason, Parent, Debug, Misc) -> no_return()` — terminate;
  gives the process a chance to clean up. Never returns.

### Integration with behaviours and special processes
`gen_server`, `gen_statem`, `gen_event` already export `system_get_state/1` and
`system_replace_state/2`, so their callback modules need not supply them. These
callbacks are primarily for **user-defined behaviours** and modules implementing
**OTP special processes**. A special process receives `{system, From, Msg}` and
delegates to `sys:handle_system_msg/6`, passing its `Module` (which must export
the five callbacks above) and its `Misc` (typically its state).
## Debug options
`debug_option()` (the option list form, e.g. passed to `debug_options/1` and to
behaviour `start` opts):
```
trace
log
{log, N :: pos_integer()}
statistics
{log_to_file, FileName :: file:name()}
{install, {Func :: dbg_fun(), FuncState :: term()}
        | {FuncId :: term(), Func :: dbg_fun(), FuncState :: term()}}
```
- `trace` — print all system events to `standard_io`.
- `log` / `{log,N}` — keep last N system events (default 10) in the debug structure.
- `statistics` — collect reductions/messages-in/messages-out and start/current time.
- `{log_to_file,FileName}` — text-format all events to a UTF-8 file.
- `{install,...}` — install a custom `dbg_fun/0` trigger.

`dbg_fun()` type:
```
fun((FuncState, Event :: system_event(), ProcState) -> done | NewFuncState)
```
`format_fun()` type (not exported):
```
fun((Device :: io:device() | file:io_device(), Event :: system_event(), Extra) -> any())
```
`name()` type: `pid() | atom() | {global, term()} | {via, module(), term()}`.

`get_items`-style retrieval: use `sys:get_log/1` to pull the logged events list
out of a `Debug` structure; `sys:get_debug/3` (deprecated) retrieved `log` or
`statistics` items by tag with a default.
## Strict rules
- Default timeout is 5000 ms; on timeout the caller exits with
  `exit({timeout, {M, F, A}})`.
- `change_code/4,5` requires the process to be **suspended** first.
- A suspended process responds ONLY to other system messages, not normal messages.
- `terminate/2,3` is **asynchronous** — termination is not guaranteed when it returns.
- `get_state/1,2` and `replace_state/2,3` are for **debugging only**, not normal code.
- `replace_state` `StateFun` may return its `State` argument unchanged to no-op.
- For `gen_event`, `replace_state`'s `StateFun` is called once per registered
  handler; a failing `StateFun` only leaves that handler's state unchanged.
- If a callback's `system_get_state/1` / `system_replace_state/2` crashes, the
  caller exits `{callback_failed, {Module, Fn}, {Class, Reason}}`; if no such
  callback exists and `StateFun` crashes, exits
  `{callback_failed, StateFun, {Class, Reason}}`.
- `no_debug/1,2` removes ALL debugging including `install/2,3` triggers.
- To install the same `dbg_fun` multiple times, each install needs a unique `FuncId`.
- `handle_system_msg/6` never returns; `Module` must export `system_continue/3`,
  `system_terminate/4`, `system_code_change/4`, `system_get_state/1`,
  `system_replace_state/2`.
- `get_debug/3` is deprecated; use `get_log/1`.
## Verbatim quotes
- "A functional interface to system messages."
- "The default time-out is 5000 ms, unless otherwise specified. `timeout`
  defines the time to wait for the process to respond to a request. If the
  process does not respond, the function evaluates `exit({timeout, {M, F, A}})`."
- "The debug structure is a list of `dbg_opt/0`, which is an internal data type
  used by function `handle_system_msg/6`. No debugging is performed if it is an
  empty list."
- "Processes that are not implemented as one of the standard behaviors must still
  understand system messages."
- "When the process is suspended, it only responds to other system messages, but
  not other messages."
- "The process must be suspended to handle this message." (change_code)
- "The termination is done asynchronously, so it is not guaranteed that the
  process is terminated when the function returns." (terminate)
- "These functions are intended only to help with debugging." (get_state)
- "These functions are intended only to help with debugging, and are not to be
  called from normal code." (replace_state)
- "Function `system_get_state/1` is primarily useful for user-defined behaviors
  and modules that implement OTP special processes."
- "This function never returns. It calls either of the following functions:
  `Module:system_continue(Parent, NDebug, Misc)` ... or
  `Module:system_terminate(Reason, Parent, Debug, Misc)`."
- "`Module` must export the following: `system_continue/3`, `system_terminate/4`,
  `system_code_change/4`, `system_get_state/1`, `system_replace_state/2`."
- "sys:get_debug/3 is deprecated; incorrectly documented and only for internal
  use. Can often be replaced with sys:get_log/1."
## Version notes
- Page meta: stdlib v8.0.1, OTP 29.0.2 (major-vsn 29). ExDoc v0.40.3.
- `get_state/1,2` since OTP R16B01.
- `replace_state/2,3` since OTP R16B01.
- `system_get_state/1` callback since OTP 17.0.
- `system_replace_state/2` callback since OTP 17.0.
- `terminate/2,3` since OTP 18.0.
- `get_log/1` since OTP 22.0.
- `get_debug/3` deprecated.
- Copyright 1996-2026 Ericsson AB.
## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html
- https://www.erlang.org/doc/apps/stdlib/gen_statem.html
- https://www.erlang.org/doc/apps/stdlib/gen_event.html
- https://www.erlang.org/doc/apps/stdlib/io.html
- https://www.erlang.org/doc/apps/erts/erlang.html
- https://www.erlang.org/doc/apps/kernel/file.html
- https://www.erlang.org/doc/apps/stdlib/sys.md (llms-friendly source)
### Skipped
- In-page anchors (#t:..., #c:..., #function/arity, #module-..., #summary, #types, #callbacks-..., #process-implementation-functions)
- GitHub source links (erlang/otp blob OTP-29.0.2 lib/stdlib/src/sys.erl#L...)
- ../../index.html, llms.txt, stdlib.epub, ex_doc, erlang.org, ericsson.com
- CSS/asset links
