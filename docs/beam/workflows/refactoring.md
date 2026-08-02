# Refactoring Workflow

## Purpose

Step-by-step procedure for refactoring BEAM/OTP code while preserving observable behavior. Covers supervision tree restructuring, gen_fsm→gen_statem migration, special-process→behaviour conversion, and child spec cleanup. All refactors are verified by tests before and after.

## What happens first

1. Establish a green test baseline — the existing tests MUST pass before refactoring. If tests are missing, write characterization tests first.
2. Identify the refactor type: supervision tree restructuring, behaviour migration, special-process conversion, or child spec cleanup.
3. Read the relevant docs for the target state (not just the current state).

## Context gathering

- Read the current code and its supervision tree structure.
- Read the target behaviour doc (e.g. gen-statem.md for a gen_fsm migration).
- Identify all callers of the code being refactored (client API must remain stable).

## Docs consulted

- `supervision.md` — for child spec and supervisor flag changes.
- `gen-server.md` — for gen_server callback refactoring.
- `gen-statem.md` — for gen_fsm→gen_statem migration target.
- `proc-lib-and-sys.md` — for special-process→behaviour conversion (proc_lib/sys contract).
- `overview.md` — for the generic/specific split and behaviour choice.

## Skills loaded

- `beam-supervision` — for supervisor/child spec refactoring.
- `beam-gen-server` — for gen_server callback refactoring.
- `beam-gen-statem` — for gen_statem migration.
- `beam-processes` — for process/link/monitor refactoring.

## Checks run

```sh
# Before refactor (baseline):
erlc +warn -o ebin src/*.erl && dialyzer --src src/ --plt .plt
# Run tests (Common Test / ExUnit / gleam test):
rebar3 ct    # or: mix test    # or: gleam test

# After refactor (same checks):
erlc +warn -o ebin src/*.erl && dialyzer --src src/ --plt .plt
rebar3 ct    # or: mix test    # or: gleam test
```

## Process

1. **Baseline**: confirm tests pass before any change.
2. **Plan**: document the refactor scope — what changes, what stays the same (especially the client API).
3. **Refactor in small steps**: each step should compile and pass tests.
4. **Supervision tree restructuring**: when moving children between supervisors or changing strategy, verify start order (left-to-right) and shutdown order (right-to-left) still respect dependencies. Verify intensity/period at each level (not same at every level).
5. **gen_fsm→gen_statem migration**: map states to gen_statem callback_mode (`state_functions` or `handle_event_function`); map events to gen_statem event types; map timeouts to the three gen_statem timeout kinds; implement `callback_mode/0`.
6. **Special-process→behaviour conversion**: if a hand-written special process (`proc_lib` + `sys` + `system_*` callbacks) can be replaced by `gen_server` or `gen_statem`, convert it — the behaviour provides the sys/debug/code_change contract for free.
7. **Child spec cleanup**: verify `child_spec/1` returns complete specs (`id`, `start`, `restart`, `shutdown`, `type`); remove hand-written maps in favor of module-provided `child_spec/1` where possible.
8. **Verify**: tests pass after refactor; Dialyzer clean; no behavior change in client API.

## Evidence reported

- Refactor type and scope.
- Test baseline: green before.
- Test result: green after.
- Dialyzer: clean before and after.
- Client API: unchanged (or documented delta if intentionally changed).
- Supervision tree: before/after shape if restructured.
- Callback contract: verified for any migrated behaviour.

## When human judgment is needed

- **Behavior change**: if the refactor changes observable behavior, switch to the implementation workflow and document the delta.
- **Supervision tree redesign**: if the refactor changes restart semantics or blast radius.
- **Hot code upgrade compatibility**: if the code is in a release that uses appup/relup, refactoring may break upgrade paths.
- **Cross-process API changes**: if the refactor changes the message protocol between processes.

## Related docs

- `../supervision.md`, `../gen-server.md`, `../gen-statem.md`, `../proc-lib-and-sys.md`, `../overview.md`

## Related workflows

- `implementation.md` — if the refactor requires new code.
- `code-review.md` — review the refactored code.
- `validation.md` — the full gate suite.
