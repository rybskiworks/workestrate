# Crawl: kernel/app.html (app resource file spec)
- seed_url: https://www.erlang.org/doc/apps/kernel/app.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/app.html
- family: Erlang/OTP kernel file-format docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: applications.md
## Purpose
The **application resource file** (the `app(4)` file) specifies the resources an
application uses and how the application is started. There must always be one
application resource file called `Application.app` for each application
`Application` in the system.

The file is read by the **application controller** when an application is
loaded/started. It is also used by functions in `systools`, for example when
generating start scripts.

## Resource file term format
The file must be called `Application.app` (where `Application` is the application
name) and located in the `ebin` directory for the application. The file must
contain a single Erlang term, the *application specification*:

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

Type/Default table:

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

## Option keys (full list + meaning + defaults)
- **`description`** (string, default `""`) — A one-line description of the application.
- **`id`** (string, default `""`) — Product identification, or similar.
- **`vsn`** (string, default `""`) — Version of the application.
- **`modules`** (`[Module]`, default `[]`) — All modules introduced by this
  application. `systools` uses this list when generating start scripts and tar
  files. A module can only be defined in one application.
- **`maxP`** (int, default `infinity`) — *Deprecated — is ignored.* Maximum
  number of processes allowed in the application.
- **`maxT`** (int, default `infinity`) — Maximum time, in milliseconds, that the
  application is allowed to run. After the specified time, the application
  terminates automatically.
- **`registered`** (`[Name]`, default `[]`) — All names of registered processes
  started in this application. `systools` uses this list to detect name clashes
  between different applications.
- **`included_applications`** (`[App]`, default `[]`) — All applications included
  by this application. When this application is started, all included
  applications are loaded automatically, but not started, by the application
  controller. It is assumed that the top-most supervisor of the included
  application is started by a supervisor of this application.
- **`applications`** (`[App]`, default `[]`) — All applications that must be
  started before this application. If an application is also listed in
  `optional_applications`, then the application is not required to exist (but
  if it exists, it is also guaranteed to be started before this one). `systools`
  uses this list to generate correct start scripts. Defaults to the empty list,
  but notice that all applications have dependencies to (at least) Kernel and
  STDLIB.
- **`optional_applications`** (`[App]`, default `[]`) — A list of `applications`
  that are optional. Note: if you want an optional dependency to be
  automatically started before the current application whenever it is available,
  it must be listed on both `applications` and `optional_applications`.
- **`env`** (`[{Par,Val}]`, default `[]`) — Configuration parameters used by the
  application. The value of a configuration parameter is retrieved by calling
  `application:get_env/1,2`. Values in the resource file can be overridden by
  values in a configuration file (`config(4)`) or by command-line flags
  (`erts:erl(1)`).
- **`mod`** (`{Module,StartArgs}`, default `[]`) — Specifies the application
  callback module and a start argument, see `application`. Key `mod` is
  necessary for an application implemented as a supervision tree, otherwise the
  application controller does not know how to start it. `mod` can be omitted for
  applications without processes, typically code libraries, e.g. STDLIB.
- **`start_phases`** (`[{Phase,PhaseArgs}]`, default `undefined`) — A list of
  start phases and corresponding start arguments for the application. If
  present, the application master, in addition to the usual call to
  `Module:start/2`, also calls
  `Module:start_phase(Phase,Type,PhaseArgs)` for each start phase defined by
  key `start_phases`. Only after this extended start procedure does
  `application:start(Application)` return.
- **`runtime_dependencies`** (`[ApplicationVersion]`, default `[]`) — A list of
  application versions that the application depends on (e.g. `"kernel-3.0"`).
  Versions specified are *minimum requirements*: a larger version than the one
  specified satisfies the requirement. See section *Versions* in the System
  Principles User's Guide for comparison rules. The application version
  specifies a source code version; an indirect requirement is that the installed
  binary of the specified version is built compatible with the rest of the
  system. Some dependencies can be required only in specific runtime scenarios;
  these are specified/documented in the corresponding "App" documentation of the
  specific application.

## mod / start_phases / env specifics
- **`mod` for supervision-tree apps**: required so the application controller
  knows how to start the app. Omit for process-less code libraries (e.g.
  STDLIB).
- **`start_phases` + included applications**: start phases can synchronize
  startup of an application and its included applications. In this case `mod`
  must be specified as:
  ```erlang
  {mod, {application_starter, [Module, StartArgs]}}
  ```
  The application master then calls `Module:start/2` for the primary
  application, followed by calls to `Module:start_phase/3` for each start phase
  (as defined for the primary application), both for the primary application and
  for each of its included applications for which the start phase is defined.
  This implies that for an included application, the set of start phases must be
  a *subset* of the set of phases defined for the primary application. See
  *Applications* in OTP Design Principles.
- **`env`**: configuration key/value list read via `application:get_env/1,2`.
  Overridable by `config(4)` files and `erl` command-line flags.

## Strict rules (required vs optional keys; dependency semantics)
- For the **application controller**, all keys are optional. Respective default
  values are used for any omitted keys.
- The functions in **`systools`** require more information. If they are used,
  the following keys are **mandatory**:
  - `description`
  - `vsn`
  - `modules`
  - `registered`
  - `applications`
- The other keys are **ignored by `systools`**.
- **Dependency semantics**:
  - `applications` = hard dependencies that must be started before this app.
  - `optional_applications` = optional deps; to get auto-start-before behavior
    when available, list the app in *both* `applications` and
    `optional_applications`.
  - `included_applications` = loaded (not started) automatically when the
    including app starts; their top supervisor is started by a supervisor of
    this application.
  - `runtime_dependencies` = minimum-version source-code requirements (e.g.
    `"kernel-3.0"`).
  - Implicit baseline: all applications depend on (at least) Kernel and STDLIB.
- **`maxP` is deprecated and ignored.**
- A module can only be defined in one application (uniqueness across `modules`
  lists).

## Verbatim quotes
- "The application resource file specifies the resources an application uses,
  and how the application is started. There must always be one application
  resource file called `Application.app` for each application `Application` in
  the system."
- "The file is read by the application controller when an application is
  loaded/started. It is also used by the functions in `systools`, for example
  when generating start scripts."
- "The file must contain a single Erlang term, which is called an application
  specification."
- "For the application controller, all keys are optional. The respective
  default values are used for any omitted keys."
- "The functions in `systools` require more information. If they are used, the
  following keys are mandatory: description, vsn, modules, registered,
  applications. The other keys are ignored by `systools`."
- "Key `mod` is necessary for an application implemented as a supervision tree,
  otherwise the application controller does not know how to start it. `mod` can
  be omitted for applications without processes, typically code libraries, for
  example, STDLIB."
- "for an included application, the set of start phases must be a subset of the
  set of phases defined for the primary application."
- "Application versions specified as runtime dependencies are minimum
  requirements. That is, a larger application version than the one specified in
  the dependency satisfies the requirement."
- "all applications have dependencies to (at least) Kernel and STDLIB."

## Version notes
- Page meta: `kernel v11.0.2`, major version `29`, title
  `app — OTP 29.0.2 (kernel 11.0.2)`.
- ExDoc v0.40.3.
- Source: github.com/erlang/otp blob OTP-29.0.2 `lib/kernel/doc/references/app.md`.
- Copyright © 1996-2026 Ericsson AB.
- `maxP` is marked *Deprecated — is ignored* in OTP 29.
- Relationship to other docs:
  - `application.html` — the `application` module (callback module / API
    referenced by `mod` and `env`).
  - `../../system/applications.html` — *Applications* in OTP Design Principles
    (system-level description of applications, start phases, included apps).
  - `config.html` — `config(4)`, the config file that can override `env`.
  - `../../apps/sasl/systools.html` — `systools`, the tool that enforces the
    mandatory keys.
  - `../../apps/erts/erl_cmd.html` — `erts:erl(1)`, command-line flags that can
    override `env`.
  - `../../system/versions.html` — *Versions* in System Principles (application
    version comparison rules for `runtime_dependencies`).

## Discovered links
### Relevant (crawl later)
1. https://www.erlang.org/doc/apps/kernel/application.html — `application` module (callback module + API for `mod`/`env`)
2. https://www.erlang.org/doc/apps/kernel/config.html — `config(4)` (overrides `env`)
3. https://www.erlang.org/doc/system/applications.html — *Applications* in OTP Design Principles (system doc; start phases, included apps)
4. https://www.erlang.org/doc/system/versions.html — *Versions* in System Principles (version comparison for `runtime_dependencies`)
5. https://www.erlang.org/doc/apps/sasl/systools.html — `systools` (enforces mandatory keys, generates start scripts)
6. https://www.erlang.org/doc/apps/erts/erl_cmd.html — `erts:erl(1)` (command-line flags overriding `env`)

### Skipped
- https://www.erlang.org/doc/apps/kernel/app.md (markdown source mirror of this page)
- https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/doc/references/app.md (raw source)
- https://www.erlang.org/doc/apps/kernel/eep48_chapter.html (prev page, unrelated EEP-48)
- https://www.erlang.org/doc/apps/kernel/llms.txt (site llms index)
- https://www.erlang.org/doc/apps/kernel/kernel.epub (epub download)
- https://github.com/elixir-lang/ex_doc (ExDoc tooling)
- https://www.erlang.org / https://www.ericsson.com (site/corporate)
