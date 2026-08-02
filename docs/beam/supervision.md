# Supervision

## Purpose

This document defines BEAM/OTP supervision guidance in Erlang/OTP terminology, grounded strictly in the official Erlang/OTP 29.0.2 documentation. Future agents who write, review, refactor, debug, or validate supervision trees, supervisor callback modules, child specifications, and restart policies should follow these rules so behavior is consistent, predictable, and aligned with the authoritative source.

Supervision is the primary fault-tolerance mechanism in OTP: a supervisor starts, stops, and monitors child processes and restarts them when necessary. Supervisors build a hierarchical process structure called a supervision tree for fault-tolerant applications.

This doc is Erlang-native. No Elixir/Gleam syntax is used.

## Sources used

- Crawl file: `docs/beam/.crawl/03-sup-princ.md` — canonical URL: https://www.erlang.org/doc/system/sup_princ.html (Erlang System Documentation v29.0.2)
- Crawl file: `docs/beam/.crawl/13-supervisor-module.md` — canonical URL: https://www.erlang.org/doc/apps/stdlib/supervisor.html (stdlib 8.0.1, OTP 29.0.2)

All verbatim quotes are tagged with their source crawl (`03` or `13`) and section. The `supervisor` behaviour lives in the `stdlib` application.

## Core guidance

### What a supervisor is

From crawl 03, "## Verbatim quotes" #1 (Supervision Principles):

> "A supervisor is responsible for starting, stopping, and monitoring its child processes. The basic idea of a supervisor is that it is to keep its child processes alive by restarting them when necessary."

From crawl 13, "## Purpose":

> "A supervisor is a process that supervises other processes (child processes). A child can be another supervisor or a worker process (normally `gen_event`, `gen_server`, or `gen_statem`). Supervisors build a hierarchical process structure called a supervision tree for fault-tolerant applications."

A supervisor is an OTP behaviour implemented by the `stdlib` `supervisor` module. A callback module implements `init/1` and returns `{ok, {SupFlags, [ChildSpec]}}` (or `ignore`). The supervisor process traps exits, monitors its children via links, and applies the restart policy encoded in `SupFlags` and each child spec.

### Child specs and ordering

From crawl 03, "## Verbatim quotes" #2 (Supervision Principles):

> "Which child processes to start and monitor is specified by a list of child specifications. The child processes are started in the order specified by this list, and are terminated in the reverse order."

From crawl 13, "## Verbatim quotes":

> "When the supervisor is started, the child processes are started in order from left to right according to this list. When the supervisor is going to terminate, it first terminates its child processes in reversed start order, from right to left."

### supervisor_flags map (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## supervisor_flags map (exact keys + defaults)":

```
sup_flags() :: #{
    strategy       => strategy(),        % optional, default one_for_one
    intensity      => non_neg_integer(), % optional, default 1
    period         => pos_integer(),     % optional, default 5
    hibernate_after=> timeout(),         % optional, available since OTP 28.0
    auto_shutdown   => auto_shutdown()    % optional, default never
}
```

Tuple form `{RestartStrategy, Intensity, Period}` kept for backwards compatibility; map is preferred.

| key | type | default | notes |
|-----|------|---------|-------|
| `strategy` | `strategy()` | `one_for_one` | restart strategy |
| `intensity` | `non_neg_integer()` | `1` | MaxR restarts |
| `period` | `pos_integer()` | `5` | MaxT seconds window |
| `hibernate_after` | `timeout()` | (see notes) | OTP 28.0+; `simple_one_for_one` never hibernates by default; other strategies default to hibernating after inactivity |
| `auto_shutdown` | `auto_shutdown()` | `never` | OTP 24.0+ |

### child_spec map (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## child_spec map (exact keys + defaults + legal values)":

```
child_spec() :: #{
    id          := child_id(),     % MANDATORY
    start       := mfargs(),        % MANDATORY  {M,F,A}
    restart     => restart(),       % optional, default permanent
    significant => significant(),    % optional, default false
    shutdown    => shutdown(),      % optional, default 5000 (worker) / infinity (supervisor)
    type        => worker(),        % optional, default worker
    modules     => modules()        % optional, default [M] from start {M,F,A}
}
```

Tuple form `{Id, StartFunc, Restart, Shutdown, Type, Modules}` kept for backwards compatibility; map is preferred.

| key | mandatory | type | default | legal values |
|-----|-----------|------|---------|--------------|
| `id` | yes | `child_id()` (term, not a pid) | — | any term; identifies child spec internally |
| `start` | yes | `mfargs()` = `{M::module(), F::atom(), A::[term()]}` | — | must create+link child, return `{ok,Child}` \| `{ok,Child,Info}` \| `ignore` \| `{error,Error}` |
| `restart` | no | `restart()` | `permanent` | `permanent` \| `transient` \| `temporary` |
| `significant` | no | `significant()` = `boolean()` | `false` | `true` invalid if restart is `permanent`; invalid if `auto_shutdown` is `never` |
| `shutdown` | no | `shutdown()` | `5000` if type=worker; `infinity` if type=supervisor | `brutal_kill` \| integer (ms) \| `infinity` |
| `type` | no | `worker()` | `worker` | `worker` \| `supervisor` |
| `modules` | no | `modules()` | `[M]` (M from start) | `[Module]` (list with one module) \| `dynamic` (for gen_event) |

Internally supervisor also tracks pid `Child` of child process, or `undefined`.

### init/1 callback

From crawl 13, "## Key functions (exact arities)":

```erlang
-callback init(Args :: term()) ->
    {ok, {SupFlags :: sup_flags(), [ChildSpec :: child_spec()]}} | ignore.
```

### strategy() values (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## strategy values (exact behavior)":

`strategy() :: one_for_all | one_for_one | rest_for_one | simple_one_for_one`

- `one_for_one` — only the terminated child is restarted. **DEFAULT.**
- `one_for_all` — if one child terminates and is to be restarted, all other children are terminated and then all children are restarted.
- `rest_for_one` — if one child terminates and is to be restarted, the "rest" (children after the terminated one in start order) are terminated. Then the terminated child and all children after it are restarted.
- `simple_one_for_one` — simplified `one_for_one`; all children are dynamically added instances of the same process type running the same code.

#### simple_one_for_one caveats (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## strategy values (exact behavior)" and "## Strict rules":

- `delete_child/2` and `restart_child/2` are invalid → return `{error,simple_one_for_one}`.
- `terminate_child/2` requires the child's `pid/0` as second arg; using the child spec id returns `{error,simple_one_for_one}`.
- Shuts down all children asynchronously (cleanup in parallel, stop order undefined).
- `init/1` child spec list must contain exactly one child spec (id ignored); no child started during init; all started dynamically via `start_child/2`.
- `start_child/2` takes `ExtraArgs` list, appended to start args.
- Shutdown is asynchronous (parallel cleanup, undefined order).
- Never hibernates by default (children come and go at high rates).
- `which_children/1` returns `Id = undefined` for each child.
- `count_children/1` `active` count not verified for liveness (likely accurate unless supervisor heavily overloaded).

**Deprecation status:** NOT marked deprecated in OTP 29.0.2 docs. Still a documented legal value. (Note: OTP design principles have historically discouraged it in favor of `supervisor` with dynamic children, but the module reference itself does not deprecate it.)

### restart() values (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## restart values (exact behavior table)":

`restart() :: permanent | transient | temporary`

| value | restart behavior |
|-------|------------------|
| `permanent` | always restarted (default). Cannot be `significant`. |
| `temporary` | never restarted — even when strategy is `rest_for_one` or `one_for_all` and a sibling's death causes the temporary process to be terminated. Child spec auto-deleted on termination; `restart_child/2` cannot be used. |
| `transient` | restarted only if it terminates abnormally, i.e. with exit reason other than `normal`, `shutdown`, or `{shutdown,Term}`. |

From crawl 03, "## Verbatim quotes" #18–#20 (Child Specification — restart):

> "A permanent child process is always restarted."

> "A temporary child process is never restarted (not even when the supervisor restart strategy is rest_for_one or one_for_all and a sibling death causes the temporary process to be terminated)."

> "A transient child process is restarted only if it terminates abnormally, that is, with an exit reason other than normal, shutdown, or {shutdown,Term}."

### shutdown() values (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## shutdown values (exact behavior)":

`shutdown() :: brutal_kill | timeout()`

| value | behavior |
|-------|----------|
| `brutal_kill` | child unconditionally terminated via `exit_signal(Child, kill)`. |
| integer (ms) | supervisor sends `exit_signal(Child, shutdown)` and waits for exit signal with reason `shutdown` back; if none within N ms, child unconditionally killed via `exit_signal(Child, kill)`. |
| `infinity` | wait indefinitely. **Required** when child is another supervisor (to give subtree time to shut down). Allowed for workers but dangerous — worker cleanup must always return. |

Defaults: `5000` for `type=worker`; `infinity` for `type=supervisor`.

From crawl 13, "## shutdown values (exact behavior)" — warning:

> "Setting the shutdown time to anything other than infinity for a child of type supervisor can cause a race condition where the child in question unlinks its own children, but fails to terminate them before it is killed."

From crawl 03, "## Verbatim quotes" #23–#26 (Child Specification — shutdown):

> "brutal_kill means that the child process is unconditionally terminated using exit_signal(Child, kill)."

> "An integer time-out value means that the supervisor tells the child process to terminate by calling exit_signal(Child, shutdown) and then waits for an exit signal back. If no exit signal is received within the specified time, the child process is unconditionally terminated using exit_signal(Child, kill)."

> "If the child process is another supervisor, it should be set to infinity to give the subtree enough time to shut down. It is also allowed to set it to infinity if the child process is a worker."

> "The shutdown key is optional. If it is not given, and the child is of type worker, the default value 5000 will be used; if the child is of type supervisor, the default value infinity will be used."

Note: `exit_signal/2` (not `exit/2`) is used for child termination — reflects OTP 29 BIF naming (crawl 03, "## Version notes"; crawl 13, "## Version notes").

### auto_shutdown() values (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## Strict rules" (auto_shutdown) and crawl 03, "## auto_shutdown values":

`auto_shutdown() :: never | any_significant | all_significant`. Default `never`.

| auto_shutdown | semantics |
|---------------|-----------|
| `never` | Automatic shutdown is disabled. Specifying significant children is not accepted — supervisor refuses to start; dynamic start of significant children rejected. This is the default. |
| `any_significant` | Supervisor automatically shuts itself down when any significant child terminates — i.e. when a transient significant child terminates normally OR when a temporary significant child terminates normally or abnormally. |
| `all_significant` | Supervisor automatically shuts itself down when all significant children have terminated — i.e. when the last active significant child terminates. Same rules as any_significant apply. |

- `significant=true` is invalid when restart type is `permanent`.
- `significant=true` is invalid when `auto_shutdown` is `never`.
- Feature since OTP 24.0; compiles on older OTP but leaks processes (auto shutdowns won't happen) — implementors must take precautions.

From crawl 13, "## Verbatim quotes" — auto_shutdown OTP 24.0 leak warning:

> "The automatic shutdown feature appeared in OTP 24.0, but applications using this feature will also compile and run with older OTP versions. However, such applications, when compiled with an OTP version that predates the appearance of the automatic shutdown feature, will leak processes because the automatic shutdowns they rely on will not happen."

From crawl 03, "## Verbatim quotes" #12 (Automatic Shutdown):

> "The automatic shutdown facility only applies when significant children terminate by themselves, not when their termination was caused by the supervisor. Specifically, neither the termination of a child as a consequence of a sibling's termination in the one_for_all or rest_for_one strategies nor the manual termination of a child by supervisor:terminate_child/2 will trigger an automatic shutdown."

### intensity/period semantics

From crawl 13, "## Verbatim quotes":

> "if more than MaxR restarts occur within MaxT seconds, the supervisor terminates all child processes and then itself. The termination reason for the supervisor itself in that case will be shutdown. intensity defaults to 1 and period defaults to 5."

From crawl 03, "## Verbatim quotes" #8 (Maximum Restart Intensity):

> "If more than MaxR number of restarts occur in the last MaxT seconds, the supervisor terminates all the child processes and then itself. The termination reason for the supervisor itself in that case will be shutdown."

From crawl 03, "## Verbatim quotes" #9 (Maximum Restart Intensity):

> "The intention of the restart mechanism is to prevent a situation where a process repeatedly dies for the same reason, only to be restarted again."

Tuning guidance (crawl 03, "## intensity/period semantics"): `intensity` = burst tolerance; `period` must be long enough that sustained rate is acceptable. Avoid `intensity=1`/`period=6` (no burst tolerance). Avoid very high `period` with bursts (e.g. 5/3600 treats separate incidents as one). In multi-level supervision, total restarts before top-level gives up = product of `intensity` values of all supervisors above the failing child — do not set same values on all levels.

### hibernate_after (OTP 28.0+)

From crawl 13, "## Strict rules" (hibernate_after):

- `simple_one_for_one` never hibernates by default.
- Other strategies default to hibernating after inactivity; finetune via `hibernate_after` flag (e.g. when supervisor regularly queried via `which_children/1`).

### Start/stop ordering and synchronous startup

From crawl 03, "## Verbatim quotes" #30 (Starting a Supervisor):

> "supervisor:start_link/3 is synchronous. It does not return until all child processes have been started."

From crawl 13, "## Strict rules" (Start/stop ordering):

- Children started left-to-right in child spec list order.
- On termination, children terminated in reversed start order (right-to-left).
- `start_link/2,3` does not return until `Module:init/1` returns AND all child processes have been started (synchronized startup).
- If any child start function fails, supervisor terminates all already-started children with reason `shutdown` then itself, returns `{error, {shutdown, Reason}}`.

From crawl 03, "## Verbatim quotes" #36 (Stopping):

> "When asked to shut down, a supervisor terminates all child processes in reverse start order according to the respective shutdown specifications before terminating itself."

### Dynamic children lost on supervisor restart

From crawl 03, "## Verbatim quotes" #31 (Adding a Child Process):

> "if a supervisor dies and is recreated, then all child processes that were dynamically added to the supervisor are lost."

### Top supervisors and auto-shutdown

From crawl 03, "## Strict rules / invariants":

- Top supervisors of Applications should NOT be configured for automatic shutdown (application terminates when top supervisor exits; if permanent, all other applications and the runtime system terminate).
- Supervisors configured for automatic shutdown should NOT be made `permanent` children of their parent supervisors (would be restarted immediately after auto-shutdown, then shut down again, possibly exhausting parent's Maximum Restart Intensity).

### Deadlock warning

From crawl 13, "## Verbatim quotes" (re: stop/3):

> "Calling this function from a (sub-)child process of the given supervisor will result in a deadlock which will last until either the shutdown timeout of the child or the timeout given to stop/3 has expired."

A supervisor should not be stopped manually via `supervisor:terminate_child/2` from a child located in its own tree (deadlock risk; restructuring difficulty; unresponsiveness) — crawl 03, "## Strict rules / invariants".

### Key functions (VERBATIM from crawl 13)

Reproduced verbatim from crawl 13, "## Key functions (exact arities)":

| function | arity | signature / notes |
|----------|-------|-------------------|
| `start_link` | /2 | `start_link(Module, Args) -> startlink_ret()` — nameless supervisor, not registered. Equivalent to /3 without registration. |
| `start_link` | /3 | `start_link(SupName, Module, Args) -> startlink_ret()` — registered supervisor. `SupName :: {local,Name}` \| `{global,Name}` \| `{via,Module,Name}`. Calls `Module:init/1`; does not return until init returns and all children started. |
| `start_child` | /2 | `start_child(SupRef, ChildSpec)` (one_for_one/one_for_all/rest_for_one) OR `start_child(SupRef, ExtraArgs)` (simple_one_for_one, appends ExtraArgs to start args via `apply(M,F,A++ExtraArgs)`). Returns `startchild_ret()`. |
| `terminate_child` | /2 | `terminate_child(SupRef, Id) -> ok \| {error, Error}` where `Id :: pid() \| child_id()`, `Error :: not_found \| simple_one_for_one`. For simple_one_for_one, Id must be pid. |
| `restart_child` | /2 | `restart_child(SupRef, Id) -> {ok,Child} \| {ok,Child,Info} \| {error, Error}` where `Error :: running \| restarting \| not_found \| simple_one_for_one \| term()`. Invalid for temporary children (spec auto-deleted). |
| `delete_child` | /2 | `delete_child(SupRef, Id) -> ok \| {error, Error}` where `Error :: running \| restarting \| not_found \| simple_one_for_one`. Child must not be running. |
| `which_children` | /1 | `which_children(SupRef) -> [{Id, Child, Type, Modules}]`. Id=undefined for simple_one_for_one. Child = pid \| `restarting` \| `undefined`. |
| `count_children` | /1 | `count_children(SupRef) -> [{specs,N},{active,N},{supervisors,N},{workers,N}]`. (since OTP R13B04) |
| `check_childspecs` | /1 | `check_childspecs(ChildSpecs) -> ok \| {error,Error}`. Equivalent to `check_childspecs(ChildSpecs, undefined)`. |
| `check_childspecs` | /2 | `check_childspecs(ChildSpecs, AutoShutdown) -> ok \| {error,Error}`. (since OTP 24.0). If AutoShutdown not undefined, also validates child specs against the auto_shutdown option. |
| `get_childspec` | /2 | `get_childspec(SupRef, Id) -> {ok, child_spec()} \| {error, not_found}`. Id :: pid() \| child_id(). (since OTP 18.0) |
| `which_child` | /2 | `which_child(SupRef, Id) -> {ok, {Id, Child, Type, Modules}} \| {error, not_found}`. (since OTP 28.0) |
| `stop` | /1 | `stop(SupRef) -> ok`. Equivalent to `stop(SupRef, normal, infinity)`. (since OTP 29.0) |
| `stop` | /3 | `stop(SupRef, Reason, Timeout) -> ok`. (since OTP 29.0). Orders supervisor to exit with Reason, waits for termination. Supervisor terminates all children first. Reason other than normal/shutdown/{shutdown,Term} triggers logger error report. Timeout=infinity waits indefinitely. Exits caller with `timeout` if not terminated in time, `noproc` if process absent, `{nodedown,Node}` on remote failure. Warning: calling from a sub-child causes deadlock. |

### modules key

From crawl 03, "## Verbatim quotes" #28 (Child Specification — modules):

> "modules has to be a list consisting of a single element. The value of that element depends on the behaviour of the process: If the child process is a gen_event, the element has to be the atom dynamic. Otherwise, the element should be Module, where Module is the name of the callback module."

`modules` is used by the release handler.

## Practical rules

1. Use `one_for_one` for independent workers. A failure in one should not affect others.
2. Use `one_for_all` only when children are tightly coupled and must restart as a unit.
3. Use `rest_for_one` for dependency chains; order children from least-dependent to most-dependent.
4. Set supervisor children to `shutdown => infinity`.
5. Set worker `shutdown` timeouts high enough to flush state, close connections, and ack in-flight work.
6. Use `brutal_kill` only when cleanup is impossible or when hanging is worse than data loss.
7. Keep `intensity` conservative and increase it only for a well-understood bursty failure mode.
8. Do not reuse the same `intensity`/`period` at every supervision level.
9. Prefer `transient` for workers that should finish cleanly without restart.
10. Prefer `temporary` for workers whose lifecycle is managed externally.
11. Do not call `supervisor:terminate_child/2` from a child in its own tree.
12. Do not mark top application supervisors with `auto_shutdown`.
13. Do not place an auto-shutdown supervisor as a `permanent` child of another supervisor.
14. Do not set `significant => true` on a `permanent` child or in a `never` auto-shutdown supervisor.
15. Set `modules => [Module]` for single-module behaviours; `modules => dynamic` for `gen_event`.

## Review checklist

When reviewing supervision-related code:

- [ ] `strategy` matches the coupling of the children.
- [ ] `intensity` and `period` are tuned for the workload, not blindly left at `1`/`5`.
- [ ] Different supervision levels use different intensities to preserve fault isolation.
- [ ] Supervisor children use `shutdown => infinity`.
- [ ] Worker `shutdown` timeouts are sufficient for cleanup.
- [ ] `restart` is `permanent`, `transient`, or `temporary` as appropriate for the worker class.
- [ ] `temporary` children are not accidentally expected to restart on cascade.
- [ ] `auto_shutdown` is not applied to top application supervisors.
- [ ] Auto-shutdown supervisors are not `permanent` children of another supervisor.
- [ ] Only `transient` or `temporary` children are marked `significant => true`.
- [ ] No `significant => true` in a `never` auto-shutdown supervisor.
- [ ] Child `id` values are unique under static supervisors.
- [ ] Children are ordered correctly for `rest_for_one` semantics.
- [ ] No child calls `supervisor:terminate_child/2` on itself via its own supervisor.
- [ ] `modules` is set correctly for release handling (`[Module]` or `dynamic`).
- [ ] `simple_one_for_one` use (if any) is justified; not deprecated in OTP 29.0.2 docs but historically discouraged.

## Implementation checklist

When implementing a supervisor or supervising a worker:

- [ ] Module exports `init/1` and returns `{ok, {SupFlags, [ChildSpec]}}` or `ignore`.
- [ ] `SupFlags` explicitly sets `strategy`, `intensity`, `period`, and `auto_shutdown` when defaults are not appropriate.
- [ ] Every child spec has a unique `id` (static supervisors).
- [ ] Every child spec has a valid `start` MFA returning `{ok,Child}` | `{ok,Child,Info}` | `ignore` | `{error,Error}`.
- [ ] `type` is `supervisor` for child supervisors.
- [ ] `shutdown` is `infinity` for child supervisors.
- [ ] `restart` is chosen deliberately; default `permanent` is documented.
- [ ] Children are ordered from least-dependent to most-dependent when using `rest_for_one`.
- [ ] `auto_shutdown` configuration is justified and follows the warnings.
- [ ] `significant` is only set on `transient` or `temporary` children and only in `any_significant`/`all_significant` supervisors.
- [ ] `modules` is set to `[Module]` for single-module behaviours or `dynamic` for `gen_event`.
- [ ] The supervisor is started with `supervisor:start_link/2,3` (or a registered-name variant).
- [ ] Application top supervisor is not configured to auto-shutdown.
- [ ] For `simple_one_for_one`: `init/1` returns exactly one child spec; `start_child/2` takes `ExtraArgs`; `terminate_child/2` uses pid.

## Runtime / debugging checklist

When investigating a supervision problem at runtime:

- [ ] Inspect children with `supervisor:which_children/1`.
- [ ] Count active/spec counts with `supervisor:count_children/1`.
- [ ] Check whether a child is `restarting` or `undefined` in `which_children/1` output.
- [ ] For `simple_one_for_one`, note `which_children/1` returns `Id = undefined`.
- [ ] Verify restart frequency with logs: supervisor shutdown reason is `shutdown` when max intensity is exceeded.
- [ ] Check parent supervisor logs: if a supervisor exited with `shutdown`, its own parent restarts it only if the parent's child spec says `permanent`.
- [ ] Confirm `shutdown` timeout behavior: a worker killed after its shutdown timeout appears as `killed` in logs, not `shutdown`.
- [ ] Use `sys:get_status/1` and `sys:get_state/1` on behaviours for inspection (do not use in production logic).
- [ ] Look for deadlock patterns: a child calling `supervisor:terminate_child/2` or `supervisor:stop/3` on its own supervisor stalls both processes.
- [ ] Use `supervisor:get_childspec/2` (OTP 18.0+) and `supervisor:which_child/2` (OTP 28.0+) for targeted inspection.
- [ ] Use `supervisor:check_childspecs/1,2` to validate child spec lists before starting.

## Validation hooks

Validation that should be performed automatically or manually for supervision code:

- [ ] Dialyzer: ensure `init/1` returns the correct `{ok, {sup_flags(), [child_spec()]}}` shape or `ignore`.
- [ ] Static analysis: flag finite `shutdown` on `type => supervisor` children.
- [ ] Static analysis: flag `significant => true` on `permanent` children.
- [ ] Static analysis: flag `significant => true` in `auto_shutdown => never` supervisors.
- [ ] Static analysis: flag `auto_shutdown` on top-level application supervisors.
- [ ] Static analysis: flag auto-shutdown supervisors configured as `permanent` children of a parent.
- [ ] Unit test: start the supervisor, kill each child, assert the expected restart/cascade behavior.
- [ ] Unit test: exceed `intensity` in a test and assert the supervisor exits with `shutdown`.
- [ ] Unit test: stop the supervisor and assert children terminate in reverse order.
- [ ] Unit test: verify dynamic children are lost (or intentionally not restored) after supervisor restart.
- [ ] Unit test: for `simple_one_for_one`, assert `delete_child/2` and `restart_child/2` return `{error,simple_one_for_one}`.
- [ ] Run `supervisor:check_childspecs/2` with the intended `auto_shutdown` mode in test setup.

## Examples

### ch_sup example (from crawl 03)

Example callback module (`ch_sup`) — supervisor starting a gen_server `ch3`:

```erlang
-module(ch_sup).
-behaviour(supervisor).

-export([start_link/0]).
-export([init/1]).

start_link() ->
    supervisor:start_link(ch_sup, []).

init(_Args) ->
    SupFlags = #{strategy => one_for_one, intensity => 1, period => 5},
    ChildSpecs = [#{id => ch3,
                    start => {ch3, start_link, []},
                    restart => permanent,
                    shutdown => brutal_kill,
                    type => worker,
                    modules => [ch3]}],
    {ok, {SupFlags, ChildSpecs}}.
```

Child spec for `ch3` (full):

```erlang
#{id => ch3,
  start => {ch3, start_link, []},
  restart => permanent,
  shutdown => brutal_kill,
  type => worker,
  modules => [ch3]}
```

Same child spec simplified (relying on defaults):

```erlang
#{id => ch3,
  start => {ch3, start_link, []},
  shutdown => brutal_kill}
```

Child spec for an event manager (`gen_event`):

```erlang
#{id => error_man,
  start => {gen_event, start_link, [{local, error_man}]},
  modules => dynamic}
```

(`error_man` shutdown defaults to 5000 ms; permanent by default.)

Child spec for another supervisor:

```erlang
#{id => sup,
  start => {sup, start_link, []},
  restart => transient,
  type => supervisor} % will cause default shutdown=>infinity
```

`init/1` returning empty flags map (all defaults):

```erlang
init(_Args) ->
    SupFlags = #{},
    ChildSpecs = [#{id => ch3,
                    start => {ch3, start_link, []},
                    shutdown => brutal_kill}],
    {ok, {SupFlags, ChildSpecs}}.
```

### simple_one_for_one supervisor (from crawl 03)

```erlang
-module(simple_sup).
-behaviour(supervisor).

-export([start_link/0]).
-export([init/1]).

start_link() ->
    supervisor:start_link(simple_sup, []).

init(_Args) ->
    SupFlags = #{strategy => simple_one_for_one,
                 intensity => 0,
                 period => 1},
    ChildSpecs = [#{id => call,
                    start => {call, start_link, []},
                    shutdown => brutal_kill}],
    {ok, {SupFlags, ChildSpecs}}.
```

Adding a child to `simple_one_for_one`: `supervisor:start_child(Pid, [id1])` → starts child via `apply(call, start_link, []++[id1])` i.e. `call:start_link(id1)`.

Terminating a `simple_one_for_one` child: `supervisor:terminate_child(Sup, Pid)`.

### Runtime child management

```erlang
%% Add a child to a running static supervisor.
ChildSpec = #{id => extra_worker,
              start => {my_worker, start_link, [extra_arg]},
              restart => transient,
              shutdown => 5000,
              type => worker,
              modules => [my_worker]},
{ok, Pid} = supervisor:start_child(my_sup, ChildSpec).

%% Terminate and remove it.
ok = supervisor:terminate_child(my_sup, extra_worker),
ok = supervisor:delete_child(my_sup, extra_worker).

%% Inspect the tree.
supervisor:which_children(my_sup).
supervisor:count_children(my_sup).
```

## Common mistakes

1. **Setting a finite shutdown timeout for a child supervisor.** Fix: always use `infinity` for `type => supervisor` children; non-infinity risks a race where the child unlinks its own children but fails to terminate them before being killed.
2. **Expecting `temporary` children to restart on cascade.** Fix: `temporary` children never restart, even under `one_for_all` or `rest_for_one`; child spec is auto-deleted on termination.
3. **Leaving default `intensity => 1` for bursty workloads.** Fix: tune intensity/period; avoid `intensity=1`/`period=6` (no burst tolerance); avoid very high period with bursts.
4. **Using identical intensities at every supervision level.** Fix: vary intensities so that fault isolation does not collapse; total restarts multiply across levels.
5. **Calling `supervisor:terminate_child/2` or `supervisor:stop/3` from a child on its own supervisor.** Fix: let the child exit with `normal`/`shutdown`/`{shutdown,Term}` and let the supervisor handle it; use `auto_shutdown` for bounded lifecycles.
6. **Marking a top application supervisor with `auto_shutdown`.** Fix: keep application top supervisors with `auto_shutdown => never`.
7. **Making an auto-shutdown supervisor a `permanent` child.** Fix: use `transient` or `temporary` for auto-shutdown supervisors, or structure the tree so the shutdown is intentional.
8. **Marking a `permanent` child as `significant`.** Fix: only `transient` or `temporary` children can be significant.
9. **Setting `significant => true` in a `never` auto-shutdown supervisor.** Fix: supervisor refuses to start; dynamic start of significant children is rejected.
10. **Forgetting that `permanent` restarts on `normal`.** Fix: use `transient` for workers that should be allowed to exit cleanly without restart.
11. **Conflicting `id` values under a static supervisor.** Fix: ensure every child `id` is unique.
12. **Ordering `rest_for_one` children backwards.** Fix: least-dependent first, most-dependent last.
13. **Ignoring `modules` for release handling.** Fix: set `modules => [Module]` for single callback modules and `modules => dynamic` for `gen_event`.
14. **Using `delete_child/2` or `restart_child/2` on a `simple_one_for_one` supervisor.** Fix: these return `{error,simple_one_for_one}`; use `terminate_child/2` with a pid.
15. **Assuming dynamic children survive supervisor restart.** Fix: dynamic children added via `supervisor:start_child/2` are lost if the supervisor dies and is recreated.
16. **Relying on `auto_shutdown` on OTP < 24.0.** Fix: code compiles but leaks processes because auto shutdowns will not happen.

## Strict vs contextual guidance

### Strict

Hard constraints from OTP semantics or safety invariants. Violations are bugs.

- A child supervisor MUST use `shutdown => infinity`.
- `significant => true` is only allowed on children with `restart => transient` or `restart => temporary`.
- `significant => true` is invalid when `auto_shutdown => never`.
- A child MUST NOT call `supervisor:terminate_child/2` or `supervisor:stop/3` on its own supervisor (deadlock).
- `id` values MUST be unique among children of a static supervisor.
- `init/1` MUST return `{ok, {SupFlags, [ChildSpec]}}` or `ignore`, with `SupFlags` containing valid keys.
- For `simple_one_for_one`, `init/1` MUST return exactly one child spec; `terminate_child/2` MUST use a pid.
- Top application supervisors MUST NOT be configured for automatic shutdown.
- Auto-shutdown supervisors MUST NOT be `permanent` children of their parent supervisors.

### Convention

Strong defaults. Deviations should be documented.

- Default `restart` is `permanent`; override to `transient` for clean-exit workers and `temporary` for externally managed workers.
- Default worker `shutdown` is `5000`; increase when cleanup needs more time.
- Use `one_for_one` for independent workers.
- Use `rest_for_one` for ordered dependencies; order least-dependent first.
- Use `one_for_all` only for tightly coupled children.
- Keep `intensity` conservative and increase only for understood bursty failures.
- Use distinct `intensity`/`period` values at different supervision levels.
- Set `modules => [Module]` for single-module behaviours; `modules => dynamic` for `gen_event`.

### Contextual

Depend on the specific subsystem and must be chosen deliberately.

- Exact `intensity` and `period` values for a given supervisor.
- Whether a worker should be `permanent`, `transient`, or `temporary`.
- Whether `auto_shutdown` is appropriate for a bounded-lifecycle subsystem.
- Whether a worker's `shutdown` timeout should be short (fail fast) or long (graceful cleanup).
- Whether `brutal_kill` is acceptable for a worker that cannot be trusted to shut down cleanly.
- Whether `simple_one_for_one` is appropriate (not deprecated in OTP 29.0.2 docs, but historically discouraged).

### Policy

Project-level decisions that this repo must make explicitly.

- Default `intensity`/`period` per supervisor class.
- Default `restart` value per worker class.
- Whether `auto_shutdown` is permitted at all, and in which layers.
- Whether to allow `brutal_kill` and under what conditions.
- Whether `simple_one_for_one` may remain in existing legacy code and on what migration timeline.
- Whether every supervisor callback module must explicitly set all `SupFlags` keys or may rely on OTP defaults.
- Naming conventions for child `id` values (atoms, tuples, module names).

## Policy decisions for individual repos

Individual repositories should answer the following questions and, ideally, encode the answers in linting or review templates.

1. **Default intensity/period per supervisor class.** What should the global default be for top application supervisors, intermediate supervisors, and leaf supervisors? Should the defaults differ by subsystem criticality?
2. **Restart defaults per worker class.** Should ordinary workers default to `permanent`, while one-shot jobs default to `transient` and externally managed processes default to `temporary`?
3. **Auto-shutdown policy.** Is `auto_shutdown` allowed for non-top supervisors? Must it be documented and reviewed case-by-case?
4. **Child supervisor shutdown.** Is `shutdown => infinity` for supervisor children enforced by static analysis?
5. **Significant children.** In which subsystems is `significant => true` permitted, and who must approve it?
6. **`brutal_kill` usage.** Under what conditions may `brutal_kill` be used, and who must approve deviations from finite shutdown timeouts?
7. **`modules` key completeness.** Must every child spec include `modules`, or can projects rely on the `[Module]` default for single-module behaviours?
8. **Supervisor registration.** Should supervisors typically be registered locally with `{local, Name}`, or should unnamed supervisors be preferred? When are global or via names acceptable?
9. **Child ordering conventions.** For `rest_for_one` supervisors, is a dependency comment required above each child in the child spec list?
10. **`simple_one_for_one` policy.** Although not deprecated in OTP 29.0.2 docs, is its use permitted in new code, or restricted to legacy with a migration plan?

## Related docs

- [otp-behaviours](otp-behaviours.md)
- [gen-server](gen-server.md)
- [gen-statem](gen-statem.md)
- [gen-event](gen-event.md)
- [applications](applications.md)
- [releases](releases.md)
- [proc-lib-and-sys](proc-lib-and-sys.md)
- [common-mistakes](common-mistakes.md)

## Related skills

- `beam-supervision`
- `beam-gen-server`
- `beam-errors-failures`
- `beam-observability-debugging`
