# Crawl: events.html
- seed_url: https://www.erlang.org/doc/system/events.html
- canonical_url: https://www.erlang.org/doc/system/events.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2; major-vsn 29)
- feeds_docs: gen-event.md, otp-behaviours.md

## Purpose
Documents the `gen_event` behaviour: how event managers are started, how event
handlers are added/deleted, how events are notified, and the callback contract
each handler module must implement. Intended to be read alongside the
`gen_event` reference page in STDLIB.

## Key concepts
- **Event manager**: a named object (process) to which events can be sent.
  An event can be an error, an alarm, or information to be logged.
- **Event handler**: a callback module installed in the manager. Zero, one, or
  many handlers can be installed in a single manager.
- When the manager is notified of an event, the event is processed by ALL
  installed handlers, in the order they were added.
- The manager maintains a list of `{Module, State}` pairs — `Module` is the
  handler, `State` is that handler's internal state.
- The event manager is a process; each event handler is a callback module
  (NOT a separate process). Handlers share the manager's process — there is
  no process isolation between handlers.
- Naming: `{local, Name}` (locally registered), `{global, Name}` (via
  `global:register_name/2`), or omitted (use pid).
- `start_link/1` for supervision tree membership; `start/1` for standalone.

## Strict rules / invariants
- `init/1` MUST return `{ok, State}` — `State` becomes the handler's internal state.
- `handle_event/2` MUST return `{ok, State1}` — `State1` is the new handler state.
- `terminate/2` is the opposite of `init/1` (cleanup); its return value is IGNORED.
- Handlers are invoked in the same order they were added.
- `start_link/1` must be used when the manager is part of a supervision tree
  (started by a supervisor); `start/1` for standalone.
- If part of a supervision tree, no stop function is needed — the supervisor
  terminates the manager per its shutdown strategy.
- `handle_info/2` must be implemented to receive non-event messages (e.g. EXIT
  messages when linked to other processes via `add_sup_handler/3` and trapping
  exits).
- `code_change/3` must be implemented for hot code upgrade.

## Examples
`terminal_logger` — writes error messages to the terminal:
```erlang
-module(terminal_logger).
-behaviour(gen_event).
-export([init/1, handle_event/2, terminate/2]).

init(_Args) ->
    {ok, []}.

handle_event(ErrorMsg, State) ->
    io:format("***Error*** ~p~n", [ErrorMsg]),
    {ok, State}.

terminate(_Args, _State) ->
    ok.
```

`file_logger` — writes error messages to a file (state holds the file descriptor):
```erlang
-module(file_logger).
-behaviour(gen_event).
-export([init/1, handle_event/2, terminate/2]).

init(File) ->
    {ok, Fd} = file:open(File, read),
    {ok, Fd}.

handle_event(ErrorMsg, Fd) ->
    io:format(Fd, "***Error*** ~p~n", [ErrorMsg]),
    {ok, Fd}.

terminate(_Args, Fd) ->
    file:close(Fd).
```

Shell session:
```text
1> gen_event:start({local, error_man}).
{ok,<0.31.0>}
2> gen_event:add_handler(error_man, terminal_logger, []).
ok
3> gen_event:notify(error_man, no_reply).
***Error*** no_reply
ok
4> gen_event:delete_handler(error_man, terminal_logger, []).
ok
```

## Behaviour / callback details
(event manager vs handler; gen_event:start_link; add_handler/3; add_sup_handler/3
supervised; notify/2 sync_notify/2; call/3; handler callback contract
init/1 handle_event/2 handle_call/2 handle_info/2 terminate/2 code_change/3;
handler termination; handlers share the manager's process / no isolation)

**Manager vs handler:**
- The event manager is a single process. Each event handler is a callback
  module, NOT a separate process. All installed handlers execute within the
  manager's process, so there is NO process isolation between handlers — a
  crash or long-running handler affects the whole manager.
- The manager maintains a list of `{Module, State}` pairs.

**Starting:**
- `gen_event:start_link({local, error_man})` — spawns and links to a new event
  manager process. Use when part of a supervision tree.
- `gen_event:start/1` — standalone (not part of a supervision tree).
- Name arg: `{local, Name}` | `{global, Name}` | omitted (pid used).

**Adding handlers:**
- `gen_event:add_handler(EventMgr, Module, Args)` — sends a message to the
  manager telling it to add handler `Module`. The manager calls
  `Module:init(Args)`. `Args` is the third argument. `init/1` must return
  `{ok, State}`. This is UNSUPERVISED — if the handler's callback fails, the
  handler is simply removed; no supervisor is informed.
- `gen_event:add_sup_handler/3` — supervised variant. The handler is linked to
  the calling process; if the handler is removed due to a callback failure, an
  EXIT message is sent to the process that called `add_sup_handler/3`. This is
  why `handle_info/2` must handle `{'EXIT', Pid, Reason}` when the manager is
  trapping exits and linked to other processes besides the supervisor.

**Notifying:**
- `gen_event:notify(EventMgr, Event)` — asynchronous. The event is made into a
  message and sent to the manager. On receipt, the manager calls
  `handle_event(Event, State)` for each installed handler in add-order.
  `handle_event/2` must return `{ok, State1}`.
- `sync_notify/2` — synchronous variant (referenced by the gen_event module
  page; not detailed in this system doc page).

**Calling a handler:**
- `gen_event:call/3` — synchronous call to a specific handler's `handle_call/2`
  (referenced by the gen_event module page; this system doc page does not
  detail `handle_call/2` return shapes).

**Deleting:**
- `gen_event:delete_handler(EventMgr, Module, Args)` — the manager calls
  `Module:terminate(Args, State)`. `Args` is the third argument. `terminate/2`
  is the opposite of `init/1` (cleanup); its return value is IGNORED.

**Stopping:**
- When the manager is stopped, each installed handler gets `terminate/2`
  called (same as deleting).
- In a supervision tree: no stop function needed; supervisor terminates it per
  its shutdown strategy (`sup_princ.html#shutdown`).
- Standalone: `gen_event:stop(error_man)`.

**Handler callback contract (return shapes):**
- `init(Args) -> {ok, State}` (also `{ok, State, hibernate}` per module ref).
- `handle_event(Event, State) -> {ok, State1}` (also `remove_handler`,
  `{swap_handler, Args1, Module2, Args2}` per module ref).
- `handle_call(Request, State)` — not detailed on this page; see gen_event
  module ref for `{ok, Reply, State1}` / `{remove_handler, Reply}` /
  `{swap_handler, Reply, Args1, Module2, Args2}` shapes.
- `handle_info(Info, State) -> {noreply, State1}` (page shows `{noreply, State1}`;
  note: gen_event module ref uses `{ok, State1}` terminology — this system doc
  page literally shows `{noreply, State1}` in its `handle_info` example).
- `terminate(Args, State)` — return value IGNORED.
- `code_change(OldVsn, State, Extra) -> {ok, NewState}`.

**Handler termination:**
- `terminate/2` is called when a handler is deleted, when the manager stops,
  or when a handler callback returns `remove_handler`. It must undo `init/1`
  (close files, etc.). Return value is ignored.

## Verbatim quotes
(numbered, exact, with section heading)

1. **Event Handling Principles**:
   "In OTP, an *event manager* is a named object to which events can be sent. An
   *event* can be, for example, an error, an alarm, or some information that is
   to be logged."

2. **Event Handling Principles**:
   "In the event manager, zero, one, or many *event handlers* are installed.
   When the event manager is notified about an event, the event is processed by
   all the installed handlers."

3. **Event Handling Principles**:
   "An event manager is implemented as a process and each event handler is
   implemented as a callback module."

4. **Event Handling Principles**:
   "The event manager essentially maintains a list of `{Module, State}` pairs,
   where each `Module` is an event handler, and `State` is the internal state
   of that event handler."

5. **Starting an Event Manager**:
   "`gen_event:start_link/1` spawns and links to a new event manager process."

6. **Starting an Event Manager**:
   "`gen_event:start_link/1` must be used if the event manager is part of a
   supervision tree, meaning that it was started by a supervisor. There is
   another function, `gen_event:start/1`, to start a standalone event manager
   that is not part of a supervision tree."

7. **Adding an Event Handler**:
   "The event manager calls the callback function `terminal_logger:init([])`,
   where the argument `[]` is the third argument to `add_handler`. `init/1` is
   expected to return `{ok, State}`, where `State` is the internal state of the
   event handler."

8. **Notifying about Events**:
   "The event is made into a message and sent to the event manager. When the
   event is received, the event manager calls `handle_event(Event, State)` for
   each installed event handler, in the same order as they were added. The
   function is expected to return a tuple `{ok,State1}`, where `State1` is a new
   value for the state of the event handler."

9. **Deleting an Event Handler**:
   "The event manager calls the callback function
   `terminal_logger:terminate([], State)`, where the argument `[]` is the third
   argument to `delete_handler`. `terminate/2` is to be the opposite of `init/1`
   and do any necessary cleaning up. Its return value is ignored."

10. **Stopping**:
    "When an event manager is stopped, it gives each of the installed event
    handlers the chance to clean up by calling `terminate/2`, the same way as
    when deleting a handler."

11. **In a Supervision Tree**:
    "If the event manager is part of a supervision tree, no stop function is
    needed. The event manager is automatically terminated by its supervisor."

12. **Handling Other Messages**:
    "If the `gen_event` process is to be able to receive other messages than
    events, the callback function `handle_info(Info, State)` must be implemented
    to handle them. Examples of other messages are exit messages if the event
    manager is linked to other processes than the supervisor (for example via
    `gen_event:add_sup_handler/3`) and is trapping exit signals."

## Version notes
- Page generated by ExDoc v0.40.3.
- Project: "Erlang System Documentation v29.0.2"; sidebar shows "OTP 29.0.2";
  meta `major-vsn` = 29.
- Source: github.com/erlang/otp blob OTP-29.0.2 system/doc/design_principles/events.md
- Copyright © 1996-2026 Ericsson AB.
- Note: this system doc page does NOT detail `sync_notify/2`, `call/3`, or the
  full `handle_call/2` / `handle_event/2` return variants (`remove_handler`,
  `{swap_handler,...}`) — those live on the STDLIB `gen_event` module reference
  page (`../apps/stdlib/gen_event.html`).

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/apps/stdlib/gen_event.html  (gen_event module reference — full callback contract, sync_notify/2, call/3, return variants)
- https://www.erlang.org/doc/system/sup_princ.html  (Supervisor Behaviour — shutdown strategy, sup_princ.html#shutdown)
- https://www.erlang.org/doc/system/statem.html  (gen_statem Behaviour — prev page)
- https://www.erlang.org/doc/apps/kernel/global.html  (global:register_name/2 — global naming)

### Skipped
- https://www.erlang.org/doc/system/events.md  (copy-markdown link to self source)
- https://www.erlang.org/doc/system/index.html  (sidebar project home)
- https://www.erlang.org/doc/search.html  (search)
- https://www.erlang.org/doc/system/llms.txt  (llms.txt index)
- https://www.erlang.org/doc/system/Erlang%20System%20Documentation.epub  (epub download)
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/events.md  (view source)
- https://github.com/elixir-lang/ex_doc  (ExDoc)
- https://erlang.org  (Erlang)
- https://www.ericsson.com  (Ericsson)
- https://cdn.jsdelivr.net/npm/mermaid@11.14.0/dist/mermaid.min.js  (CDN asset)
