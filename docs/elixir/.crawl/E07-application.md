# Crawl: hexdocs.pm/elixir/Application.html
- seed_url: https://hexdocs.pm/elixir/Application.html
- canonical_url: https://elixir.hexdocs.pm/Application.html
- family: Elixir core module
- fetch: 200
- elixir_version: v1.20.2
- feeds_docs: configuration-and-runtime.md, mix-project-structure.md

## Purpose
The `Application` module provides the Elixir API for working with applications
and application environment. An application is a component that can be started
and stopped as a unit, encapsulating resources (processes, ports, ETS tables)
behind a single supervision tree. Elixir applications are BEAM/OTP applications
underneath — the `Application` module is a thin Elixir wrapper over Erlang's
`:application` module, exposing callbacks (`start/2`, `stop/1`, `prep_stop/1`,
`start_phase/3`, `config_change/3`) and functions to query/manipulate the
application environment and lifecycle.

## Application callback (start/2, stop/1)
- `@callback start(start_type, start_args) :: {:ok, pid} | {:ok, pid, state} | {:error, reason}`
  Called when an application is started (via `Application.start/2` or
  `ensure_started/2`). Should start the top-level process — the top supervisor
  of the supervision tree per OTP design principles. `start_type`:
    - `:normal` — normal startup, or distributed failover when `:start_phases`
      is `:undefined`.
    - `{:takeover, node}` — distributed takeover from another node.
    - `{:failover, node}` — distributed failover.
  Returns `{:ok, pid}` or `{:ok, pid, state}`; the `state` is later passed to
  `stop/1` (and `prep_stop/1` if defined).
- `@callback stop(state) :: term` — called after the application has been
  stopped, i.e. after its supervision tree has been stopped. Should do the
  opposite of `start/2` and perform cleanup. Return value is ignored. `state`
  is from `start/2` (or `[]`), or from `prep_stop/1` if present.
  `use Application` provides a default `stop/1` that does nothing and returns
  `:ok`.
- Optional callbacks:
    - `prep_stop/1` — called before the top-level supervisor is terminated;
      receives `start/2`'s state (or `[]`); its return value becomes the
      `state` passed to `stop/1`.
    - `start_phase/3` — synchronous phased startup; called once per phase in
      the `:start_phases` spec key, after `start/2` but before
      `Application.start/2` returns.
    - `config_change/3` — invoked after code upgrade if the application
      environment changed; receives `changed`, `new`, `removed`.

## env API (get_env/fetch_env!/compile_env/put_env)
- `get_env(app, key, default \\ nil)` — returns value for `key` in `app`'s env,
  or `default` if absent. **Warning: read only your own application's
  environment; do not read other applications' env.**
- `fetch_env!(app, key)` — returns value or raises `ArgumentError` if missing.
  Same own-application-only warning.
- `get_all_env(app)` — all key-value pairs for `app`.
- `put_env(app, key, value, opts \\ [])` — sets `key` to `value`. Options:
  `:timeout` (timeout), `:persistent` (boolean). **Compile environment rule: do
  not use `put_env` to change values read via `Application.compile_env/2` —
  compile env must be set exclusively before compilation, in config files.**
- `delete_env(app, key, opts \\ [])` — deletes `key`; same options as
  `put_env/4`.
- `compile_env(app, key_or_path, default \\ nil)` (macro, since 1.10.0) — reads
  application env at compile time. Tracks when config values change between
  compile time and runtime. `key_or_path` is an atom or a path list starting
  with an atom:
    - `Application.compile_env(:my_app, :key)` => `[foo: [bar: :baz]]`
    - `Application.compile_env(:my_app, [:key, :foo])` => `[bar: :baz]`
    - `Application.compile_env(:my_app, [:key, :foo, :bar])` => `:baz`
    - missing path segment returns the `default`.
- `compile_env!(app, key_or_path)` — compile-time read or raises.
- `compile_env!(env, app, key_or_path)` (since 1.14.0) — reads compile env from
  a macro; first arg is a `Macro.Env` (typically `__CALLER__`). Raises if the
  `Macro.Env` comes from a function (must be a macro context).

## spec/loaded/started, ensure_all_started
- `spec(app)` — returns the application spec as a keyword list, or `nil` if
  not loaded. Keys: `:description`, `:id`, `:vsn`, `:modules`, `:maxP`,
  `:maxT`, `:registered`, `:included_applications`, `:optional_applications`,
  `:applications`, `:mod`, `:start_phases`. (Env is not returned — use
  `fetch_env/2`.) See Erlang's application specification for field meanings.
- `loaded_applications/0` — list of `{app, description, vsn}` for loaded apps.
- `started_applications/0` — list of `{app, description, vsn}` for started
  apps.
- `start(app, type \\ :temporary)` — starts `app` with a `restart_type/0`
  (`:temporary` | `:permanent` | `:transient`). Loads first if needed;
  included applications (from `:included_applications`) are loaded but not
  started. All `:applications` deps must already be started or
  `{:error, {:not_started, app}}` is returned.
- `stop(app)` — stops `app`; the application stays loaded.
- `ensure_started(app, type \\ :temporary)` — like `start/2` but returns `:ok`
  if already started.
- `ensure_all_started(app_or_apps, type_or_opts \\ [])` — ensures `app`/apps
  and all child applications are started. Second arg is either a
  `restart_type/0` or a keyword list:
    - `:type` — `:temporary` (default) | `:permanent` | `:transient`.
    - `:mode` (since v1.15.0) — `:serial` (default) | `:concurrent`.
  Returns `{:ok, [apps]}` or `{:error, term}`.

## :mod + extra_applications in mix.exs
The `:mod` and `:extra_applications` keys live in the `application/0` function
of `mix.exs`, which Mix compiles into the `.app` resource file
(`ebin/<app>.app`). They are not `Application` module functions but configure
the BEAM application spec that `Application` then drives:
- `:mod` — `{Module, start_args}`. Tells OTP which module implements the
  `Application` behaviour callbacks; `Module.start/2` is invoked with
  `start_type` and `start_args` when the application starts. Without `:mod`,
  the application has no callback module (library-only application).
- `:extra_applications` — list of apps (e.g. `[:logger, :crypto]`) to prepend
  to the `:applications` list beyond what Mix auto-detects from dependencies.
  These are started before the application itself.
- Related spec keys surfaced by `spec/1`: `:applications` (must be started
  before this app), `:included_applications` (loaded but not started),
  `:optional_applications`, `:start_phases` (for `start_phase/3`).

## Application ↔ BEAM application mapping (→ docs/beam/applications.md)
- Elixir `Application` is a thin wrapper over Erlang's `:application` module;
  every Elixir application is a BEAM/OTP application with a `.app` resource
  file generated by Mix from `mix.exs`'s `application/0`.
- Callbacks map 1:1 to OTP application callback module semantics:
  `c:start/2` ↔ `:application` start callback (returns the top supervisor
  pid); `c:stop/1` ↔ stop callback; `c:prep_stop/1` ↔ prep_stop;
  `c:start_phase/3` ↔ start_phases; `c:config_change/3` ↔ config_change.
- `Application.spec/1` returns the same fields documented in
  `docs/beam/applications.md` for the `.app` resource file
  (`:description`, `:id`, `:vsn`, `:modules`, `:registered`,
  `:applications`, `:included_applications`, `:optional_applications`,
  `:mod`, `:start_phases`, `:maxP`, `:maxT`).
- `Application.start/2`, `stop/1`, `ensure_all_started/2` map to
  `:application.start/1-2`, `:application.stop/1`, and
  `:application.ensure_all_started/1-2`.
- `loaded_applications/0` and `started_applications/0` map to
  `:application.loaded_applications/0` and `:application.started_applications/0`.
- `compile_env` is Elixir-specific: it records compile-time config reads so
  Elixir can warn when runtime config diverges — there is no direct Erlang
  equivalent; the underlying env store is still `:application.get_env/put_env`.
- See `docs/beam/applications.md` for the `.app` file format, application
  callback module contract, `:mod` semantics, included vs optional
  applications, and start phases.

## Strict rules
- Read only your own application's environment via `get_env`/`fetch_env!` —
  never read other applications' env (documented warning).
- Do not use `put_env` to change values read via `Application.compile_env/2`;
  compile environment must be set exclusively before compilation in config
  files.
- `compile_env!/3` with a `Macro.Env` must be invoked from a macro context
  (`__CALLER__`); it raises if the env comes from a function.
- `start/2` must start the top-level supervisor (OTP design principles).
- `stop/1` is called after the supervision tree is stopped; do cleanup opposite
  of `start/2`; return value ignored.
- All apps in the `:applications` spec key must be started before
  `Application.start/2` succeeds, else `{:error, {:not_started, app}}`.
- Included applications are loaded but not started by `start/2`.

## Verbatim quotes
- "You must use this function to read only your own application environment.
  Do not read the environment of other applications." (`get_env/3`,
  `fetch_env!/2`)
- "Do not use this function to change environment variables read via
  Application.compile_env/2. The compile environment must be exclusively set
  before compilation, in your config files." (`put_env/4`)
- "This function should start the top-level process of the application (which
  should be the top supervisor of the application's supervision tree if the
  application follows the OTP design principles around supervision)."
  (`c:start/2`)
- "This function is called after an application has been stopped, i.e., after
  its supervision tree has been stopped. It should do the opposite of what the
  start/2 callback did, and should perform any necessary cleanup. The return
  value of this callback is ignored." (`c:stop/1`)
- "If not, {:error, {:not_started, app}} is returned, where app is the name of
  the missing application." (`start/2`)
- "Any included application, defined in the :included_applications key of the
  .app file will also be loaded, but they won't be started." (`start/2`)
- "This allows Elixir to track when configuration values change between
  compile time and runtime." (`compile_env/3`)

## Version notes
- `compile_env/3` introduced in Elixir 1.10.0.
- `compile_env!/3` (macro-env arity) introduced in Elixir 1.14.0.
- `ensure_all_started/2` `:mode` option (`:serial`/`:concurrent`) introduced in
  v1.15.0.
- Page reports Elixir v1.20.2.

## Discovered links
### Relevant (crawl later)
- https://hexdocs.pm/elixir/Application.html#start_phase/3 (start phases)
- https://hexdocs.pm/elixir/Application.html#spec/1 (application spec keys)
- https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html (.app generation)
- https://hexdocs.pm/elixir/Config.html (config files, compile env)
- https://hexdocs.pm/elixir/Application.html#ensure_all_started/2

### Skipped
- https://hexdocs.pm/elixir/Supervisor.html (covered by E02)
- https://hexdocs.pm/elixir/GenServer.html (covered by E01)
- https://www.erlang.org/doc/man/application.html (BEAM-side, in
  docs/beam/applications.md)
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/Logger.html
- navigation/footer links
