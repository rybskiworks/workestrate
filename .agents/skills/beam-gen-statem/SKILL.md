---
name: beam-gen-statem
description: |
  Operational guide for the BEAM `gen_statem` behaviour — callback modes, state
  enter calls, event types, the three timeout kinds, action lists, and code
  change. Load when writing, reviewing, or debugging state-machine processes.
  Does NOT cover `gen_server` (see `beam-gen-server`), `gen_event` handlers in
  depth (see `beam-gen-event`), supervision (see `beam-supervision`), or process
  primitives (see `beam-processes`).
---

## Triggers

- Writing, reviewing, or debugging a `gen_statem` callback module.
- Migrating from the deprecated `gen_fsm` behaviour.
- Choosing between `state_functions`, `handle_event_function`, and `state_enter`.

## References

- `docs/beam/gen-statem.md`
  - https://www.erlang.org/doc/system/statem.html
  - https://www.erlang.org/doc/apps/stdlib/gen_statem.html
- `docs/beam/gen-event.md`
  - https://www.erlang.org/doc/system/events.html

## Key Rules

- `gen_statem` replaces the deprecated `gen_fsm` since OTP 20.0; use for new code.
- Event-Driven Mealy machine that keeps a server `Data` item besides the state.
- `Module:callback_mode/0` is MANDATORY and called on start, after code change,
  and after changing the callback module; the result is cached. It must return
  `state_functions`, `handle_event_function`, or a list containing one of those
  plus `state_enter`.
- `state_functions`: state must be an atom; use one `Module:StateName/3` per state.
  `StateName` cannot be `terminate` (collides with `Module:terminate/3`).
- `handle_event_function`: state can be any term; single `Module:handle_event/4`
  handles all states.
- `state_enter`: enabled by including `state_enter` in the `callback_mode/0` list.
  The engine calls the state callback with `(enter, OldState, Data)` or
  `(enter, OldState, State, Data)` on every state change (`NextState =/= State`)
  and before entering the initial state (`OldState =:= State`). Re-enter without
  changing state via `repeat_state` / `repeat_state_and_data`. `code_change/4`
  state transform is a rename, not a state change, so no enter call is made.
- Complete event types: `cast`, `{call, From}`, `info`, `state_timeout`,
  `{timeout, Name}`, `timeout`, `internal`. All event types can be generated via
  `{next_event, EventType, EventContent}`; `internal` cannot come from an
  external source.
- Three timeout kinds:
  - Event timeout (`timeout`): set by `{timeout, Time, EventContent}` or bare
    `Time`; cancelled by ANY event, including postponed and inserted events.
  - State timeout (`state_timeout`): set by `{state_timeout, Time, EventContent}`;
    cancelled by a state change.
  - Generic timeout (`{timeout, Name}`): set by `{{timeout, Name}, Time,
    EventContent}`; any number of named timers, NO automatic cancel.
- State callback return shapes include: `{next_state, State, NewData}`,
  `{next_state, State, NewData, Actions}`, `{keep_state, NewData}`,
  `{keep_state, NewData, Actions}`, `keep_state_and_data`,
  `{keep_state_and_data, Actions}`, `{repeat_state, NewData}`,
  `{repeat_state, NewData, Actions}`, `repeat_state_and_data`,
  `{repeat_state_and_data, Actions}`, `{stop, Reason}`.
- Reply via the ACTION `{reply, From, Reply}` in an actions list, or by calling
  `gen_statem:reply(From, Reply)`.
- `init/1` returns `{ok, State, Data}` | `{ok, State, Data, Actions}` |
  `{stop, Reason}` | `ignore`.
- `code_change/4` (`OldVsn`, `OldState`, `OldData`, `Extra`) returns
  `{ok, NewState, NewData} | Reason`. If callback mode must change, do it here.
- `terminate/3` (`Reason`, `State`, `Data`) returns any term.
- `format_status/1` is the OTP 25+ status callback; `format_status/2` is
  deprecated.
- Hibernate via the `{hibernate, true}` action or `hibernate` atom.
- `gen_statem:call/2,3` is synchronous; `gen_statem:cast/2` is asynchronous.
- `gen_event` is an event-manager behaviour: one process hosts many handler
  callbacks. Use `gen_event:add_handler/3`, `add_sup_handler/3`, `notify/2`,
  `call/3`, and `delete_handler/3`. For `supervisor` child specs, use
  `modules: dynamic` for `gen_event` managers.

## Quick Commands

```erl
gen_statem:start_link({local, my_fsm}, my_fsm_mod, [], []).
gen_statem:call(my_fsm, {event, Data}).
gen_statem:cast(my_fsm, {async_event, Data}).
sys:get_state(my_fsm).          %% {State, Data}
sys:get_status(my_fsm).
sys:trace(my_fsm, true).
```

## Anti-patterns

- Using the deprecated `gen_fsm` for new code.
- Forgetting to export `callback_mode/0` or returning a non-constant body.
- Mixing `state_functions` and `handle_event_function` conventions.
- Expecting an event timeout to survive any incoming message.
- Returning a reply tuple instead of using the `{reply, From, Reply}` action.
- Enabling `state_enter` callbacks without adding `state_enter` to
  `callback_mode/0`.
- Changing state, postponing, inserting events, or changing the callback module
  from a state enter call.

## Related Skills

- `beam-gen-server`
- `beam-supervision`
- `beam-errors-failures`
- `beam-observability-debugging`
