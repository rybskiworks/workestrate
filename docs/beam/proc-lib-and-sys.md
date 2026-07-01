# proc_lib and sys

## Purpose

`proc_lib` and `sys` are the two STDLIB modules that together form the foundation for OTP-compliant processes that are *not* one of the standard behaviours. `proc_lib` provides asynchronous and synchronous start of processes adhering to the OTP design principles; it is the layer the OTP standard behaviours (`gen_server`, `gen_statem`, `gen_event`, `supervisor`, `supervisor_bridge`) are built on when starting new processes, and the recommended foundation for user-defined **special processes**. `sys` is the functional interface to **system messages** — the API used to inspect, debug, suspend, resume, code-change, and terminate processes that understand the system-message protocol.

This doc consolidates the special-process contract, the `proc_lib` start/handshake/crash-report layer, and the `sys` debug/lifecycle/callback protocol into one source-verified reference.

## Sources used

- Crawl file: `docs/beam/.crawl/10-spec-proc.md` — canonical URL: https://www.erlang.org/doc/system/spec_proc.html (OTP 29.0.2)
- Crawl file: `docs/beam/.crawl/14-proc-lib.md` — canonical URL: https://www.erlang.org/doc/apps/stdlib/proc_lib.html (stdlib v8.0.1, OTP 29.0.2)
- Crawl file: `docs/beam/.crawl/15-sys.md` — canonical URL: https://www.erlang.org/doc/apps/stdlib/sys.html (stdlib v8.0.1, OTP 29.0.2)

## Core guidance

A **special process** is a hand-written process that obeys the OTP design principles without using `gen_server`/`gen_statem`/`gen_event`. Verbatim from crawl 10:

> "Such a process is to: * Be started in a way that makes the process fit into a supervision tree * Support the sys debug facilities * Take care of system messages."

`proc_lib` is the foundation for writing one. Verbatim from crawl 14:

> "Functions for asynchronous and synchronous start of processes adhering to the OTP design principles."

> "This module is used to start processes adhering to the OTP Design Principles. Specifically, the functions in this module are used by the OTP standard behaviors (for example, gen_server and gen_statem) when starting new processes. The functions can also be used to start special processes, user-defined processes that comply to the OTP design principles."

`sys` is the partner layer. Verbatim from crawl 15:

> "A functional interface to system messages."

> "Processes that are not implemented as one of the standard behaviors must still understand system messages."

When a process starts via `proc_lib`, useful information is initialized and stored on the process dictionary: the registered name (or pid) of the **parent** process; the parent **ancestors**; and information about the **function initially called** in the process. This stored metadata is what makes crash reports, `initial_call/1`, and `translate_initial_call/1` work — none of which raw `erlang:spawn*` provides.

### What proc_lib adds over raw `erlang:spawn*`

Verbatim from crawl 14 ("## Strict rules" → "### What proc_lib adds over raw erlang:spawn*"):

1. **Stored process metadata** — parent pid/registered name, ancestors, and the initial function are recorded on process start. This powers crash reports, `initial_call/1`, and `translate_initial_call/1`.
2. **Broadened "normal" termination** — a proc_lib process is considered to terminate *normally* not only for exit reason `normal`, but also for `shutdown` and `{shutdown, Term}`. Verbatim:

   > "While in 'plain Erlang', a process is said to terminate normally only for exit reason normal, a process started using proc_lib is also said to terminate normally if it exits with reason shutdown or {shutdown,Term}. shutdown is the reason used when an application (supervision tree) is stopped."

3. **Automatic crash reports** — when a proc_lib process terminates abnormally (any reason other than `normal`, `shutdown`, or `{shutdown, Term}`), a crash report is generated and written to the terminal by the default logger handler set up by Kernel. The report contains the stored info (ancestors, initial function), the termination reason, and info about other processes that terminated as a result.
4. **No emulator error reports** — unlike plain Erlang, proc_lib processes do *not* generate the emulator's own error reports to the terminal. All exceptions are converted to exits which are ignored by the default logger handler.
5. **Synchronous start** — `start/3,4,5`, `start_link/3,4,5`, `start_monitor/3,4,5` spawn a process and *wait* for it to acknowledge via `init_ack/1,2` (or fail via `init_fail/2,3`). Raw `spawn` returns immediately with a pid and no knowledge of whether the child initialized.

## Practical rules

### proc_lib

- Start special processes through `proc_lib` (`spawn_link` or `start_link`/`start_monitor`), never plain `spawn`, so ancestor/initial-call metadata and crash reports are correct.
- All initialization (including name registration) MUST be done in `init/1`.
- The child MUST acknowledge start via `proc_lib:init_ack(Parent, {ok, self()})` (or `init_fail` on failure). `proc_lib:start_link/3` is synchronous and does not return until `init_ack/1,2` or `init_fail/2,3` has been called, or the process has exited.
- Do NOT use `init_ack` to signal a failed start. Verbatim from crawl 14:

  > "Do not use this function to return an error indicating that the process start failed. When doing so the start function can return before the failing process has exited, which may block VM resources required for a new start attempt to succeed. Use init_fail/2,3 for that purpose."

- Do NOT catch the exception raised by `init_fail`. Verbatim from crawl 14:

  > "Do not consider catching the exception from this function. That would defeat its purpose. A process started by a start[_link|_monitor]/3,4,5 function should end in a value (that will be ignored) or an exception that will be handled by this module."

- `init_fail/2,3` (OTP 26.0+) tells Parent init failed and immediately raises `Exception`; the start function then returns `Return`.
- Hibernate via `proc_lib:hibernate/3`, never the raw BIF. Verbatim from crawl 14:

  > "Always use this function instead of the BIF for processes started using proc_lib functions."

- `start_spawn_option()` is a restricted set: `link | {priority, _} | {fullsweep_after, _} | {min_heap_size, _} | {min_bin_vheap_size, _} | {max_heap_size, _} | {message_queue_data, _}`. `monitor` is NOT allowed in `start/5`, `start_link/5`, `start_monitor/5` SpawnOpts — it causes `badarg`.
- `start_link` exit-propagation: if the started process is killed or crashes with a reason other than `normal`, the link kills the calling process, so `start_link/5` does NOT return — unless the caller traps exits. When the caller traps exits and `start_link/5` returns due to the spawned process exiting, the function receives (consumes) the `'EXIT'` message — also when it times out and kills the spawned process.
- `start_monitor` DOWN-message rule: `start_monitor/3,4,5` returns `{Ret, Mon}`. If it returns due to the spawned process exiting (any error value), a `'DOWN'` message WILL be delivered to the calling process — also when it times out and kills the spawned process.
- `stop/1,3` (since OTP 18.0) orders the process to exit with `Reason` and waits for it to terminate. Returns `ok` on success; raises `timeout` on timeout; raises `noproc` if the process does not exist. Verbatim from crawl 14:

  > "The implementation of this function is based on the terminate system message, and requires that the process handles system messages correctly."

- `initial_call/1` returns `{Module, Function, Args} | false`. `Args` is now a list of placeholder atoms (`Argument__1`, `Argument__2`, ...) not the real arguments (memory + code-upgrade safety).
- `translate_initial_call/1` translates the initial call to more useful info; defaults to `{proc_lib, init_p, 5}` when no info is found. Used by `c:i/0` and `c:regs/0`.
- `set_label/1`, `get_label/1` (OTP 27.0+) — set/get a label for the current process; any term; used in tools and crash reports.
- `format/1,2,3` are deprecated for new Logger handlers (the `report_cb` callback is included as log-event metadata). They exist for legacy `error_logger` event handlers.

### sys

- Default timeout is 5000 ms; on timeout the caller exits with `exit({timeout, {M, F, A}})`.
- The debug structure is a list of `dbg_opt/0` (opaque); an empty list means no debugging is performed.
- `change_code/4,5` requires the process to be **suspended** first. Verbatim from crawl 15: "The process must be suspended to handle this message."
- A suspended process responds ONLY to other system messages, not normal messages.
- `terminate/2,3` is **asynchronous** — termination is not guaranteed when it returns.
- `get_state/1,2` and `replace_state/2,3` are for **debugging only**, not normal code.
- `no_debug/1,2` removes ALL debugging including `install/2,3` triggers.
- To install the same `dbg_fun` multiple times, each install needs a unique `FuncId`.
- `handle_system_msg/6` never returns; `Module` must export `system_continue/3`, `system_terminate/4`, `system_code_change/4`, `system_get_state/1`, `system_replace_state/2`.
- `get_debug/3` is deprecated; use `get_log/1`.

### The `get_status` result shape (verbatim from crawl 15)

```
Status :: {status, Pid :: pid(), {module, Module :: module()}, [SItem]}
SItem ::
    (PDict :: [{Key :: term(), Value :: term()}]) |   % process dictionary
    (SysState :: running | suspended) |                % current sys state
    (Parent :: pid()) |                                % parent pid
    (Dbg :: [dbg_opt()]) |                             % active debug opts
    (Misc :: term())                                   % module-specific
```

The `Misc` value varies by process type: `gen_server` — the callback module state; `gen_statem` — current state name and state data; `gen_event` — info about each registered handler; bare `sys` process — the `Misc` value passed to `handle_system_msg/6`. Callback modules for `gen_server`/`gen_statem`/`gen_event` can override `Misc` by exporting `format_status/1`. `get_state/1,2` is a convenience wrapper that extracts the state from the `Misc` portion without forcing the caller to parse the full `get_status` tuple.

### The `system_event()` type (verbatim from crawl 15)

Events produced by `gen_server`, `gen_statem`, `gen_event`:

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
- `{in,Msg,State}` — `gen_statem` when `Msg` arrives in `State` (`Msg` is `{EventType,EventContent}`).
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

### The `debug_option()` type (verbatim from crawl 15)

The option list form, passed to `debug_options/1` and to behaviour `start` opts:

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

### The system-message protocol (from crawl 15)

1. **Plain system messages** — received as `{system, From, Msg}`. Content is NOT interpreted by the receiving process module; `Msg` and `From` are passed to `handle_system_msg/6`.
2. **Shutdown messages** — if the process traps exits, it must handle `{'EXIT', Parent, Reason}` from its parent (supervisor) as an order to terminate, normally with the same `Reason`.
3. **get_modules message** — only if modules change dynamically at runtime (e.g. `gen_event`). Message: `{_Label, {From, Ref}, get_modules}`. Reply: `From ! {Ref, Modules}` where `Modules` is the list of currently active modules. Used by the release handler to find processes running a module so they can be suspended and code-changed.

### sys callbacks `Module` must export (from crawl 15)

- `system_code_change(Misc, Module, OldVsn, Extra) -> {ok, NMisc}` — called from `handle_system_msg/6` to perform code change; converts `Misc` to new structure. `OldVsn` is the `vsn` attribute of old `Module` (atom `undefined` if none).
- `system_continue(Parent, Debug, Misc) -> no_return()` — continue execution (e.g. after suspend). Never returns.
- `system_get_state(Misc) -> {ok, State}` — (since OTP 17.0) return a term reflecting current state. `State` is what `get_state/2` returns.
- `system_replace_state(StateFun, Misc) -> {ok, NState, NMisc}` — (since OTP 17.0) replace state using `StateFun`; `NState` is what `replace_state/3` returns.
- `system_terminate(Reason, Parent, Debug, Misc) -> no_return()` — terminate; gives the process a chance to clean up. Never returns.

`gen_server`, `gen_statem`, `gen_event` already export `system_get_state/1` and `system_replace_state/2`, so their callback modules need not supply them. These callbacks are primarily for **user-defined behaviours** and modules implementing **OTP special processes**.

## Review checklist

- [ ] Process is started via `proc_lib` (`spawn_link`/`start_link`/`start_monitor`), not plain `spawn`.
- [ ] All initialization (including `register`) happens in `init/1`, before `init_ack`.
- [ ] `init_ack(Parent, {ok, self()})` is called on success; `init_fail/2,3` on failure (never `init_ack` for failure).
- [ ] The exception from `init_fail` is NOT caught.
- [ ] `{system, From, Request}` content is NOT interpreted — passed to `sys:handle_system_msg/6`.
- [ ] `sys:handle_debug/4` is called for each system event; the returned `Deb` is threaded back into the loop.
- [ ] If the process traps exits, it watches for `{'EXIT', Parent, Reason}` and terminates with that reason.
- [ ] `Module` exports `system_continue/3`, `system_terminate/4`, `system_code_change/4`, `system_get_state/1`, `system_replace_state/2`.
- [ ] Hibernate uses `proc_lib:hibernate/3`, not the BIF.
- [ ] `stop/3` is only used against processes that handle system messages correctly.
- [ ] `get_state`/`replace_state` are not used in normal (non-debug) code paths.
- [ ] `change_code` is preceded by `suspend`.
- [ ] For user-defined behaviours, callbacks are declared with `-callback` (not `behaviour_info/1`); each `-spec` is a subtype of the respective `-callback`.

## Implementation checklist

- [ ] `start_link/0` (or equivalent) calls `proc_lib:start_link(Mod, init, [Parent])`.
- [ ] `init/1` registers the name, builds state, inits `Deb = sys:debug_options([])`, calls `proc_lib:init_ack(Parent, {ok, self()})`, then enters the loop.
- [ ] Loop `receive`s application messages, wrapping each in/out event with `sys:handle_debug/4`.
- [ ] Loop has a `{system, From, Request}` clause delegating to `sys:handle_system_msg(Request, From, Parent, Module, Deb, State)`.
- [ ] `system_continue/3` re-enters the loop with `(Parent, Deb, State)`.
- [ ] `system_terminate/4` calls `exit(Reason)`.
- [ ] `system_get_state/1` returns `{ok, State}`.
- [ ] `system_replace_state/2` applies `StateFun` and returns `{ok, NewState, NewState}`.
- [ ] `system_code_change/4` (optional) converts `Misc` for release upgrades.
- [ ] A `write_debug/3` format function is passed as a fun to `sys:handle_debug/4`.
- [ ] Exit-trapping variant handles `{'EXIT', Parent, Reason}` → `exit(Reason)`.

## Runtime / debugging checklist

- [ ] `sys:get_state/1,2` returns the callback module state (gen_server: state term; gen_statem: `{CurrentState, CurrentData}`; gen_event: list of `{Module, Id, HandlerState}`).
- [ ] `sys:get_status/1,2` returns the full status tuple (see shape below).
- [ ] `sys:replace_state/1,2,3` replaces state via `StateFun(State)`; for gen_event called once per handler.
- [ ] `sys:suspend/1,2` / `sys:resume/1,2` to pause/resume.
- [ ] `sys:terminate/1,2,3` to order asynchronous termination.
- [ ] `sys:change_code/4,5` after `suspend` for code change.
- [ ] `sys:log/1,2,3` (`Flag :: true | {true, N} | false | get | print`) to control event logging.
- [ ] `sys:log_to_file/1,2,3` to log events to a UTF-8 file.
- [ ] `sys:trace/1,2,3` to print all system events to `standard_io`.
- [ ] `sys:statistics/1,2,3` returns `{start_time, _}`, `{current_time, _}`, `{reductions, _}`, `{messages_in, _}`, `{messages_out, _}`.
- [ ] `sys:install/1,2,3` to install a custom `dbg_fun/0`; `sys:remove/1,2,3` to remove it.
- [ ] `sys:no_debug/1,2` to turn off ALL debugging.
- [ ] `sys:get_log/1` (since OTP 22.0) to retrieve logged events; do NOT use deprecated `get_debug/3`.

## Validation hooks

- `proc_lib:initial_call/1` — verify the recorded initial call of a proc_lib-started process (returns placeholder atoms, not real args).
- `proc_lib:translate_initial_call/1` — used by `c:i/0` and `c:regs/0` to display useful initial-call info.
- `proc_lib:get_label/1` — retrieve the label set via `set_label/1` (OTP 27.0+).
- `sys:get_state/1,2` — extract callback state for debugging.
- `sys:get_status/1,2` — full status tuple including `SysState`, `Parent`, `Dbg`, `Misc`.
- `sys:get_log/1` — pull logged `system_event()`s out of a `Debug` structure.
- `sys:statistics/1,2,3` with `get` — retrieve collected statistics.
- Crash reports emitted automatically by `proc_lib` on abnormal termination (reason other than `normal`/`shutdown`/`{shutdown,Term}`).

## Examples

The canonical special-process example is `ch4` (from crawl 10) — the simple channel server reimplemented with `sys` + `proc_lib`:

```erlang
-module(ch4).
-export([start_link/0]).
-export([alloc/0, free/1]).
-export([init/1]).
-export([system_continue/3, system_terminate/4,
         write_debug/3,
         system_get_state/1, system_replace_state/2]).

start_link() ->
    proc_lib:start_link(ch4, init, [self()]).

alloc() ->
    ch4 ! {self(), alloc},
    receive
        {ch4, Res} -> Res
    end.

free(Ch) ->
    ch4 ! {free, Ch},
    ok.

init(Parent) ->
    register(ch4, self()),
    Chs = channels(),
    Deb = sys:debug_options([]),
    proc_lib:init_ack(Parent, {ok, self()}),
    loop(Chs, Parent, Deb).

loop(Chs, Parent, Deb) ->
    receive
        {From, alloc} ->
            Deb2 = sys:handle_debug(Deb, fun ch4:write_debug/3,
                                    ch4, {in, alloc, From}),
            {Ch, Chs2} = alloc(Chs),
            From ! {ch4, Ch},
            Deb3 = sys:handle_debug(Deb2, fun ch4:write_debug/3,
                                    ch4, {out, {ch4, Ch}, From}),
            loop(Chs2, Parent, Deb3);
        {free, Ch} ->
            Deb2 = sys:handle_debug(Deb, fun ch4:write_debug/3,
                                    ch4, {in, {free, Ch}}),
            Chs2 = free(Ch, Chs),
            loop(Chs2, Parent, Deb2);
        {system, From, Request} ->
            sys:handle_system_msg(Request, From, Parent,
                                  ch4, Deb, Chs)
    end.

system_continue(Parent, Deb, Chs) ->
    loop(Chs, Parent, Deb).

system_terminate(Reason, _Parent, _Deb, _Chs) ->
    exit(Reason).

system_get_state(Chs) ->
    {ok, Chs}.

system_replace_state(StateFun, Chs) ->
    NChs = StateFun(Chs),
    {ok, NChs, NChs}.

write_debug(Dev, Event, Name) ->
    io:format(Dev, "~p event = ~p~n", [Name, Event]).
```

Exit-trapping variant (parent-death handling):

```erlang
init(Parent) ->
    ...,
    process_flag(trap_exit, true),
    ...,
    loop(Parent).

loop(Parent) ->
    receive
        ...
        {'EXIT', Parent, Reason} ->
            %% Clean up here, if needed.
            exit(Reason);
        ...
    end.
```

User-defined behaviour skeleton (`simple_server`):

```erlang
-module(simple_server).
-export([start_link/2, init/3, ...]).

-callback init(State :: term()) -> 'ok'.
-callback handle_req(Req :: term(), State :: term()) -> {'ok', Reply :: term()}.
-callback terminate() -> 'ok'.
-callback format_state(State :: term()) -> term().

-optional_callbacks([format_state/1]).

start_link(Name, Module) ->
    proc_lib:start_link(?MODULE, init, [self(), Name, Module]).

init(Parent, Name, Module) ->
    register(Name, self()),
    ...,
    Dbg = sys:debug_options([]),
    proc_lib:init_ack(Parent, {ok, self()}),
    loop(Parent, Module, Dbg, ...).
```

## Common mistakes

- Starting a special process with plain `erlang:spawn`/`spawn_link` — loses ancestor/initial-call metadata, crash reports, and broadened normal-termination semantics.
- Calling `init_ack` with an error tuple to signal failed start — can block VM resources; use `init_fail/2,3` instead.
- Catching the exception raised by `init_fail` — defeats its purpose.
- Interpreting the `Request` of `{system, From, Request}` instead of delegating to `sys:handle_system_msg/6`.
- Forgetting to thread the updated `Deb` from `sys:handle_debug/4` back into the loop.
- Using the raw `hibernate/3` BIF instead of `proc_lib:hibernate/3` — breaks exception handling and logging on wake-up.
- Calling `sys:change_code/4,5` without `suspend`-ing the process first.
- Using `sys:get_state`/`sys:replace_state` in normal (non-debug) code paths.
- Assuming `sys:terminate/2,3` has terminated the process when it returns — termination is asynchronous.
- Passing `monitor` in `start_link/5`/`start_monitor/5` SpawnOpts — causes `badarg`.
- Using deprecated `sys:get_debug/3` instead of `sys:get_log/1`.
- Declaring a user-defined behaviour with `behaviour_info/1` instead of `-callback` attributes.
- Combining `-optional_callbacks` with `behaviour_info/1` (not allowed).

## Strict vs contextual guidance

**Strict (non-negotiable):**
- Start special processes through `proc_lib`; never plain `spawn`.
- All initialization in `init/1`; acknowledge via `init_ack`/`init_fail`.
- Do NOT interpret `{system, From, Request}` — delegate to `sys:handle_system_msg/6`.
- `handle_system_msg/6` never returns; `Module` must export all five `system_*` callbacks.
- `change_code` requires prior `suspend`.
- `get_state`/`replace_state` are debugging-only.
- Use `proc_lib:hibernate/3`, not the BIF.
- Each `-spec` in a callback module MUST be a subtype of the respective `-callback` contract.

**Contextual:**
- Prefer `start_link` (synchronous) over `spawn_link` (asynchronous) when the parent needs to know the child initialized before proceeding.
- Use `start_monitor` (OTP 23.0+) when the parent wants monitoring without linking.
- Writing a special process is only warranted when a standard behaviour does not fit; otherwise the manual contract is error-prone boilerplate.
- `system_code_change/4` is optional unless release upgrades target the process.
- `format/1,2,3` only needed for legacy `error_logger` handlers; new Logger handlers use `report_cb` metadata.

## Policy decisions for individual repos

- **Preferred start function:** `start_link/3,4,5` (sync, linked) vs `start_monitor/3,4,5` (sync, monitored) vs `spawn_link/1,2,3,4` (async). Default to `start_link` for supervised children.
- **Special processes vs standard behaviours:** default to standard behaviours; require explicit justification (and review) before introducing a hand-written special process.
- **Debug options at start:** whether to enable `trace`/`log`/`statistics` in dev/test via `debug_options/1` or behaviour start opts.
- **`init_fail` adoption:** require OTP 26.0+ baseline if `init_fail/2,3` is used for start-failure signalling.
- **`set_label`/`get_label` adoption:** require OTP 27.0+ baseline if process labels are used.
- **Crash-report formatting:** rely on Logger `report_cb` metadata (default); only keep `proc_lib:format/1,2,3` for legacy `error_logger` handlers.
- **User-defined behaviours:** mandate `-callback`/`-optional_callbacks` over `behaviour_info/1`.

## Related docs

- `otp-behaviours.md`
- `gen-server.md`
- `gen-statem.md`
- `gen-event.md`
- `supervision.md`
- `runtime-debugging.md`
- `logger-and-config.md`
- `common-mistakes.md`

## Related skills

- `beam-gen-server`
- `beam-supervision`
- `beam-errors-failures`
- `beam-observability-debugging`
