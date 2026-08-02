# Crawl: stdlib/gen_statem.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/gen_statem.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/gen_statem.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib v8.0.1)
- feeds_docs: gen-statem.md

## Purpose
Generic state machine behaviour. Since Erlang/OTP 20.0 replaces its predecessor
`gen_fsm`, and should be used for new code. A `gen_statem` is a generic state
machine server process with a standard set of interface functions, tracing and
error reporting, and fits into an OTP supervision tree.

Features beyond `gen_fsm`: co-located state code, arbitrary term state, event
postponing, self-generated events, state time-out, multiple generic named
time-outs, absolute time-out time, automatic state enter calls, reply from other
state than the request (traceable with sys), multiple replies, changing the
callback module.

## Callback modes & signatures (exact)

Two callback modes, selected by the return value of `Module:callback_mode/0`:

- `state_functions` — state must be an atom (`state_name/0`); one callback
  function per state `Module:StateName/3` is used. (gen_fsm-like.)
- `handle_event_function` — state can be any term; the single callback
  `Module:handle_event/4` is used for all states.

`callback_mode/0` is called when starting the gen_statem, after code change, and
after changing the callback module via actions `change_callback_module`,
`push_callback_module`, `pop_callback_module`. The result is cached.

### StateEnter mode
`state_enter` is the atom added to the `callback_mode/0` return list to enable
state enter calls. At every state change (`NextState =/= CurrentState`) the
engine calls the state callback with arguments `(enter, OldState, Data)` or
`(enter, OldState, State, Data)` depending on callback mode. A state enter call
is also done right before entering the initial state (then `OldState =:= State`).
A state enter call may be repeated without a state change by returning
`repeat_state` / `repeat_state_and_data`. `code_change/4` state transform is a
state rename, not a state change (no enter call).

### Exact callback signatures (verbatim from typespecs)

```erlang
%% Mandatory
-callback init(Args :: term()) -> init_result(state()).
%% init_result(StateType, DataType) ::
%%     {ok, State, Data}
%%   | {ok, State, Data, Actions :: [action()] | action()}
%%   | ignore
%%   | {stop, Reason :: term()}
%%   | {error, Reason :: term()}.   % {error,_} since OTP 26.0

-callback callback_mode() -> callback_mode_result().
%% callback_mode_result() :: callback_mode() | [callback_mode() | state_enter()].
%% callback_mode() :: state_functions | handle_event_function.
%% state_enter()  :: state_enter.

%% State callback in state_functions mode (optional, one per state atom)
-callback 'StateName'(enter, OldStateName :: state_name(), data()) ->
                state_enter_result(state_name());
            (EventType :: event_type(), EventContent :: event_content(), Data :: data()) ->
                event_handler_result(state_name()).
%% StateName cannot be `terminate` (collides with terminate/3).

%% State callback in handle_event_function mode (optional)
-callback handle_event(enter, OldState, CurrentState, Data) ->
                state_enter_result(CurrentState)
            when OldState :: state(), CurrentState :: state(), Data :: data();
          (EventType, EventContent, CurrentState, Data) ->
                event_handler_result(state())
            when EventType :: event_type(),
                 EventContent :: event_content(),
                 CurrentState :: state(),
                 Data :: data().

%% Optional
-callback code_change(OldVsn :: term() | {down, term()},
                      OldState :: state(), OldData :: data(), Extra :: term()) ->
        {ok, NewState :: state(), NewData :: data()} | (Reason :: term()).

-callback format_status(Status) -> NewStatus
        when Status :: format_status(), NewStatus :: format_status().   % since OTP 25.0

-callback format_status(StatusOption, [[{Key,Value}] | state() | data()]) -> Status :: term()
        when StatusOption :: normal | terminate.   % DEPRECATED; use format_status/1

-callback terminate(Reason :: normal | shutdown | {shutdown, term()} | term(),
                    CurrentState :: state(), data()) -> any().
```

Supporting types:
```erlang
-type state()       :: state_name() | term().
-type state_name()  :: atom().
-type data()        :: term().
-type from()        :: {To :: pid(), Tag :: reply_tag()}.
-type event_type()  :: external_event_type() | timeout_event_type() | internal.
-type external_event_type() :: {call, From :: from()} | cast | info.
-type timeout_event_type()  :: timeout | {timeout, Name :: term()} | state_timeout.
-type event_content() :: term().
```

## Return / action shapes (verbatim from typespecs)

### `init_result(StateType, DataType)`
```erlang
{ok, State, Data}
| {ok, State, Data, Actions :: [action()] | action()}
| ignore
| {stop, Reason :: term()}
| {error, Reason :: term()}   % since OTP 26.0
```

### `event_handler_result(StateType, DataType)` (state callback after an event)
```erlang
{next_state, NextState, NewData}
| {next_state, NextState, NewData, Actions :: [action()] | action()}
| state_callback_result(action(), DataType)
```
`{next_state, NextState, NewData [, Actions]}` — state transition to `NextState`
(may equal current state); sets `NewData`; executes `Actions`. If
`NextState =/= CurrentState` it is a state change.

### `state_callback_result(ActionType, DataType)` (common to event & enter calls)
```erlang
{keep_state, NewData}
| {keep_state, NewData, Actions :: [ActionType] | ActionType}
| keep_state_and_data
| {keep_state_and_data, Actions :: [ActionType] | ActionType}
| {repeat_state, NewData}
| {repeat_state, NewData, Actions :: [ActionType] | ActionType}
| repeat_state_and_data
| {repeat_state_and_data, Actions :: [ActionType] | ActionType}
| stop
| {stop, Reason :: term()}
| {stop, Reason :: term(), NewData :: DataType}
| {stop_and_reply, Reason :: term(), Replies :: [reply_action()] | reply_action()}
| {stop_and_reply, Reason :: term(), Replies :: [reply_action()] | reply_action(), NewData :: DataType}
```
- `{keep_state, NewData [, Actions]}` == `{next_state, CurrentState, NewData [, Actions]}`.
- `keep_state_and_data` == `{keep_state, CurrentData [, Actions]}`.
- `{repeat_state, NewData [, Actions]}` — if state enter calls are on, the state
  enter call is repeated; otherwise same as `{keep_state, NewData [, Actions]}`.
- `{stop, Reason [, NewData]}` — terminates via `Module:terminate/3`; exit signal
  sent to linked processes/ports.
- `stop` == `{stop, normal}`.
- `{stop_and_reply, Reason, Replies [, NewData]}` — sends all `Replies`, then
  terminates like `{stop, Reason [, NewData]}`.
- "All these terms are tuples or atoms and will be so in all future versions."

### `state_enter_result(State, DataType)` (state callback after a state enter call)
```erlang
{next_state, State, NewData}                       % State MUST equal current state
| {next_state, State, NewData, Actions :: [enter_action()] | enter_action()}
| state_callback_result(enter_action(), DataType)
```
From a state enter call: `postpone` is not allowed; `{next_event,_,_}` is not
allowed; changing state is not allowed (crash if `NextState =/= State`).

### `action()` (transition actions; not allowed from state enter calls)
```erlang
postpone
| {postpone, Postpone :: postpone()}
| {next_event, EventType :: event_type(), EventContent :: event_content()}
| {change_callback_module, NewModule :: module()}   % since OTP 22.3
| {push_callback_module, NewModule :: module()}     % since OTP 22.3
| pop_callback_module                               % since OTP 22.3
| enter_action()
```
where
```erlang
-type enter_action() ::
        hibernate
      | {hibernate, Hibernate :: hibernate()}
      | timeout_action()
      | reply_action().
-type reply_action() :: {reply, From :: from(), Reply :: term()}.
-type postpone() :: boolean().
-type hibernate() :: boolean().
```
- `{reply, From, Reply}` — replies to a `call/2,3` caller. `From` must be the
  `From` from `{call, From}` event.
- `{next_event, EventType, EventContent}` — stores event for insertion as next to
  process, before already queued events; order preserved. Use `internal` event
  type to distinguish inserted events from external ones.
- Actions execute in containing list order; later transition options override
  earlier of the same type (last wins).

## Timeouts (event / state_timeout / generic) and postpone

Three timeout kinds, each set by a `timeout_action()` and generating a distinct
`timeout_event_type()`:

| Timeout kind   | Setting action                       | Event type generated   |
|----------------|---------------------------------------|------------------------|
| Event timeout  | `{timeout, Time, EventContent [, Opts]}` (or bare `Time`) | `timeout` |
| Generic (named)| `{{timeout, Name}, Time, EventContent [, Opts]}` (since OTP 20.0) | `{timeout, Name}` |
| State timeout  | `{state_timeout, Time, EventContent [, Opts]}` (since OTP 19.3) | `state_timeout` |

```erlang
-type timeout_action() ::
        (Time :: event_timeout())                                              % short for {timeout, Time, Time}
      | {timeout, Time :: event_timeout(), EventContent}
      | {timeout, Time, EventContent, Options :: timeout_option() | [timeout_option()]}
      | {{timeout, Name :: term()}, Time :: generic_timeout(), EventContent}
      | {{timeout, Name}, Time, EventContent, Options}
      | {state_timeout, Time :: state_timeout(), EventContent}
      | {state_timeout, Time, EventContent, Options}
      | timeout_cancel_action()       % since OTP 22.1
      | timeout_update_action();      % since OTP 22.1
-type timeout_cancel_action() ::
        {timeout, cancel} | {{timeout, Name :: term()}, cancel} | {state_timeout, cancel};
-type timeout_update_action() ::
        {timeout, update, EventContent}
      | {{timeout, Name :: term()}, update, EventContent}
      | {state_timeout, update, EventContent};
-type timeout_option() :: {abs, Abs :: boolean()}.
```

Behaviour notes:
- Event timeout: any event (incl. retried/inserted, and a state_timeout zero
  event generated before it) cancels it. Cannot be explicitly cancelled (auto).
  `Time = infinity` → no timer. Relative `0` → timeout event enqueued before any
  not-yet-received external event (but after already queued events).
- State timeout: a state change cancels it; if set during a state change it runs
  in `NextState`. Restart by setting again; cancel via `infinity` or
  `{state_timeout, cancel}`.
- Generic timeout: identified by `Name`; setting same `Name` restarts it; cancel
  via `infinity` or `{{timeout, Name}, cancel}`.
- `timeout_event_type()` = `timeout | {timeout, Name} | state_timeout`.
- In short: the action to set a timeout with `EventType` is `{EventType, Time, ...}`.

Postpone: `postpone` / `{postpone, boolean()}`. If true, the current event is
postponed; after a state change (`NextState =/= State`) it is retried. Ignored
from `init/1` and `enter_loop` (no event to postpone). Not allowed from state
enter calls.

Transition sequence (after state callback returns): process actions in order
(replies sent, transition options set) → if state enter calls and initial/repeat
→ call current state callback with `(enter, State, Data)`/`(enter, State, State,
Data)` → if state enter calls and state changed → call new state callback with
`(enter, OldState, Data)`/`(enter, OldState, State, Data)` → if postpone true,
postpone current event (on state change, reset queue to oldest postponed) →
insert `next_event` events before queued events → handle timeout timers (zero
timeouts enqueued before external events) → call state callback with oldest
enqueued event, or go into receive/hibernate.

## Key functions

```erlang
%% Start functions (proc_lib-based)
-spec start(Module, Args, Opts) -> start_ret()
  when Module :: module(), Args :: term(), Opts :: [start_opt()].
-spec start(ServerName, Module, Args, Opts) -> start_ret().   % registered, not linked
-spec start_link(Module, Args, Opts) -> start_ret().         % linked, not registered
-spec start_link(ServerName, Module, Args, Opts) -> start_ret().  % linked + registered
-spec start_monitor(Module, Args, Opts) -> start_mon_ret().  % since OTP 23.0; monitored, not linked
-spec start_monitor(ServerName, Module, Args, Opts) -> start_mon_ret().

%% start_ret() :: {ok, pid()} | ignore | {error, term()}.
%% start_mon_ret() :: {ok, {pid(), reference()}} | ignore | {error, term()}.
%% start_opt() :: {timeout, timeout()} | {spawn_opt, [proc_lib:start_spawn_option()]}
%%              | {hibernate_after, timeout()} | {debug, [sys:debug_option()]}.
%% server_name() :: {local, atom()} | {global, term()} | {via, module(), term()}.
%% Note: spawn option `monitor` not allowed (badarg).

%% Stop
-spec stop(ServerRef) -> ok.                                  % == stop(ServerRef, normal, infinity)
-spec stop(ServerRef, Reason, Timeout) -> ok.                % calls terminate/3

%% Synchronous call
-spec call(ServerRef, Request) -> Reply.                     % == call(ServerRef, Request, infinity)
-spec call(ServerRef, Request, Timeout) -> Reply.
%%   Timeout :: timeout() | {clean_timeout, T} | {dirty_timeout, T}
%%   ({clean/_dirty} now equivalent to plain Timeout; late replies prevented via aliases)
%%   Event delivered as {call, From}. Reply via {reply, From, Reply} action or reply/2,3.

%% Asynchronous cast
-spec cast(ServerRef, Msg) -> ok.                            % event type `cast`

%% Async call API (since OTP 23.0)
-spec send_request(ServerRef, Request) -> request_id().
-spec send_request(ServerRef, Request, Label, ReqIdCollection) -> request_id_collection().
-spec receive_response(ReqId) -> Response | timeout.        % since OTP 24.0
-spec receive_response(ReqId, Timeout) -> ...
-spec receive_response(ReqIdCollection, Timeout, Delete) -> ...   % since OTP 25.0
-spec wait_response(ReqId) -> Response | timeout.
-spec wait_response(ReqId, WaitTime) -> ...
-spec wait_response(ReqIdCollection, WaitTime, Delete) -> ...
-spec check_response(Msg, ReqId) -> Result.                 % since OTP 23.0
-spec check_response(Msg, ReqIdCollection, Delete) -> ...   % since OTP 25.0

%% ReqId collection helpers
-spec reqids_new() -> request_id_collection().              % since OTP 25.0
-spec reqids_size(ReqIdCollection) -> non_neg_integer().
-spec reqids_to_list(ReqIdCollection) -> [{request_id(), term()}].
-spec reqids_add(ReqId, Label, ReqIdCollection) -> NewReqIdCollection.

%% Reply helpers (replies not visible in sys debug output)
-spec reply(Replies :: [reply_action()] | reply_action()) -> ok.
-spec reply(From :: from(), Reply :: term()) -> ok.

%% Take over a proc_lib-started process
-spec enter_loop(Module, Opts, State, Data) -> no_return().
-spec enter_loop(Module, Opts, State, Data, Server_or_Actions) -> no_return().
-spec enter_loop(Module, Opts, State, Data, Server, Actions) -> no_return().
%%   Opts :: [enter_loop_opt()], State :: state(), Data :: data(),
%%   Server :: server_name() | pid(), Actions :: [action()] | action().
```

## Strict rules

- `callback_mode/0` body should return an inline constant value; otherwise the
  module "is doing something strange."
- In `state_functions` mode the state MUST be an atom; `StateName` cannot be
  `terminate` (collides with `Module:terminate/3`).
- From a state enter call: `postpone` is NOT allowed; `{next_event, _, _}` is
  NOT allowed; changing state is NOT allowed (crash if `NextState =/= State`).
  Use `{keep_state, ...}` / `keep_state_and_data` from enter calls.
- `code_change/4` returning a failure `Reason` cannot be an `{ok,_,_}` tuple
  (regarded as success) nor `{ok,_}` (also invalid). Recommended: an atom
  (wrapped in `{error, Reason}`).
- A release upgrade/downgrade with `Change = {advanced, Extra}` when
  `code_change/4` is not implemented crashes with `undef`.
- If `code_change/4` should change `callback_mode`, it MUST be done via
  `code_change/4` (else new callback mode not honoured → crash).
- `spawn_opt` `monitor` is not allowed in start options (badarg).
- `format_status/2` is deprecated; use `format_status/1` (since OTP 25.0).
- `terminate/3` return value is ignored.
- For reasons other than `normal`, `shutdown`, `{shutdown, Term}` an error
  report is issued via `logger`.
- `start_link/3,4` does not return until `Module:init/1` has returned or failed.
- `start` (not linked) cannot be used by a supervisor to start a child;
  `start_monitor` likewise (not linked).
- `enter_loop` requires the calling process to have been started by a proc_lib
  start function; if `Server` is a `server_name()` it must already be registered.
- `reply/2,3` replies are not visible in sys debug output.

## Verbatim quotes

- "All these terms are tuples or atoms and will be so in all future versions of
  gen_statem." (re: `state_callback_result` stop/keep/repeat shapes)
- "If this function's body does not return an inline constant value the callback
  module is doing something strange." (re: `callback_mode/0`)
- "Note that the state `terminate` is not possible to use since it would collide
  with the optional callback function `Module:terminate/3`."
- "Should you return `{next_state, NextState, ...}` with `NextState =/= State` the
  gen_statem crashes." (from a state enter call)
- "It is not allowed to change states from this call." (state enter call)
- "In short; the action to set a time-out with `EventType` is
  `{EventType, Time, ...}`."
- "Any event that arrives cancels this time-out. Note that a retried or inserted
  event counts as arrived. So does a state time-out zero event, if it was
  generated before this time-out is requested." (event timeout)
- "A state change cancels a `state_timeout/0` and any new transition option of
  this type belongs to the new state."
- "Setting a timer with the same `Name` while it is running will restart it with
  the new time-out value. Therefore it is possible to cancel a specific time-out
  by setting it to `infinity`."
- "The behaviour of a time-out zero (a time-out with time 0) differs subtly from
  Erlang's `receive ... after 0 ... end`. The latter receives one message if
  there is one, while using the `timeout_action/0` `{timeout, 0}` does not
  receive any external event. `gen_server`'s time-out works like Erlang's
  `receive ... after 0 ... end`, in contrast to `gen_statem`."
- "A reply sent with this function is not visible in sys debug output." (reply/2,3)
- "Using spawn option `monitor` is not allowed, it causes a `badarg` failure."

## Version notes

- Module: `gen_statem` behaviour, stdlib v8.0.1, OTP 29.0.2.
- Replaces `gen_fsm` since Erlang/OTP 20.0.
- `callback_mode/0`: since OTP 19.1.
- `state_enter` mode: since OTP 19.2.
- `state_timeout`: since OTP 19.3.
- Generic named timeouts `{{timeout, Name}, ...}`: since OTP 20.0.
- `change_callback_module` / `push_callback_module` / `pop_callback_module`
  actions: since OTP 22.3.
- `timeout_cancel_action` / `timeout_update_action`: since OTP 22.1.
- `start_monitor/3,4` and async call API (`send_request/2`, `wait_response/2,3`,
  `check_response/2`): since OTP 23.0.
- `receive_response/2,3`: since OTP 24.0.
- `format_status/1`: since OTP 25.0 (replaces deprecated `format_status/2`).
- `check_response/3`, `receive_response/3`, `reqids_*` as listed: since OTP 25.0.
- `init/1` returning `{error, Reason}`: allowed since OTP 26.0.
- `{ok, ...}` init tuples existed since OTP 19.1 (before that not ok-tagged).
- Late-reply issue with `clean_timeout`/`dirty_timeout` now prevented via process
  aliases; those forms now equivalent to plain `Timeout`.

## Discovered links

### Relevant (crawl later)
- stdlib/gen_fsm.html (rewrite guide, migration): https://www.erlang.org/doc/apps/stdlib/gen_fsm.html
- stdlib/gen_server.html (call/3 semantics): https://www.erlang.org/doc/apps/stdlib/gen_server.html
- stdlib/supervisor.html (child spec): https://www.erlang.org/doc/apps/stdlib/supervisor.html
- stdlib/sys.html (debug options, get_status, system_event): https://www.erlang.org/doc/apps/stdlib/sys.html
- stdlib/proc_lib.html (start_spawn_option, hibernate/3): https://www.erlang.org/doc/apps/stdlib/proc_lib.html
- kernel/logger.html (error reports): https://www.erlang.org/doc/apps/kernel/logger.html
- system/statem.html (gen_statem Behaviour, User's Guide): https://www.erlang.org/doc/system/statem.html
- system/sup_princ.html#child-specification: https://www.erlang.org/doc/system/sup_princ.html
- system/release_handling.html#instr (release handling / appup): https://www.erlang.org/doc/system/release_handling.html
- system/ref_man_processes.html (process aliases, errors, blocking/signaling): https://www.erlang.org/doc/system/ref_man_processes.html
- system/index.html (OTP Design Principles root): https://www.erlang.org/doc/system/index.html

### Skipped
- Search/documentation settings UI links, in-page anchors to functions already
  captured, and external erlang.org top-level navigation.
