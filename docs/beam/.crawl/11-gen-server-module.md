# Crawl: stdlib/gen_server.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/gen_server.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/gen_server.html
- family: Erlang/OTP stdlib module docs
- fetch: HTTP 200
- otp_version: OTP 29.0.2 (stdlib v8.0.1)
- feeds_docs: gen-server.md

## Purpose
Generic server behaviour module providing the server in a client-server relation.
A `gen_server` process implemented using this module has a standard set of
interface functions and includes functionality for tracing and error reporting.
It fits into an OTP supervision tree. All specific parts live in a callback
module exporting a predefined set of functions.

Mapping (gen_server module -> callback module):
- `start` / `start_monitor` / `start_link`  ----> `Module:init/1`
- `stop`                                    ----> `Module:terminate/2`
- `call` / `send_request` / `multi_call`    ----> `Module:handle_call/3`
- `cast` / `abcast`                         ----> `Module:handle_cast/2`
- (any other message)                       ----> `Module:handle_info/2`
- (after `{continue,_}`)                     ----> `Module:handle_continue/2`
- (release upgrade/downgrade)               ----> `Module:code_change/3`
- (status formatting)                       ----> `Module:format_status/1` (or deprecated `/2`)

## Callback signatures (exact)
Verbatim from the `-callback` typespecs on the page.

### init/1
```erlang
-callback init(Args :: term()) ->
    {ok, State :: term()} |
    {ok, State :: term(), Action :: action()} |
    {stop, Reason :: term()} |
    ignore |
    {error, Reason :: term()}.
```
Called by the new process to initialize the server whenever started via
`start/3,4`, `start_monitor/3,4`, or `start_link/3,4`. `Args` is the `Args`
argument provided to the start function.

### handle_call/3
```erlang
-callback handle_call(Request :: term(), From :: from(), State :: term()) ->
    {reply, Reply :: term(), NewState :: term()} |
    {reply, Reply :: term(), NewState :: term(), Action :: action()} |
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), Reply :: term(), NewState :: term()} |
    {stop, Reason :: term(), NewState :: term()}.
```
Called whenever a request sent using `call/2,3`, `multi_call/2,3,4`, or
`send_request/2,4` is received.

### handle_cast/2
```erlang
-callback handle_cast(Request :: term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```
Called whenever a request sent using `cast/2` or `abcast/2,3` is received.

### handle_info/2  (optional)
```erlang
-callback handle_info(Info :: timeout | term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```
Called on time-out (`Info = timeout`) or any non-request/non-system message.
Default implementation logs the unexpected `Info`, drops it, returns
`{noreply, State}`.

### handle_continue/2  (optional, since OTP 21.0)
```erlang
-callback handle_continue(Info :: term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```
Invoked immediately after a previous callback returns a tuple containing
`{continue, Continue}`. If `{continue,_}` is used and this callback is not
implemented, the process exits with `undef`.

### terminate/2  (optional)
```erlang
-callback terminate(Reason :: normal | shutdown | {shutdown, term()} | term(),
                    State :: term()) -> term().
```
Called when the gen_server is about to terminate. Return value is ignored.
After it returns, the process terminates with `Reason`.

### code_change/3  (optional)
```erlang
-callback code_change(OldVsn :: term() | {down, term()},
                      State :: term(),
                      Extra :: term()) ->
    {ok, NewState :: term()} | {error, Reason :: term()}.
```
Called during release upgrade/downgrade when instruction
`{update, Module, Change, ...}` is specified. For upgrade `OldVsn = Vsn`;
for downgrade `OldVsn = {down, Vsn}`. Returning `{error, Reason}` rolls back.

### format_status/1  (optional, since OTP 25.0)
```erlang
-callback format_status(Status) -> NewStatus
    when Status    :: format_status(),
         NewStatus :: format_status().
%% where
-type format_status() ::
    #{state => term(), message => term(), reason => term(),
      log => [sys:system_event()]}.
```
Called by `sys:get_status/1,2` and on abnormal termination (for logger).
Receives a map `Status`, must return a map `NewStatus` with the same keys
(values may be transformed). Used to strip sensitive data / compact large
state. If exported but crashes, the default returns that
`Module:format_status/1 has crashed` (to hide sensitive data).

### format_status/2  (DEPRECATED, since OTP R13B04)
```erlang
-callback format_status(Opt, StatusData) -> Status
    when Opt        :: normal | terminate,
         StatusData :: [PDict | State],
         PDict      :: [{Key :: term(), Value :: term()}],
         State      :: term(),
         Status     :: term().
```
Deprecated. Page states: "the callback gen_server:format_status(_,_) is
deprecated; use format_status/1 instead." Use `format_status/1` for new code.

## Return-tuple shapes (per callback, verbatim from the typespecs)

### init/1
- `{ok, State}`
- `{ok, State, Action}`  (Action per `action/0`)
- `{stop, Reason}`       (process exits with `Reason`)
- `ignore`               (process exits with reason `normal`)
- `{error, Reason}`     (since OTP 26.0; graceful/"silent" termination,
  process exits with reason `normal`)

### handle_call/3
- `{reply, Reply, NewState}`
- `{reply, Reply, NewState, Action}`
- `{noreply, NewState}`            (reply must be sent via `reply(From, Reply)`)
- `{noreply, NewState, Action}`
- `{stop, Reason, Reply, NewState}` (replies to client, then terminates)
- `{stop, Reason, NewState}`        (no reply; reply via `reply/2` first if needed)

### handle_cast/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

### handle_info/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

### handle_continue/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

### code_change/3
- `{ok, NewState}`
- `{error, Reason}`   (rolls back the upgrade)

### terminate/2
- Any `term()` (return value is ignored).

### The `action/0` type (last arg / 3rd/4th tuple element)
```erlang
-type action() ::
    (Time :: timeout()) |                              % legacy timeout
    hibernate |                                        % hibernate until next msg
    {timeout, Time :: timeout(), Message :: term()} |
    {timeout, Time :: timeout(), Message :: term(),
              Options :: timeout_option() | [timeout_option()]} |
    {hibernate, Time :: timeout(), Message :: term()} |
    {hibernate, Time :: timeout(), Message :: term(),
                Options :: timeout_option() | [timeout_option()]} |
    {continue, Continue :: term()}.
```
- `Time :: timeout()` -> integer ms; on expiry `handle_info(timeout, _)` is
  called. `infinity` waits indefinitely (default). `Time = 0` cancels if a
  message is already waiting. NOTE: a system message restarts this legacy
  timeout (known flaw).
- `{timeout, Time, Message[, Options]}` -> like above but `Message` is the
  delivered `Info` arg; not affected by system messages. `Time = 0` delivered
  immediately; `Time = infinity` ignored (no timer started).
- `{hibernate, Time, Message[, Options]}` -> hibernate while waiting for msg
  or timeout.
- `hibernate` -> process hibernates (`erlang:hibernate/0`) until next msg.
- `{continue, Continue}` -> immediately invokes `Module:handle_continue/2`
  with `Continue` as first arg, before any external message/request.

## Key functions
Exact arities & options from the `-spec` declarations.

### call/2,3
```erlang
-spec call(ServerRef :: server_ref(), Request :: term()) -> Reply :: term().
% Equivalent to call(ServerRef, Request, 5000).

-spec call(ServerRef :: server_ref(), Request :: term(),
           Timeout :: timeout()) -> Reply :: term().
```
Synchronous call: sends request, waits for reply or timeout. `Timeout` is an
integer (ms) or `infinity`. On timeout the *calling* process exits with
`{Reason, Location}` where `Reason = timeout`. Since OTP 24 uses process
aliases, so late replies are not received. May exit with reasons: `timeout`,
`noproc`, `{nodedown,Node}`, `calling_self`, `shutdown`, `normal`,
`{shutdown,Term}`, or `_OtherTerm` (server died).

### cast/2
```erlang
-spec cast(ServerRef :: server_ref(), Request :: term()) -> ok.
```
Asynchronous; returns `ok` immediately, ignores nonexistent node/server.
Server calls `Module:handle_cast(Request, _)`.

### reply/2
```erlang
-spec reply(Client :: from(), Reply :: term()) -> ok.
```
Explicit reply to a client that called `call/2,3` or `multi_call/2,3,4`,
when the reply cannot be passed in the return value of `handle_call/3`.
`Client` must be the `From` argument given to `handle_call/3`.

### start/3,4
```erlang
-spec start(Module :: module(), Args :: term(),
            Options :: [start_opt()]) -> start_ret().
-spec start(ServerName :: server_name(), Module :: module(), Args :: term(),
            Options :: [start_opt()]) -> start_ret().
```
Standalone gen_server (not part of supervision tree, no supervisor).
`start/3` = not registered; `start/4` = registered.

### start_link/3,4
```erlang
-spec start_link(Module :: module(), Args :: term(),
                 Options :: [start_opt()]) -> start_ret().
-spec start_link(ServerName :: server_name(), Module :: module(),
                 Args :: term(), Options :: [start_opt()]) -> start_ret().
```
Creates a gen_server as part of a supervision tree, linked to the caller.
Synchronous: does not return until `Module:init/1` has returned or failed.
`start_link/3` = linked, not registered; `start_link/4` = linked + registered.

### start_monitor/3,4  (since OTP 23.0)
```erlang
-spec start_monitor(Module :: module(), Args :: term(),
                    Options :: [start_opt()]) -> start_mon_ret().
-spec start_monitor(ServerName :: server_name(), Module :: module(),
                    Args :: term(), Options :: [start_opt()]) -> start_mon_ret().
```
Standalone gen_server, atomically sets up a monitor (not a link) to the new
server. On unsuccessful start the caller is blocked until the monitor's
`'DOWN'` message has been received and removed.

### stop/1,2,3  (since OTP 18.0)
```erlang
-spec stop(ServerRef :: server_ref()) -> ok.
% Equivalent to stop(ServerRef, normal, infinity).

-spec stop(ServerRef :: server_ref(), Reason :: term(),
           Timeout :: timeout()) -> ok.
```
Orders the server to exit with `Reason`, waits for termination. Server calls
`Module:terminate/2` before exiting. Returns `ok` if it terminates with the
expected reason. Any reason other than `normal`, `shutdown`, or
`{shutdown,Term}` causes an error report via `logger`. Exits caller with
`timeout` / `noproc` / `{nodedown,Node}` on failure.

### abcast/2,3
```erlang
-spec abcast(Name :: atom(), Request :: term()) -> abcast.
% Equivalent to abcast(Nodes, Name, Request) over all connected nodes.

-spec abcast(Nodes :: [node()], Name :: atom(),
             Request :: term()) -> abcast.
```
Asynchronous cast to all gen_servers locally registered as `Name` on `Nodes`.
Returns `abcast` immediately; ignores nonexistent nodes/servers. Servers call
`Module:handle_cast/2`.

### multi_call/2,3,4
```erlang
-spec multi_call(Name :: atom(), Request :: term()) ->
    {Replies :: [{Node :: node(), Reply :: term()}], BadNodes :: [node()]}.
-spec multi_call(Nodes :: [node()], Name :: atom(), Request :: term()) ->
    {Replies :: [{Node :: node(), Reply :: term()}], BadNodes :: [node()]}.
% Equivalent to multi_call(Nodes, Name, Request, infinity).

-spec multi_call(Nodes :: [node()], Name :: atom(), Request :: term(),
                 Timeout :: timeout()) ->
    {Replies :: [{Node :: node(), Reply :: term()}], BadNodes :: [node()]}.
```
Synchronous call to all gen_servers locally registered as `Name` on `Nodes`.
Servers call `Module:handle_call/3`. Uses a middleman process to discard late
answers after timeout.

### Supporting types
```erlang
-type from() :: {Client :: pid(), Tag :: reply_tag()}.

-type server_name() ::
    {local, LocalName :: atom()} |
    {global, GlobalName :: term()} |
    {via, RegMod :: module(), ViaName :: term()}.

-type server_ref() ::
    pid() |
    (LocalName :: atom()) |
    {Name :: atom(), Node :: atom()} |
    {global, GlobalName :: term()} |
    {via, RegMod :: module(), ViaName :: term()}.

-type start_ret() :: {ok, Pid :: pid()} | ignore | {error, Reason :: term()}.
-type start_mon_ret() ::
    {ok, {Pid :: pid(), MonRef :: reference()}} | ignore | {error, Reason :: term()}.

-type start_opt() ::
    {timeout, Timeout :: timeout()} |
    {spawn_opt, SpawnOptions :: [proc_lib:start_spawn_option()]} |
    enter_loop_opt().
% NOTE: spawn option `monitor` is not allowed -> badarg.

-type enter_loop_opt() ::
    {hibernate_after, HibernateAfterTimeout :: timeout()} |
    {debug, Dbgs :: [sys:debug_option()]}.

-type timeout_option() :: {abs, Abs :: boolean()}.   % not exported
-type response_timeout() :: timeout() | {abs, integer()}.  % not exported
```

`start_ret/0` detail:
- `{ok, Pid}` - created & initialized.
- `{error, {already_started, OtherPid}}` - a process with that `ServerName`
  exists; the new gen_server exited `normal` before calling `init/1`.
- `{error, timeout}` - `init/1` did not return within the start timeout;
  process killed with `exit_signal(_, kill)`.
- `ignore` - `init/1` returned `ignore` (process exits `normal`).
- `{error, Reason}` - `init/1` returned `{stop, Reason}` or `{error, Reason}`,
  or failed with `Reason`.

## Strict rules
- **Bad return terminates the process.** "If a callback function fails or
  returns a bad value, the gen_server process terminates."
- **`throw` is not an error.** "an exception of class `throw` is not regarded
  as an error but as a valid return, from all callback functions."
- **`trap_exit` is NOT automatic.** "a gen_server process does not trap exit
  signals automatically, this must be explicitly initiated in the callback
  module." (e.g. `process_flag(trap_exit, true)` in `init/1`).
- **`init/1` is synchronous.** `start_link/3,4` "does not return until
  `Module:init/1` has returned or failed." Same for `start/3,4`,
  `start_monitor/3,4`.
- **`terminate/2` reason semantics.** If supervisor-ordered termination and
  the gen_server traps exits and the child spec shutdown strategy is an
  integer (not `brutal_kill`), `terminate/2` is called with `Reason=shutdown`.
  If not trapping exits, the process terminates immediately (no `terminate/2`).
  Any reason other than `normal`, `shutdown`, `{shutdown,Term}` is treated as
  an error and reported via `logger`.
- **Exit signals propagate.** On termination an exit signal with the same
  reason is sent to linked processes and ports.
- **`{continue, Continue}` runs before external messages.** It invokes
  `handle_continue/2` immediately after the previous callback, before receiving
  any external message/request.
- **`hibernate` cost.** Hibernation implies at least two GCs (when hibernating
  and shortly after waking); "not something you want to do between each call to
  a busy server."
- **`spawn_opt` `monitor` disallowed** in `start_opt` -> `badarg`.
- **`format_status/2` is deprecated**; use `format_status/1`.
- **`{error, Reason}` from `init/1`** (since OTP 26.0) results in graceful
  ("silent") termination (exits `normal`), unlike `{stop, Reason}` which exits
  with `Reason`.

## Verbatim quotes
1. (Intro) "If a callback function fails or returns a bad value, the
   gen_server process terminates. However, an exception of class `throw` is
   not regarded as an error but as a valid return, from all callback
   functions."
2. (Intro, trap_exit) "a gen_server process does not trap exit signals
   automatically, this must be explicitly initiated in the callback module."
3. (Intro, continue) "If the gen_server process needs to perform an action
   after initialization or to break the execution of a callback into multiple
   steps, it can return `{continue, Continue}` in place of the time-out or
   hibernation value, which will invoke the `Module:handle_continue/2`
   callback, before receiving any external message / request."
4. (action/0, hibernate) "hibernate - The process goes into hibernation (by
   calling `erlang:hibernate/0`), waiting for the next request or message to
   arrive."
5. (handle_continue/2 note) "If such a `{continue,_}` tuple is used and the
   callback is not implemented, the process will exit with undef error."
6. (start_link/3,4) "To ensure a synchronized startup procedure, `start_link/3,4`
   does not return until `Module:init/1` has returned or failed."
7. (init/1, {error,Reason}) "{error,Reason} since OTP 26.0 ... The
   difference between returning `{stop, _}` and `{error, _}` from
   `Module:init/1`, is that `{error, _}` results in a graceful ('silent')
   termination since the gen_server process exits with reason `normal`."
8. (format_status/2 deprecation) "This callback is deprecated. the callback
   `gen_server:format_status(_,_)` is deprecated; use `format_status/1`
   instead."

## Version notes
- Page built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- OTP version documented: **OTP 29.0.2** (stdlib v8.0.1).
- `format_status/1` since OTP 25.0; `format_status/2` since OTP R13B04
  (deprecated).
- `handle_continue/2` since OTP 21.0.
- `start_monitor/3,4` since OTP 23.0.
- `stop/1,2,3` since OTP 18.0.
- `call/2,3` uses process aliases since OTP 24 (late replies not received).
- `{error, Reason}` return from `init/1` since OTP 26.0.
- `start_link/3,4` consistency fix in OTP 26.0 (previously `{stop,Reason}`
  from `init/1` could return `{error,Reason}` before the process had
  terminated).
- `check_response/2` since OTP 23.0; `/3` since OTP 25.0.
- `receive_response/2` since OTP 24.0; `/3` since OTP 25.0.
- `wait_response/2` since OTP 23.0; `/3` since OTP 25.0.
- `send_request/2` since OTP 23.0; `/4` since OTP 25.0.
- `reqids_*` since OTP 25.0.

## Discovered links
### Relevant (crawl later)
- ../../system/gen_server_concepts.html  (gen_server Behaviour, OTP Design Principles)
- ../../system/ref_man_processes.html#errors  (error handling via exit signals)
- ../../system/ref_man_processes.html#blocking-signaling-over-distribution  (blocking signaling -> delayed call timeouts)
- ../../system/release_handling.html#instr  (release handling instructions, appup)
- gen_event.html  (sibling behaviour)
- gen_statem.html  (sibling behaviour)
- proc_lib.html  (spawn/start helpers; hibernate/3; start_spawn_option/0)
- supervisor.html  (supervision tree)
- sys.html  (system messages, debugging; get_status/1, log/2, debug_option/0, system_event/0)

### Skipped
- Search/Settings/Copy Markdown/View Source/View llms.txt/Download ePub UI links.
- Internal anchor-only links within the same page.
