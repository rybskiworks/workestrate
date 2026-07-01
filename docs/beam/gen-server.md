# gen_server Behaviour

## Purpose

Source-verified guidance for the OTP `gen_server` behaviour in Erlang/OTP.
BEAM-common; Erlang syntax only. Reproduces the callback contract, return-tuple
shapes, and supporting types verbatim from the official Erlang/OTP
documentation. Agents who write, review, refactor, debug, or validate
`gen_server` callback modules should follow these rules.

## Sources used

- Crawl file: `docs/beam/.crawl/02-gen-server-concepts.md`
  — https://www.erlang.org/doc/system/gen_server_concepts.html
- Crawl file: `docs/beam/.crawl/11-gen-server-module.md`
  — https://www.erlang.org/doc/apps/stdlib/gen_server.html

Only these two sources are cited.

## Core guidance

### What gen_server is

From crawl 02 ("Verbatim quotes" #1):

> "The client-server model is characterized by a central server and an
> arbitrary number of clients. The client-server model is used for resource
> management operations, where several different clients want to share a
> common resource. The server is responsible for managing this resource."

From crawl 11 ("Purpose"):

> "Generic server behaviour module providing the server in a client-server
> relation. A `gen_server` process implemented using this module has a
> standard set of interface functions and includes functionality for tracing
> and error reporting. It fits into an OTP supervision tree. All specific
> parts live in a callback module exporting a predefined set of functions."

Colocation (crawl 02, "Verbatim quotes" #5):

> "The interface functions (start_link/0, alloc/0, and free/1) are located in
> the same module as the callback functions (init/1, handle_call/3, and
> handle_cast/2). It is usually good programming practice to have the code
> corresponding to one process contained in a single module."

### Mapping (gen_server module -> callback module) — crawl 11

- `start` / `start_monitor` / `start_link`  ----> `Module:init/1`
- `stop`                                    ----> `Module:terminate/2`
- `call` / `send_request` / `multi_call`    ----> `Module:handle_call/3`
- `cast` / `abcast`                         ----> `Module:handle_cast/2`
- (any other message)                       ----> `Module:handle_info/2`
- (after `{continue,_}`)                     ----> `Module:handle_continue/2`
- (release upgrade/downgrade)               ----> `Module:code_change/3`
- (status formatting)                       ----> `Module:format_status/1` (or deprecated `/2`)

### Callback signatures (exact, verbatim from crawl 11)

#### init/1

```erlang
-callback init(Args :: term()) ->
    {ok, State :: term()} |
    {ok, State :: term(), Action :: action()} |
    {stop, Reason :: term()} |
    ignore |
    {error, Reason :: term()}.
```

#### handle_call/3

```erlang
-callback handle_call(Request :: term(), From :: from(), State :: term()) ->
    {reply, Reply :: term(), NewState :: term()} |
    {reply, Reply :: term(), NewState :: term(), Action :: action()} |
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), Reply :: term(), NewState :: term()} |
    {stop, Reason :: term(), NewState :: term()}.
```

#### handle_cast/2

```erlang
-callback handle_cast(Request :: term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```

#### handle_info/2  (optional)

```erlang
-callback handle_info(Info :: timeout | term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```

Called on time-out (`Info = timeout`) or any non-request/non-system message.
Default logs the unexpected `Info`, drops it, returns `{noreply, State}`.

#### handle_continue/2  (optional, since OTP 21.0)

```erlang
-callback handle_continue(Info :: term(), State :: term()) ->
    {noreply, NewState :: term()} |
    {noreply, NewState :: term(), Action :: action()} |
    {stop, Reason :: term(), NewState :: term()}.
```

#### terminate/2  (optional)

```erlang
-callback terminate(Reason :: normal | shutdown | {shutdown, term()} | term(),
                    State :: term()) -> term().
```

Return value is ignored. After it returns, the process terminates with `Reason`.

#### code_change/3  (optional)

```erlang
-callback code_change(OldVsn :: term() | {down, term()},
                      State :: term(),
                      Extra :: term()) ->
    {ok, NewState :: term()} | {error, Reason :: term()}.
```

For upgrade `OldVsn = Vsn`; for downgrade `OldVsn = {down, Vsn}`. Returning
`{error, Reason}` rolls back.

#### format_status/1  (optional, since OTP 25.0)

```erlang
-callback format_status(Status) -> NewStatus
    when Status    :: format_status(),
         NewStatus :: format_status().
%% where
-type format_status() ::
    #{state => term(), message => term(), reason => term(),
      log => [sys:system_event()]}.
```

Called by `sys:get_status/1,2` and on abnormal termination (for logger). If
exported but crashes, the default returns that `Module:format_status/1 has
crashed` (to hide sensitive data).

#### format_status/2  (DEPRECATED, since OTP R13B04)

```erlang
-callback format_status(Opt, StatusData) -> Status
    when Opt        :: normal | terminate,
         StatusData :: [PDict | State],
         PDict      :: [{Key :: term(), Value :: term()}],
         State      :: term(),
         Status     :: term().
```

Deprecated (crawl 11, "Verbatim quotes" #8):

> "This callback is deprecated. the callback
> `gen_server:format_status(_,_)` is deprecated; use `format_status/1`
> instead."

### The `action/0` type (verbatim from crawl 11)

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

Variants (crawl 11, "### The `action/0` type"):

- `Time :: timeout()` (legacy timeout) — integer ms; on expiry
  `handle_info(timeout, _)` is called. `infinity` waits indefinitely
  (default). `Time = 0` cancels if a message is already waiting. NOTE: a
  system message restarts this legacy timeout (known flaw).
- `{timeout, Time, Message[, Options]}` — `Message` is the delivered `Info`
  arg; not affected by system messages. `Time = 0` delivered immediately;
  `Time = infinity` ignored (no timer started).
- `{hibernate, Time, Message[, Options]}` — hibernate while waiting for msg
  or timeout.
- `hibernate` — process hibernates (`erlang:hibernate/0`) until next msg.
- `{continue, Continue}` — immediately invokes `Module:handle_continue/2`
  with `Continue` as first arg, before any external message/request.

### Supporting types (verbatim from crawl 11)

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

### Return-tuple shapes (per callback, verbatim from crawl 11)

#### init/1
- `{ok, State}`
- `{ok, State, Action}`  (Action per `action/0`)
- `{stop, Reason}`       (process exits with `Reason`)
- `ignore`               (process exits with reason `normal`)
- `{error, Reason}`     (since OTP 26.0; graceful/"silent" termination,
  process exits with reason `normal`)

#### handle_call/3
- `{reply, Reply, NewState}`
- `{reply, Reply, NewState, Action}`
- `{noreply, NewState}`            (reply must be sent via `reply(From, Reply)`)
- `{noreply, NewState, Action}`
- `{stop, Reason, Reply, NewState}` (replies to client, then terminates)
- `{stop, Reason, NewState}`        (no reply; reply via `reply/2` first if needed)

#### handle_cast/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

#### handle_info/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

#### handle_continue/2
- `{noreply, NewState}`
- `{noreply, NewState, Action}`
- `{stop, Reason, NewState}`

#### code_change/3
- `{ok, NewState}`
- `{error, Reason}`   (rolls back the upgrade)

#### terminate/2
- Any `term()` (return value is ignored).

### call vs cast vs reply (crawl 02, "call vs cast vs reply")

- `call(ServerName, Request)` — synchronous; client blocks until `Reply`
  arrives via `{reply, Reply, State1}`.
- `cast(ServerName, Request)` — asynchronous; returns `ok` immediately; on
  receipt `handle_cast(Request, State)` is invoked.
- `reply/2` (crawl 11) — explicit reply to a client that called `call/2,3` or
  `multi_call/2,3,4`, when the reply cannot be passed in the return value of
  `handle_call/3`. `Client` must be the `From` given to `handle_call/3`. So
  when `handle_call/3` returns `{noreply,...}`, reply via `reply(From, Reply)`.

### Timeouts (crawl 11, "### The `action/0` type")

Legacy timeout fires `handle_info(timeout, _)`; `infinity` is the default;
`Time = 0` cancels if a message is already waiting; a system message restarts
the legacy timeout (known flaw). `{timeout, Time, Message}` is NOT affected by
system messages; `Time = 0` delivered immediately; `Time = infinity` ignored.

### hibernate (crawl 11)

From "Verbatim quotes" #4:

> "hibernate - The process goes into hibernation (by calling
> `erlang:hibernate/0`), waiting for the next request or message to arrive."

Costly — at least two GCs (when hibernating and shortly after waking); "not
something you want to do between each call to a busy server" (crawl 11,
"Strict rules").

### {continue, Continue} (crawl 11)

From "Verbatim quotes" #3:

> "If the gen_server process needs to perform an action after initialization
> or to break the execution of a callback into multiple steps, it can return
> `{continue, Continue}` in place of the time-out or hibernation value, which
> will invoke the `Module:handle_continue/2` callback, before receiving any
> external message / request."

If `{continue,_}` is used and `handle_continue/2` is not implemented, the
process exits with `undef` (crawl 11, "Verbatim quotes" #5).

### Naming / registration (crawl 02 + 11)

From crawl 02 ("Verbatim quotes" #4):

> "If the name is omitted, the gen_server is not registered. Instead its pid
> must be used. The name can also be given as {global, Name}, in which case
> the gen_server is registered using global:register_name/2."

Forms (from `server_name()`, crawl 11): `{local, Name}`, `{global, Name}`,
`{via, RegMod, ViaName}`, or omit (use pid).

### start / start_link / start_monitor (crawl 02 + 11)

From crawl 02 ("Verbatim quotes" #8):

> "gen_server:start_link/4 must be used if the gen_server is part of a
> supervision tree, meaning that it was started by a supervisor. There is
> another function, gen_server:start/4, to start a standalone gen_server that
> is not part of a supervision tree."

- `start/3,4` — standalone, not in supervision tree. `start/3` not registered;
  `start/4` registered.
- `start_link/3,4` — part of supervision tree, linked to caller. `start_link/3`
  linked not registered; `start_link/4` linked + registered.
- `start_monitor/3,4` (since OTP 23.0) — standalone, atomically sets up a
  monitor (not a link). On unsuccessful start the caller is blocked until the
  monitor's `'DOWN'` message has been received and removed.

### WHY init is synchronous (crawl 11)

From "Verbatim quotes" #6:

> "To ensure a synchronized startup procedure, `start_link/3,4` does not
> return until `Module:init/1` has returned or failed."

Same for `start/3,4`, `start_monitor/3,4` (crawl 11, "Strict rules").

### bad-return-terminates / throw-is-valid-return (crawl 11)

From "Verbatim quotes" #1:

> "If a callback function fails or returns a bad value, the gen_server process
> terminates. However, an exception of class `throw` is not regarded as an
> error but as a valid return, from all callback functions."

### trap_exit is NOT automatic (crawl 11)

From "Verbatim quotes" #2:

> "a gen_server process does not trap exit signals automatically, this must
> be explicitly initiated in the callback module."

(e.g. `process_flag(trap_exit, true)` in `init/1`.)

### {error, Reason} from init/1 (since OTP 26.0) (crawl 11)

From "Verbatim quotes" #7:

> "{error,Reason} since OTP 26.0 ... The difference between returning
> `{stop, _}` and `{error, _}` from `Module:init/1`, is that `{error, _}`
> results in a graceful ('silent') termination since the gen_server process
> exits with reason `normal`."

So `{error, Reason}` exits `normal` (graceful/"silent"); `{stop, Reason}`
exits with `Reason`.

### start_ret detail (crawl 11)

- `{ok, Pid}` — created & initialized.
- `{error, {already_started, OtherPid}}` — a process with that `ServerName`
  exists; the new gen_server exited `normal` before calling `init/1`.
- `{error, timeout}` — `init/1` did not return within the start timeout;
  process killed with `exit_signal(_, kill)`.
- `ignore` — `init/1` returned `ignore` (process exits `normal`).
- `{error, Reason}` — `init/1` returned `{stop, Reason}` or `{error, Reason}`,
  or failed with `Reason`.

### call/2,3 exit reasons (crawl 11)

On timeout the *calling* process exits with `{Reason, Location}` where
`Reason = timeout`. Since OTP 24 uses process aliases, so late replies are not
received. May exit with: `timeout`, `noproc`, `{nodedown,Node}`,
`calling_self`, `shutdown`, `normal`, `{shutdown,Term}`, or `_OtherTerm`
(server died).

### stop/1,2,3 (since OTP 18.0) (crawl 11)

Orders the server to exit with `Reason`, waits for termination. Server calls
`Module:terminate/2` before exiting. Returns `ok` if it terminates with the
expected reason. Any reason other than `normal`, `shutdown`, or
`{shutdown,Term}` causes an error report via `logger`. Exits caller with
`timeout` / `noproc` / `{nodedown,Node}` on failure.

### Supervised shutdown with cleanup (crawl 02)

From "Verbatim quotes" #13:

> "If it is necessary to clean up before termination, the shutdown strategy
> must be a time-out value and the gen_server must be set to trap exit
> signals in function init. When ordered to shut down, the gen_server then
> calls the callback function terminate(shutdown, State):"

### Standalone stop (crawl 02)

From "Verbatim quotes" #14:

> "The callback function handling the stop request returns a tuple
> `{stop,normal,State1}`, where normal specifies that it is a normal
> termination and State1 is a new value for the state of the gen_server.
> This causes the gen_server to call terminate(normal, State1) and then it
> terminates gracefully."

## Practical rules

1. `start_link/3,4` for supervised servers; `start/3,4` for standalone (crawl 02).
2. Start is synchronous — do not send requests before it returns (crawl 11, #6).
3. Server name passed to `call`/`cast` must agree with the name used to start (crawl 02).
4. For supervised cleanup: shutdown strategy must be a timeout AND
   `process_flag(trap_exit, true)` in `init/1`; then `terminate(shutdown, State)` runs (crawl 02, #13).
5. `init` returns `{ok, State}` (or `{ok, State, Action}`, `ignore`,
   `{stop, Reason}`, `{error, Reason}`) (crawl 11).
6. `handle_call/3` returns `{reply, Reply, State1}`; if `{noreply,...}`, reply via `reply(From, Reply)` (crawl 11).
7. `handle_cast/2` returns `{noreply, State1}` or `{stop, Reason, State1}` (crawl 02).
8. Implement `handle_info/2` for non-request messages (e.g. `'EXIT'` when trapping exits) (crawl 02).
9. `{stop, normal, State1}` → `terminate(normal, State1)` then graceful termination (crawl 02, #14).
10. Colocate one process's interface and callbacks in one module (crawl 02, #5).
11. `trap_exit` is NOT automatic — set it explicitly if cleanup is needed (crawl 11, #2).
12. Bad return terminates; `throw` is a valid return, not an error (crawl 11, #1).
13. Prefer `{error, Reason}` over `{stop, Reason}` from `init/1` for graceful ("silent") termination (crawl 11, #7).
14. Use `format_status/1`; do NOT use deprecated `format_status/2` (crawl 11, #8).
15. Do not pass `spawn_opt` `monitor` in `start_opt` — `badarg` (crawl 11).
16. Do not hibernate between each call to a busy server — costly (≥2 GCs) (crawl 11).

## Review checklist

- [ ] `-behaviour(gen_server).` declared?
- [ ] Required callbacks exported (`init/1`, `handle_call/3`, `handle_cast/2`)?
- [ ] `handle_info/2` implemented if non-request messages expected?
- [ ] If `{continue, _}` returned, is `handle_continue/2` implemented? (else `undef` — crawl 11, #5)
- [ ] If `handle_call/3` returns `{noreply,...}`, is `reply(From, Reply)` called? (crawl 11)
- [ ] `start_link/3,4` (not `start/3,4`) for supervised servers? (crawl 02, #8)
- [ ] Server name in `call`/`cast` agrees with start name? (crawl 02)
- [ ] If cleanup needed: `process_flag(trap_exit, true)` in `init/1` AND shutdown strategy is a timeout? (crawl 02, #13)
- [ ] `format_status/1` used, not deprecated `format_status/2`? (crawl 11, #8)
- [ ] Return tuples all from the documented set (no invented shapes)?
- [ ] `throw` not abused as disguised error control flow? (crawl 11, #1)

## Implementation checklist

- [ ] Declare `-behaviour(gen_server).`.
- [ ] Export interface functions and callbacks.
- [ ] `init/1` returns a documented tuple.
- [ ] `handle_call/3`, `handle_cast/2` return documented tuples.
- [ ] `handle_info/2` if non-request messages expected.
- [ ] `handle_continue/2` if `{continue, _}` used anywhere.
- [ ] `terminate/2` if cleanup needed (return value ignored).
- [ ] `code_change/3` if release upgrades expected.
- [ ] `format_status/1` if state needs redaction/compaction.
- [ ] Choose start function: `start_link` (supervised, linked), `start` (standalone), `start_monitor` (standalone, monitored).
- [ ] Choose registration: `{local, Name}`, `{global, Name}`, `{via, RegMod, ViaName}`, or omit (pid).
- [ ] `process_flag(trap_exit, true)` in `init/1` if supervised cleanup required.

## Runtime / debugging checklist

- [ ] `sys:get_status/1,2` to inspect state (formatted via `format_status/1` if exported) (crawl 11).
- [ ] `{debug, Dbgs}` in `start_opt`/`enter_loop_opt` for tracing (crawl 11).
- [ ] `call/2,3` may exit: `timeout`, `noproc`, `{nodedown,Node}`, `calling_self`, `shutdown`, `normal`, `{shutdown,Term}`, `_OtherTerm` (crawl 11).
- [ ] Since OTP 24, `call/2,3` uses process aliases — late replies not received (crawl 11).
- [ ] Bad return / callback failure terminates the process; check logs (crawl 11, #1).
- [ ] `stop` reason other than `normal`/`shutdown`/`{shutdown,Term}` → error report via `logger` (crawl 11).
- [ ] On termination an exit signal with the same reason is sent to linked processes/ports (crawl 11).
- [ ] `terminate/2` NOT called if not trapping exits and killed (e.g. `brutal_kill`); called with `Reason=shutdown` only when trapping exits and shutdown strategy is a timeout (crawl 11).

## Validation hooks

- Compile with `erlc +warn_missing_spec` to catch missing callback specs.
- Run Dialyzer to verify return-tuple shapes match the `-callback` typespecs above.
- Use `ct`/`eunit` to exercise `call`/`cast` paths; assert reply values and state transitions.
- Use `sys:get_status/1,2` in tests to verify `format_status/1` output.
- Verify `start_link` returns `{ok, Pid}` synchronously before sending requests.
- Verify `{error, Reason}` from `init/1` exits `normal` (graceful); `{stop, Reason}` exits with `Reason`.

## Examples

### ch3 — minimal channel allocator (crawl 02)

```erlang
-module(ch3).
-behaviour(gen_server).

-export([start_link/0, alloc/0, free/1]).
-export([init/1, handle_call/3, handle_cast/2]).

start_link() ->
    gen_server:start_link({local, ch3}, ch3, [], []).

alloc() ->
    gen_server:call(ch3, alloc).

free(Ch) ->
    gen_server:cast(ch3, {free, Ch}).

init(_Args) ->
    {ok, channels()}.

handle_call(alloc, _From, Chs) ->
    {Ch, Chs2} = alloc(Chs),
    {reply, Ch, Chs2}.

handle_cast({free, Ch}, Chs) ->
    Chs2 = free(Ch, Chs),
    {noreply, Chs2}.
```

Argument walkthrough (crawl 02): `gen_server:start_link({local, ch3}, ch3, [], []) => {ok, Pid}`
1. `{local, ch3}` — name; omit => not registered (use pid); `{global, Name}` => `global:register_name/2`.
2. `ch3` — callback module.
3. `[]` — term passed as-is to `init`.
4. `[]` — options list.

### Supervised shutdown with cleanup (crawl 02)

```erlang
init(Args) ->
    ...,
    process_flag(trap_exit, true),
    ...,
    {ok, State}.
...
terminate(shutdown, State) ->
    %% Code for cleaning up here
    ...
    ok.
```

### Standalone stop (crawl 02)

```erlang
stop() -> gen_server:cast(ch3, stop).
handle_cast(stop, State) -> {stop, normal, State};
handle_cast({free, Ch}, State) -> ...
terminate(normal, State) -> ok.
```

### handle_info — exit messages (crawl 02)

```erlang
handle_info({'EXIT', Pid, Reason}, State) ->
    %% Code to handle exits here.
    ...
    {noreply, State1}.
```

### code_change/3 (crawl 02)

```erlang
code_change(OldVsn, State, Extra) ->
    %% Code to convert state (and more) during code change.
    ...
    {ok, NewState}.
```

### {continue, Continue} — deferred init work (crawl 11)

```erlang
init(Args) ->
    {ok, #{args => Args}, {continue, setup}}.

handle_continue(setup, State) ->
    %% Post-init action before any external message/request.
    {noreply, do_setup(State)}.
```

## Common mistakes

- `start/3,4` for a supervised server — use `start_link/3,4` (crawl 02, #8).
- `{noreply,...}` from `handle_call/3` without `reply(From, Reply)` — client blocks until timeout (crawl 11).
- `{continue, _}` without `handle_continue/2` — process exits `undef` (crawl 11, #5).
- Expecting `terminate/2` on supervised shutdown without `process_flag(trap_exit, true)` in `init/1` (crawl 02, #13).
- Assuming `trap_exit` is automatic — it is NOT (crawl 11, #2).
- Using deprecated `format_status/2` instead of `format_status/1` (crawl 11, #8).
- `{stop, Reason}` from `init/1` when graceful ("silent") termination is desired — use `{error, Reason}` (exits `normal`) (crawl 11, #7).
- Hibernating a busy server between every call — costly (≥2 GCs) (crawl 11).
- Relying on legacy timeout where a system message restarts the timer (known flaw); prefer `{timeout, Time, Message}` (crawl 11).
- `spawn_opt` `monitor` in `start_opt` — `badarg` (crawl 11).
- Treating `throw` as an error — it is a valid return from all callbacks (crawl 11, #1).

## Strict vs contextual guidance

### Strict (always follow)

- `start_link/3,4` for supervised; `start/3,4` for standalone (crawl 02).
- `init` returns only: `{ok, State}`, `{ok, State, Action}`, `{stop, Reason}`, `ignore`, `{error, Reason}` (crawl 11).
- All callbacks return only documented tuple shapes (crawl 11).
- `trap_exit` must be explicitly set if cleanup on exit is needed (crawl 11, #2).
- Bad return terminates; `throw` is a valid return (crawl 11, #1).
- Use `format_status/1`, not deprecated `format_status/2` (crawl 11, #8).
- `{continue, _}` requires `handle_continue/2` or the process exits `undef` (crawl 11, #5).

### Contextual (depends on the server)

- Whether to trap exits (only if cleanup needed on supervised shutdown) (crawl 02, #13).
- Whether to hibernate (costly; not for busy servers) (crawl 11).
- Legacy timeout vs `{timeout, Time, Message}` (latter not affected by system messages) (crawl 11).
- Whether to implement `terminate/2`, `code_change/3`, `format_status/1` (all optional) (crawl 11).
- `{error, Reason}` (graceful) vs `{stop, Reason}` (exits with Reason) from `init/1` (crawl 11, #7).
- Registration form: `{local, Name}`, `{global, Name}`, `{via, RegMod, ViaName}`, or none (pid) (crawl 11).

## Policy decisions for individual repos

- Default start function for supervised servers: `start_link/3,4`.
- Default registration: `{local, Name}` unless distribution is required.
- Require `handle_info/2` explicitly implemented (even if just logs and drops) rather than relying on the default.
- Require `format_status/1` for any server holding sensitive or large state.
- Prefer `{error, Reason}` over `{stop, Reason}` from `init/1` for graceful startup failure.
- Disallow `format_status/2` in new code.
- Disallow `spawn_opt` `monitor` in `start_opt`.
- Decide repo-wide whether to hibernate idle servers (and at what threshold).

## Related docs

- otp-behaviours.md
- gen-statem.md
- gen-event.md
- supervision.md
- proc-lib-and-sys.md
- runtime-debugging.md
- links-monitors-and-exits.md
- common-mistakes.md

## Related skills

- beam-gen-server
- beam-supervision
- beam-errors-failures
- beam-observability-debugging
