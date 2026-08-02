# Implementation Workflow

## Purpose

Step-by-step procedure for implementing OTP behaviours and supervision trees in BEAM-common code (Erlang, Elixir, or Gleam). The agent gathers context, follows OTP design principles, implements callbacks with exact contracts, runs checks, and reports evidence.

## What happens first

1. Determine whether a process is actually needed — prefer pure functions when the task has no mutable state, concurrency isolation, or failure containment requirement.
2. If a process is needed, identify which behaviour: `gen_server` (client-server stateful), `gen_statem` (state machine), `supervisor` (tree), or a special process (rare — only when no standard behaviour fits).
3. Read the relevant docs before writing any callback code.

## Context gathering

- Read `overview.md` to confirm the generic/specific split and the behaviour choice.
- Read the behaviour-specific doc (`gen-server.md` or `gen-statem.md`) for the exact callback contract.
- Read `supervision.md` for child spec and restart strategy design.
- Read `applications.md` if the code is part of an application callback module.

## Docs consulted

- `overview.md` — OTP design principles, generic/specific split, behaviours, special processes.
- `gen-server.md` — gen_server callback contract, return shapes, naming, start/start_link.
- `gen-statem.md` — gen_statem callback modes, event types, timeouts, actions.
- `supervision.md` — supervisor flags, child specs, restart strategies, shutdown values.
- `applications.md` — application callback module, .app file, env.
- `ets-data.md` — when implementing ETS-backed state or choosing a data store.
- `ports-io.md` — when implementing external program communication or IO.
- `distribution.md` — when implementing distributed features.
- `gen-event.md` — when implementing event handlers.
- `timers.md` — when implementing timer-based logic.

## Skills loaded

- `beam-gen-server` (if implementing a gen_server) or `beam-gen-statem` (if implementing a gen_statem).
- `beam-supervision` (if designing a supervisor or child specs).
- `beam-applications-releases` (if implementing an application callback module).

## Checks run

```sh
erlc +warn +report -o ebin src/*.erl         # Erlang compile with warnings
# or
rebar3 compile                                 # Erlang (rebar3)
# or
mix compile --warnings-as-errors              # Elixir
# or
gleam build                                    # Gleam

dialyzer --src src/ --plt .plt                 # Dialyzer (BEAM-common; PLT setup varies)
# or
rebar3 dialyzer                                # Erlang (rebar3)
# or
mix dialyzer                                   # Elixir (Dialyxir)
```

## Process

1. **Design**: confirm the behaviour choice and the generic/specific split. Design the client API (runs in caller) separately from server callbacks (run in the process).
2. **Implement callbacks**: write `init/1`, `handle_call/3`, `handle_cast/2`, `handle_info/2` (and `handle_continue/2` if needed). Return tuples MUST match the exact contract — bad returns terminate the process.
3. **Naming**: choose registration (atom, `{global, term()}`, `{via, Module, term()}`). Do NOT use dynamic atoms.
4. **Child spec**: provide `child_spec/1` (or rely on the behaviour default). Set `restart`, `shutdown`, `type` appropriately.
5. **Supervision**: place the worker under a supervisor with the correct strategy (`one_for_one` default; `one_for_all` for coupled; `rest_for_one` for ordered deps).
6. **Error handling**: decide error-tuple vs exception vs let-it-crash per layer. Set `trap_exit` only if cleanup-on-exit is needed.
7. **Compile and fix warnings**.
8. **Test**: write tests that start the process under a supervisor and exercise the client API.

## Evidence reported

- Behaviour chosen and why.
- Callback contract verified (return shapes match).
- Child spec documented (restart, shutdown, type).
- Supervisor strategy and intensity/period documented.
- Compile: clean (no warnings).
- Tests: passing.

## When human judgment is needed

- **Custom special process**: if no standard behaviour fits, a special process (proc_lib + sys) may be needed — confirm with a human that the complexity is justified.
- **Supervision tree restructuring**: if the change affects the tree shape or restart semantics beyond a single worker.
- **Hot code upgrade**: if `code_change/3` and `@vsn` are required (release strategy decision).
- **trap_exit outside supervisors**: if a worker needs to trap exits for cleanup — confirm this is intentional, not defensive programming.

## Related docs

- `../overview.md`, `../gen-server.md`, `../gen-statem.md`, `../supervision.md`, `../applications.md`

## Related workflows

- `code-review.md` — once implementation is complete, review against the corpus checklists.
- `validation.md` — the full gate suite, run before merge.
- `refactoring.md` — if restructuring existing code rather than writing new.
