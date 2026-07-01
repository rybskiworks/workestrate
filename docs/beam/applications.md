# Applications

## Purpose

This document defines BEAM/OTP guidance for Erlang/OTP applications: the `.app` resource file, the `application` callback module, the application controller, application environment, start phases, included applications, and restart types. Future agents who write, review, refactor, debug, or validate BEAM application code should follow these rules so behavior is consistent, predictable, and aligned with the official Erlang/OTP documentation.

This doc is BEAM-common, not Elixir-specific. Examples are in Erlang and use the `application` module from `kernel`.

## Sources used

- `.crawl/05-applications.md` — https://www.erlang.org/doc/system/applications.html (PRIMARY — application concepts, structure, lifecycle, controller, env, start types)
- `.crawl/18-application-module.md` — https://www.erlang.org/doc/apps/kernel/application.html (PRIMARY — `application` module API, callback typespecs, env precedence, lifecycle functions)
- `.crawl/23-app-resource.md` — https://www.erlang.org/doc/apps/kernel/app.html (PRIMARY — `.app` resource file spec, full option-key list, defaults)

This page reflects Erlang/OTP 29.0.2 semantics. The `application` module and `app(4)` resource file format live in the `kernel` application.

## Core guidance

### What an application is

From `applications.html`:

> "After creating code to implement a specific functionality, you might consider transforming it into an *application* — a component that can be started and stopped as a unit, as well as reused in other systems."

From `application.html`:

> "In OTP, application denotes a component implementing some specific functionality, that can be started and stopped as a unit, and that can be reused in other systems."

An OTP application is the unit of code packaging, dependency tracking, and lifecycle management. It has a name (an atom), a resource file (`Application.app`), and optionally a callback module implementing the `application` behaviour to start/stop a supervision tree.

### The `.app` resource file (full term format)

From `app.html`:

> "The application resource file specifies the resources an application uses, and how the application is started. There must always be one application resource file called `Application.app` for each application `Application` in the system."

> "The file is read by the application controller when an application is loaded/started. It is also used by the functions in `systools`, for example when generating start scripts."

> "The file must contain a single Erlang term, which is called an application specification."

The full term format (verbatim from `app.html`):

```erlang
{application, Application,
  [{description,  Description},
   {id,           Id},
   {vsn,          Vsn},
   {modules,      Modules},
   {maxP,         MaxP},
   {maxT,         MaxT},
   {registered,   Names},
   {included_applications, Apps},
   {optional_applications, Apps},
   {applications, Apps},
   {env,          Env},
   {mod,          Start},
   {start_phases, Phases},
   {runtime_dependencies, RTDeps}]}.
```

Type/default table (verbatim from `app.html`):

```
Value                  Default
------                 -------
Application  atom()               -
Description  string()             ""
Id           string()             ""
Vsn          string()             ""
Modules      [Module]             []
MaxP         int()                infinity
MaxT         int()                infinity
Names        [Name]               []
Apps         [App]                []
Env          [{Par,Val}]          []
Start        {Module,StartArgs}   []
Phases       [{Phase,PhaseArgs}]  undefined
RTDeps       [ApplicationVersion] []

Module = Name = App = Par = Phase = atom()
Val = StartArgs = PhaseArgs = term()
ApplicationVersion = string()
```

### `.app` option keys (all keys, meaning, defaults)

- **`description`** (string, default `""`) — One-line description of the application.
- **`id`** (string, default `""`) — Product identification, or similar.
- **`vsn`** (string, default `""`) — Version of the application.
- **`modules`** (`[Module]`, default `[]`) — All modules introduced by this application. `systools` uses this list when generating start scripts and tar files. A module can only be defined in one application.
- **`maxP`** (int, default `infinity`) — *Deprecated — is ignored.* Maximum number of processes allowed in the application.
- **`maxT`** (int, default `infinity`) — Maximum time, in milliseconds, that the application is allowed to run. After the specified time, the application terminates automatically.
- **`registered`** (`[Name]`, default `[]`) — All names of registered processes started in this application. `systools` uses this list to detect name clashes between different applications.
- **`included_applications`** (`[App]`, default `[]`) — All applications included by this application. When this application is started, all included applications are loaded automatically, but not started, by the application controller. It is assumed that the top-most supervisor of the included application is started by a supervisor of this application.
- **`optional_applications`** (`[App]`, default `[]`) — A list of applications that are optional. To get auto-start-before behavior when an optional dependency is available, list the app in *both* `applications` and `optional_applications`.
- **`applications`** (`[App]`, default `[]`) — All applications that must be started before this application. `systools` uses this list to generate correct start scripts. All applications have dependencies to (at least) Kernel and STDLIB.
- **`env`** (`[{Par,Val}]`, default `[]`) — Configuration parameters used by the application. Retrieved via `application:get_env/1,2`. Overridable by `config(4)` files and `erl` command-line flags.
- **`mod`** (`{Module,StartArgs}`, default `[]`) — Specifies the application callback module and a start argument. Necessary for an application implemented as a supervision tree; can be omitted for process-less code libraries (e.g. STDLIB).
- **`start_phases`** (`[{Phase,PhaseArgs}]`, default `undefined`) — A list of start phases and corresponding start arguments. If present, the application master calls `Module:start_phase(Phase,Type,PhaseArgs)` for each start phase in addition to `Module:start/2`. For included applications, the set of start phases must be a *subset* of the phases defined for the primary application.
- **`runtime_dependencies`** (`[ApplicationVersion]`, default `[]`) — Minimum-version source-code requirements (e.g. `"kernel-3.0"`). A larger version than specified satisfies the requirement.

### Required vs optional keys

From `app.html`:

> "For the application controller, all keys are optional. The respective default values are used for any omitted keys."

> "The functions in `systools` require more information. If they are used, the following keys are mandatory: description, vsn, modules, registered, applications. The other keys are ignored by `systools`."

### The application callback module

From `applications.html`:

> "How to start and stop the code for the application, including its supervision tree, is described by two callback functions: `start(StartType, StartArgs) -> {ok, Pid} | {ok, Pid, State}` `stop(State)`"

The full callback set (from `application.html`):

```erlang
%% Required
-callback start(StartType :: start_type(), StartArgs :: term()) ->
        {ok, pid()} | {ok, pid(), State :: term()} | {error, Reason :: term()}.
-callback stop(State :: term()) -> term().

%% Optional
-callback prep_stop(State) -> NewState.
-callback start_phase(Phase, StartType, PhaseArgs) -> ok | {error, Reason}.
-callback config_change(Changed, New, Removed) -> ok.
```

Types:

```erlang
-type start_type() :: normal | {takeover, Node :: node()} | {failover, Node :: node()}.
-type restart_type() :: permanent | transient | temporary.
```

- **`start/2`** (required) — Called when starting the application; must create the supervision tree by starting the top supervisor. Returns `{ok,Pid}` or `{ok,Pid,State}`. `State` defaults to `[]` and is passed to `prep_stop/1` (or `stop/1` if no `prep_stop/1`).
- **`stop/1`** (required) — Called *after* the application has been stopped (i.e. after the supervision tree has been shut down automatically); does cleanup only. Return value is ignored.
- **`prep_stop/1`** (optional) — Called when an application is about to be stopped, *before* shutting down the processes. `State` is the state from `start/2` (or `[]`). `NewState` is passed to `stop/1`.
- **`start_phase/3`** (optional) — For synchronizing startup of an application and its included applications. Called for each start phase defined for the primary application, for the primary app and all included apps for which the phase is defined.
- **`config_change/3`** (optional) — Called by an application *after a code replacement*, if configuration parameters have changed. `Changed`/`New` are `{Par,Val}` lists; `Removed` is a list of `Par`.

### StartType values

From `applications.html`:

> "`StartType` is usually the atom `normal`. It has other values only in the case of a takeover or failover; see Distributed Applications."

From `application.html`:
- `normal` — a normal startup; also when a distributed app is started at the current node because of a **failover** from another node AND `start_phases == undefined`.
- `{takeover, Node}` — distributed app started at current node because of a takeover from `Node`.
- `{failover, Node}` — distributed app started at current node because of a failover from `Node`, AND `start_phases /= undefined`.

### Application controller and application master

From `applications.html`:

> "When an Erlang runtime system is started, a number of processes are started as part of the Kernel application. One of these processes is the *application controller* process, registered as `application_controller`. All operations on applications are coordinated by the application controller."

On start, the controller loads the app (if not loaded), checks the `applications` key to ensure dependencies are running, then creates an **application master**. The application master establishes itself as group leader of all processes in the application and forwards I/O to the previous group leader. This is what supports `application:get_application/0`, `application:get_env/1`, and clean termination of all processes belonging to the application on stop.

> "The application master starts the application by calling the application callback function `start/2` in the module with the start argument defined by the `mod` key in the `.app` file."

> "The application master stops the application by telling the top supervisor to shut down. The top supervisor tells all its child processes to shut down, and so on; the entire tree is terminated in reverse start order. The application master then calls the application callback function `stop/1`."

Loading/unloading an application does **not** load/unload the code used by it; code loading is handled by the code server.

### Application environment API and precedence

Configuration parameters are `{Par,Val}` tuples under the `env` key. `Par` is an atom; `Val` is any term. Retrieved via `application:get_env(App, Par)`.

Key functions (from `application.html`):
- `get_env(Par)` / `get_env(Application, Par)` / `get_env(Application, Par, Def)` — returns `undefined | {ok, Val}` (or `Def` for /3).
- `get_all_env()` / `get_all_env(Application)` — returns `[{Par,Val}]`.
- `set_env(Application, Par, Val)` / `set_env(Application, Par, Val, Opts)` — sets a parameter. `Opts` includes `{persistent, boolean()}`.
- `unset_env(Application, Par)` / `unset_env(Application, Par, Opts)`.
- `get_application()` / `get_application(PidOrModule)`.

**Env precedence** (lowest → highest):

1. `.app` resource file `env` section (baseline).
2. `set_env` (non-persistent) — overridden by `.app` file on load/reload.
3. `set_env` with `{persistent, true}` — sticks across load/reload; not overridden by `.app`.
4. `config(4)` system config file (`-config Name`) — overrides `.app` `env`.
5. Command-line flags `-App Par Val` — highest precedence (set at system startup).

> "If `set_env/4` is called before the application is loaded, the application environment values specified in file `Application.app` override the ones previously set. This is also true for application reloads."

> "Option `persistent` can be set to `true` to guarantee that parameters set with `set_env/4` are not overridden by those defined in the application resource file on load."

**Warning** (verbatim):

> "Use this function only if you know what you are doing, that is, on your own applications. Careless use can put the application in a weird, inconsistent, and malfunctioning state."

### Restart types (start type semantics)

From `applications.html`, `application:start(Application, Type)` where `Type` is `temporary` (default), `permanent`, or `transient`:

- **`permanent`** — "If a permanent application terminates, all other applications and the runtime system are also terminated."
- **`transient`** — "If a transient application terminates with reason `normal`, this is reported but no other applications are terminated. If a transient application terminates abnormally, that is with any other reason than `normal`, all other applications and the runtime system are also terminated."
- **`temporary`** — "If a temporary application terminates, this is reported but no other applications are terminated."

> "An application can always be stopped explicitly by calling `application:stop/1`. Regardless of the mode, no other applications are affected."

> "The transient mode is of little practical use, since when a supervision tree terminates, the reason is set to `shutdown`, not `normal`."

### Lifecycle notes

- `start/2` does **not** start dependencies listed in `applications` — only checks they run. Use `ensure_all_started` to recursively start dependencies.
- App remains **loaded** after `stop/1` — must `unload/1` to remove the spec.
- Stop order: supervision tree (including included apps) terminated in **reversed start order**; app master terminates last.
- `takeover/2` runs two instances simultaneously during handover for data transfer.

## Practical rules

1. Name the `.app` file `Application.app` (where `Application` is the application name atom); place it in `ebin/`.
2. Include `mod` for supervision-tree apps; omit for library apps.
3. List every module introduced by the app in `modules`; a module must only be in one application.
4. Always list `kernel` and `stdlib` in `applications` (implicit baseline dependencies).
5. Use `application:start/2` with an explicit `Type`; default is `temporary`.
6. Prefer `application:get_env/3` with a default over `get_env/2` to avoid `undefined` handling.
7. Avoid `set_env`/`unset_env` on applications you do not own.
8. Use `ensure_all_started/1,2,3` rather than manually starting dependencies.

## Review checklist

- [ ] `.app` file named `Application.app` and located in `ebin/`.
- [ ] All `systools`-mandatory keys present: `description`, `vsn`, `modules`, `registered`, `applications`.
- [ ] `modules` list is complete and has no overlap with other applications.
- [ ] `registered` lists all registered process names.
- [ ] `applications` includes `kernel` and `stdlib`.
- [ ] `mod` present for supervision-tree apps; callback module exports `start/2` and `stop/1`.
- [ ] `start/2` returns `{ok, Pid}` or `{ok, Pid, State}`.
- [ ] `stop/1` does only cleanup (tree shutdown is automatic).
- [ ] Start type chosen deliberately (`temporary` vs `permanent`).

## Implementation checklist

- [ ] Callback module declares `-behaviour(application)` and exports `start/2`, `stop/1`.
- [ ] `start/2` starts the top supervisor via `supervisor:start_link/2,3`.
- [ ] Optional callbacks (`prep_stop/1`, `start_phase/3`, `config_change/3`) implemented only when needed.
- [ ] `env` defaults in `.app` are safe to run with; overrides via `sys.config` or CLI flags.
- [ ] `start_phases` for included apps is a subset of the primary app's phases.

## Runtime / debugging checklist

- [ ] `application:loaded_applications/0` — loaded apps.
- [ ] `application:which_applications/0` — running apps.
- [ ] `application:get_env(App, Par)` — inspect config values.
- [ ] `application:get_all_env(App)` — all config for an app.
- [ ] `application:get_application/0` — which app the calling process belongs to.
- [ ] `application:get_key(App, Key)` — inspect spec keys.
- [ ] `application:get_supervisor(App)` — root supervisor pid (OTP 26+).

## Validation hooks

- Verify `.app` parses as a single Erlang term: `file:consult("ebin/my_app.app")`.
- Verify app starts: `application:start(my_app)` returns `ok`.
- Verify env precedence: set in `.app`, override in `sys.config`, override on CLI; confirm `get_env` returns expected value at each layer.
- Verify clean stop: `application:stop(my_app)` returns `ok` and `which_applications/0` no longer lists it.

## Examples

### Minimal `.app` for a library application

```erlang
{application, libapp, []}.
```

### Minimal `.app` for a supervision-tree application

```erlang
{application, ch_app,
 [{mod, {ch_app,[]}}]}.
```

This causes `ch_app:start(normal, [])` on start and `ch_app:stop([])` on stop.

### Full `.app` (systools-required keys + env)

```erlang
{application, ch_app,
 [{description, "Channel allocator"},
  {vsn, "1"},
  {modules, [ch_app, ch_sup, ch3]},
  {registered, [ch3]},
  {applications, [kernel, stdlib, sasl]},
  {mod, {ch_app,[]}},
  {env, [{file, "/usr/local/log"}]}
 ]}.
```

### Application callback module

```erlang
-module(ch_app).
-behaviour(application).

-export([start/2, stop/1]).

start(_Type, _Args) ->
    ch_sup:start_link().

stop(_State) ->
    ok.
```

### Loading and starting

```erlang
1> application:load(ch_app).
ok
2> application:loaded_applications().
[{kernel,"ERTS  CXC 138 10","2.8.1.3"},
 {stdlib,"ERTS  CXC 138 10","1.11.4.3"},
 {ch_app,"Channel allocator","1"}]
3> application:start(ch_app).
ok
4> application:get_env(ch_app, file).
{ok,"/usr/local/log"}
5> application:stop(ch_app).
ok
```

### System config file override

`test.config`:
```erlang
[{ch_app, [{file, "testlog"}]}].
```
Run `erl -config test` → `application:get_env(ch_app, file)` returns `{ok,"testlog"}`.

### Command-line override

```
% erl -ch_app file '"testlog"'
```

## Common mistakes

1. **Omitting `mod` for a supervision-tree app.** The controller cannot start it. Fix: add `{mod, {Module, StartArgs}}`.
2. **Listing a module in two applications' `modules`.** Fix: a module must only be in one application.
3. **Forgetting `kernel`/`stdlib` in `applications`.** Fix: all apps depend on at least Kernel and STDLIB.
4. **Using `transient` expecting clean exits to not restart.** Fix: supervision tree termination reason is `shutdown`, not `normal`, so `transient` is of little practical use.
5. **Doing real work in `stop/1`.** Fix: `stop/1` is called after the tree is already shut down; do only cleanup.
6. **Expecting `start/2` to start dependencies.** Fix: `start/2` only checks dependencies run; use `ensure_all_started`.
7. **Using `set_env` on apps you don't own.** Fix: heed the warning; careless use causes inconsistent state.
8. **Assuming `stop/1` unloads the app.** Fix: app remains loaded; call `unload/1` to remove the spec.
9. **Omitting systools-mandatory keys.** Fix: include `description`, `vsn`, `modules`, `registered`, `applications` when using `systools`.

## Strict vs contextual guidance

### Strict

- The `.app` file MUST be named `Application.app` and contain a single Erlang term.
- A module MUST only be defined in one application's `modules` list.
- `start/2` MUST return `{ok, Pid}` or `{ok, Pid, State}`.
- `stop/1` MUST NOT attempt to shut down the supervision tree (handled automatically).
- `start_phases` for included apps MUST be a subset of the primary app's phases.
- `maxP` is deprecated and ignored; do not rely on it.

### Convention

- Include all `systools`-mandatory keys even if not currently using `systools`.
- Use `temporary` as the default start type; use `permanent` only for critical apps whose failure should halt the node.
- Put env defaults in `.app`; override via `sys.config` or CLI flags rather than `set_env`.

### Contextual

- Whether to use `start_phases` for synchronized startup with included apps.
- Whether to implement `prep_stop/1` for pre-shutdown work.
- Whether to implement `config_change/3` for code-replacement config updates.
- Exact `env` parameter values and override strategy.

### Policy

- Default start type per application class (`temporary` vs `permanent`).
- Whether `set_env`/`unset_env` is permitted at runtime, and on which apps.
- Whether `included_applications` may be used, or whether flat dependency graphs are preferred.
- Naming conventions for `registered` process names.

## Policy decisions for individual repos

1. **Default start type.** Should ordinary apps default to `temporary`, with `permanent` reserved for critical infrastructure?
2. **`set_env` policy.** Is runtime `set_env` permitted, and only on owned apps? Is `{persistent, true}` required?
3. **`included_applications` usage.** Are included apps allowed, or should dependencies be explicit?
4. **`start_phases` usage.** Is the start-phases mechanism permitted, or should startup synchronization use other means?
5. **`env` override strategy.** Are overrides via `sys.config` only, or are CLI flags also used in production?
6. **`maxT` usage.** Is `maxT` (auto-termination after timeout) used for any apps, or always `infinity`?

## Related docs

- [releases](releases.md)
- [logger-and-config](logger-and-config.md)
- [supervision](supervision.md)
- [otp-behaviours](otp-behaviours.md)

## Related skills

- `beam-applications-releases`
