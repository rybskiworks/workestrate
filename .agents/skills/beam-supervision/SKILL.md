---
name: beam-supervision
description: |
  Operational guide for BEAM/OTP supervisors, child specs, restart strategies,
  shutdown values, intensity/period, and automatic shutdown. Load when writing,
  reviewing, or debugging supervision trees. Does NOT cover `gen_server` callback
  contracts (see `beam-gen-server`), `gen_statem` (see `beam-gen-statem`),
  exit-signal propagation (see `beam-errors-failures`), or `sys`/`proc_lib`
  special-process internals (see `beam-observability-debugging`).
---

## Triggers

- Writing, reviewing, or debugging a `supervisor` callback module.
- Designing or tuning a supervision tree.
- Configuring child specs, restart strategies, shutdown, or intensity/period.

## References

- `docs/beam/supervision.md`
  - https://www.erlang.org/doc/system/sup_princ.html
  - https://www.erlang.org/doc/apps/stdlib/supervisor.html
- `docs/beam/overview.md`
  - https://www.erlang.org/doc/system/design_principles.html
- `docs/beam/otp-behaviours.md`
  - https://www.erlang.org/doc/system/design_principles.html
  - https://www.erlang.org/doc/system/spec_proc.html
  - https://www.erlang.org/doc/apps/stdlib/proc_lib.html
  - https://www.erlang.org/doc/apps/stdlib/sys.html

## Key Rules

- `init/1` returns `{ok, {SupFlags, [ChildSpec]}}` or `ignore`.
- `SupFlags` is a map (preferred); tuple `{Strategy, Intensity, Period}` is kept for backwards compatibility.
- SupFlags keys and defaults: `strategy => one_for_one`, `intensity => 1`, `period => 5`, `hibernate_after => timeout()` (OTP 28.0+), `auto_shutdown => never` (OTP 24.0+).
- Child spec mandatory keys: `id` and `start => {M,F,A}`. Defaults: `restart => permanent`, `significant => false`, `shutdown => 5000` for `worker` / `infinity` for `supervisor`, `type => worker`, `modules => [M]` (`dynamic` for `gen_event`).
- `significant => true` is invalid when `restart => permanent` or `auto_shutdown => never`.
- Strategies: `one_for_one` (default), `one_for_all`, `rest_for_one`, `simple_one_for_one`.
- For `simple_one_for_one`: `init/1` must list exactly one spec; children are added dynamically with `start_child/2` ExtraArgs; `delete_child/2` and `restart_child/2` return `{error, simple_one_for_one}`; `terminate_child/2` takes a pid; `which_children/1` returns `Id = undefined`.
- In `simple_one_for_one`, the `active` count from `count_children/1` may not be fully verified for liveness under heavy load.
- `permanent`: always restarted, even on `normal` exit. `transient`: restarted only on abnormal exit (not `normal`, `shutdown`, or `{shutdown,Term}`). `temporary`: never restarted; child spec is auto-deleted on termination.
- `shutdown => brutal_kill` sends `kill` and `terminate` is NOT called. Integer ms sends `shutdown`, waits, then escalates to `kill`. `infinity` waits indefinitely and is REQUIRED for child supervisors.
- More than `intensity` restarts in `period` seconds makes the supervisor terminate all children and itself with reason `shutdown`.
- Children start left-to-right and terminate right-to-left (reverse start order).
- `auto_shutdown` values: `never` (default), `any_significant`, `all_significant`. Only `transient` or `temporary` children can be `significant`. Do not use on top application supervisors; auto-shutdown supervisors must NOT be `permanent` children.
- `supervisor:start_link/2,3` is synchronous and does not return until `init/1` returns and all children are started. Registered variants accept `{local,Name}`, `{global,Name}`, or `{via,Module,Name}`.
- `supervisor:stop/1,3` (OTP 29.0+) orders the supervisor to exit with `Reason`; calling it from a sub-child causes deadlock.
- `which_children/1` returns `[{Id, Child, Type, Modules}]` where `Child` is `pid()`, `restarting`, or `undefined`.
- `restart_child/2` cannot be used for `temporary` children because their spec is auto-deleted on termination.
- Use `supervisor:check_childspecs/1,2` to validate specs; pass the intended `auto_shutdown` value to `/2` to validate significant-child rules.
- `count_children/1` returns `[{specs,N},{active,N},{supervisors,N},{workers,N}]`.
- In multi-level trees, effective restart budget compounds; do not reuse the same intensity/period at every level.
- NEVER call `supervisor:terminate_child/2` from a child located in its own supervision tree (deadlock).

## Quick Commands

```erl
{ok, Pid} = supervisor:start_link({local, my_sup}, my_sup, []).
supervisor:which_children(my_sup).
supervisor:count_children(my_sup).
{ok, Child} = supervisor:start_child(my_sup, ChildSpec).
{ok, _} = supervisor:restart_child(my_sup, ChildId).
{ok, _} = supervisor:get_childspec(my_sup, ChildId).
{ok, _} = supervisor:which_child(my_sup, ChildId).
ok = supervisor:terminate_child(my_sup, ChildId).
ok = supervisor:delete_child(my_sup, ChildId).
ok = supervisor:check_childspecs(ChildSpecs).
ok = supervisor:stop(my_sup).
sys:get_status(my_sup).
```

## Anti-patterns

- Finite shutdown timeout on a child supervisor; use `infinity`.
- Identical `intensity`/`period` at every supervision level.
- Using `simple_one_for_one` in new code without justification.
- Calling `supervisor:terminate_child/2` from a child in its own tree.
- Using `brutal_kill` on supervisor children.
- Marking one-shot workers as `permanent`.
- `significant => true` on a `permanent` child or in a `never` auto-shutdown supervisor.
- Configuring `auto_shutdown` on a top application supervisor.
- Expecting dynamic children to survive a supervisor restart; they are lost when the supervisor is recreated.

## Related Skills

- `beam-gen-server`
- `beam-gen-statem`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-observability-debugging`
