# Crawl: sup_princ.html

- seed_url: https://www.erlang.org/doc/system/sup_princ.html
- canonical_url: https://www.erlang.org/doc/system/sup_princ.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: 29.0.2 (from page title: "Erlang System Documentation v29.0.2")
- feeds_docs: supervision.md

## Purpose

A supervisor is responsible for starting, stopping, and monitoring its child
processes. The basic idea of a supervisor is that it is to keep its child
processes alive by restarting them when necessary. Which child processes to
start and monitor is specified by a list of child specifications. The child
processes are started in the order specified by this list, and are terminated
in the reverse order.

## Key concepts

- Supervisor = process that starts/stops/monitors child processes and restarts them when necessary.
- Child specifications = list defining which children to start/monitor; started in list order, terminated in reverse order.
- Supervisor flags map (`SupFlags`) = strategy + intensity + period + auto_shutdown.
- Restart strategy: one_for_one | one_for_all | rest_for_one | simple_one_for_one.
- Maximum restart intensity: `intensity` (MaxR) restarts within `period` (MaxT) seconds → supervisor terminates all children then itself (reason `shutdown`).
- Automatic shutdown: supervisor can shut itself down when significant children terminate (OTP 24.0+).
- Significant children: only meaningful with auto_shutdown = any_significant | all_significant.
- `supervisor:start_link/2,3` is synchronous; does not return until all child processes have been started.
- Dynamic children added via `supervisor:start_child/2` are lost if the supervisor dies and is recreated.
- simple_one_for_one: simplified one_for_one; all children are dynamically added instances of the same process; supervisor starts no children at startup.

## Strict rules / invariants

- Child processes are started in the order specified by the child spec list, and terminated in the reverse order.
- If more than MaxR restarts occur in the last MaxT seconds, the supervisor terminates all child processes and then itself; the supervisor's own termination reason is `shutdown`.
- It is invalid to set `significant => true` for a child with restart type `permanent` OR in a supervisor with `auto_shutdown => never`.
- In `never` mode, specifying significant children is not accepted: supervisor refuses to start; dynamic start of significant children is rejected.
- Automatic shutdown only applies when significant children terminate *by themselves* — NOT when termination was caused by the supervisor (sibling termination in one_for_all/rest_for_one, or manual `supervisor:terminate_child/2`).
- Top supervisors of Applications should NOT be configured for automatic shutdown (application terminates when top supervisor exits; if permanent, all other applications and the runtime system terminate).
- Supervisors configured for automatic shutdown should NOT be made `permanent` children of their parent supervisors (would be restarted immediately after auto-shutdown, then shut down again, possibly exhausting parent's Maximum Restart Intensity).
- `modules` must be a list of a single element: `dynamic` for gen_event, otherwise the callback module name.
- `start` must be (or result in) a call to supervisor:start_link/2,3 | gen_server:start_link/3,4 | gen_statem:start_link/3,4 | gen_event:start_link/0,1,2 (or a compliant function).
- Setting shutdown time to anything other than `infinity` for a child of type `supervisor` can cause a race condition where the child unlinks its own children but fails to terminate them before being killed.
- A supervisor should not be stopped manually via `supervisor:terminate_child/2` from a child located in its own tree (deadlock risk; restructuring difficulty; unresponsiveness).

## Examples

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
(error_man shutdown defaults to 5000 ms; permanent by default.)

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

simple_one_for_one supervisor callback module:
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

Adding a child to simple_one_for_one: `supervisor:start_child(Pid, [id1])` → starts child via `apply(call, start_link, []++[id1])` i.e. `call:start_link(id1)`.

Terminating a simple_one_for_one child: `supervisor:terminate_child(Sup, Pid)`.

## Behaviour / callback details

### Supervisor flags map

Type definition:
```erlang
sup_flags() = #{strategy => strategy(),           % optional
                intensity => non_neg_integer(),   % optional
                period => pos_integer(),          % optional
                auto_shutdown => auto_shutdown()} % optional
    strategy() = one_for_all
               | one_for_one
               | rest_for_one
               | simple_one_for_one
    auto_shutdown() = never
                    | any_significant
                    | all_significant
```

Returned by `init/1` as `{ok, {SupFlags, ChildSpecs}}`.

| key | type | optional | default | notes |
|-----|------|----------|---------|-------|
| `strategy` | `strategy()` | yes | `one_for_one` | restart strategy |
| `intensity` | `non_neg_integer()` | yes | `1` | MaxR restarts |
| `period` | `pos_integer()` | yes | `5` | MaxT seconds window |
| `auto_shutdown` | `auto_shutdown()` | yes | `never` | self-shutdown on significant child termination |

### Child spec keys

Type definition:
```erlang
child_spec() = #{id => child_id(),             % mandatory
                 start => mfargs(),            % mandatory
                 restart => restart(),         % optional
                 significant => significant(), % optional
                 shutdown => shutdown(),       % optional
                 type => worker(),             % optional
                 modules => modules()}         % optional
    child_id() = term()
    mfargs() = {M :: module(), F :: atom(), A :: [term()]}
    modules() = [module()] | dynamic
    restart() = permanent | transient | temporary
    significant() = boolean()
    shutdown() = brutal_kill | timeout()
    worker() = worker | supervisor
```

| key | type | mandatory | default | notes |
|-----|------|-----------|---------|-------|
| `id` | `child_id()` (term) | YES | — | internal identifier (a.k.a. "name") |
| `start` | `mfargs()` = `{M,F,A}` | YES | — | `apply(M,F,A)`; must be/result in supervisor/gen_server/gen_statem/gen_event start_link |
| `restart` | `restart()` | no | `permanent` | when to restart terminated child |
| `significant` | `boolean()` | no | (false implied) | significant for auto-shutdown; invalid `true` if restart=permanent or auto_shutdown=never |
| `shutdown` | `shutdown()` | no | `5000` if type=worker; `infinity` if type=supervisor | how to terminate child |
| `type` | `worker()` | no | `worker` | worker | supervisor |
| `modules` | `modules()` = `[module()] | dynamic` | no | `[M]` where M from start `{M,F,A}` | `dynamic` for gen_event; used by release handler |

### Restart strategies (semantics table)

| strategy | semantics on child termination |
|----------|--------------------------------|
| `one_for_one` | If a child process terminates, only that process is restarted. |
| `one_for_all` | If a child process terminates, all remaining child processes are terminated. Subsequently, all child processes, including the terminated one, are restarted. (Terminated right to left; restarted left to right.) |
| `rest_for_one` | If a child process terminates, the child processes *after* the terminated process in start order are terminated. Subsequently, the terminated child process and the remaining child processes are restarted. |
| `simple_one_for_one` | Simplified one_for_one; all children are dynamically added instances of the same process type. Supervisor starts no children at startup; children added via `supervisor:start_child(Sup, List)`. Children shut down asynchronously (order undefined). |

### Restart values (restart() table)

| restart value | semantics |
|---------------|-----------|
| `permanent` | A permanent child process is always restarted. |
| `temporary` | A temporary child process is never restarted (not even when the supervisor restart strategy is rest_for_one or one_for_all and a sibling death causes the temporary process to be terminated). |
| `transient` | A transient child process is restarted only if it terminates abnormally, that is, with an exit reason other than `normal`, `shutdown`, or `{shutdown,Term}`. |

### Shutdown values (shutdown() table)

| shutdown value | semantics |
|----------------|-----------|
| `brutal_kill` | The child process is unconditionally terminated using `exit_signal(Child, kill)`. |
| integer time-out value | The supervisor tells the child process to terminate by calling `exit_signal(Child, shutdown)` and then waits for an exit signal back. If no exit signal is received within the specified time, the child process is unconditionally terminated using `exit_signal(Child, kill)`. |
| `infinity` | If the child process is another supervisor, it should be set to `infinity` to give the subtree enough time to shut down. It is also allowed to set it to `infinity` if the child process is a worker. (Warning: setting non-infinity for a supervisor child risks race condition; setting infinity for a worker makes supervision tree termination depend on that child.) |

### auto_shutdown values

| auto_shutdown | semantics |
|---------------|-----------|
| `never` | Automatic shutdown is disabled. Specifying significant children is not accepted — supervisor refuses to start; dynamic start of significant children rejected. This is the default. |
| `any_significant` | Supervisor automatically shuts itself down when any significant child terminates — i.e. when a transient significant child terminates normally OR when a temporary significant child terminates normally or abnormally. |
| `all_significant` | Supervisor automatically shuts itself down when all significant children have terminated — i.e. when the last active significant child terminates. Same rules as any_significant apply. |

### Start/stop ordering

- Children started in the order specified by the child spec list.
- Children terminated in the reverse order (reverse start order) according to respective shutdown specifications.
- `supervisor:start_link/2,3` is synchronous — does not return until all child processes have been started.
- On supervisor shutdown: terminates all child processes in reverse start order per their shutdown specs, then terminates itself.
- simple_one_for_one: shuts down all children asynchronously; order undefined; children clean up in parallel.
- Starting, restarting, and manually terminating children are synchronous operations executed in the context of the supervisor process (supervisor blocked during these).

### intensity/period semantics

- `intensity` (MaxR) + `period` (MaxT): if more than MaxR restarts occur in the last MaxT seconds, supervisor terminates all child processes and then itself; supervisor's own termination reason = `shutdown`.
- Defaults: intensity=1, period=5.
- Tuning guidance: intensity = burst tolerance; period must be long enough that sustained rate is acceptable. Avoid intensity=1/period=6 (no burst tolerance). Avoid very high period with bursts (e.g. 5/3600 treats separate incidents as one). In multi-level supervision, total restarts before top-level gives up = product of intensity values of all supervisors above the failing child — do not set same values on all levels.

## Verbatim quotes

1. **Supervision Principles**: "A supervisor is responsible for starting, stopping, and monitoring its child processes. The basic idea of a supervisor is that it is to keep its child processes alive by restarting them when necessary."

2. **Supervision Principles**: "Which child processes to start and monitor is specified by a list of child specifications. The child processes are started in the order specified by this list, and are terminated in the reverse order."

3. **Supervisor Flags**: "The SupFlags variable in the return value from init/1 represents the supervisor flags. The ChildSpecs variable in the return value from init/1 is a list of child specifications."

4. **Restart Strategy**: "The strategy key is optional in this map. If it is not given, it defaults to one_for_one."

5. **one_for_one**: "If a child process terminates, only that process is restarted."

6. **one_for_all**: "If a child process terminates, all remaining child processes are terminated. Subsequently, all child processes, including the terminated one, are restarted."

7. **rest_for_one**: "If a child process terminates, the child processes after the terminated process in start order are terminated. Subsequently, the terminated child process and the remaining child processes are restarted."

8. **Maximum Restart Intensity**: "If more than MaxR number of restarts occur in the last MaxT seconds, the supervisor terminates all the child processes and then itself. The termination reason for the supervisor itself in that case will be shutdown."

9. **Maximum Restart Intensity**: "The intention of the restart mechanism is to prevent a situation where a process repeatedly dies for the same reason, only to be restarted again."

10. **Maximum Restart Intensity**: "The keys intensity and period are optional in the supervisor flags map. If they are not given, they default to 1 and 5, respectively."

11. **Automatic Shutdown**: "A supervisor can be configured to automatically shut itself down when significant children terminate."

12. **Automatic Shutdown**: "The automatic shutdown facility only applies when significant children terminate by themselves, not when their termination was caused by the supervisor. Specifically, neither the termination of a child as a consequence of a sibling's termination in the one_for_all or rest_for_one strategies nor the manual termination of a child by supervisor:terminate_child/2 will trigger an automatic shutdown."

13. **never**: "Automatic shutdown is disabled. In this mode, specifying significant children is not accepted. If the child specs returned from init contain significant children, the supervisor will refuse to start. Attempts to start significant children dynamically will be rejected. This is the default setting."

14. **any_significant**: "The supervisor will automatically shut itself down when any significant child terminates, that is, when a transient significant child terminates normally or when a temporary significant child terminates normally or abnormally."

15. **all_significant**: "The supervisor will automatically shut itself down when all significant children have terminated, that is, when the last active significant child terminates. The same rules as for any_significant apply."

16. **Child Specification — id**: "id is used to identify the child specification internally by the supervisor. The id key is mandatory."

17. **Child Specification — start**: "start defines the function call used to start the child process. It is a module-function-arguments tuple used as apply(M, F, A). ... The start key is mandatory."

18. **Child Specification — restart (permanent)**: "A permanent child process is always restarted."

19. **Child Specification — restart (temporary)**: "A temporary child process is never restarted (not even when the supervisor restart strategy is rest_for_one or one_for_all and a sibling death causes the temporary process to be terminated)."

20. **Child Specification — restart (transient)**: "A transient child process is restarted only if it terminates abnormally, that is, with an exit reason other than normal, shutdown, or {shutdown,Term}."

21. **Child Specification — restart default**: "The restart key is optional. If it is not given, the default value permanent will be used."

22. **Child Specification — significant**: "significant defines whether a child is considered significant for automatic self-shutdown of the supervisor. It is invalid to set this option to true for a child with restart type permanent or in a supervisor with auto_shutdown set to never."

23. **Child Specification — shutdown (brutal_kill)**: "brutal_kill means that the child process is unconditionally terminated using exit_signal(Child, kill)."

24. **Child Specification — shutdown (integer)**: "An integer time-out value means that the supervisor tells the child process to terminate by calling exit_signal(Child, shutdown) and then waits for an exit signal back. If no exit signal is received within the specified time, the child process is unconditionally terminated using exit_signal(Child, kill)."

25. **Child Specification — shutdown (infinity)**: "If the child process is another supervisor, it should be set to infinity to give the subtree enough time to shut down. It is also allowed to set it to infinity if the child process is a worker."

26. **Child Specification — shutdown default**: "The shutdown key is optional. If it is not given, and the child is of type worker, the default value 5000 will be used; if the child is of type supervisor, the default value infinity will be used."

27. **Child Specification — type default**: "The type key is optional. If it is not given, the default value worker will be used."

28. **Child Specification — modules**: "modules has to be a list consisting of a single element. The value of that element depends on the behaviour of the process: If the child process is a gen_event, the element has to be the atom dynamic. Otherwise, the element should be Module, where Module is the name of the callback module."

29. **Child Specification — modules default**: "The modules key is optional. If it is not given, it defaults to [M], where M comes from the child's start {M,F,A}."

30. **Starting a Supervisor**: "supervisor:start_link/3 is synchronous. It does not return until all child processes have been started."

31. **Adding a Child Process**: "if a supervisor dies and is recreated, then all child processes that were dynamically added to the supervisor are lost."

32. **Simplified one_for_one Supervisors**: "A supervisor with restart strategy simple_one_for_one is a simplified one_for_one supervisor, where all child processes are dynamically added instances of the same process."

33. **Simplified one_for_one Supervisors**: "When started, the supervisor does not start any child processes. Instead, all child processes need to be added dynamically by calling supervisor:start_child(Sup, List)."

34. **Simplified one_for_one Supervisors**: "Because a simple_one_for_one supervisor can have many children, it shuts them all down asynchronously. This means that the children will do their cleanup in parallel and therefore the order in which they are stopped is not defined."

35. **Simplified one_for_one Supervisors**: "Starting, restarting, and manually terminating children are synchronous operations which are executed in the context of the supervisor process. This means that the supervisor process will be blocked while it is performing any of those operations. Child processes are responsible for keeping their start and shutdown phases as short as possible."

36. **Stopping**: "When asked to shut down, a supervisor terminates all child processes in reverse start order according to the respective shutdown specifications before terminating itself."

## Version notes

- Page documents Erlang/OTP System Documentation v29.0.2.
- Automatic shutdown feature was introduced in OTP 24.0. Applications using this feature will also compile and run with older OTP versions, but when compiled with an OTP version predating the feature, they will leak processes because the automatic shutdowns they rely on will not happen.
- Source markdown: https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/sup_princ.md
- The page notes `exit_signal/2` (rather than the older `exit/2`) is used for child termination — reflects OTP 29 BIF naming.

## Discovered links

### Relevant (crawl later)

- ../apps/stdlib/supervisor.html — STDLIB supervisor module reference (canonical API: start_link/2,3; start_child/2; terminate_child/2; delete_child/2)
- ../apps/stdlib/gen_server.html#start_link/4 — gen_server start_link (child start function)
- ../apps/stdlib/gen_statem.html#start_link/4 — gen_statem start_link (child start function)
- ../apps/stdlib/gen_event.html#start_link/2 — gen_event start_link (child start function)
- ../apps/erts/erlang.html#apply/3 — apply/3 (used for child start)
- ../apps/erts/erlang.html#exit_signal/2 — exit_signal/2 (used for child termination)
- release_handling.html — Release Handling (referenced by `modules` key semantics for upgrades/downgrades)
- events.html — events / gen_event Behaviour (sibling system doc)
- events.html#mgr — event manager section
- gen_server_concepts.html#ex — gen_server Behaviour (sibling system doc)
- spec_proc.html — sys and proc_lib (next page in system docs)
- applications.html — Applications (sibling system doc)

### Skipped

- #adding-a-child-process (in-page anchor)
- #all_significant (in-page anchor)
- #any_significant (in-page anchor)
- #automatic-shutdown (in-page anchor)
- #child-specification (in-page anchor)
- #example (in-page anchor)
- #manual-stopping-versus-automatic-shutdown (in-page anchor)
- #maximum-restart-intensity (in-page anchor)
- #never (in-page anchor)
- #one_for_all (in-page anchor)
- #one_for_one (in-page anchor)
- #rest_for_one (in-page anchor)
- #restart-strategy (in-page anchor)
- #simple_one_for_one (in-page anchor)
- #simplified-one_for_one-supervisors (in-page anchor)
- #starting-a-supervisor (in-page anchor)
- #stopping (in-page anchor)
- #stopping-a-child-process (in-page anchor)
- #supervision-principles (in-page anchor)
- #supervisor-flags (in-page anchor)
- #tuning-the-intensity-and-period (in-page anchor)
- sup_princ.html#automatic-shutdown (self anchor)
- sup_princ.html#flags (self anchor)
- sup_princ.html#max_intensity (self anchor)
- sup_princ.html#restart (self anchor)
- sup_princ.html#shutdown (self anchor)
- sup_princ.html#significant_child (self anchor)
- sup_princ.html#simple (self anchor)
- sup_princ.html#spec (self anchor)
- sup_princ.html#strategy (self anchor)
- sup_princ.md (self, markdown source)
- sup_princ.html#automatic-shutdown etc. (self anchors)
- ../index.html (system docs index — navigation)
- https://erlang.org (site home)
- https://www.erlang.org/doc/system/sup_princ.html (self canonical)
- https://www.ericsson.com (copyright)
- https://github.com/elixir-lang/ex_doc (doc tooling)
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/sup_princ.md#L1 (source)
- /assets/css/algolia-typeahead.css (asset)
- dist/html-erlang-KCHZLXSC.css (asset)
- Erlang System Documentation.epub (download)
- llms.txt (site metadata)
