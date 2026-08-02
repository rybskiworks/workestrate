---
name: beam-applications-releases
description: |
  Operational guide for BEAM/OTP applications and releases — the `.app` resource
  file, application callbacks, application environment, start phases, included
  applications, releases, `sys.config`/`vm.args`, embedded vs interactive mode,
  `appup`/`relup` hot code upgrades, and the code server. Load when creating
  application callback modules, building releases, configuring
  `sys.config`/`vm.args`, or implementing hot code upgrades. Does NOT cover
  supervision tree internals (see `beam-supervision`) or logger configuration
  (see `beam-logger-config`).
---

# BEAM/OTP Applications and Releases

## Triggers

Load this skill when:

- Creating or reviewing application callback modules.
- Writing or editing `.app` resource files.
- Configuring application environment or `sys.config`/`vm.args`.
- Building OTP releases or working with `.rel`/`.boot` files.
- Writing `appup`/`relup` files or implementing `code_change/3` for hot upgrades.

## References

This skill is grounded in the following docs. Read them for full detail; the
canonical source URLs are copied from each doc's `## Sources used` section.

- `docs/beam/applications.md`
  - https://www.erlang.org/doc/system/applications.html
  - https://www.erlang.org/doc/apps/kernel/application.html
  - https://www.erlang.org/doc/apps/kernel/app.html
- `docs/beam/releases.md`
  - https://www.erlang.org/doc/system/release_handling.html
  - https://www.erlang.org/doc/system/code_loading.html
  - https://www.erlang.org/doc/apps/kernel/code.html

## Key Rules

- `.app` resource file: single Erlang term named `Application.app` in `ebin/`.
  All keys are optional for the controller. Full key set from `app(4)`:
  `description`, `id`, `vsn`, `modules`, `maxP` (deprecated, ignored), `maxT`,
  `registered`, `included_applications`, `optional_applications`,
  `applications`, `env`, `mod`, `start_phases`, `runtime_dependencies`.
  `systools` requires `description`, `vsn`, `modules`, `registered`,
  `applications`.
- Application callbacks (export from `-behaviour(application)` module):
  - `start(StartType, StartArgs) -> {ok, Pid} | {ok, Pid, State} | {error, Reason}`.
  - `stop(State) -> term()` (return ignored; called after the tree is shut down).
  - Optional: `prep_stop(State) -> NewState`; `start_phase(Phase, StartType, PhaseArgs) -> ok | {error, Reason}`; `config_change(Changed, New, Removed) -> ok`.
- `StartType`: `normal` | `{takeover, Node}` | `{failover, Node}`.
- `restart_type`: `permanent` (runtime terminates if app terminates),
  `transient` (runtime terminates only on abnormal app exit),
  `temporary` (default — app termination is reported but does not affect runtime).
- Application env API: `get_env(App, Key, Default)`, `get_all_env(App)`,
  `set_env(App, Key, Val)`, `unset_env(App, Key)`. Startup precedence
  (lowest → highest): `.app` `env` defaults < config files (`-config`/`sys.config`,
  last wins) < command-line flags `-App Par Val` (always override).
  Non-persistent `set_env` is overridden by `.app` on load/reload; persistent
  `set_env` sticks. During release install, `set_env` values are disregarded.
- Included applications are loaded but NOT started by the application controller;
  the primary app's supervisors must start their trees. Their `start_phases`
  must be a subset of the primary app's phases.
- Release = ERTS + a subset of OTP apps + user apps; booted by a `.boot` script.
- `.rel` file: `{release, {Name, Vsn}, {erts, ErtsVsn}, [{App, AppVsn}]}`.
- `sys.config`: single Erlang term `[{App, [{Par, Val}]}]` overriding app env at
  boot. `vm.args`: VM flags such as `-sname`, `-name`, `-setcookie`, `+S`,
  `-mode embedded|interactive`.
- Embedded mode loads modules at startup from the boot script and is required
  for release handling/hot upgrades. Interactive mode (default) loads code on
  demand.
- `.appup` file: `{Vsn, [{UpFromVsn, Instructions}], [{DownToVsn, Instructions}]}`;
  versions may be regular expressions.
- Low-level instructions: `load_module`, `add_module`, `delete_module`, `apply`,
  `purge`, `soft_purge`, `restart_new_emulator` (must be FIRST, requires
  `heart`), `restart_emulator` (must be at END).
- High-level instructions: `update` (with `{advanced, Extra}` to invoke
  `code_change/3`, or `supervisor` for supervisor spec changes),
  `add_application`, `remove_application`, `restart_application`.
- Hot upgrade path: `release_handler:unpack_release/1` →
  `release_handler:install_release/1` → `release_handler:make_permanent/1`.
  `update` suspends processes (`sys:suspend`), calls `code_change/3`
  (`sys:change_code`), purges old code, then resumes (`sys:resume`).
- BEAM supports TWO module versions (current + old). `code:purge/1` removes old
  code and KILLS any processes still running it. `code:soft_purge/1` removes old
  code only if no process is running it, otherwise returns `false`.
- `heart` is required for `restart_new_emulator` and `restart_emulator`
  instructions.
- Useful `code` functions: `load_file/1`, `ensure_loaded/1`, `purge/1`,
  `soft_purge/1`, `delete/1`, `add_path/2`, `get_path/0`, `set_path/1`,
  `stick_dir/1`, `is_loaded/1`, `which/1`.

## Quick Commands

```erl
application:start(my_app).
application:start(my_app, permanent).
application:stop(my_app).
application:ensure_all_started(my_app).
application:get_env(my_app, key, default).
application:which_applications().
code:soft_purge(my_module).
code:purge(my_module).
code:which(my_module).
release_handler:which_releases().
```

## Anti-patterns

- Forgetting `mod` in `.app` for a supervision-tree app.
- Doing heavy work in `start/2` (blocks application startup).
- Using `code:purge/1` when `code:soft_purge/1` would do.
- Expecting `mix release`/immutable releases to perform hot upgrades without
  OTP SASL/release handling.
- Forgetting `heart` for `restart_new_emulator`/`restart_emulator` instructions.
- Not implementing `code_change/3` when hot upgrades are in scope.
- Using `application:set_env/3` for values that are read at compile time.

## Related Skills

- `beam-supervision`
- `beam-gen-server`
- `beam-logger-config`
- `beam-observability-debugging`
