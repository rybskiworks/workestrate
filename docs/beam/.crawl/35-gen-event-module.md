# Crawl: stdlib/gen_event.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/gen_event.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/gen_event.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib 8.0.1)
- feeds_docs: gen-event.md

## Purpose
Generic event handling behavior. Provides a generic event manager process with
any number of event handlers that are added and deleted dynamically. An event
manager has a standard set of interface functions, includes functionality for
tracing and error reporting, and fits into an OTP supervision tree. Each event
handler is a callback module exporting a predefined set of functions.

The behavior function -> callback mapping (verbatim table):
```
gen_event module                   Callback module
----------------                   ---------------
gen_event:start
gen_event:start_monitor
gen_event:start_link       ----->  -

gen_event:add_handler
gen_event:add_sup_handler  ----->  Module:init/1

gen_event:notify
gen_event:sync_notify      ----->  Module:handle_event/2

gen_event:send_request
gen_event:call             ----->  Module:handle_call/2

-                          ----->  Module:handle_info/2

gen_event:delete_handler   ----->  Module:terminate/2

gen_event:swap_handler
gen_event:swap_sup_handler ----->  Module1:terminate/2
                                   Module2:init/1

gen_event:which_handlers   ----->  -

gen_event:stop             ----->  Module:terminate/2

-                          ----->  Module:code_change/3
```

## Key functions (exact arities)
- `start/0` — `start() -> start_ret()`. Equivalent to `start([])`.
- `start/1` — `start(EventMgrName :: emgr_name()) | (Options :: options()) -> start_ret()`. Stand-alone event manager, possibly nameless. Equivalent to `start(EventMgrName, Options)`.
- `start/2` — `start(EventMgrName, Options) -> start_ret()` (since OTP 20.0).
- `start_link/0` — `start_link() -> start_ret()`. Equivalent to `start_link([])`.
- `start_link/1` — `start_link(EventMgrName) | (Options) -> start_ret()`. Equivalent to `start_link(EventMgrName, Options)`.
- `start_link/2` — `start_link(EventMgrName :: emgr_name(), Options :: options()) -> start_ret()` (since OTP 20.0). Part of a supervision tree; links to caller.
- `start_monitor/0,1,2` (since OTP 23.0) — stand-alone, monitored; returns `{ok, {Pid, Mon}}` (`start_mon_ret()`), NOT `{ok, Pid}`.
- `add_handler(EventMgrRef, Handler, Args) -> term()` — add handler, calls `Module:init/1`.
- `add_sup_handler(EventMgrRef, Handler, Args) -> term()` — add supervised handler (links calling process).
- `swap_handler(EventMgrRef, OldHandler, NewHandler) -> ok | {error, term()}` — `OldHandler/NewHandler :: {handler(), term()}`.
- `swap_sup_handler(EventMgrRef, OldHandler, NewHandler) -> ok | {error, term()}` — swap + supervise NewHandler.
- `notify(EventMgrRef, Event) -> ok` — async event notification.
- `sync_notify(EventMgrRef, Event) -> ok` — synchronous; returns `ok` only after all handlers have handled the event.
- `call(EventMgrRef, Handler, Request) -> term()` — equivalent to `call(..., 5000)`.
- `call(EventMgrRef, Handler, Request, Timeout) -> term()` — synchronous call to a handler; exits calling process on failure with `{Reason, {gen_event, call, ArgList}}`.
- `send_request/3,5`, `receive_response/2,3`, `wait_response/2,3`, `check_response/2,3` — async call API (OTP 23.0+ for some).
- `delete_handler(EventMgrRef, Handler, Args) -> term()` — deletes handler, calls `Module:terminate/2`; returns terminate's return value, or `{error, module_not_found}`.
- `which_handlers(EventMgrRef) -> [handler()]` — list all installed handlers.
- `stop(EventMgrRef) -> ok` — equivalent to `stop(EventMgrRef, normal, infinity)`.
- `stop(EventMgrRef, Reason, Timeout) -> ok` (since OTP 18.0) — orders manager to exit with Reason; calls `Module:terminate(stop, ...)` for each handler. Non-normal/shutdown reasons issue a logger error report. Exits caller with `timeout`/`noproc` on failure.

Type aliases:
- `start_ret() :: {ok, pid()} | {error, term()}`
- `start_mon_ret() :: {ok, {pid(), reference()}} | {error, term()}`
- `add_handler_ret() :: ok | term() | {'EXIT', term()}`
- `del_handler_ret() :: ok | term() | {'EXIT', term()}`
- `handler() :: atom() | {atom(), Id :: term()}` (Module or {Module, Id}).

Note: there is NO `get_state` function for gen_event managers on this page.
gen_event does not expose `get_state`/`get_state` like gen_server. Introspection
of manager state is via `sys:get_status/1` (referenced in See Also), not a
gen_event-native get_state. (The crawl task asked about get_state for managers;
it is not part of the gen_event module API.)

## add_handler vs add_sup_handler (supervised semantics)
`add_handler/3`: adds a handler by calling `Module:init/1`. On success returns
`ok`. If `init/1` fails with Reason -> returns `{'EXIT', Reason}`; if `init/1`
returns `{error, Reason}` -> returns `{error, Reason}`. No link between the
calling process and the handler/manager is established.

`add_sup_handler/3`: adds the handler as `add_handler/3`, BUT the event manager
supervises the connection by linking the event handler and the calling process.

- If the CALLING PROCESS later terminates with Reason: the event manager deletes
  any supervised event handlers by calling `Module:terminate/2`, then calls
  `Module:handle_info/2` for each remaining handler. (Handlers attached to a
  manager that in turn has a supervised handler should expect
  `Module:handle_info({'EXIT', Pid, Reason}, State)` callbacks.)
- If the EVENT HANDLER is deleted later, the event manager sends a message
  `{gen_event_EXIT, Handler, Reason}` to the calling process. Reason is one of:
  - `normal` — handler removed via `delete_handler/3` or a callback returned
    `remove_handler`.
  - `shutdown` — handler removed because the event manager is terminating.
  - `{swapped, NewHandler, Pid}` — process Pid replaced the handler with
    NewHandler via `swap_handler/3` or `swap_sup_handler/3`.
  - Other `term()` — handler removed because of an error (term depends on error).

So: a supervised handler's death is reported to its supervisor process via
`{gen_event_EXIT, Handler, Reason}`; the supervisor process dying causes the
handler to be terminated. The manager itself is NOT killed by a handler error
(see Strict rules).

## Handler callback contract & return shapes (verbatim)
### `init/1`
```
-callback init(InitArgs :: term()) ->
                  {ok, State :: term()} | {ok, State :: term(), hibernate} | {error, Reason :: term()}.
```
- InitArgs is the `Args` of `add_handler/3`/`add_sup_handler/3`, OR — for a swap
  — `{Args, Term}` where `Term` is the return value of the old handler's
  `terminate/2`.
- `{ok, State}` or `{ok, State, hibernate}` on success. `{ok, State, hibernate}`
  causes the event manager to hibernate via `proc_lib:hibernate/3`.

### `handle_event/2`
```
-callback handle_event(Event :: term(), State :: term()) ->
                          {ok, NewState :: term()} |
                          {ok, NewState :: term(), hibernate} |
                          {swap_handler,
                           Args1 :: term(),
                           NewState :: term(),
                           Handler2 :: atom() | {atom(), Id :: term()},
                           Args2 :: term()} |
                          remove_handler.
```
- Called for each installed handler on `notify/2` / `sync_notify/2`.
- `{ok, NewState}` / `{ok, NewState, hibernate}` keeps handler (one hibernate
  request from any handler hibernates the WHOLE manager).
- `{swap_handler, Args1, NewState, Handler2, Args2}`: handler replaced by
  Handler2 via `Module:terminate(Args1, NewState)` then
  `Module2:init({Args2, Term})` where Term = terminate's return value.
- `remove_handler`: handler deleted via `Module:terminate(remove_handler, State)`.

### `handle_call/2`
```
-callback handle_call(Request :: term(), State :: term()) ->
                         {ok, Reply :: term(), NewState :: term()} |
                         {ok, Reply :: term(), NewState :: term(), hibernate} |
                         {swap_handler,
                          Reply :: term(),
                          Args1 :: term(),
                          NewState :: term(),
                          Handler2 :: atom() | {atom(), Id :: term()},
                          Args2 :: term()} |
                         {remove_handler, Reply :: term()}.
```
- Same shapes as `handle_event/2` but each carries a `Reply` returned to the
  `call/3,4` client. `remove_handler` form here is `{remove_handler, Reply}`.

### `handle_info/2` (optional)
```
-callback handle_info(Info :: term(), State :: term()) ->
                         {ok, NewState :: term()} |
                         {ok, NewState :: term(), hibernate} |
                         {swap_handler, Args1, NewState, Handler2, Args2} |
                         remove_handler.
```
- Called for non-event/non-request messages. In particular called when a process
  that called `add_sup_handler/3` terminates — expect
  `Module:handle_info({'EXIT', Pid, Reason}, State)`.
- Default implementation logs unexpected Info and returns `{ok, State}`.

### `terminate/2` (optional)
```
-callback terminate(Args ::
                        term() |
                        {stop, Reason :: term()} |
                        stop | remove_handler |
                        {error, {'EXIT', Reason :: term()}} |
                        {error, term()},
                    State :: term()) ->
                       term().
```
- `Args` semantics:
  - the `Args` of `delete_handler/3`/`swap_handler/3`/`swap_sup_handler/3`;
  - `{stop, Reason}` if supervised connection to a process that terminated with Reason;
  - `stop` if the event manager is terminating;
  - `remove_handler` if another callback returned `remove_handler` / `{remove_handler, Reply}`;
  - `{error, Term}` if a callback returned an unexpected value Term;
  - `{error, {'EXIT', Reason}}` if a callback failed.
- Return value: becomes `delete_handler/3`'s return value; passed to new
  handler's `init/1` on swap; otherwise ignored. Default impl does no cleanup.

### `code_change/3` (optional)
```
-callback code_change(OldVsn :: term() | {down, term()}, State :: term(), Extra :: term()) ->
                         {ok, NewState :: term()}.
```
- Called on release upgrade/downgrade with `{update, Module, Change, ...}` in appup.
- `OldVsn = Vsn` (upgrade) or `{down, Vsn}` (downgrade). If no `vsn` attribute,
  version = Beam file checksum. `Extra` from `{advanced, Extra}`.
- If `{advanced, Extra}` is specified but `code_change/3` is not implemented,
  the handler crashes with `undef`.

### `format_status/1,2` (optional, since OTP 25.0)
For sys status formatting; not detailed in extraction scope.

## Strict rules (handlers share the manager process / no isolation; what kills a handler)
- All event handlers run in the SAME process — the single event manager process.
  There is NO per-handler process isolation. Handlers are invoked sequentially
  within the manager process.
- gen_event is more tolerant of callback errors than other behaviors: if a
  callback function for an installed handler fails with Reason OR returns a bad
  value Term, the event manager DOES NOT FAIL. It deletes only that handler by
  calling `Module:terminate/2` with `{error, {'EXIT', Reason}}` or `{error, Term}`
  respectively. No other event handler is affected.
- A handler is killed (deleted) when:
  - `delete_handler/3` is called on it;
  - a callback returns `remove_handler` (or `{remove_handler, Reply}` from `handle_call/2`);
  - `swap_handler/3`/`swap_sup_handler/3` replaces it;
  - `stop/1,3` is called on the manager (all handlers get `terminate(stop, ...)`);
  - its supervised calling process dies (for `add_sup_handler` handlers) —
    manager deletes supervised handlers and calls `handle_info` for the rest;
  - a callback crashes or returns a bad value (handler-only deletion; manager survives).
- The event manager traps exit signals automatically.
- Hibernation: if ANY handler returns `{ok, NewState, hibernate}` (or init returns
  `{ok, State, hibernate}`), the WHOLE event manager process hibernates via
  `proc_lib:hibernate/3`. Hibernation costs >= 2 GCs; avoid for busy managers.
- `notify/2` does not fail even if the manager does not exist, UNLESS the manager
  is specified as a Name (registered name).
- Distributed signaling: blocking signaling over distribution can significantly
  delay `call` timeouts (see Erlang Reference Manual, Processes chapter).

## Verbatim quotes
- "An event manager implemented using this module has a standard set of
  interface functions and includes functionality for tracing and error reporting.
  It also fits into an OTP supervision tree."
- "As each event handler is one callback module, an event manager has many
  callback modules that are added and deleted dynamically. gen_event is
  therefore more tolerant of callback module errors than the other behaviors.
  If a callback function for an installed event handler fails with Reason, or
  returns a bad value Term, the event manager does not fail. It deletes the
  event handler by calling callback function Module:terminate/2, giving as
  argument {error, {'EXIT', Reason}} or {error, Term}, respectively. No other
  event handler is affected."
- "Notice that an event manager does trap exit signals automatically."
- "Notice that when multiple event handlers are invoked, it is sufficient that
  one single event handler returns a hibernate request for the whole event
  manager to go into hibernation."
- (add_sup_handler) "the event manager also supervises the connection by linking
  the event handler and the calling process."
- (add_sup_handler) "If the event handler is deleted later, the event manager
  sends a message {gen_event_EXIT,Handler,Reason} to the calling process."
- (swap_handler) "First the old event handler OldHandler is deleted... Then the
  new event handler NewHandler is added and initiated by calling
  NewModule:init({Args2,Term})... This makes it possible to transfer information
  from OldHandler to NewHandler."
- (swap_handler) "The new handler is added even if the specified old event handler
  is not installed, in which case Term = error, or if OldModule:terminate/2 fails
  with Reason, in which case Term = {'EXIT', Reason}. The old handler is deleted
  even if NewModule:init/1 fails."
- (swap_handler) "If there was a supervised connection between OldHandler and a
  process Pid, there is a supervised connection between NewHandler and Pid instead."
- (sync_notify) "This function will return ok after the event has been handled by
  all event handlers."
- (call) "When this call fails it exits the calling process. The exit term is of
  the form {Reason, Location} where Location = {gen_event, call, ArgList}."
- (stop) "Any other reason than normal, shutdown, or {shutdown, Term} causes an
  error report to be issued using logger."
- (start_link warning) "The start was made synchronous in OTP 26.0 and a
  guarantee was implemented that no process link 'EXIT' message from a failed
  start will linger in the caller's inbox."

## Version notes
- Page: OTP 29.0.2, stdlib 8.0.1.
- `start/2`, `start_link/2` since OTP 20.0.
- `start_monitor/0,1,2` since OTP 23.0.
- `stop/3` since OTP 18.0.
- `format_status/1` since OTP 25.0.
- `receive_response/2` since OTP 24.0; `check_response/2` since OTP 23.0.
- start_link start made synchronous in OTP 26.0 (fixed lingering EXIT message
  race present before OTP 26.0).
- See Also: `supervisor`, `sys`.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html#call/3 (call exit-term semantics)
- https://www.erlang.org/doc/apps/stdlib/supervisor.html (supervision tree integration)
- https://www.erlang.org/doc/apps/stdlib/sys.html (sys debugging / get_status)
- https://www.erlang.org/doc/apps/stdlib/sys.html#get_status/1 (manager status introspection — closest to "get_state" for managers)
- https://www.erlang.org/doc/apps/stdlib/proc_lib.html#hibernate/3 (hibernation mechanism)
- https://www.erlang.org/doc/system/events.html (OTP Design Principles: gen_event section)
- https://www.erlang.org/doc/system/ref_man_processes.html#blocking-signaling-over-distribution (distributed signaling / call timeout delays)
- https://www.erlang.org/doc/apps/sasl/appup.html (release upgrade appup instructions / code_change)
- https://www.erlang.org/doc/apps/kernel/logger.html (error reports from stop/3)

### Skipped
- https://www.erlang.org/doc/apps/erts/erlang.html#hibernate/3 (erts primitive; out of beam-doc scope here)
- https://www.erlang.org/doc/apps/erts/erlang.html#exit/1, #exit_signal/2, #register/2, #monotonic_time/1 (erts primitives)
- https://www.erlang.org/doc/apps/kernel/global.html, #register_name/2 (global naming)
- https://www.erlang.org/doc/index.html, stdlib.epub, llms.txt, https://erlang.org, https://www.ericsson.com (navigation/non-doc)
- GitHub source links (https://github.com/erlang/otp/blob/OTP-29.0.2/lib/stdlib/src/gen_event.erl#L...) — source, not docs
- In-page anchor links (#add_handler/3, #c:init/1, #t:start_ret/0, etc.) — same page
