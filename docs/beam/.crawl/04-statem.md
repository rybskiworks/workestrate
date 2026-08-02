# Crawl: statem.html

- seed_url: https://www.erlang.org/doc/system/statem.html
- canonical_url: https://www.erlang.org/doc/system/statem.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: Erlang System Documentation v29.0.2 (page title: "gen_statem Behaviour — Erlang System Documentation v29.0.2"; source markdown footer references OTP-29.0.2)
- feeds_docs: gen-statem.md

## Purpose

`gen_statem` is an Event-Driven Mealy state machine behaviour for Erlang/OTP. It is a generalization of `gen_fsm` (the predecessor) and is intended for processes whose logic is conveniently described as a state machine. It keeps a server `Data` item besides the state; because of this data item and the absence of restrictions on the number of states or distinct input events, a `gen_statem` machine is Turing complete, but "it feels mostly like an Event-Driven Mealy machine."

You should consider `gen_statem` over `gen_server` when you need any of its key features:
- Co-located callback code for each state, for all event types (`call`, `cast`, `info`).
- Postponing events (a substitute for selective receive).
- Inserted events (events from the state machine to itself; purely internal events in particular).
- State enter calls (callback on state entry co-located with the rest of each state's callback code).
- Easy-to-use time-outs: state time-outs, event time-outs, and generic (named) time-outs.

For simple state machines not needing these features, `gen_server` is perfectly suitable (smaller call overhead, ~2 vs 3.3 microseconds roundtrip).

## Key concepts

- **Event-Driven Mealy machine**: events drive state transitions; actions are part of the model.
- **Callback module**: contains functions implementing the state machine. The behaviour engine holds the state machine state, server data, timer references, a queue of postponed messages, and other metadata. It receives all process messages, handles system messages, and calls the callback module with state-machine-specific events.
- **Callback modes** (property of the callback module, set at server start, selected by `Module:callback_mode/0`):
  - `state_functions` — one callback function per state: `Module:StateName(EventType, EventContent, Data)`. Restricts states to atoms; engine branches on state name. Encourages co-locating per-state event handling.
  - `handle_event_function` — one single callback function: `Module:handle_event(EventType, EventContent, State, Data)`. Allows non-atom/complex/hierarchical states; enables event-centered or state-centered strategies.
- **State enter calls**: optionally enabled by returning a list `[CallbackMode, state_enter]` from `callback_mode/0`. The engine calls the state callback with `(enter, OldState, ...)` whenever the state changes.
- **State callback return values** (the transition result):
  - `{next_state, NextState, NewData [, Actions]}` — set next state and update server data; if `Actions` present, execute transition actions. If `NextState =/= State` it is a state change: the event queue is restarted from the oldest postponed event, any current state time-out is canceled, and a state enter call is performed (if enabled). The current `State` becomes `OldState` in a state enter call.
  - `{keep_state, NewData [, Actions]}` — same as `next_state` with `NextState =:= State` (no state change).
  - `keep_state_and_data` | `{keep_state_and_data, Actions}` — same as `keep_state` with `NewData =:= Data` (no change in server data).
  - `{repeat_state, NewData [, Actions]}` | `repeat_state_and_data` | `{repeat_state_and_data, Actions}` — same as `keep_state`/`keep_state_and_data`, but if state enter calls are enabled, repeat the state enter call as if the state was entered again. In this case `State` and `OldState` become equal in the repeated state enter call (state re-entered from itself).
  - `{stop, Reason [, NewData]}` — stop the server with reason `Reason`; if `NewData` present, first update server data.
  - `{stop_and_reply, Reason, [NewData,] ReplyActions}` — same as stop values, but first execute the given transition actions that may only be reply actions.
- **First state**: `Module:init(Args)` is called before any state callback. It gets `Args` from `gen_statem:start/3,4` or `start_link/3,4`, and returns `{ok, State, Data}` or `{ok, State, Data, Actions}`. A `postpone` action from `init/1` is ignored (no event to postpone).
- **Transition actions**: commanded by returning a list of actions in the state callback return value. The only immediate action is `reply`; the others are collected and handled later during the state transition. Inserted events are stored and inserted all together; the rest set transition options where the last of a specific type overrides the previous.
- **Event types and event content**: events are categorized into types; all types for a given state are handled in the same callback function, receiving `EventType` and `EventContent`. The meaning of `EventContent` depends on `EventType`.
- **Time-outs**: 3 types — `state_timeout` (one, auto-canceled by state change), `{timeout, Name}` generic (any number, no auto-cancel), `timeout` event time-out (one, auto-canceled by any event).
- **Postponing events**: a postponed event is retried after a state change (`OldState =/= NewState`). Models selective receive.
- **Inserted events**: generated via `{next_event, EventType, EventContent}`; the `internal` type can only be generated this way (cannot come from an external source).
- **Callback module switching**: `{change_callback_module, NewModule}`, `{push_callback_module, NewModule}`, `pop_callback_module` — change the callback module for a running server during any state transition (not from a state enter call).
- **Hibernation**: via `hibernate` action or `{hibernate_after, Timeout}` start option.
- **State filtering**: `Module:format_status/2` to format internal state in error log / `sys:get_status/1,2`.

## Strict rules / invariants

- A catch-all `receive` must never be used from a `gen_statem` (or any `gen_*`) behaviour, because the receive statement is within the `gen_*` engine itself; `sys`-compatible behaviours must respond to system messages in their engine receive loop. A catch-all receive can discard system messages leading to unexpected behaviour. If a selective receive must be used, ensure only pertinent messages are received, and the callback must return in due time so the engine can handle system messages.
- In a **state enter call** (event `(enter, OldState, ...)`), there are restrictions on the allowed return value and state transition actions: you must **not** change the state, must **not** postpone this non-event, must **not** insert any events, and must **not** change the callback module.
- `{change_callback_module, NewModule}` / push / pop cannot be done from a state enter call.
- A postponed event is retried **only after a state change** (`OldState =/= NewState`). If a change in a value changes the set of events that is handled, that value should be in the `State` (not just server `Data`), otherwise postponed events will not be retried. An incorrect decision of what belongs in the state may become a hard-to-find bug when postponing is introduced.
- When a time-out is started, any running time-out of the same type (`state_timeout`, `{timeout, Name}`, or `timeout`) is canceled (restarted with new time and event content). Different `EventContent`s do not create different time-outs.
- The `internal` event type can **only** be generated through the `next_event` action; it cannot come from an external source.
- `Module:callback_mode/0` is mandatory. The callback mode is a property of the callback module and is set at server start; it may be changed due to a code upgrade/downgrade or when changing the callback module.
- When switching callback modules, the new callback module completely replaces the previous one, so all relevant callback functions have to handle the state and data from the previous callback module.
- `pop_callback_module` fails the server if the internal stack is empty.
- `gen_statem:start_link/3,4` must be used if the gen_statem is part of a supervision tree; `gen_statem:start/3,4` for a standalone (non-supervised) server.
- The first state entered after `gen_statem:init/1` gets a state enter call with `OldState` equal to the current state.

## Examples

The page builds a running **code lock** example (a door with a button code), evolved across several variants:

1. **Basic `state_functions` code lock** (`code_lock`): states `locked`/`open`; `cast` button events; `state_timeout` of 10 s to re-lock. `init(Code)` returns `{ok, locked, Data}`; `callback_mode() -> state_functions`.
2. **All State Events**: a `code_length/0` call handled in every state via a common `handle_common/3` function (or `?HANDLE_COMMON` macro). Reply via `{reply, From, Reply}` action in `{keep_state, ...}`.
3. **One State Callback** (`handle_event_function`): all events handled in `Module:handle_event/4`, branching first on event then on state.
4. **Event Time-Outs**: restart code sequence if no button for 30 s, via `{timeout, Time, EventContent}` or bare integer `Time`.
5. **Generic Time-Outs**: named time-out `{{timeout, open}, 10_000, lock}` to start a timer in one state and respond in another, cancel without state change, or run multiple in parallel.
6. **Erlang Timers**: `erlang:start_timer/4` + `info` event `{timeout, Tref, Msg}` for cases needing `erlang:cancel_timer(Tref)` return value.
7. **Postponing Events**: postpone button events in `open` to handle them later in `locked` via `{keep_state, Data, [postpone]}`.
8. **State Enter Actions**: `callback_mode() -> [state_functions, state_enter]`; handle `(enter, OldState, Data)` clauses to perform do_lock/do_unlock on entry.
9. **Inserted Events / `internal` type**: down/up button press/release pre-processing; generate `{next_event, internal, {button, Button}}`.
10. **Example Revisited**: full combined example with state enter calls and a new state diagram.
11. **`handle_event_function` variant** of the revisited example (branches on state first because of state enter calls).
12. **Filter the State**: `format_status/2` to strip sensitive `code` from error logs.
13. **Complex State**: state `{StateName, LockButton}` (non-atom tuple) with `handle_event_function` so changing the lock button is a state change that retries postponed events.
14. **Hibernation**: `hibernate` action in the action list when entering `{open, _}`; or `{hibernate_after, Timeout}` start option.

A **ballpoint pen** (push-end / push-side) is used as an introductory everyday state-machine example, and a **selective receive** plain-Erlang `code_lock_1` variant is shown for contrast.

## Behaviour / callback details

### Callback modes
- `state_functions` — `Module:StateName(EventType, EventContent, Data)`. Atom-only states; engine branches on state name.
- `handle_event_function` — `Module:handle_event(EventType, EventContent, State, Data)`. Single function; non-atom states allowed.
- Selected by mandatory `Module:callback_mode() -> CallbackMode | [CallbackMode, state_enter]`.

### State Enter mode
- Enabled by returning `[CallbackMode, state_enter]` from `callback_mode/0`.
- Engine calls state callback with `(enter, OldState, ...)` on every state change.
- Restrictions: must not change state, postpone, insert events, or change callback module.
- First state after `init/1` gets a state enter call with `OldState =:= current state`.
- `{repeat_state, ...}` repeats the state enter call with `State =:= OldState`.

### Event types (complete list and origin)
- `cast` — from `gen_statem:cast(ServerRef, Msg)`; `Msg` is `EventContent`.
- `{call, From}` — from `gen_statem:call/2`, `send_request/2`, or `send_request/4`; `Request` is `EventContent`. `From` is the reply address used via `{reply, From, Reply}` action or `gen_statem:reply(From, Reply)`.
- `info` — any regular process message; the message is `EventContent`.
- `state_timeout` — from transition action `{state_timeout, Time, EventContent}` when the time-out expires.
- `{timeout, Name}` — from transition action `{{timeout, Name}, Time, EventContent}` when the generic time-out expires.
- `timeout` — from transition action `{timeout, Time, EventContent}` (or short form `Time`) when the event time-out expires.
- `internal` — only from transition action `{next_event, internal, EventContent}`; cannot come from an external source. All event types above can also be generated via `{next_event, EventType, EventContent}`.

### Three timeout kinds
1. **state_timeout** — one per state; automatically canceled by a state change. Action forms: `{state_timeout, Time, EventContent [, Opts]}` | `{state_timeout, update, EventContent}` | `{state_timeout, cancel}`.
2. **generic time-out `{timeout, Name}`** — any number, differing by `Name`; no automatic canceling. Action forms: `{{timeout, Name}, Time, EventContent [, Opts]}` | `{{timeout, Name}, update, EventContent}` | `{{timeout, Name}, cancel}`.
3. **event time-out `timeout`** — one; automatically canceled by any event (postponed and inserted events cancel it just as external events do). Action form: `{timeout, Time, EventContent [, Opts]}` (or bare integer `Time`). Canceling/restarting/updating is neither possible nor necessary — whatever event you act on has already canceled it.

Common time-out semantics:
- Starting a time-out cancels any running time-out of the same type (restarted with new time/content). Different `EventContent` does not create different time-outs.
- `infinity` time: optimized by not starting; any running same-tag time-out is canceled; `EventContent` ignored (set to `undefined`).
- Explicit cancel: `{TimeoutType, cancel}`.
- Update while running: `{TimeoutType, update, NewEventContent}`; if no such time-out running, a time-out event is immediately delivered as a zero time-out.
- Zero time (`0`): not started; the time-out event is inserted to be processed after any events already enqueued and before any not-yet-received external events. Note: some time-outs are auto-canceled, so e.g. combining postponing an event in a state change with starting an event time-out of 0 yields no time-out event (canceled by the postponed event delivered due to the state change).

### Actions list (transition actions)
- `{postpone, Boolean}` — if `true`, postpone the current event.
- `{hibernate, Boolean}` — if `true`, hibernate the gen_statem.
- `{state_timeout, Time, EventContent [, Opts]}` | `{state_timeout, update, EventContent}` | `{state_timeout, cancel}` — start/update/cancel a state time-out.
- `{{timeout, Name}, Time, EventContent [, Opts]}` | `{{timeout, Name}, update, EventContent}` | `{{timeout, Name}, cancel}` — start/update/cancel a generic time-out.
- `{timeout, Time, EventContent [, Opts]}` — start an event time-out.
- `{reply, From, Reply}` — reply to a caller (the only immediate action).
- `{next_event, EventType, EventContent}` — generate the next event to handle (inserted events).
- `{change_callback_module, NewModule}` — change callback module for the running server (during any state transition, not from a state enter call).
- `{push_callback_module, NewModule}` — push current module to an internal stack and set new module.
- `pop_callback_module` — pop top module from the stack and set it as new module (fails if stack empty).

Notes: Out of these, only `reply` is immediate. Inserted events are stored and inserted all together; the rest set transition options where the last of a specific type overrides the previous. The `Opts` field can set absolute (instead of relative) time, etc. See `action()` and `transition_option()` types in module `gen_statem`.

### Reply via `{reply, From, Reply}`
- Used to reply to a `{call, From}` event. Either via the transition action `{reply, From, Reply}` in an action list, or by calling `gen_statem:reply(From, Reply)` from the callback module. Convenient in `{keep_state, ...}` returns when you want to stay in the current state regardless of which it is.

### Postpone
- `{postpone, true}` (or atom `postpone`) postpones the current event; retried after a state change (`OldState =/= NewState`). Models selective receive explicitly for a single received event.

### Hibernate
- `hibernate` atom (or `{hibernate, true}`) in the action list; costly, not after every event. Also `{hibernate_after, Timeout}` start option for `start/3,4`, `start_link/3,4`, `enter_loop/4,5,6`.

### `code_change/4`
- The callback mode may be changed due to a code upgrade/downgrade. (The page notes callback mode "may be changed due to a code upgrade/downgrade, or when changing the callback module." Detailed `code_change/4` signature is in the Reference Manual `gen_statem` module, not expanded on this system doc page.)

### `init` return shapes
- `Module:init(Args)` returns `{ok, State, Data}` or `{ok, State, Data, Actions}`. Behaves like a state callback function but gets `Args` from the start function. A `postpone` action from `init/1` is ignored (no event to postpone).

## Verbatim quotes

1. (Section: Event-Driven State Machines) — "Similar to most gen_ behaviours, gen_statem keeps a server Data item besides the state. Because of this data item, and since there is no restriction on the number of states (assuming sufficient virtual machine memory), or on the number of distinct input events, a state machine implemented with this behaviour is Turing complete. But it feels mostly like an Event-Driven Mealy machine."

2. (Section: When to use gen_statem) — "You should consider using gen_statem over gen_server if your process logic is convenient to describe as a state machine and you need any of these gen_statem key features: Co-located callback code for each state, for all event types, such as call, cast, and info; Postponing events - a substitute for selective receive; Inserted events - events from the state machine to itself; for purely internal events in particular; State enter calls - callback on state entry co-located with the rest of each state's callback code; Easy-to-use time-outs - state time-outs, event time-outs, and generic time-outs (named time-outs)."

3. (Section: Callback Modes) — "The gen_statem behaviour supports two callback modes: state_functions - Events are handled by one callback function per state. handle_event_function - Events are handled by one single callback function."

4. (Section: Callback Modes) — "The Module:callback_mode() function may also return a list containing the callback mode and the atom state_enter in which case state enter calls are activated for the callback mode."

5. (Section: State Callback) — "state_functions - The event is handled by: Module:StateName(EventType, EventContent, Data) ... handle_event_function - The event is handled by: Module:handle_event(EventType, EventContent, State, Data)."

6. (Section: State Callback return values) — "If NextState =/= State it's a state change and gen_statem does some extra things: the event queue is restarted from the oldest postponed event, any current state time-out is canceled, and a state enter call is performed, if enabled. The current State becomes OldState in a state enter call."

7. (Section: The First State) — "If you use the postpone action from this function, that action is ignored, since there is no event to postpone."

8. (Section: Transition Actions) — "Out of these transition actions, the only immediate action is reply for replying to a caller. The other actions are collected and handled later during the state transition. Inserted events are stored and inserted all together, and the rest set transition options where the last of a specific type override the previous."

9. (Section: Event Types and Event Content) — "The following is a complete list of event types and from where they come: cast ... {call, From} ... info ... state_timeout ... {timeout, Name} ... timeout ... internal - Generated by transition action {next_event, internal, EventContent}. All event types above can also be generated using the next_event action: {next_event, EventType, EventContent}."

10. (Section: State Enter Calls) — "Since the state enter call is not an event there are restrictions on the allowed return value and state transition actions. You must not change the state, postpone this non-event, insert any events, or change the callback module."

11. (Section: Time-Outs) — "There are 3 types of time-outs in gen_statem: state_timeout - There is one state time-out that is automatically canceled by a state change. {timeout, Name} - There are any number of generic time-outs differing by their Name. They have no automatic canceling. timeout - There is one event time-out that is automatically canceled by any event. Note that postponed and inserted events cancel this time-out just as external events do."

12. (Section: Time-Outs) — "When a time-out is started, any running time-out of the same type (state_timeout, {timeout, Name}, or timeout) is canceled, that is, the time-out is restarted with the new time and event content. All time-outs have an EventContent that is part of the transition action that starts the time-out. Different EventContent s do not create different time-outs."

13. (Section: Zero Time-Out) — "If a time-out is started with the time 0 it will actually not be started. Instead the time-out event will immediately be inserted to be processed after any events already enqueued, and before any not yet received external events."

14. (Section: Postponing Events) — "A postponed event is retried after a state change, that is, OldState =/= NewState."

15. (Section: Selective Receive) — "A catch-all receive should never be used from a gen_statem behaviour (or from any gen_* behaviour), as the receive statement is within the gen_* engine itself."

16. (Section: Inserted Events) — "the internal type can only be generated through action next_event. Hence, it cannot come from an external source, so you can be certain that an internal event is an event from your state machine to itself."

17. (Section: Callback Modes / handle_event_function) — "With handle_event_function, you are free to mix strategies, as all events and states are handled in the same callback function. ... The mode enables the use of non-atom states, for example, complex states, or even hierarchical states."

## Version notes

- Page title: "gen_statem Behaviour — Erlang System Documentation v29.0.2".
- Source markdown footer links to `https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/statem.md#L1`, confirming OTP-29.0.2.
- Built using ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- `gen_statem` is the successor to `gen_fsm` (event time-out feature "inherited from gen_statem's predecessor gen_fsm").
- The page is part of the "Erlang System Documentation" (design principles), positioned between `gen_server Behaviour` (previous) and `gen_event Behaviour` (next).

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/system/gen_server.html (gen_server Behaviour — previous page)
- https://www.erlang.org/doc/system/gen_event.html (gen_event Behaviour — next page)
- https://www.erlang.org/doc/system/gen_server_concepts.html (gen_server concepts)
- https://www.erlang.org/doc/system/events.html (events)
- https://www.erlang.org/doc/system/sup_princ.html (Supervision principles; #shutdown referenced)
- https://www.erlang.org/doc/apps/stdlib/gen_statem.html (Reference Manual: gen_statem module — types action/0, transition_option/0, event_type/0, state_callback_result/2, callback_mode/0, etc.)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html (gen_server module reference)
- https://www.erlang.org/doc/apps/stdlib/gen_fsm.html (gen_fsm — predecessor)
- https://www.erlang.org/doc/apps/stdlib/sys.html (sys; get_status/1,2)
- https://www.erlang.org/doc/apps/stdlib/proc_lib.html (proc_lib; hibernate/3)
- https://www.erlang.org/doc/apps/stdlib/maps.html (maps)
- https://www.erlang.org/doc/apps/kernel/global.html (global; register_name/2)
- https://www.erlang.org/doc/apps/erts/erlang.html (erts: erlang module; start_timer/4, cancel_timer/2, hibernate/3, process_flag/2)

### Skipped
- (blank href)
- Erlang System Documentation.epub (download asset)
- llms.txt
- statem.md (view markdown source of this same page)
- https://github.com/elixir-lang/ex_doc (ExDoc source)
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/statem.md#L1 (source markdown of this page)
- https://erlang.org (site root)
- https://www.ericsson.com (Ericsson)
- https://en.wikipedia.org/wiki/Mealy_machine (external reference)
- In-page anchor fragments of the current page (e.g. ../apps/stdlib/gen_statem.html#t:action/0, #c:callback_mode/0, etc.) — covered by the gen_statem reference manual entry above.
