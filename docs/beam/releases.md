# Releases

## Purpose

This document defines BEAM/OTP guidance for releases and release handling: what a release is, boot scripts, embedded vs interactive mode, the `.appup`/`relup` instruction set, the `release_handler` sequence, code replacement (old/current code, purge/soft_purge), `on_load`, and target systems with `heart`. Future agents who build, upgrade, downgrade, or debug OTP releases should follow these rules so behavior is consistent, predictable, and aligned with the official Erlang/OTP documentation.

This doc is BEAM-common, not Elixir-specific. Examples are in Erlang. Release handling uses the `sasl` application (`release_handler`, `systools`); code loading uses the `code` module in `kernel`.

## Sources used

- `.crawl/08-release-handling.md` — https://www.erlang.org/doc/system/release_handling.html (PRIMARY — release handling framework, appup/relup, instruction set, release_handler sequence, config_change, target systems, heart)
- `.crawl/32-code-loading.md` — https://www.erlang.org/doc/system/code_loading.html (PRIMARY — interactive vs embedded, old/current code, on_load)
- `.crawl/22-code.md` — https://www.erlang.org/doc/apps/kernel/code.html (PRIMARY — `code` module API, purge/soft_purge/delete, two-version rule, code path, modes)
- `.crawl/46-appup-cookbook.md` — https://www.erlang.org/doc/system/appup_cookbook.html (PRIMARY — practical `.appup` recipes: functional module change, add/delete module, supervisor change, gen_server state migration, special process/port code change)

This page reflects Erlang/OTP 29.0.2 semantics. Release handling lives in `sasl`; the code server lives in `kernel`.

## Core guidance

### What a release is

A **release** is a complete system: the Erlang Run-Time System (ERTS) plus a set of applications with specific versions, packaged together. The minimal system enabling release handling consists of Kernel, STDLIB, and SASL.

From `release_handling.html`:

> "An important feature of the Erlang programming language is the ability to change module code at runtime, code replacement, as described in Code Replacement in the Erlang Reference Manual."

> "Based on this feature, the OTP application SASL provides a framework for upgrading and downgrading between different versions of an entire release in runtime. This is called release handling."

> "The framework consists of: Offline support - systools for generating scripts and building release packages; Online support - release_handler for unpacking and installing release packages"

### Boot scripts and embedded vs interactive mode

The code server loads code according to a strategy, set by the `-mode` flag:

From `code_loading.html`:

> "The code server loads code according to a code loading strategy, which is either interactive (default) or embedded. In interactive mode, code is searched for in a code path and loaded when first referenced. In embedded mode, code is loaded at start-up according to a boot script."

From `code.html`:

> "In interactive mode, which is default, only the modules needed by the runtime system are loaded during system startup. Other code is dynamically loaded when first referenced."

> "In embedded mode, modules are not auto-loaded. Trying to use a module that has not been loaded results in an error. This mode is recommended when the boot script loads all modules, as it is typically done in OTP releases."

Release handling requires embedded mode: the runtime must know which release it is running and be able to change boot script and system config at runtime on reboot.

### Code replacement: old/current code and the two-version rule

From `code_loading.html`:

> "The code of a module can exist in two variants in a system: current and old."

> "Both old and current code are valid, and can be evaluated concurrently. Fully qualified function calls always refer to current code. Old code can still be evaluated because of processes lingering in the old code."

> "If a third instance of the module is loaded, the code server removes (purges) the old code and any processes lingering in it are terminated."

From `code.html`:

> "The code for a module can exist in two variants in a system: current code and old code."

> "Both old and current code for a module are valid, and can even be executed concurrently. The difference is that exported functions in old code are unavailable. Hence, a global call cannot be made to an exported function in old code, but old code can still be executed because of processes lingering in it."

Purge functions (from `code.html`):
- `purge/1` — "Purges the code for Module, that is, removes code marked as old. If some processes still linger in the old code, these processes are killed before the code is removed." Returns `true` if any process was killed.
- `soft_purge/1` — "Purges the code for Module ... but only if no processes linger in it. Returns false if the module cannot be purged because of processes lingering in old code, otherwise true." Never kills.
- `delete/1` — "Removes the current code for Module, that is, the current code for Module is made old." Returns `false` if old code already exists (must purge first).

To switch a process from old to current code, it must make a **fully qualified function call** (e.g. `m:loop()`).

### `on_load` directive

From `code_loading.html`:

> "The function must return `ok` if the module is to become the new current code for the module and become callable."

> "Returning any other value or generating an exception causes the new code to be unloaded."

> "In embedded mode, first all modules are loaded. Then all `on_load` functions are called. The system is terminated unless all of the `on_load` functions return `ok`."

### Residence vs functional modules

From `release_handling.html`:

> "Residence module - The module where a process has its tail-recursive loop function(s). If these functions are implemented in several modules, all those modules are residence modules for the process."

> "Functional module - A module that is not a residence module for any process."

> "For a process implemented using an OTP behaviour, the behaviour module is the residence module for that process. The callback module is a functional module."

This distinction determines which instruction to use: `load_module` for functional modules (simple replacement); `update` with `{advanced,Extra}` for residence modules needing state transformation.

### The `.appup` file

An `.appup` file named `Application.appup` is placed in the `ebin` directory. Structure:

```erlang
{Vsn,
 [{UpFromVsn1, InstructionsU1}, ..., {UpFromVsnK, InstructionsUK}],
 [{DownToVsn1, InstructionsD1}, ..., {DownToVsnK, InstructionsDK}]}.
```

- `Vsn` (string) = current version per `.app` file.
- `UpFromVsn` / `DownToVsn` = previous version to upgrade from / downgrade to; may be regular expressions.
- `Instructions` = list of release-handling instructions.

### Release-handling instruction set (verbatim forms)

From `release_handling.html`, the instruction set used in `.appup` files:

- `{load_module, Module}` — simple code replacement for a functional module (load new, remove old).
- `{update, Module, {advanced, Extra}}` — synchronized code replacement; behaviour processes call `code_change/3` with `Extra` + other info. Used when changing internal state of a behaviour.
- `{update, Module, supervisor}` — used when changing the start specification of a supervisor.
- `{add_module, Module}` — loads a newly introduced module (required in embedded mode).
- `{delete_module, Module}` — unloads a module; kills any process with `Module` as residence module.
- `{add_application, Application}` — loads modules (via `add_module` instructions) per `modules` key in `.app`, then starts the application.
- `{remove_application, Application}` — stops application, unloads modules (via `delete_module`), unloads app spec from application controller.
- `{restart_application, Application}` — stop then start (like `remove_application` + `add_application` in sequence).
- `{apply, {M, F, A}}` — low-level; release handler evaluates `apply(M, F, A)`.
- `restart_new_emulator` (low-level) — for new ERTS version or upgrade of Kernel/STDLIB/SASL core apps. Must be first instruction in relup. Requires `heart`.
- `restart_emulator` (low-level) — unrelated to ERTS/core upgrades; forces runtime restart after all upgrade instructions. Only one allowed, must be at end.

The release handler understands only the **low-level** instructions. **High-level** instructions are translated to low-level by `systools:make_relup`.

### `update` mechanics (suspend / change_code / resume)

When a module is updated, the release handler finds processes using the module by traversing each running application's supervision tree and checking child specs. A process uses a module if the name is listed in `Modules` of its child spec. If `Modules=dynamic` (event managers), the event manager informs the release handler of currently installed handlers.

> "The release handler suspends, asks for code change, and resumes processes by calling the functions `sys:suspend/1,2`, `sys:change_code/4,5`, and `sys:resume/1,2`, respectively."

> "update with argument `{advanced,Extra}` is used when changing the internal state of a behaviour as described above. It causes behaviour processes to call the callback function `code_change/3`, passing the term `Extra` and some other information as arguments."

### The `relup` file

Generated by `systools:make_relup/3,4` from `.rel`, `.app`, `.appup` inputs; deduces added/deleted/upgraded/downgraded apps and transforms `.appup` instructions into a single ordered list of low-level instructions.

> "This file does not need to be created manually. It can be generated by `systools:make_relup/3,4`. The relevant versions of the `.rel` file, `.app` files, and `.appup` files are used as input."

### `release_handler` sequence (unpack → install → make_permanent)

- `release_handler:unpack_release(ReleaseName) => {ok, Vsn}` — creates `$ROOT/lib/releases/Vsn` with `.rel`, `start.boot`, `sys.config`, `relup`. New-version app dirs placed under `$ROOT/lib`; unchanged apps not affected.
- `release_handler:install_release(Vsn) => {ok, FromVsn, []}` — evaluates `relup` instructions step by step. On error, system rebooted using old release version. On success, system uses new version, but a reboot reverts to previous version until made permanent. Downgrade: call `install_release(FromVsn)`.
- `release_handler:make_permanent(Vsn) => ok` — new version becomes default; previous becomes "old". Tracked in `$ROOT/releases/RELEASES` and `$ROOT/releases/start_erl.data`.
- `release_handler:remove_release(Vsn) => ok` — removes an installed-but-not-permanent release.
- `release_handler:which_releases(current)` — check if upgrade is complete.

> "If an error occurs during the installation, the system is rebooted using the old version of the release. If installation succeeds, the system is afterwards using the new version of the release, but if anything happens and the system is rebooted, it starts using the previous version again."

### `config_change` callback

After install, the application controller compares old/new config params for all running applications and calls `Module:config_change(Changed, New, Removed)`. `Module` is the application callback module per `mod` key. Optional; may be omitted.

### Application spec updates on install

> "When a new version of a release is installed, the application specifications are automatically updated for all loaded applications."

Config priority (increasing): boot-script data (from `App.app`) → new `sys.config` → `-App Par Val` command-line args. Values from other config files and `application:set_env/3` are **disregarded**.

### Target systems and `heart`

> "For release handling to work properly, the runtime system must have knowledge about which release it is running. It must also be able to change (in runtime) which boot script and system configuration file to use if the system is rebooted, for example, by heart after a failure. Thus, Erlang must be started as an embedded system..."

> "The system must be configured using only one system configuration file, called `sys.config`."

> "All versions of a release, except the first one, must contain a relup file."

`heart` is required for reboot-based upgrades (`restart_new_emulator`, `restart_emulator`). On UNIX during `restart_new_emulator`, `HEART_COMMAND` is ignored; defaults to `$ROOT/bin/start`, overridable via SASL param `start_prg`.

### Appup Cookbook — practical upgrade recipes

> "This section includes examples of .appup files for typical cases of upgrades/downgrades done in runtime."

The central distinction the cookbook enforces is between functional modules (simple `load_module` suffices) and residence modules (synchronized replacement required). In an OTP-structured system:

> "In a system implemented according to the OTP design principles, all processes, except system processes and special processes, reside in one of the behaviours supervisor, gen_server, gen_statem, gen_event, or gen_fsm. These belong to the STDLIB application and upgrading/downgrading normally requires a runtime system restart. Thus, OTP provides no support for changing residence modules except in the case of special processes."

#### Changing a functional module

- **Scenario:** functional module changed (new function, bug fix), no resident process state.
- **Recipe:** `load_module` (simple code replacement).
- **appup:**

```erlang
{"2",
 [{"1", [{load_module, m}]}],
 [{"1", [{load_module, m}]}]
}.
```

- **Note:** a callback module is a functional module; pure code extension uses the same `load_module` recipe.

#### Adding/deleting a module

- **Scenario:** new functional module `m` introduced in v2.
- **Recipe:** `add_module` on upgrade, `delete_module` on downgrade.
- **appup:**

```erlang
{"2",
 [{"1", [{add_module, m}]}],
 [{"1", [{delete_module, m}]}]
}.
```

- **Module dependencies:** if `m1` calls a new function in `ch3`, `ch3` must be loaded before `m1` on upgrade (reverse on downgrade). Express with `DepMods`:

```erlang
{load_module, Module, DepMods}
{update, Module, {advanced, Extra}, DepMods}
```

- "systools knows the difference between up- and downgrading and generates a correct relup, where ch3 is loaded before m1 when upgrading, but m1 is loaded before ch3 when downgrading."

#### Changing a supervisor

The supervisor behaviour supports changing internal state (restart strategy, max restart frequency, child specs) but child add/delete is NOT automatic.

> "The supervisor behaviour supports changing the internal state, that is, changing the restart strategy and maximum restart frequency properties, as well as changing the existing child specifications. Child processes can be added or deleted, but this is not handled automatically."

Changing properties (restart strategy, intensity): `{update, Module, supervisor}`.

```erlang
{"2",
 [{"1", [{update, ch_sup, supervisor}]}],
 [{"1", [{update, ch_sup, supervisor}]}]
}.
```

Changing child specs: same instruction. > "The changes do not affect existing child processes. ... The id of the child specification cannot be changed. Changing the Modules field of the child specification can affect the release handling process itself, as this field is used to identify which processes are affected when doing a synchronized code replacement."

Adding/deleting child processes: NOT auto-started/terminated; use `apply` instructions. Order matters.

> "New child specifications are automatically added, but not deleted. Child processes are not automatically started or terminated, this must be done using apply instructions. ... The order of the instructions is important."

appup (add child `m1` on upgrade, delete on downgrade):

```erlang
{"2",
 [{"1",
   [{update, ch_sup, supervisor},
    {apply, {supervisor, restart_child, [ch_sup, m1]}}
   ]}],
 [{"1",
   [{apply, {supervisor, terminate_child, [ch_sup, m1]}},
    {apply, {supervisor, delete_child, [ch_sup, m1]}},
    {update, ch_sup, supervisor}
   ]}]
}.
```

If module `m1` is newly introduced, also load/delete it (`add_module` before `update` on upgrade; `delete_module` after `update` on downgrade). The supervisor must be registered for direct `apply` access.

#### Changing a gen_server state via code_change

- **Scenario:** internal state format must change (e.g. `Chs` → `{Chs,N}`).

> "In this case, simple code replacement is not sufficient. The process must explicitly transform its state using the callback function code_change/3 before switching to the new version of the callback module. Thus, synchronized code replacement is used."

- **Recipe:** `{update, Module, {advanced, Extra}}`.

> "The third element of the update instruction is a tuple {advanced,Extra}, which says that the affected processes are to do a state transformation before loading the new version of the module."

- **appup:**

```erlang
{"2",
 [{"1", [{update, ch3, {advanced, []}}]}],
 [{"1", [{update, ch3, {advanced, []}}]}]
}.
```

- **Callback:**

```erlang
code_change({down, _Vsn}, {Chs, N}, _Extra) ->
    {ok, Chs};
code_change(_Vsn, Chs, _Extra) ->
    {ok, {Chs, 0}}.
```

- **Semantics:** first arg is `{down, Vsn}` on downgrade, else `Vsn` on upgrade. > "The first argument is {down,Vsn} if there is a downgrade, or Vsn if there is an upgrade. The term Vsn is fetched from the 'original' version of the module, that is, the version you are upgrading from, or downgrading to." `Vsn` comes from the `vsn` module attribute; absent, it is the beam checksum.

#### Changing code a process is running (special process)

- **Scenario:** a special process's residence module changes; process must make a fully-qualified call to switch to new code.

> "When a new version of a residence module for a special process is loaded, the process must make a fully qualified call to its loop function to switch to the new code. Thus, synchronized code replacement must be used."

- **Rule:** the residence module name(s) MUST be listed in `Modules` of the child spec. > "The name(s) of the user-defined residence module(s) must be listed in the Modules part of the child specification for the special process. Otherwise the release handler cannot find the process."
- **Recipe:** `{update, Module, {advanced, Extra}}` — makes the special process call `system_code_change/4`.
- **Child spec:**

```erlang
{ch4, {ch4, start_link, []},
 permanent, brutal_kill, worker, [ch4]}
```

- **appup:**

```erlang
{"2",
 [{"1", [{update, ch4, {advanced, []}}]}],
 [{"1", [{update, ch4, {advanced, []}}]}]
}.
```

- **Callback:**

```erlang
system_code_change(Chs, _Module, _OldVsn, _Extra) ->
    {ok, Chs}.
```

#### Changing non-Erlang code (port program)

Application-dependent; OTP provides no special support. Pattern: controlling `gen_server` implements `code_change/3` that closes old port and opens new one.

- **appup:**

```erlang
["2",
 [{"1", [{update, portc, {advanced,port}}]}],
 [{"1", [{update, portc, {advanced,port}}]}]
].
```

- Ship `priv` dir: `systools:make_tar("my_release", [{dirs,[priv]}]).`

#### No-appup-needed cases

> "When adding or removing an application, no .appup file is needed. When generating relup, the .rel files are compared and the add_application and remove_application instructions are added automatically."

> "When installing a release, the application specifications are automatically updated before evaluating the relup script. Thus, no instructions are needed in the .appup file."

- Included applications: no dedicated instructions; `.relup` must be hand-written.
- Runtime restart only: a minimal hand-written `.relup` with `restart_emulator` suffices and no `.appup` files are required.
- `restart_new_emulator`: "Intended when ERTS, Kernel, STDLIB, or SASL is upgraded. It is automatically added when the relup file is generated by systools:make_relup/3,4. It is executed before all other upgrade instructions."
- `restart_emulator`: "Used when a restart of the runtime system is required after all other upgrade instructions are executed."

## Practical rules

1. Start the runtime in embedded mode (`-mode embedded`) for release handling.
2. Use exactly one system config file named `sys.config` in `$ROOT/releases/Vsn`.
3. Generate `relup` with `systools:make_relup/3,4`; do not hand-write unless the instructions are purely low-level.
4. Use `load_module` for functional modules; use `update` with `{advanced,Extra}` for residence modules needing state changes.
5. Ensure `add_module` is used for newly introduced modules in embedded mode.
6. Terminate all processes with a module as residence module **before** `delete_module` to avoid failing supervisor restarts.
7. Keep code changes small and backwards compatible.
8. Make the new release permanent after `restart_new_emulator` once the new runtime is operational.
9. Use `load_module` for functional modules; `update` with `{advanced,Extra}` for residence modules needing state changes.
10. Use `add_module`/`delete_module` for newly introduced/removed modules in embedded mode.
11. List residence module name(s) in child spec `Modules` for special processes, or the release handler cannot find them.
12. Express module load-order dependencies via `DepMods`, not manual ordering — `systools` handles up/down direction.
13. For supervisor child add/delete, use `apply` instructions in the correct order (upgrade: update supervisor → start child; downgrade: terminate child → delete child spec → update supervisor).
14. The supervisor child spec `id` cannot be changed via supervisor update.
15. No `.appup` needed for adding/removing applications or application spec changes (auto-generated).
16. For included applications, hand-write the `.relup`.

## Review checklist

- [ ] Runtime started in embedded mode.
- [ ] Exactly one `sys.config` used.
- [ ] `relup` present for all release versions except the first.
- [ ] `restart_new_emulator` is the first instruction in relup (auto-enforced by `systools:make_relup`).
- [ ] At most one `restart_emulator`, placed at the end.
- [ ] `heart` started for reboot-based upgrades.
- [ ] `.appup` covers both upgrade and downgrade paths.
- [ ] `code_change/3` implemented for behaviours whose state format changes.
- [ ] Is `load_module` used only for functional modules (no resident process state)?
- [ ] Is `{update, Module, {advanced, Extra}}` used for residence modules with state changes?
- [ ] Are residence module names listed in child spec `Modules` for special processes?
- [ ] Are module load-order dependencies expressed via `DepMods`?
- [ ] For supervisor child add/delete, are `apply` instructions in the correct order?
- [ ] Is the supervisor registered for direct `apply` access (or is a helper used)?
- [ ] Is `code_change/3` implemented for both upgrade and downgrade directions?
- [ ] Is `system_code_change/4` implemented for special processes?

## Implementation checklist

- [ ] `.appup` file named `Application.appup` in `ebin/`.
- [ ] `UpFromVsn`/`DownToVsn` correct (or regex).
- [ ] Child spec `Modules` lists the callback module for processes that need `update`.
- [ ] `Modules=dynamic` set for event managers.
- [ ] `code_change/3` returns `{ok, NewState}` for the new version.
- [ ] `config_change/3` implemented if config params change across versions.
- [ ] `code_change/3` handles `{down, Vsn}` for downgrade and `Vsn` for upgrade.
- [ ] `vsn` module attribute set if `Vsn` passed to `code_change` must be meaningful.
- [ ] `priv` dir included in release tarball for port programs: `systools:make_tar(..., [{dirs,[priv]}])`.
- [ ] Included applications have a hand-written `.relup` (no auto-generated instructions).

## Runtime / debugging checklist

- [ ] `release_handler:which_releases/0,1` — list releases and state (`current`/`old`/`permanent`).
- [ ] `code:which(Module)` — which on-disk `.beam` is loaded.
- [ ] `code:module_status/0` — `not_loaded | loaded | removed | modified`.
- [ ] `code:modified_modules/0` — loaded modules whose on-disk MD5 differs.
- [ ] `code:soft_purge/1` before `delete/1` to avoid killing processes.
- [ ] `sys:suspend/1` / `sys:change_code/4` / `sys:resume/1` — manual code change.

## Validation hooks

- Generate boot script: `systools:make_script("my_rel-2")` returns `ok`.
- Build tar: `systools:make_tar("my_rel-2")` returns `ok`.
- Generate relup: `systools:make_relup("my_rel-2", ["my_rel-1"], ["my_rel-1"])` returns `ok`.
- Unpack: `release_handler:unpack_release("my_rel-2")` returns `{ok, Vsn}`.
- Install: `release_handler:install_release(Vsn)` returns `{ok, FromVsn, []}`.
- Make permanent: `release_handler:make_permanent(Vsn)` returns `ok`.
- Verify: `release_handler:which_releases(current)` lists the new release as `current`.

## Examples

### `.appup` for a functional module change

```erlang
{"2",
 [{"1", [{load_module, ch3}]}],
 [{"1", [{load_module, ch3}]}]
}.
```

### Generate relup and build package

```erlang
1> systools:make_relup("ch_rel-2", ["ch_rel-1"], ["ch_rel-1"]).
ok
1> systools:make_script("ch_rel-2").
ok
2> systools:make_tar("ch_rel-2").
ok
```

### Install sequence in a running target

```erlang
1> release_handler:unpack_release("ch_rel-2").
{ok,"B"}
3> release_handler:install_release("B").
{ok,"A",[]}
7> release_handler:make_permanent("B").
ok
```

### Code-switch loop (old → current via fully qualified call)

```erlang
-module(m).
-export([loop/0]).

loop() ->
    receive
        code_switch ->
            m:loop();
        Msg ->
            %% ...
            loop()
    end.
```

### `on_load` for NIF initialization

```erlang
-module(m).
-on_load(load_my_nifs/0).

load_my_nifs() ->
    NifPath = ...,    %% path to the NIF library
    Info = ...,       %% initialize the Info term
    erlang:load_nif(NifPath, Info).
```

### Appup Cookbook recipes

#### Functional module change

```erlang
{"2",
 [{"1", [{load_module, m}]}],
 [{"1", [{load_module, m}]}]
}.
```

#### Add/delete module

```erlang
{"2",
 [{"1", [{add_module, m}]}],
 [{"1", [{delete_module, m}]}]
}.
```

#### Supervisor child add/delete

```erlang
{"2",
 [{"1",
   [{update, ch_sup, supervisor},
    {apply, {supervisor, restart_child, [ch_sup, m1]}}
   ]}],
 [{"1",
   [{apply, {supervisor, terminate_child, [ch_sup, m1]}},
    {apply, {supervisor, delete_child, [ch_sup, m1]}},
    {update, ch_sup, supervisor}
   ]}]
}.
```

#### gen_server state migration (code_change/3)

appup:

```erlang
{"2",
 [{"1", [{update, ch3, {advanced, []}}]}],
 [{"1", [{update, ch3, {advanced, []}}]}]
}.
```

callback:

```erlang
code_change({down, _Vsn}, {Chs, N}, _Extra) ->
    {ok, Chs};
code_change(_Vsn, Chs, _Extra) ->
    {ok, {Chs, 0}}.
```

#### Special process (system_code_change/4)

child spec:

```erlang
{ch4, {ch4, start_link, []},
 permanent, brutal_kill, worker, [ch4]}
```

appup:

```erlang
{"2",
 [{"1", [{update, ch4, {advanced, []}}]}],
 [{"1", [{update, ch4, {advanced, []}}]}]
}.
```

callback:

```erlang
system_code_change(Chs, _Module, _OldVsn, _Extra) ->
    {ok, Chs}.
```

## Common mistakes

1. **Using `load_module` for a residence module that changed state format.** Fix: use `update` with `{advanced, Extra}` so `code_change/3` runs.
2. **Forgetting `add_module` in embedded mode.** Fix: newly introduced modules need `add_module`; the code server does not auto-load in embedded mode.
3. **Calling `delete_module` while processes still run in it.** Fix: terminate all residence-module processes first; `delete_module` kills them and can break supervisor restarts.
4. **Not making the release permanent after `restart_new_emulator`.** Fix: call `make_permanent/1` once the new runtime is operational, else a reboot reverts.
5. **Using multiple config files in release mode.** Fix: use exactly one `sys.config`; use the include-file mechanism for composition.
6. **Expecting `set_env` values to survive release install.** Fix: on install, config priority is `App.app` → `sys.config` → CLI flags; `set_env` values are disregarded.
7. **Using `purge/1` when you cannot afford to kill processes.** Fix: use `soft_purge/1` to refuse the upgrade rather than terminate lingering processes.
8. **Loading a third instance without purging.** Fix: the code server auto-purges old code on third load, killing lingering processes; use `soft_purge` first if that is unacceptable.
9. **Using `load_module` for a residence module whose state format changed.** Fix: use `update` with `{advanced,Extra}`.
10. **Omitting residence module names from child spec `Modules` for special processes.** Fix: the release handler cannot find the process; list the residence module name(s).
11. **Forgetting `code_change/3` downgrade clause (`{down, Vsn}`).** Fix: state migration only works one direction.
12. **Wrong instruction order for supervisor child add/delete.** Fix: order matters (upgrade: update → start; downgrade: terminate → delete → update).
13. **Writing a `.appup` for application add/remove.** Fix: not needed — auto-generated from `.rel` comparison.
14. **Changing child spec `id` via supervisor update.** Fix: not possible — `id` cannot be changed.

## Strict vs contextual guidance

### Strict

- Release handling MUST run in embedded mode.
- Exactly one `sys.config` MUST be used.
- All release versions except the first MUST contain a `relup`.
- `restart_new_emulator` MUST be the first relup instruction.
- At most one `restart_emulator`, placed at the end.
- `heart` MUST be started for reboot-based upgrades.
- Code replacement is module-level only; at most two versions (current + old) coexist.
- `on_load` MUST return `ok` or the new code is unloaded; in embedded mode, all `on_load` functions must return `ok` or the system terminates.
- `load_module` only for functional modules; `update` with `{advanced,Extra}` for residence modules.
- Residence module names MUST be in child spec `Modules` for special processes.
- Child spec `id` cannot be changed via supervisor update.
- Instruction order matters for supervisor child add/delete.
- No `.appup` needed for application add/remove or spec changes.

### Convention

- Keep code changes small and backwards compatible.
- Generate `relup` with `systools:make_relup` rather than hand-writing.
- Use `soft_purge/1` before `delete/1` to avoid killing processes.
- Set `modules => [Module]` in child specs so `update` can find processes.

### Contextual

- Whether to support downgrade paths (both up and down in `.appup`).
- Exact `code_change/3` state transformation logic.
- Whether to use `restart_emulator` for non-core upgrades.
- Whether to support downgrade paths (both `code_change/3` clauses).
- Whether `vsn` attribute is set for meaningful `Vsn` in `code_change`.
- Whether included applications use hand-written `.relup` or application restart.

### Policy

- Whether release handling (SASL) is used at all, or only static releases.
- Whether `heart` is mandatory in production.
- Upgrade/downgrade cadence and rollback policy.
- Whether `on_load` NIF initialization is permitted and under what conditions.

## Policy decisions for individual repos

1. **Release handling usage.** Is SASL `release_handler` used for hot upgrades, or are releases replaced by full restart?
2. **`heart` policy.** Is `heart` mandatory in production for automatic reboot?
3. **Downgrade support.** Must every `.appup` provide both upgrade and downgrade instructions?
4. **`code_change` discipline.** Is `code_change/3` required for all behaviour callbacks, or only when state format changes?
5. **Config file strategy.** Is `sys.config` the sole config source, or are includes used?
6. **`on_load`/NIF policy.** Are `on_load` NIF initializers permitted, and what happens on failure?
7. **`code_change/3` downgrade support.** Is downgrade support mandatory for all behaviour callbacks?
8. **`vsn` module attributes.** Are `vsn` module attributes required for all upgradeable modules?
9. **Included applications.** Are included applications permitted (requiring hand-written `.relup`)?
10. **Port-program upgrades.** Do port-program upgrades ship `priv` dirs in release tarballs?

## Related docs

- [applications](applications.md)
- [logger-and-config](logger-and-config.md)
- [supervision](supervision.md) — child spec `Modules` field, supervisor restart strategies.
- [proc-lib-and-sys](proc-lib-and-sys.md) — special processes, `system_code_change/4`.

## Related skills

- `beam-applications-releases`
