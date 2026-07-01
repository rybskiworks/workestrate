---
name: constraint-beam-supervision
description: |
  Enforces BEAM/OTP supervisor and child-spec correctness during code execution.
  Load when writing or reviewing supervisor callback modules, child specs,
  restart strategies, shutdown values, or auto-shutdown configuration. Applies to
  Erlang, Elixir, and Gleam-on-BEAM. Does NOT cover gen_server callback contracts
  (see beam-gen-server) or exit-signal propagation (see constraint-beam-failure).
metadata:
  org.kind: constraint
---

# Constraint: BEAM Supervision and Child Specs

This constraint enforces the supervisor callback contract, child-spec map
correctness, restart/shutdown value semantics, and auto-shutdown invariants.
Violations either refuse to start the supervisor or cause restart cascades,
deadlocks, or lost children.

## Triggers

Load this skill when:

- Writing or reviewing a `supervisor` callback module (`init/1`).
- Defining or overriding child specs (`id`, `start`, `restart`, `shutdown`,
  `type`, `modules`, `significant`).
- Choosing a restart strategy (`one_for_one` / `one_for_all` / `rest_for_one` /
  `simple_one_for_one`).
- Configuring `intensity`/`period`, `auto_shutdown`, or `hibernate_after`.
- Reviewing a supervision tree for fault-isolation correctness.

## Rules

1. `init/1` MUST return `{ok, {SupFlags, [ChildSpec]}}` or `ignore`; `SupFlags`
   is a map (preferred; the tuple form is kept for backwards compatibility).
2. Every child spec MUST have `id` and `start => {M,F,A}` (mandatory). Defaults:
   `restart => permanent`, `shutdown => 5000` (worker) / `infinity` (supervisor),
   `type => worker`, `modules => [M]` (`dynamic` for `gen_event`).
3. A child supervisor MUST use `shutdown => infinity`; a finite shutdown on
   `type => supervisor` risks a race where the child unlinks its own children but
   fails to terminate them before being killed.
4. `restart` values: `permanent` (always restarted, even on `normal`),
   `transient` (restarted only on abnormal exit — not `normal`/`shutdown`/
   `{shutdown,Term}`), `temporary` (never restarted; spec auto-deleted on
   termination; `restart_child/2` cannot be used).
5. `temporary` children never restart, even under `one_for_all`/`rest_for_one`
   when a sibling dies.
6. `significant => true` is invalid when `restart => permanent` OR when
   `auto_shutdown => never`; only `transient`/`temporary` children may be
   significant.
7. Do NOT configure `auto_shutdown` (other than `never`) on top application
   supervisors; do NOT make an auto-shutdown supervisor a `permanent` child of
   its parent.
8. More than `intensity` restarts in `period` seconds terminates all children and
   the supervisor itself with reason `shutdown`. Do not reuse the same
   `intensity`/`period` at every level (the effective budget compounds across the
   tree).
9. Children start left-to-right and terminate right-to-left (reverse start
   order); order `rest_for_one` children least-dependent first.
10. A child MUST NOT call `supervisor:terminate_child/2` or `supervisor:stop/3`
    on its own supervisor (deadlock).
11. For `simple_one_for_one`: `init/1` returns exactly one child spec;
    `delete_child/2`/`restart_child/2` return `{error,simple_one_for_one}`;
    `terminate_child/2` takes a pid; dynamic children are lost when the
    supervisor is recreated.
12. `modules` must be `[Module]` for single-module behaviours or `dynamic` for
    `gen_event` (used by the release handler).

## References

- Operational skill: `beam-supervision`.
- Docs: `docs/beam/supervision.md`.

## Out of scope

- `gen_server`/`gen_statem` callback contracts — see `beam-gen-server` /
  `beam-gen-statem`.
- Exit-signal propagation and `trap_exit` — see `constraint-beam-failure`.
- Application start/stop lifecycle — see `beam-applications-releases`.

## Violation examples

### Finite shutdown on a child supervisor

```erlang
%% FORBIDDEN: finite shutdown on type => supervisor
#{id => sub, start => {sub, start_link, []},
  type => supervisor, shutdown => 5000}
```

Correct: set `shutdown => infinity` (the default for `type => supervisor`) so the
subtree shuts down cleanly.

### `significant => true` on a `permanent` child

```erlang
%% FORBIDDEN: permanent children cannot be significant
#{id => worker, start => {worker, start_link, []},
  restart => permanent, significant => true}
```

Correct: only `transient` or `temporary` children may be `significant`, and only
in an `any_significant`/`all_significant` supervisor.

### `temporary` child expected to restart on cascade

```erlang
%% FORBIDDEN assumption: a temporary child never restarts, even under one_for_all
strategy => one_for_all,  %% sibling death terminates the temporary child; it is NOT restarted
```

Correct: use `transient` if the child should restart on abnormal exit, or accept
that `temporary` children do not survive sibling failures.

### Child terminating its own supervisor

```erlang
%% FORBIDDEN: deadlock — child calls terminate_child on its own supervisor
supervisor:terminate_child(MySup, sibling_id)   %% called from a child of MySup
```

Correct: let the child exit with `normal`/`shutdown`/`{shutdown,Term}` and let
the supervisor handle restart; use `auto_shutdown` for bounded lifecycles.

## How to check

```bash
# Validate child specs against the intended auto_shutdown mode (Erlang shell / test).
#   supervisor:check_childspecs(ChildSpecs).
#   supervisor:check_childspecs(ChildSpecs, AutoShutdown).
```

Manual review:

- Every child supervisor has `shutdown => infinity`.
- `significant => true` only on `transient`/`temporary` children, never in a
  `never` supervisor.
- No `auto_shutdown` on top application supervisors; auto-shutdown supervisors
  are not `permanent` children.
- `intensity`/`period` differ across supervision levels.
- No child calls `terminate_child/2`/`stop/3` on its own supervisor.
- `modules` is `[Module]` or `dynamic`.
