# Crawl: spec_proc.html
- seed_url: https://www.erlang.org/doc/system/spec_proc.html
- canonical_url: https://www.erlang.org/doc/system/spec_proc.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (ExDoc v0.40.3; copyright 1996-2026 Ericsson AB)
- feeds_docs: proc-lib-and-sys.md, otp-behaviours.md

## Purpose
The `sys` module provides functions for simple debugging of processes implemented using behaviours, and—together with `proc_lib`—functions to implement a **special process** that complies with the OTP design principles *without* using a standard behaviour. The same functions can also be used to implement user-defined (non-standard) behaviours. Both `sys` and `proc_lib` belong to the STDLIB application.

A special process is one that:
- Is started in a way that makes it fit into a supervision tree.
- Supports the `sys` debug facilities.
- Takes care of system messages.

System messages are messages with a special meaning used in the supervision tree (e.g. trace output requests, suspend/resume during release handling). Processes implemented using standard behaviours automatically understand these messages.

## Key concepts
- **Special process**: a hand-written process that obeys OTP design principles without using `gen_server`/`gen_statem`/`gen_event`.
- **proc_lib start**: `proc_lib:spawn_link/3,4` (async) or `proc_lib:start_link/3,4,5` (sync). Stores ancestor + initial-call info needed for supervision tree integration; generates a crash report if the process terminates with a reason other than `normal` or `shutdown`.
- **init_ack / init_fail**: synchronous start does not return until `proc_lib:init_ack/1,2` or `proc_lib:init_fail/2,3` is called, or the process exits.
- **Debug structure (`Deb`)**: initialized via `sys:debug_options/1`; threaded through the loop; updated per system event via `sys:handle_debug/4`.
- **System event**: user-defined; typically incoming/outgoing messages represented as `{in,Msg[,From]}` and `{out,Msg,To[,State]}`.
- **System messages**: received as `{system, From, Request}`; content must NOT be interpreted by the process—delegate to `sys:handle_system_msg/6`.
- **system_* callbacks**: `system_continue/3`, `system_terminate/4`, `system_get_state/1`, `system_replace_state/2`, and (optionally) `system_code_change/4`.
- **User-defined behaviours**: write code like a special process but delegate to a callback module; declare callbacks with `-callback` attributes (and `-optional_callbacks`); `behaviour_info/1` is the legacy alternative.

## Strict rules / invariants
1. A special process MUST be started through a `proc_lib` function (`spawn_link` or `start_link`), not plain `spawn`, so ancestor/initial-call metadata and crash reports are correct.
2. All initialization (including name registration) MUST be done in `init/1`.
3. The new process MUST acknowledge start to the parent via `proc_lib:init_ack(Parent, {ok, self()})` (or `init_fail` on failure); `proc_lib:start_link` is synchronous and blocks until ack or exit.
4. The content and meaning of `{system, From, Request}` messages MUST NOT be interpreted by the process—they MUST be passed as-is to `sys:handle_system_msg/6`.
5. `sys:handle_system_msg/6` does **not** return; it eventually calls `Module:system_continue/3` (to continue) or `Module:system_terminate/4` (to terminate), and may call `Module:system_get_state/1`, `Module:system_replace_state/2`, or `system_code_change/4` while handling.
6. A process in a supervision tree is expected to terminate with the **same reason as its parent**.
7. If a special process traps exits, it MUST watch for `{'EXIT', Parent, Reason}` and terminate with that same reason once the parent has terminated.
8. `sys:handle_debug/4` MUST be called for each system event to be logged/traced; it returns an updated `Deb` that must be threaded back into the loop.
9. `-optional_callbacks` MUST be used together with `-callback`; it cannot be combined with `behaviour_info/1`.
10. Each `-spec` contract in a callback module MUST be a subtype of the respective `-callback` contract.

## Examples
The canonical example is `ch4`—the simple channel server from the Overview, reimplemented with `sys` + `proc_lib`:

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

## Behaviour / callback details
**Special processes; proc_lib spawn/start/start_link/start_monitor:**
- `proc_lib:spawn_link/3,4` — asynchronous start.
- `proc_lib:start_link/3,4,5` — synchronous start; spawns + links, blocks until ack/exit.
- (Family also includes `start_monitor` and `spawn` variants referenced from proc_lib module doc.)
- Starting through `proc_lib` stores info necessary for a process within a supervision tree (ancestors, initial call). If the process terminates with a reason other than `normal` or `shutdown`, a **crash report** is generated (see Kernel Logging).

**init_ack / init_fail:**
- `proc_lib:init_ack(Parent, {ok, self()})` acknowledges successful start to the parent.
- `proc_lib:init_fail/2,3` signals failure.
- `proc_lib:start_link/3` is synchronous and does not return until `init_ack/1,2` or `init_fail/2,3` has been called, or the process has exited.

**sys integration (debug):**
- `Deb = sys:debug_options([])` initializes the debug structure (empty list ⇒ debugging initially disabled).
- For each system event call `sys:handle_debug(Deb, Func, Info, Event) => Deb1`.
  - `Deb` — debug structure from `sys:debug_options/1`.
  - `Func` — user format fun called as `Func(Dev, Event, Info)`; `Dev` is the I/O device.
  - `Info` — any term, passed as-is to `Func`.
  - `Event` — the system event; typically `{in,Msg[,From]}` (incoming) and `{out,Msg,To[,State]}` (outgoing).
- Returns updated `Deb1` which must be threaded back into the loop.

**System messages/events/callbacks:**
- System messages are received as `{system, From, Request}`.
- The process MUST NOT interpret `Request`; it calls `sys:handle_system_msg(Request, From, Parent, Module, Deb, State)`.
  - `Request`/`From` — passed as-is from the received message.
  - `Parent` — pid of the parent process.
  - `Module` — name of the module implementing the special process.
  - `Deb` — the debug structure.
  - `State` — term describing internal state; passed on to the `system_*` callbacks.
- `sys:handle_system_msg/6` does **not** return. It handles the system message and eventually calls one of:
  - `Module:system_continue(Parent, Deb, State)` — if process execution is to continue.
  - `Module:system_terminate(Reason, Parent, Deb, State)` — if the process is to terminate.
- While handling, it can also call:
  - `Module:system_get_state(State)` — return state (`{ok, State}`).
  - `Module:system_replace_state(StateFun, State)` — replace state using `StateFun`; see `sys:replace_state/3`.
  - `system_code_change(Misc, Module, OldVsn, Extra)` — perform a code change.

**The system_* callbacks a special process must handle (the sys-compatible contract):**
- `system_continue/3` — resume the loop with `(Parent, Deb, State)`.
- `system_terminate/4` — terminate with `Reason` (typically `exit(Reason)`).
- `system_get_state/1` — return `{ok, State}`.
- `system_replace_state/2` — apply `StateFun` to state, return `{ok, NewState, NewState}`.
- `system_code_change/4` — optional, for release upgrades.
- Plus a `write_debug/3` format function (passed as a fun to `sys:handle_debug`).

**ch4.erl pattern:** spawn via `proc_lib:start_link(Mod, init, [Parent])`; in `init/1` register name, build state, init `Deb` via `sys:debug_options([])`, `proc_lib:init_ack(Parent, {ok, self()})`, then enter `loop(State, Parent, Deb)`. The loop `receive`s application messages (calling `sys:handle_debug/4` around each in/out event) and a `{system, From, Request}` clause that delegates to `sys:handle_system_msg/6`. The `system_*` callbacks resume/terminate/inspect/replace state.

**Crash reports:** generated automatically by `proc_lib` when the process terminates with a reason other than `normal` or `shutdown` (cross-ref Kernel Logging).

**Why standard behaviours are preferred:** standard behaviours (`gen_server`, `gen_statem`, `gen_event`) automatically understand system messages, provide debug facilities, crash reporting, code change, and supervision-tree integration out of the box. Writing a special process via `sys`+`proc_lib` is only warranted when a standard behaviour does not fit; otherwise the manual contract (system messages, debug threading, parent-exit handling) is error-prone boilerplate.

## Verbatim quotes
1. (Intro) "The sys module has functions for simple debugging of processes implemented using behaviours. It also has functions that, together with functions in the proc_lib module, can be used to implement a special process that complies to the OTP design principles without using a standard behaviour. These functions can also be used to implement user-defined (non-standard) behaviours."
2. (Intro) "Both sys and proc_lib belong to the STDLIB application."
3. (Special Processes) "This section describes how to write a process that complies to the OTP design principles, without using a standard behaviour. Such a process is to: * Be started in a way that makes the process fit into a supervision tree * Support the sys debug facilities * Take care of system messages."
4. (Special Processes) "System messages are messages with a special meaning, used in the supervision tree. Typical system messages are requests for trace output, and requests to suspend or resume process execution (used during release handling). Processes implemented using standard behaviours automatically understand these messages."
5. (Starting the Process) "A function in the proc_lib module is to be used to start the process. Several functions are available, for example, proc_lib:spawn_link/3,4 for asynchronous start and proc_lib:start_link/3,4,5 for synchronous start."
6. (Starting the Process) "Information necessary for a process within a supervision tree, such as details on ancestors and the initial call, is stored when a process is started through one of these functions."
7. (Starting the Process) "If the process terminates with a reason other than normal or shutdown, a crash report is generated. For more information about the crash report, see Logging in Kernel User's Guide."
8. (Starting the Process) "proc_lib:start_link/3 is synchronous and does not return until proc_lib:init_ack/1,2 or proc_lib:init_fail/2,3 has been called, or the process has exited."
9. (Debugging) "To support the debug facilities in sys, a debug structure is needed. The Deb term is initialized using sys:debug_options/1"
10. (Debugging) "For each system event to be logged or traced, the following function is to be called: sys:handle_debug(Deb, Func, Info, Event) => Deb1"
11. (Handling System Messages) "System messages are received as: {system, From, Request}"
12. (Handling System Messages) "The content and meaning of these messages are not to be interpreted by the process. Instead the following function is to be called: sys:handle_system_msg(Request, From, Parent, Module, Deb, State)"
13. (Handling System Messages) "sys:handle_system_msg/6 does not return. It handles the system message and eventually calls either of the following functions: * Module:system_continue(Parent, Deb, State) - if process execution is to continue. * Module:system_terminate(Reason, Parent, Deb, State) - if the process is to terminate."
14. (Handling System Messages) "A process in a supervision tree is expected to terminate with the same reason as its parent."
15. (Handling System Messages) "If a special process is configured to trap exits, it must take notice of 'EXIT' messages from its parent process and terminate using the same exit reason once the parent process has terminated."
16. (User-Defined Behaviours) "To implement a user-defined behaviour, write code similar to code for a special process, but call functions in a callback module for handling specific tasks."
17. (User-Defined Behaviours) "We recommend using the -callback attribute rather than the behaviour_info() function. The reason is that the extra type information can be used by tools to produce documentation or find discrepancies."
18. (User-Defined Behaviours) "Each -spec contract is to be a subtype of the respective -callback contract."

## Version notes
- Page rendered for **OTP 29.0.2** (header: "Erlang System Documentation / OTP 29.0.2").
- ExDoc version v0.40.3.
- Shell examples show "Erlang/OTP 27 [erts-15.0]" banner (illustrative; doc itself is OTP 29.0.2).
- Copyright © 1996-2026 Ericsson AB.
- Navigation: Previous Page = "Supervisor Behaviour" (sup_princ.html); Next Page = "Applications" (applications.html).

## Discovered links
### Relevant (crawl later)
- ../apps/stdlib/sys.html — `sys` module reference (STDLIB)
- ../apps/stdlib/proc_lib.html — `proc_lib` module reference (STDLIB)
- ../apps/kernel/logger_chapter.html — Logging (crash reports)
- sup_princ.html — Supervisor Behaviour (previous page)
- applications.html — Applications (next page)
- design_principles.html — Design Principles (ch1 / channels implementation)
- statem.html — gen_statem Behaviour (code_lock example source)
- ../apps/erts/erlang.html — erlang module (self/0)
- ../system/typespec.html — Types and Function Specifications
- spec_proc.md — Markdown source of this page

### Skipped
- ../index.html — doc index (nav)
- spec_proc.html#debug, spec_proc.html#msg, statem.html#example — in-page/section anchors
- ../apps/stdlib/proc_lib.html#spawn_link/4, #start_link/5, #start_link/3, #init_ack/2, #init_fail/3 — function anchors (covered by proc_lib.html)
- ../apps/stdlib/sys.html#debug_options/1, #handle_debug/4, #handle_system_msg/6, #replace_state/3 — function anchors (covered by sys.html)
- ../apps/stdlib/io.html, ../apps/stdlib/io.html#format/3 — io module
- llms.txt, Erlang System Documentation.epub, View package docs — download/nav artifacts
- https://github.com/erlang/otp/... — source repo link
- https://github.com/elixir-lang/ex_doc — ExDoc repo
- https://erlang.org, https://www.ericsson.com — site homepages
