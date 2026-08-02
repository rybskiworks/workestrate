# Crawl: stdlib/supervisor.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/supervisor.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/supervisor.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib 8.0.1)
- feeds_docs: supervision.md

## Purpose

Generic supervisor behaviour. A supervisor is a process that supervises other
processes (child processes). A child can be another supervisor or a worker
process (normally `gen_event`, `gen_server`, or `gen_statem`). Supervisors build
a hierarchical process structure called a supervision tree for fault-tolerant
applications. The supervisor is responsible for starting, stopping, and
monitoring its child processes, keeping them alive by restarting them when
necessary.

Children are defined as a list of child specifications. On start, children are
started left-to-right in order. On termination, children are terminated in
reversed start order (right-to-left).

## supervisor_flags map (exact keys + defaults)

```
sup_flags() :: #{
    strategy       => strategy(),        % optional, default one_for_one
    intensity      => non_neg_integer(), % optional, default 1
    period         => pos_integer(),     % optional, default 5
    hibernate_after=> timeout(),         % optional, available since OTP 28.0
    auto_shutdown   => auto_shutdown()    % optional, default never
}
```

Tuple form `{RestartStrategy, Intensity, Period}` kept for backwards
compatibility; map is preferred.

| key | type | default | notes |
|-----|------|---------|-------|
| `strategy` | `strategy()` | `one_for_one` | restart strategy |
| `intensity` | `non_neg_integer()` | `1` | MaxR restarts |
| `period` | `pos_integer()` | `5` | MaxT seconds window |
| `hibernate_after` | `timeout()` | (see notes) | OTP 28.0+; `simple_one_for_one` never hibernates by default; other strategies default to hibernating after inactivity |
| `auto_shutdown` | `auto_shutdown()` | `never` | OTP 24.0+ |

## child_spec map (exact keys + defaults + legal values)

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

Tuple form `{Id, StartFunc, Restart, Shutdown, Type, Modules}` kept for backwards
compatibility; map is preferred.

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

## strategy values (exact behavior)

`strategy() :: one_for_all | one_for_one | rest_for_one | simple_one_for_one`

- `one_for_one` — only the terminated child is restarted. **DEFAULT.**
- `one_for_all` — if one child terminates and is to be restarted, all other
  children are terminated and then all children are restarted.
- `rest_for_one` — if one child terminates and is to be restarted, the "rest"
  (children after the terminated one in start order) are terminated. Then the
  terminated child and all children after it are restarted.
- `simple_one_for_one` — simplified `one_for_one`; all children are dynamically
  added instances of the same process type running the same code.
  - `delete_child/2` and `restart_child/2` are invalid → return
    `{error,simple_one_for_one}`.
  - `terminate_child/2` requires the child's `pid/0` as second arg; using the
    child spec id returns `{error,simple_one_for_one}`.
  - Shuts down all children asynchronously (cleanup in parallel, stop order
    undefined).
  - `init/1` child spec list must contain exactly one child spec (id ignored);
    no child started during init; all started dynamically via `start_child/2`.
  - **Deprecation status:** NOT marked deprecated in OTP 29.0.2 docs. Still a
    documented legal value. (Note: OTP design principles have historically
    discouraged it in favor of `supervisor` with dynamic children, but the
    module reference itself does not deprecate it.)

## restart values (exact behavior table)

`restart() :: permanent | transient | temporary`

| value | restart behavior |
|-------|------------------|
| `permanent` | always restarted (default). Cannot be `significant`. |
| `temporary` | never restarted — even when strategy is `rest_for_one` or `one_for_all` and a sibling's death causes the temporary process to be terminated. Child spec auto-deleted on termination; `restart_child/2` cannot be used. |
| `transient` | restarted only if it terminates abnormally, i.e. with exit reason other than `normal`, `shutdown`, or `{shutdown,Term}`. |

## shutdown values (exact behavior)

`shutdown() :: brutal_kill | timeout()`

| value | behavior |
|-------|----------|
| `brutal_kill` | child unconditionally terminated via `exit_signal(Child, kill)`. |
| integer (ms) | supervisor sends `exit_signal(Child, shutdown)` and waits for exit signal with reason `shutdown` back; if none within N ms, child unconditionally killed via `exit_signal(Child, kill)`. |
| `infinity` | wait indefinitely. **Required** when child is another supervisor (to give subtree time to shut down). Allowed for workers but dangerous — worker cleanup must always return. |

Defaults: `5000` for `type=worker`; `infinity` for `type=supervisor`.

Warning: setting shutdown to anything other than `infinity` for a child of type
`supervisor` can cause a race where the child unlinks its own children but fails
to terminate them before being killed.

All standard OTP behaviour modules automatically adhere to the shutdown protocol.

## Key functions (exact arities)

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

Callback:
| `init` | /1 | `-callback init(Args :: term()) -> {ok, {SupFlags :: sup_flags(), [ChildSpec :: child_spec()]}} | ignore.` |

## Strict rules (intensity/period; start/stop ordering; simple_one_for_one caveats; auto_shutdown)

**Intensity/period:**
- `intensity` defaults to `1`; `period` defaults to `5`.
- If more than `MaxR` (intensity) restarts occur within `MaxT` (period) seconds,
  the supervisor terminates all child processes and then itself, with exit
  reason `shutdown`.

**Start/stop ordering:**
- Children started left-to-right in child spec list order.
- On termination, children terminated in reversed start order (right-to-left).
- `start_link/2,3` does not return until `Module:init/1` returns AND all child
  processes have been started (synchronized startup).
- If any child start function fails, supervisor terminates all already-started
  children with reason `shutdown` then itself, returns
  `{error, {shutdown, Reason}}`.

**simple_one_for_one caveats:**
- `delete_child/2` and `restart_child/2` invalid → `{error,simple_one_for_one}`.
- `terminate_child/2` requires pid, not child spec id.
- `init/1` must return exactly one child spec (id ignored); no children started
  during init.
- `start_child/2` takes `ExtraArgs` list, appended to start args.
- Shutdown is asynchronous (parallel cleanup, undefined order).
- Never hibernates by default (children come and go at high rates).
- `which_children/1` returns `Id = undefined` for each child.
- `count_children/1` `active` count not verified for liveness (likely accurate
  unless supervisor heavily overloaded).

**auto_shutdown:**
- `auto_shutdown() :: never | any_significant | all_significant`. Default `never`.
- `never`: auto shutdown disabled. Child specs with `significant=true` are
  invalid and rejected.
- `any_significant`: supervisor shuts down (reason `shutdown`) when ANY
  significant child terminates — i.e. a `transient` significant child terminates
  normally, or a `temporary` significant child terminates normally or abnormally.
- `all_significant`: supervisor shuts down when ALL significant children have
  terminated (last active significant child terminates). Same rules as
  `any_significant`.
- `significant=true` is invalid when restart type is `permanent`.
- Feature since OTP 24.0; compiles on older OTP but leaks processes (auto
  shutdowns won't happen) — implementors must take precautions.

**hibernate_after (OTP 28.0+):**
- `simple_one_for_one` never hibernates by default.
- Other strategies default to hibernating after inactivity; finetune via
  `hibernate_after` flag (e.g. when supervisor regularly queried via
  `which_children/1`).

## Verbatim quotes

> "When the supervisor is started, the child processes are started in order from
> left to right according to this list. When the supervisor is going to
> terminate, it first terminates its child processes in reversed start order,
> from right to left."

> "if more than MaxR restarts occur within MaxT seconds, the supervisor
> terminates all child processes and then itself. The termination reason for the
> supervisor itself in that case will be shutdown. intensity defaults to 1 and
> period defaults to 5."

> "The start function must create and link to the child process, and must return
> {ok,Child} or {ok,Child,Info}, where Child is the pid of the child process and
> Info any term that is ignored by the supervisor."

> "A permanent child process is always restarted. A temporary child process is
> never restarted (even when the supervisor's restart strategy is rest_for_one
> or one_for_all and a sibling's death causes the temporary process to be
> terminated). A transient child process is restarted only if it terminates
> abnormally, that is, with another exit reason than normal, shutdown, or
> {shutdown,Term}."

> "If the child process is another supervisor, the shutdown time must be set to
> infinity to give the subtree ample time to shut down."

> "Setting the shutdown time to anything other than infinity for a child of type
> supervisor can cause a race condition where the child in question unlinks its
> own children, but fails to terminate them before it is killed."

> "Notice that when the restart strategy is simple_one_for_one, the list of
> child specifications must be a list with one child specification only. (The
> child specification identifier is ignored.) No child process is then started
> during the initialization phase, but all children are assumed to be started
> dynamically using start_child/2."

> "As a simple_one_for_one supervisor can have many children, it shuts them all
> down asynchronously. This means that the children do their cleanup in parallel,
> and therefore the order in which they are stopped is not defined."

> "Calling this function from a (sub-)child process of the given supervisor will
> result in a deadlock which will last until either the shutdown timeout of the
> child or the timeout given to stop/3 has expired." (re: stop/3)

> "The automatic shutdown feature appeared in OTP 24.0, but applications using
> this feature will also compile and run with older OTP versions. However, such
> applications, when compiled with an OTP version that predates the appearance of
> the automatic shutdown feature, will leak processes because the automatic
> shutdowns they rely on will not happen."

## Version notes

- Page meta: OTP 29.0.2, stdlib 8.0.1.
- `hibernate_after` supervisor flag: available since OTP 28.0.
- `auto_shutdown` / `significant` children: since OTP 24.0.
- `check_childspecs/2`: since OTP 24.0.
- `count_children/1`: since OTP R13B04.
- `get_childspec/2`: since OTP 18.0.
- `which_child/2`: since OTP 28.0.
- `stop/1` and `stop/3`: since OTP 29.0.
- `exit_signal/2` used (not `exit/2`) in shutdown descriptions — reflects OTP 29
  terminology.

## Discovered links

### Relevant (crawl later)

- ../../system/sup_princ.html — Supervisor Behaviour (OTP Design Principles)
- ../../system/sup_princ.html#automatic-shutdown — Automatic Shutdown section
- ../../system/release_handling.html — Release Handling (OTP Design Principles)
- ../../system/appup_cookbook.html#sup — appup cookbook, supervisor section
- gen_server.html — gen_server module
- gen_statem.html — gen_statem module
- gen_event.html — gen_event module
- sys.html — sys module
- ../../apps/kernel/global.html — global module
- ../../apps/kernel/logger.html — logger module
- ../../apps/erts/erlang.html — erlang module (types: pid/0, term/0, module/0, atom/0, timeout/0, boolean/0, non_neg_integer/0, pos_integer/0, any/0, node/0; functions: apply/3, register/2, exit_signal/2)
- proplists.html#t:proplist/0 — proplists (property list type)

### Skipped

- supervisor.md (markdown source mirror of this page)
- supervisor.html (self / internal anchors: #significant_child, #auto_shutdown, #restart, #child_spec, #sup_flags, #supervision_princ)
- ../../index.html (doc index root)
- https://github.com/erlang/otp/...supervisor.erl#L... (source line anchors, ~30 links)
- /assets/css/algolia-typeahead.css, dist/html-erlang-*.css (stylesheets)
- llms.txt, stdlib.epub (download artifacts)
- https://github.com/elixir-lang/ex_doc (doc generator)
- https://erlang.org, https://www.ericsson.com (corporate)
