# Crawl: applications.html

- seed_url: https://www.erlang.org/doc/system/applications.html
- canonical_url: https://www.erlang.org/doc/system/applications.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2; major-vsn 29)
- feeds_docs: applications.md

## Purpose
Describes the OTP **application** concept: a component that can be started and
stopped as a unit and reused across systems. Covers how to define an application
via an application callback module + an application resource file (`.app`), the
directory structure for packaged applications, the application controller /
application master, loading/unloading, starting/stopping, configuration via the
`env` key and system config files, and application start types
(permanent/transient/temporary).

The page explicitly recommends reading it alongside `app` and `application` in
Kernel for full syntax/callback detail.

## Key concepts
- **Application** — a unit of code that can be started/stopped as a whole and
  reused in other systems.
- **Application callback module** — implements `start/2` and `stop/1`; describes
  how to start/stop the application (including its supervision tree).
- **Application specification / resource file (`.app`)** — term
  `{application, Application, [Opt1,...,OptN]}` placed in `Application.app`.
- **`mod` key** — `{Module, StartArgs}`; selects the callback module and the
  `StartArgs` passed to `start/2`.
- **Application controller** — process registered as `application_controller`,
  started as part of Kernel; coordinates all application operations.
- **Application master** — created per started application; becomes group leader
  of all processes in the application; starts the app by calling `start/2`;
  stops it by shutting down the top supervisor (reverse start order) then
  calling `stop/1`.
- **Configuration parameters** — `{Par,Val}` list under the `env` key; retrieved
  via `application:get_env(App, Par)`.
- **System configuration file** — `Name.config` (or `sys.config` for release
  handling); overrides `.app` `env` values; loaded via `-config Name`.
- **Command-line override** — `erl -ApplName Par1 Val1 ... ParN ValN` overrides
  both `.app` and system config values.
- **Start type** — `application:start(Application, Type)` where Type is
  `temporary` (default for `start/1`), `permanent`, or `transient`.
- **Directory structure** — `lib/Application-Vsn` per application; dev layout
  uses `src` (with `.app.src`), `priv`, `include`, `doc`, `test`; released
  layout uses `ebin` (with `.app`), `priv/lib`, `priv/bin`, `include`, `bin`,
  `doc`, `src`.

## Strict rules / invariants
- The `.app` file MUST be named `Application.app` (where `Application` is the
  application name atom).
- `.app` form: `{application, Application, [Opt1,...,OptN]}.` where each `Opt`
  is `{Key,Value}`. All keys are optional; defaults apply for omitted keys.
- A module must be included in **only one** application (the `modules` key).
- All applications have dependencies to **at least Kernel and STDLIB** (the
  `applications` key lists apps that must be started before this one).
- `start/2` must create the supervision tree by starting the top supervisor and
  return `{ok, Pid}` or `{ok, Pid, State}`; `State` defaults to `[]` and is
  passed as-is to `stop/1`.
- `stop/1` is called **after** the application has been stopped (i.e. after the
  supervision tree has been shut down automatically); it only does cleanup.
- A **library application** that cannot be started/stopped needs no callback
  module (minimal `.app`: `{application, libapp, []}.`).
- Loading/unloading an application does **not** load/unload the code used by it;
  code loading is handled by the code server.
- The application master being group leader is what lets
  `application:get_application/0`, `application:get_env/1`, and clean shutdown
  track which processes belong to the application.
- When stopping, the supervision tree is terminated in **reverse start order**.
- If release handling is used, exactly one system configuration file is to be
  used and it must be called `sys.config`.
- Directory names should not be capitalized; empty directories are encouraged
  to be omitted.
- In the released structure, `ebin` is **required** (holds `.beam` files and the
  `.app` file); `src`/`priv`/`include`/`bin`/`doc` are optional.
- In the dev structure, `src` is **required**; `priv`/`include` optional;
  `doc`(+subdirs)/`test` recommended.
- The version number should be omitted from the application directory name in a
  development environment (it is an artifact of the release step).
- Non-Erlang source dirs should be prefixed with the language name
  (e.g. `c_src`, `java_src`, `go_src`); `_src` suffix marks part of the
  application + compilation step; final build artifacts target `priv/lib` or
  `priv/bin`.

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

### Full `.app` (with systools-required keys)
```erlang
{application, ch_app,
 [{description, "Channel allocator"},
  {vsn, "1"},
  {modules, [ch_app, ch_sup, ch3]},
  {registered, [ch3]},
  {applications, [kernel, stdlib, sasl]},
  {mod, {ch_app,[]}}
 ]}.
```

### `.app` with configuration parameters (`env`)
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

### Application callback module (`ch_app`)
```erlang
-module(ch_app).
-behaviour(application).

-export([start/2, stop/1]).

start(_Type, _Args) ->
    ch_sup:start_link().

stop(_State) ->
    ok.
```

### Loading / unloading
```erlang
1> application:load(ch_app).
ok
2> application:loaded_applications().
[{kernel,"ERTS  CXC 138 10","2.8.1.3"},
 {stdlib,"ERTS  CXC 138 10","1.11.4.3"},
 {ch_app,"Channel allocator","1"}]
3> application:unload(ch_app).
ok
```

### Starting / stopping
```erlang
5> application:start(ch_app).
ok
6> application:which_applications().
[{kernel,"ERTS  CXC 138 10","2.8.1.3"},
 {stdlib,"ERTS  CXC 138 10","1.11.4.3"},
 {ch_app,"Channel allocator","1"}]
7> application:stop(ch_app).
ok
```

### Reading config
```erlang
1> application:start(ch_app).
ok
2> application:get_env(ch_app, file).
{ok,"/usr/local/log"}
```

### System config file `test.config`
```erlang
[{ch_app, [{file, "testlog"}]}].
```
Run: `erl -config test` → `application:get_env(ch_app, file).` returns
`{ok,"testlog"}`.

### Command-line override
```
% erl -ch_app file '"testlog"'
```
→ `application:get_env(ch_app, file).` returns `{ok,"testlog"}`.

### Dev directory structure
```
    ─ ${application}
      ├── doc
       │  ├── internal
       │  ├── examples
       │  └── src
      ├── include
      ├── priv
      ├── src
       │  └── ${application}.app.src
      └── test
```

### Released directory structure
```
    ─ ${application}-${version}
      ├── bin
      ├── doc
       │  ├── html
       │  ├── man[1-9]
       │  ├── pdf
       │  ├── internal
       │  └── examples
      ├── ebin
       │  └── ${application}.app
      ├── include
      ├── priv
       │  ├── lib
       │  └── bin
      └── src
```

## Behaviour / callback details

### `.app` resource file keys (covered on THIS page)
Form: `{application, Application, [{Key,Value}...]}.`

Keys explicitly described on this page (all optional; defaults shown):
- `description` — short description, a string. Default `""`.
- `vsn` — version number, a string. Default `""`.
- `modules` — all modules **introduced** by this application; `systools` uses
  this when generating boot scripts and tar files. A module must only be in one
  application. Default `[]`.
- `registered` — all names of registered processes in the application;
  `systools` uses this to detect name clashes between applications. Default `[]`.
- `applications` — all applications that must be started before this one;
  `systools` uses this to generate correct boot scripts. Default `[]`. Note:
  all applications depend on at least Kernel and STDLIB.
- `mod` — `{Module, StartArgs}`; defines the callback module and start
  argument. For `ch_app`: `{mod, {ch_app,[]}}` → `ch_app:start(normal, [])`.
- `env` — list of `{Par,Val}` configuration parameters; `Par` is an atom,
  `Val` is any term. Retrieved via `application:get_env(App, Par)`.

> NOTE on keys requested but NOT present on this page:
> The keys `id` and `start_phases` are **not** described on this page. They
> belong to the full `app` specification in the Kernel `app` module
> (linked as `../apps/kernel/app.html`). This page only enumerates the subset
> above and defers full syntax/contents to `app` in Kernel.

### Application callback module signatures (covered on THIS page)
The page describes only two callback functions:

```erlang
start(StartType, StartArgs) -> {ok, Pid} | {ok, Pid, State}
stop(State)
```

- `start/2` — called when starting the application; creates the supervision
  tree by starting the top supervisor; returns `{ok, Pid}` or
  `{ok, Pid, State}`. `State` defaults to `[]` and is passed as-is to `stop/1`.
- `stop/1` — called **after** the application has been stopped; does cleanup
  only. The actual stopping (shutting down the supervision tree) is handled
  automatically.

The example callback module declares:
```erlang
-behaviour(application).
-export([start/2, stop/1]).
```

> NOTE on callbacks requested but NOT present on this page:
> `prep_stop/1`, `start_phase/3`, and `config_change/3` are **not** described
> here. They belong to the `application` module in Kernel
> (`../apps/kernel/application.html`). This page only covers `start/2` and
> `stop/1`.

### StartType values
- `StartType` is usually the atom `normal`.
- It has other values only in the case of a **takeover** or **failover**
  (see Distributed Applications — `distributed_applications.html`).
- `StartArgs` is defined by the `mod` key in the `.app` file.

### Application start types (restart-type semantics)
Defined via `application:start(Application, Type)`:
- `application:start(Application)` ≡
  `application:start(Application, temporary)`.
- Types: `temporary` (default), `permanent`, `transient`.

Semantics:
- **permanent** — if a permanent application terminates, all other
  applications and the runtime system are also terminated.
- **transient** — if a transient application terminates with reason `normal`,
  this is reported but no other applications are terminated. If it terminates
  abnormally (any reason other than `normal`), all other applications and the
  runtime system are also terminated.
- **temporary** — if a temporary application terminates, this is reported but no
  other applications are terminated.

- An application can always be stopped explicitly via `application:stop/1`;
  regardless of mode, no other applications are affected.
- Transient mode is of little practical use, since when a supervision tree
  terminates the reason is set to `shutdown`, not `normal`.

### Application controller / application master
- The application controller is a process registered as
  `application_controller`, started as part of the Kernel application at
  runtime start. All application operations are coordinated by it.
- Use module `application` (Kernel) to load/unload/start/stop applications.
- On start, the controller loads the app (if not loaded) via
  `application:load/1`, checks the `applications` key to ensure dependencies
  are running, then creates an **application master**.
- The application master establishes itself as group leader of all processes in
  the application and forwards I/O to the previous group leader. This is what
  supports `application:get_application/0`, `application:get_env/1`, and clean
  termination of all processes belonging to the application on stop.
- The application master starts the application by calling `start/2` in the
  module named by the `mod` key.
- On stop, the application master tells the top supervisor to shut down; the
  tree terminates in reverse start order; then `stop/1` is called.

### Included applications / distributed applications
- This page links to `included_applications.html` (Included Applications) and
  `distributed_applications.html` (Distributed Applications) but does not detail
  them. Takeover/failover StartType values are covered there.

## Verbatim quotes

1. **Application Concept** — "After creating code to implement a specific
   functionality, you might consider transforming it into an *application* — a
   component that can be started and stopped as a unit, as well as reused in
   other systems."

2. **Application Concept (steps)** — "The steps to create an application are as
   follows: Create an application callback module that describes how the
   application is to be started and stopped. Create an *application
   specification* and place it in an application resource file. Among other
   things, this file specifies which modules the application consists of and
   the name of the callback module."

3. **Application Callback Module** — "How to start and stop the code for the
   application, including its supervision tree, is described by two callback
   functions: `start(StartType, StartArgs) -> {ok, Pid} | {ok, Pid, State}`
   `stop(State)`"

4. **start/2** — "`start/2` is called when starting the application and is to
   create the supervision tree by starting the top supervisor. It is expected
   to return the pid of the top supervisor and an optional term, `State`,
   which defaults to `[]`. This term is passed as is to `stop/1`."

5. **StartType** — "`StartType` is usually the atom `normal`. It has other
   values only in the case of a takeover or failover; see Distributed
   Applications."

6. **StartArgs** — "`StartArgs` is defined by the key `mod` in the application
   resource file."

7. **stop/1** — "`stop/1` is called *after* the application has been stopped and
   is to do any necessary cleaning up. The actual stopping of the application,
   that is, shutting down the supervision tree, is handled automatically as
   described in Starting and Stopping Applications."

8. **Library application** — "A library application that cannot be started or
   stopped does not need any application callback module."

9. **Application Resource File** — "To define an application, an *application
   specification* is created, which is put in an *application resource file*,
   or in short an `.app` file: `{application, Application, [Opt1,...,OptN]}.`"

10. **.app naming** — "`Application`, an atom, is the name of the application.
    The file must be named `Application.app`."

11. **.app keys optional** — "Each `Opt` is a tuple `{Key,Value}`, which defines
    a certain property of the application. All keys are optional. Default values
    are used for any omitted keys."

12. **mod key** — "The key `mod` defines the callback module and start argument
    of the application, in this case `ch_app` and `[]`, respectively. This means
    that the following is called when the application is to be started:
    `ch_app:start(normal, [])` The following is called when the application is
    stopped: `ch_app:stop([])`"

13. **systools-required keys** — "When using `systools`, the Erlang/OTP tools
    for packaging code (see Section Releases), the keys `description`, `vsn`,
    `modules`, `registered`, and `applications` are also to be specified"

14. **modules key** — "`modules` - All modules *introduced* by this application.
    `systools` uses this list when generating boot scripts and tar files. A
    module must only be included in one application. Defaults to `[]`."

15. **registered key** — "`registered` - All names of registered processes in
    the application. `systools` uses this list to detect name clashes between
    applications. Defaults to `[]`."

16. **applications key** — "`applications` - All applications that must be
    started before this application is started. `systools` uses this list to
    generate correct boot scripts. Defaults to `[]`. Notice that all
    applications have dependencies to at least Kernel and STDLIB."

17. **Directory structure** — "When packaging code using `systools`, the code
    for each application is placed in a separate directory,
    `lib/Application-Vsn`, where `Vsn` is the version number."

18. **Code server version selection** — "The code server (see module `code` in
    Kernel) automatically uses code from the directory with the highest version
    number, if more than one version of an application is present."

19. **Application controller** — "When an Erlang runtime system is started, a
    number of processes are started as part of the Kernel application. One of
    these processes is the *application controller* process, registered as
    `application_controller`. All operations on applications are coordinated by
    the application controller."

20. **Loading** — "Before an application can be started, it must be *loaded*.
    The application controller reads and stores the information from the `.app`
    file"

21. **Load/unload ≠ code load** — "Loading/unloading an application does not
    load/unload the code used by the application. Code loading is handled in the
    usual way by the code server."

22. **Application master / group leader** — "the application controller creates
    an *application master* for the application. The application master
    establishes itself as the group leader of all processes in the application
    and will forward I/O to the previous group leader."

23. **Application master purpose** — "The purpose of the application master
    being the group leader is to easily keep track of which processes that
    belong to the application. That is needed to support the
    `application:get_application/0` and `application:get_env/1` functions, and
    also when stopping an application to ensure that all processes belonging to
    the application are terminated."

24. **start/2 invocation** — "The application master starts the application by
    calling the application callback function `start/2` in the module with the
    start argument defined by the `mod` key in the `.app` file."

25. **stop/1 invocation** — "The application master stops the application by
    telling the top supervisor to shut down. The top supervisor tells all its
    child processes to shut down, and so on; the entire tree is terminated in
    reverse start order. The application master then calls the application
    callback function `stop/1` in the module defined by the `mod` key."

26. **env key** — "An application can be configured using *configuration
    parameters*. These are a list of `{Par,Val}` tuples specified by a key
    `env` in the `.app` file"

27. **env Par/Val** — "`Par` is to be an atom. `Val` is any term. The
    application can retrieve the value of a configuration parameter by calling
    `application:get_env(App, Par)` or a number of similar functions."

28. **System config file** — "The system configuration is to be called
    `Name.config` and Erlang is to be started with the command-line argument
    `-config Name`."

29. **sys.config** — "If release handling is used, exactly one system
    configuration file is to be used and that file is to be called `sys.config`."

30. **Command-line override** — "The values in the `.app` file and the values in
    a system configuration file can be overridden directly from the command
    line: `% erl -ApplName Par1 Val1 ... ParN ValN`"

31. **Start types** — "A *start type* is defined when starting the application:
    `application:start(Application, Type)` `application:start(Application)` is
    the same as calling `application:start(Application, temporary)`. The type
    can also be `permanent` or `transient`"

32. **permanent** — "If a permanent application terminates, all other
    applications and the runtime system are also terminated."

33. **transient** — "If a transient application terminates with reason `normal`,
    this is reported but no other applications are terminated. If a transient
    application terminates abnormally, that is with any other reason than
    `normal`, all other applications and the runtime system are also
    terminated."

34. **temporary** — "If a temporary application terminates, this is reported but
    no other applications are terminated."

35. **explicit stop** — "An application can always be stopped explicitly by
    calling `application:stop/1`. Regardless of the mode, no other
    applications are affected."

36. **transient caveat** — "The transient mode is of little practical use,
    since when a supervision tree terminates, the reason is set to `shutdown`,
    not `normal`."

## Version notes
- Page metadata: `Erlang System Documentation v29.0.2`; `major-vsn` 29;
  sidebar shows `OTP 29.0.2`.
- ExDoc generator v0.40.3.
- Source: `github.com/erlang/otp` at tag `OTP-29.0.2`,
  path `system/doc/design_principles/applications.md`.
- Copyright © 1996-2026 Ericsson AB.
- The shell examples in the page use legacy version strings
  (e.g. `ERTS CXC 138 10`, kernel `2.8.1.3`, stdlib `1.11.4.3`, BEAM emulator
  `5.2.3.6`) — these are illustrative and not the OTP 29 versions; they are
  carried verbatim from older doc revisions.

## Discovered links

### Relevant (crawl later)
- `../apps/kernel/app.html` — `app` (Kernel): full `.app` resource file
  syntax/keys (incl. `id`, `start_phases`, etc.). HIGH priority; this page
  defers to it for full key list.
- `../apps/kernel/application.html` — `application` (Kernel): the application
  module; full callback docs (`prep_stop/1`, `start_phase/3`,
  `config_change/3`, `get_env/*`, `start/1,2`, `stop/1`, etc.). HIGH priority.
- `../apps/kernel/config.html` — `config` (Kernel): system config file format
  (`sys.config`, `-config`). MEDIUM.
- `../apps/kernel/code.html` — `code` (Kernel): code server, `priv_dir/1`,
  version selection. MEDIUM.
- `../apps/sasl/systools.html` — `systools` (SASL): packaging tools, boot
  scripts, tar files. MEDIUM.
- `distributed_applications.html` — Distributed Applications: takeover/failover
  StartType values, distributed app config. HIGH (referenced for non-`normal`
  StartType).
- `included_applications.html` — Included Applications: included apps,
  start phases. MEDIUM-HIGH.
- `release_structure.html` — Releases: release packaging structure. MEDIUM.
- `release_handling.html` — Release handling (`sys.config` requirement).
  MEDIUM.
- `sup_princ.html` — Supervisor Behaviour (the `ch_sup` example origin).
  Already crawled as `03-sup-princ.md`. Skip.
- `spec_proc.html` — sys and proc_lib (prev page). MEDIUM.

### Skipped
- `applications.html` — self link (this page).
- `applications.md` — copy-markdown link to this page's source.
- `../index.html` — Erlang System Documentation index.
- `https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/applications.md` — view-source link to the markdown on GitHub.
- `https://erlang.org`, `https://www.ericsson.com`, `https://github.com/elixir-lang/ex_doc` — external site links.
- `https://www.erlang.org/doc/system/applications.html` — canonical self link.
- `/assets/css/algolia-typeahead.css`, `dist/html-erlang-KCHZLXSC.css`,
  `dist/sidebar_items-D241B7A5.js`, `docs_config.js`, `dist/html-Y2MUTVIN.js`,
  `assets/logo.png`, `llms.txt`, `Erlang System Documentation.epub`,
  `search.html` — site assets / chrome.
