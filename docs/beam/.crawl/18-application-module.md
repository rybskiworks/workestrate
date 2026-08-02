# Crawl: kernel/application.html
- seed_url: https://www.erlang.org/doc/apps/kernel/application.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/application.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel v11.0.2)
- feeds_docs: applications.md

## Purpose
The `application` module (kernel) is the generic OTP application controller interface. In OTP, an "application" is a component implementing specific functionality that can be started/stopped as a unit and reused across systems. This module interacts with the **application controller** — a process started at every Erlang runtime system — to control applications (start/stop/load/unload) and to access application information (configuration parameters, spec keys).

An application is defined by an **application specification**, normally located in an application resource file `Application.app`. The module also acts as a **behaviour** for applications implemented per OTP design principles as a supervision tree: the callback module exports a predefined set of functions defining how to start/stop the tree.

See Also: OTP Design Principles, kernel app, `app` (resource file spec).

## Callback signatures (start/2, stop/1, prep_stop/1, start_phase/3, config_change/3) + StartType values

Types:
```
-type start_type() :: normal | {takeover, Node :: node()} | {failover, Node :: node()}.
-type restart_type() :: permanent | transient | temporary.
```

### start/2 (callback, required)
```
-callback start(StartType :: start_type(), StartArgs :: term()) ->
        {ok, pid()} | {ok, pid(), State :: term()} | {error, Reason :: term()}.
```
Called whenever an application is started using `start/1,2`; must start the processes of the application (for a supervision-tree app, start the top supervisor). Returns `{ok,Pid}` or `{ok,Pid,State}` where `Pid` is the top supervisor pid and `State` is any term (defaults to `[]` if omitted). `State` is later passed to `Module:prep_stop/1`.

**StartType values:**
- `normal` — a normal startup.
- `normal` — also when the app is distributed and started at the current node because of a **failover** from another node, AND `start_phases == undefined`.
- `{takeover, Node}` — distributed app started at current node because of a takeover from `Node` (via `takeover/2` or because current node has higher priority than `Node`).
- `{failover, Node}` — distributed app started at current node because of a failover from `Node`, AND `start_phases /= undefined`.

`StartArgs` is the `StartArgs` defined by the application spec key `mod`.

### stop/1 (callback, required)
```
-callback stop(State :: term()) -> term().
```
Called whenever an application has stopped; opposite of `start/2`; do any cleanup. Return value is ignored. `State` is the return value of `Module:prep_stop/1` if it exists, otherwise taken from `Module:start/2`.

### prep_stop/1 (callback, optional)
```
-callback prep_stop(State) -> NewState when State :: term(), NewState :: term().
```
Called when an application is about to be stopped, **before** shutting down the processes. `State` is the state returned from `Module:start/2`, or `[]` if no state was returned. `NewState` is any term passed to `Module:stop/1`. If not defined, processes are terminated and then `Module:stop(State)` is called.

### start_phase/3 (callback, optional)
```
-callback start_phase(Phase, StartType, PhaseArgs) -> ok | {error, Reason}
              when Phase :: atom(), StartType :: start_type(),
                   PhaseArgs :: term(), Reason :: term().
```
For starting an application with included applications when synchronization is needed between processes in different applications during startup. Start phases are defined by spec key `start_phases == [{Phase,PhaseArgs}]`. For included applications, the set of phases must be a **subset** of the phases defined for the including application. Called for each start phase (defined for the primary application) for the primary app and all included apps for which the phase is defined. `StartType` as in `start/2`.

### config_change/3 (callback, optional)
```
-callback config_change(Changed, New, Removed) -> ok
              when Changed :: [{Par, Val}], New :: [{Par, Val}],
                   Removed :: [Par], Par :: atom(), Val :: term().
```
Called by an application **after a code replacement**, if configuration parameters have changed.
- `Changed` — parameter-value tuples for all config params with changed values.
- `New` — parameter-value tuples for all added config params.
- `Removed` — list of all removed parameters.

## Environment API (get_env/set_env/etc + precedence rules)

### get_env/1,2,3
```
get_env(Par) -> undefined | {ok, Val}                              % equiv get_env(get_application(), Par)
get_env(Application, Par) -> undefined | {ok, Val}
get_env(Application, Par, Def) -> Val          (since OTP R16B)     % returns Def if Par missing
```
Returns the value of config parameter `Par` for `Application`. Returns `undefined` if: app not loaded; param does not exist; calling process belongs to no application.

### get_all_env/0,1
```
get_all_env() -> Env                       % equiv get_all_env(get_application())
get_all_env(Application) -> Env            % Env :: [{Par, Val}]
```
Returns config parameters and their values for `Application`. Returns `[]` if app not loaded or calling process belongs to no application.

### set_env/1,2,3,4
```
set_env(Config) -> ok                     (since OTP 21.3)   % equiv set_env(Config, [])
set_env(Config, Opts) -> ok               (since OTP 21.3)   % multiple apps at once
set_env(Application, Par, Val) -> ok                         % equiv set_env(App,Par,Val,[])
set_env(Application, Par, Val, Opts) -> ok
  Config :: [{Application, Env}], Env :: [{Par, Val}]
  Opts  :: [{timeout, timeout()} | {persistent, boolean()}]
```
Sets config parameter `Par` for `Application`. Uses standard gen_server timeout (5000 ms); `timeout` option overrides. `set_env/2` validates Config before setting and is more efficient than calling `set_env/4` per app.

**Precedence / persistence rules:**
- If `set_env/4` is called **before** the application is loaded, the application environment values specified in file `Application.app` **override** the ones previously set. Same on application reload.
- Option `{persistent, true}` guarantees parameters set with `set_env` are **not overridden** by those defined in the application resource file on load. Persistent values stick after the app is loaded and on reload.

**Warning (verbatim):** Use this function only if you know what you are doing, that is, on your own applications. Careless use can put the application in a weird, inconsistent, and malfunctioning state.

### unset_env/2,3
```
unset_env(Application, Par) -> ok                          % equiv unset_env(App,Par,[])
unset_env(Application, Par, Opts) -> ok
  Opts :: [{timeout, timeout()} | {persistent, boolean()}]
```
Removes config parameter `Par` and its value. Supports `persistent` option (see `set_env/4`). Same warning as `set_env`.

### get_application/0,1
```
get_application() -> undefined | {ok, Application}          % equiv get_application(self())
get_application(PidOrModule) -> undefined | {ok, Application}
  PidOrModule :: Pid | Module
```
Returns name of the application to which process `Pid` or module `Module` belongs. Returns `undefined` if process/module does not exist or belongs to no application.

### Env precedence summary
1. Command-line flags `-App Par Val` (set at system startup, highest).
2. `Application.app` resource file `env` section — overrides `set_env` values set **before** load.
3. `set_env` with `{persistent, true}` — sticks across load/reload, not overridden by `.app` file.
4. `set_env` (non-persistent) — overridden by `.app` file on load/reload.

## Lifecycle API (ensure_all_started/ensure_started/load/unload/which_applications/spec/info)

### start/1,2
```
start(Application) -> ok | {error, Reason}                  % equiv start(Application, temporary)
start(Application, Type) -> ok | {error, Reason}
  Type :: restart_type()  % permanent | transient | temporary (default temporary)
```
Starts `Application`. If not loaded, controller loads it via `load/1`. Ensures included applications are loaded but does NOT start them. Checks spec key `applications` to ensure all required apps are running; returns `{error,{not_started,App}}` if a non-optional dependency is missing. Does NOT attempt to start listed dependencies (use `ensure_all_started` for that). Creates an **application master** (group leader of all app processes); master calls `Module:start/2` from spec key `mod`.

**Type semantics on termination:**
- `permanent` terminates → all other applications and the entire Erlang node are terminated.
- `transient` terminates with `Reason == normal` → reported, no other apps terminated; abnormally → all apps + node terminated.
- `temporary` terminates → reported, no other apps terminated.
- Any app can always be stopped explicitly via `stop/1` regardless of type.
- `transient` is of little practical use: when a supervision tree terminates, reason is `shutdown`, not `normal`.

### stop/1
```
stop(Application) -> ok | {error, Reason}
```
Stops `Application`. Application master calls `Module:prep_stop/1` (if defined), then tells top supervisor to shut down → entire supervision tree (including included apps) terminated in **reversed start order**. After shutdown, master calls `Module:stop/1`. App master terminates last; all processes with app master as group leader are also terminated. App remains **loaded** after stop. For distributed apps, `stop/1` must be called on all nodes where it can execute; the call on the executing node stops it; app is not moved between nodes.

### ensure_all_started/1,2,3
```
ensure_all_started(Applications) -> {ok, Started} | {error, Reason}              (since R16B02) % equiv ... temporary, serial
ensure_all_started(Applications, Type) -> {ok, Started} | {error, AppReason}     (since R16B02) % equiv ... serial
ensure_all_started(Applications, Type, Mode) -> {ok, Started} | {error, AppReason}  (since OTP 26.0)
  Applications :: atom() | [atom()]
  Type :: restart_type(), Mode :: serial | concurrent
  Started :: [atom()], AppReason :: {atom(), term()}
```
Equivalent to calling `start/1,2` repeatedly on all not-yet-started dependencies of each application. Optional dependencies are loaded and started if available. `Mode`: `serial` (one at a time, default) or `concurrent` (dependency graph built, leaves started concurrently and recursively). In both modes, no assertion about start order. Returns `{ok, AppNames}` (already-started apps omitted from list). On error returns `{error,{AppName,Reason}}`; applications started by the function are stopped to restore initial state.

### ensure_started/1,2
```
ensure_started(Application) -> ok | {error, Reason}          (since R16B01) % equiv start/1 but ok if already started
ensure_started(Application, Type) -> ok | {error, Reason}     (since R16B01)
```
Like `start/1,2` but returns `ok` for already-started applications.

### load/1,2
```
load(AppDescr) -> ok | {error, Reason}                       % equiv load(AppDescr, [])
load(AppDescr, Distributed) -> ok | {error, Reason}
  AppDescr :: Application | AppSpec :: application_spec()
  Distributed :: {Application, Nodes} | {Application, Time, Nodes} | default
  Nodes :: [node() | tuple_of(node())], Time :: pos_integer()
```
Loads application specification into the application controller; also loads specs for included applications. Does **not** load Erlang object code. App can be specified by name (controller searches code path for `Application.app`) or directly as an `AppSpec` tuple.

If `Distributed == {Application,[Time,]Nodes}`, app becomes distributed (overrides Kernel config param `distributed`). `Time` (ms) is the wait before restarting on another node after a node crash (default 0). `Nodes` is priority list left-to-right; tuples group nodes of equal priority. Example: `Nodes = [cp1@cave, {cp2@cave, cp3@cave}]`. `Distributed == default` uses Kernel config param `distributed`.

### unload/1
```
unload(Application) -> ok | {error, Reason}
```
Unloads application spec from controller (and included apps). Does not purge Erlang object code.

### loaded_applications/0
```
loaded_applications() -> [{Application, Description, Vsn}]
```
Info about applications (and included apps) loaded via `load/1,2`. `Description`/`Vsn` from spec keys `description`/`vsn`.

### which_applications/0,1
```
which_applications() -> [{Application, Description, Vsn}]    % equiv which_applications(5000)
which_applications(Timeout) -> [{Application, Description, Vsn}]
```
Currently running applications. `Timeout` for when controller is heavily loaded.

### spec/1, info/1
**Not present in OTP 29.0.2.** These functions do not appear in the current `application` module page. (The crawl task list mentioned them, but the page exposes `get_all_key/1` and `get_key/2` for spec-key access instead.)

### Additional functions present
- `get_key/1,2` — value of application spec key `Key` for `Application` (`undefined` if app not loaded / key missing / process belongs to no app).
- `get_all_key/0,1` — all spec keys and values (`undefined` if not loaded, `[]` if process belongs to no app).
- `get_supervisor/1` (since OTP 26.0) — `{ok,Pid}` of root supervisor, `undefined` if app doesn't exist or no callback module.
- `permit/2` — `permit(Application, Permission)` changes permission to run at current node; app must be loaded. Permission `false` on a loaded-but-not-started app: `start` returns `ok` but app not started until `true`. On a running app set `false` → stopped; set `true` later → restarted. Distributed: `false` moves app to another node per distribution config. Does not return until started/stopped/moved. Default permission `true` on all nodes; configurable via Kernel param `permissions`.
- `takeover/2` — `takeover(Application, Type)` takes over a distributed app running at another node `Node`; restarts locally via `Module:start({takeover,Node},StartArgs)`. Old instance not stopped until new `start/2` and `start_phase/3` complete → two instances run simultaneously during takeover (for data transfer). Old instance cannot be stopped entirely; top supervisor must stay alive.
- `start_type/0` — `StartType | undefined | local`; called by an app process at startup to determine start type. `local` if only parts restarted (by supervisor) or called outside startup. `undefined` if process belongs to no application.

## Strict rules (env precedence; application controller; distributed applications)

1. **Application controller** is a single process started at every Erlang runtime; all `application` module calls go through it. Heavily loaded controller → use `timeout` options on `set_env`/`unset_env`/`which_applications`.
2. **Env precedence:** `.app` resource file `env` overrides non-persistent `set_env` values set before load/reload. `{persistent, true}` on `set_env`/`unset_env` makes values stick across load/reload and not be overridden by `.app`. Command-line `-App Par Val` flags (set at system startup) take highest precedence (per OTP Design Principles; not restated on this page but is the canonical rule).
3. **`set_env`/`unset_env` warning:** use only on your own applications; careless use puts the app in a weird, inconsistent, malfunctioning state.
4. **`start/2` does not start dependencies** listed in `applications` — only checks they run. Use `ensure_all_started` to recursively start dependencies.
5. **Included applications** are loaded by `load`/`start` but NOT started automatically; that is the including app's responsibility.
6. **Stop order:** supervision tree (including included apps) terminated in **reversed start order**; app master terminates last; all processes with app master as group leader are killed.
7. **App remains loaded after `stop/1`** — must `unload/1` to remove spec.
8. **Distributed applications:** `load/2` `Distributed` arg overrides Kernel `distributed` config. `stop/1` on distributed app must be called on all nodes; executing-node call stops it; app is NOT moved between nodes by `stop`. `takeover/2` runs two instances simultaneously during handover.
9. **`permit/2`** blocks until app is started/stopped/moved; may return `ok` without starting if dependencies not yet started.
10. **`start_phases`** for included apps must be a subset of the including app's phases.
11. **`transient` restart type** is of little practical use (supervision tree termination reason is `shutdown`, not `normal`).

## Verbatim quotes

- "In OTP, application denotes a component implementing some specific functionality, that can be started and stopped as a unit, and that can be reused in other systems."
- "This module interacts with application controller, a process started at every Erlang runtime system."
- "If a permanent application terminates, all other applications and the entire Erlang node are also terminated."
- "If set_env/4 is called before the application is loaded, the application environment values specified in file Application.app override the ones previously set. This is also true for application reloads."
- "Option persistent can be set to true to guarantee that parameters set with set_env/4 are not overridden by those defined in the application resource file on load."
- "Use this function only if you know what you are doing, that is, on your own applications. ... Careless use of this function can put the application in a weird, inconsistent, and malfunctioning state."
- "This means that the entire supervision tree, including included applications, is terminated in reversed start order."
- "When stopped, the application is still loaded."
- "Thus, two instances of the application run simultaneously during the takeover, so that data can be transferred from the old to the new instance."
- "Notice that the transient type is of little practical use, because when a supervision tree terminates, the reason is set to shutdown, not normal."
- "In both modes, no assertion can be made about the order the applications are started."

## Version notes
- Page built with ExDoc v0.40.3; Erlang OTP 29.0.2, kernel v11.0.2.
- `ensure_all_started/1,2` since OTP R16B02; `/3` (Mode) since OTP 26.0.
- `ensure_started/1,2` since OTP R16B01.
- `get_env/3` since OTP R16B.
- `set_env/1,2` since OTP 21.3.
- `get_supervisor/1` since OTP 26.0.
- `spec/1` and `info/1` are NOT present in OTP 29 (task list referenced them but the current module exposes `get_key/2` and `get_all_key/1` instead).
- Source: https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/application.erl

## Discovered links

### Relevant (crawl later)
- app.html — application resource file (.app) specification — https://www.erlang.org/doc/apps/kernel/app.html

### Skipped
- ../../system/design_principles.html — https://www.erlang.org/doc/system/design_principles.html (already crawled: 01-design-principles.md)
- ../../apps/stdlib/supervisor.html — https://www.erlang.org/doc/apps/stdlib/supervisor.html (already crawled: 13-supervisor-module.md)
- ../../apps/erts/erlang.html#t:* — primitive type references (erts), out of crawl scope
- application.md — ExDoc "Copy Markdown" source artifact
- kernel_app.html — kernel app overview (peripheral)
- GitHub source line anchors (https://github.com/erlang/otp/...) — source, not docs
- llms.txt, kernel.epub — packaging artifacts
