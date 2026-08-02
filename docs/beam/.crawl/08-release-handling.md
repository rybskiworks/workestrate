# Crawl: release_handling.html
- seed_url: https://www.erlang.org/doc/system/release_handling.html
- canonical_url: https://www.erlang.org/doc/system/release_handling.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2; major-vsn 29)
- feeds_docs: releases.md

## Purpose
Defines OTP release handling: the SASL framework for upgrading/downgrading an
entire release at runtime, built on Erlang's code replacement feature. Covers
the workflow (offline `systools` generation of scripts/packages; online
`release_handler` unpack/install/make_permanent), the `.appup` and `relup` file
formats, the release-handling instruction set (high-level translated to
low-level by `systools:make_relup`), requirements (embedded system, heart,
sys.config, relup), distributed-systems synchronization, and application
specification updates (`config_change`).

## Key concepts
- **Code replacement** — Erlang runtime can change module code at runtime
  (see Code Replacement in Erlang Reference Manual). Foundation of release
  handling.
- **Release handling** — SASL framework for upgrading/downgrading between
  versions of an entire release in runtime.
- **Offline support** — `systools` generates scripts and builds release
  packages.
- **Online support** — `release_handler` unpacks and installs release
  packages.
- **Minimal release-handling system** — Kernel + STDLIB + SASL.
- **Residence module** — module where a process has its tail-recursive loop
  function(s). If implemented across several modules, all are residence
  modules. For an OTP behaviour process, the behaviour module is the residence
  module.
- **Functional module** — a module that is not a residence module for any
  process. For a behaviour process, the callback module is a functional module.
- **Simple code replacement** — load new version + remove old version of a
  functional module (`load_module`).
- **Synchronized code replacement** — suspend → transform state/switch code →
  remove old → resume (`update` with `{advanced,Extra}` or `supervisor`).
- **Two module versions** — the code server keeps two versions of a module;
  release handling loads new and removes old.
- **`sync_nodes`** instruction synchronizes release handler processes across
  nodes (distributed systems).
- **`heart`** — heartbeat monitoring program; required for reboot-based
  upgrades (`restart_new_emulator`, `restart_emulator`).
- **Target system** — installed system with `$ROOT`; release packages copied
  to `$ROOT/releases`.

## Strict rules / invariants
- Runtime system must know which release it is running and be able to change
  boot script + system config at runtime on reboot → **Erlang must be started
  as an embedded system**.
- System must be started with **heartbeat monitoring** (`heart`) for reboots
  to work properly.
- Boot script in a release package must be generated from the **same `.rel`
  file** as the release package itself (application info is fetched from the
  script during upgrade/downgrade).
- System must be configured using **only one** system configuration file:
  `sys.config` (auto-included in release package if found).
- All release versions except the first must contain a **`relup`** file
  (auto-included if found).
- `restart_new_emulator` must always be the **first** instruction in a relup
  (auto-enforced by `systools:make_relup/3,4`).
- A relup may contain **only one** `restart_emulator` instruction, and it
  must be placed at the **end** (auto-enforced by `systools:make_relup/3,4`).
- `restart_new_emulator` requires heartbeat monitoring.
- After `restart_new_emulator`, the new release version **must be made
  permanent** once the new runtime system is operational, else old version is
  used on next reboot.
- On UNIX during `restart_new_emulator`, `HEART_COMMAND` env var is ignored;
  command defaults to `$ROOT/bin/start`; overridable via SASL param
  `start_prg`.
- `delete_module` kills any process (in any application) with `Module` as
  residence module → user must ensure all such processes are terminated
  **before** deleting the module to avoid failing supervisor restarts.
- `add_module` is **necessary in embedded mode**; not strictly required in
  interactive mode (code server auto-loads).
- Application specs are auto-updated for all loaded applications on install;
  config priority (increasing): boot-script data (from `App.app`) → new
  `sys.config` → `-App Par Val` command-line args. Values from other config
  files and `application:set_env/3` are **disregarded**.
- Recommendation: change code in as small steps as possible and always keep
  backwards compatible.

## Examples
- `ch_rel` example: add `available/0` to `ch3` gen_server callback.
  - `ch_app.app` version bumped `"1"`→`"2"`.
  - `ch_app.appup`:
    ```
    {"2",
     [{"1", [{load_module, ch3}]}],
     [{"1", [{load_module, ch3}]}]
    }.
    ```
  - `ch_rel-2.rel` release version `"A"`→`"B"`, ERTS 14.2.5, kernel 9.2.4,
    stdlib 5.2.3, sasl 4.2.1, ch_app "2".
  - Generate relup:
    ```
    1> systools:make_relup("ch_rel-2", ["ch_rel-1"], ["ch_rel-1"]).
    ok
    ```
    With path option:
    ```
    1> systools:make_relup("ch_rel-2", ["ch_rel-1"], ["ch_rel-1"],
       [{path,["../ch_rel-1","../ch_rel-1/lib/ch_app-1/ebin"]}]).
    ok
    ```
  - Build package:
    ```
    1> systools:make_script("ch_rel-2").
    ok
    2> systools:make_tar("ch_rel-2").
    ok
    ```
  - Install sequence in running target:
    ```
    1> release_handler:unpack_release("ch_rel-2").
    {ok,"B"}
    3> release_handler:install_release("B").
    {ok,"A",[]}
    7> release_handler:make_permanent("B").
    ok
    ```
  - After install, `ch3:available/0` returns 3; `code:which(ch3)` →
    `.../lib/ch_app-2/ebin/ch3.beam`; `code:which(ch_sup)` still
    `.../lib/ch_app-1/ebin/ch_sup.beam` (supervisor code not updated).
- `sys.config` for empty config: `[].`

## Behaviour / callback details

### appup / relup
- **`.appup` file** named `Application.appup`, placed in `ebin` directory.
  Structure:
  ```
  {Vsn,
   [{UpFromVsn1, InstructionsU1}, ..., {UpFromVsnK, InstructionsUK}],
   [{DownToVsn1, InstructionsD1}, ..., {DownToVsnK, InstructionsDK}]}.
  ```
  - `Vsn` (string) = current version per `.app` file.
  - `UpFromVsn` = previous version to upgrade from.
  - `DownToVsn` = previous version to downgrade to.
  - `Instructions` = list of release-handling instructions.
  - `UpFromVsn`/`DownToVsn` may be regular expressions.
- **`relup` file** — generated by `systools:make_relup/3,4` from `.rel`,
  `.app`, `.appup` inputs; deduces added/deleted/upgraded/downgraded apps and
  transforms `.appup` instructions into a single ordered list of low-level
  instructions. Can be created manually if simple (low-level instructions
  only).

### Release instructions: low-level vs high-level
- OTP supports a set of release-handling instructions used in `.appup` files.
- The release handler understands only a subset: the **low-level**
  instructions.
- **High-level** instructions are translated to low-level by
  `systools:make_relup`. Complete list in `appup` (SASL).

### Instruction set (verbatim forms)
- `{load_module, Module}` — simple code replacement for a functional module
  (load new, remove old).
- `{update, Module, {advanced, Extra}}` — synchronized code replacement;
  behaviour processes call `code_change/3` with `Extra` + other info. Used
  when changing internal state of a behaviour.
- `{update, Module, supervisor}` — used when changing the start
  specification of a supervisor.
- `{add_module, Module}` — loads a newly introduced module (required in
  embedded mode).
- `{delete_module, Module}` — unloads a module; kills any process with
  `Module` as residence module.
- `{add_application, Application}` — loads modules (via `add_module`
  instructions) per `modules` key in `.app`, then starts the application.
- `{remove_application, Application}` — stops application, unloads modules
  (via `delete_module`), unloads app spec from application controller.
- `{restart_application, Application}` — stop then start (like
  `remove_application` + `add_application` in sequence).
- `{apply, {M, F, A}}` — low-level; release handler evaluates `apply(M, F, A)`.
- `restart_new_emulator` (low-level) — for new ERTS version or upgrade of
  Kernel/STDLIB/SASL core apps. Must be first instruction in relup. Generates
  a temporary boot file (new runtime + core apps, old other apps), calls
  `init:reboot/0`, system rebooted by `heart` using temp boot file; rest of
  relup executed after reboot as part of temp boot script. Requires heart.
- `restart_emulator` (low-level) — unrelated to ERTS/core upgrades; any app
  can use it to force runtime restart after all upgrade instructions. Only
  one allowed, must be at end. Calls `init:reboot/0`; no more instructions
  executed after restart.

### update mechanics (suspend / resume / change_code)
- When a module is updated, the release handler finds processes **using** the
  module by traversing each running application's supervision tree and
  checking all child specs:
  ```
  {Id, StartFunc, Restart, Shutdown, Type, Modules}
  ```
  A process uses a module if the name is listed in `Modules` of its child
  spec.
- If `Modules=dynamic` (event managers), the event manager process informs
  the release handler of currently installed event handlers (`gen_event`);
  membership checked against that list instead.
- Release handler suspends, asks for code change, and resumes processes by
  calling:
  - `sys:suspend/1,2`
  - `sys:change_code/4,5`
  - `sys:resume/1,2`
- `update` with `{advanced,Extra}` causes behaviour processes to call
  `code_change/3`, passing `Extra` and other information.

### release_handler sequence (unpack → install → make_permanent)
- `release_handler:unpack_release(ReleaseName) => {ok, Vsn}`
  - `ReleaseName` = release package name without `.tar.gz`.
  - `Vsn` = version per `.rel` file.
  - Creates `$ROOT/lib/releases/Vsn` with `.rel`, `start.boot`,
    `sys.config`, `relup`. New-version app dirs placed under `$ROOT/lib`;
    unchanged apps not affected.
- `release_handler:install_release(Vsn) => {ok, FromVsn, []}`
  - Evaluates `relup` instructions step by step.
  - On error → system rebooted using old release version.
  - On success → system uses new version, but a reboot reverts to previous
    version until made permanent.
  - Downgrade: call `install_release(FromVsn) => {ok, Vsn, []}`.
- `release_handler:make_permanent(Vsn) => ok`
  - New version becomes default; previous version becomes "old".
  - Version state tracked in `$ROOT/releases/RELEASES` and
    `$ROOT/releases/start_erl.data`.
- `release_handler:remove_release(Vsn) => ok`
  - Removes an installed-but-not-permanent release: deletes info from
    `RELEASES`, removes new app dirs and `$ROOT/releases/Vsn` directory.
- `release_handler:which_releases(current)` — programmatically check if
  upgrade is complete (returns expected new release).

### config_change callback
- After install, application controller compares old/new config params for
  all running applications and calls:
  ```
  Module:config_change(Changed, New, Removed)
  ```
  - `Module` = application callback module per `mod` key in `.app`.
  - `Changed`/`New` = lists of `{Par,Val}` for changed/added params.
  - `Removed` = list of removed params `Par`.
  - Optional; may be omitted in application callback module.
- When installed release is made permanent, `init` is set to point at the new
  `sys.config`.

### Target systems / heart
- First target system install: see System Principles (`create_target.html`).
- Release package copied to `$ROOT/releases`.
- `heart` required for reboot-based upgrades; on UNIX `HEART_COMMAND` ignored
  during `restart_new_emulator` (defaults `$ROOT/bin/start`, overridable via
  SASL `start_prg`).

## Verbatim quotes
1. (Release Handling Principles) "An important feature of the Erlang
   programming language is the ability to change module code at runtime, code
   replacement, as described in Code Replacement in the Erlang Reference
   Manual."
2. (Release Handling Principles) "Based on this feature, the OTP application
   SASL provides a framework for upgrading and downgrading between different
   versions of an entire release in runtime. This is called release
   handling."
3. (Release Handling Principles) "The framework consists of: Offline support
   - systools for generating scripts and building release packages; Online
   support - release_handler for unpacking and installing release packages"
4. (Release Handling Principles) "The minimal system based on Erlang/OTP,
   enabling release handling, thus consists of the Kernel, STDLIB, and SASL
   applications."
5. (Release Handling Aspects) "It is thus recommended that code is changed in
   as small steps as possible, and always kept backwards compatible."
6. (Requirements) "For release handling to work properly, the runtime system
   must have knowledge about which release it is running. It must also be
   able to change (in runtime) which boot script and system configuration
   file to use if the system is rebooted, for example, by heart after a
   failure. Thus, Erlang must be started as an embedded system..."
7. (Requirements) "The boot script included in a release package must be
   generated from the same .rel file as the release package itself."
8. (Requirements) "The system must be configured using only one system
   configuration file, called sys.config."
9. (Requirements) "All versions of a release, except the first one, must
   contain a relup file."
10. (Distributed Systems) "If the system consists of several Erlang nodes,
    each node can use its own version of the release. The release handler is
    a locally registered process and must be called at each node where an
    upgrade or downgrade is required."
11. (Release Handling Instructions) "OTP supports a set of release handling
    instructions that are used when creating .appup files. The release
    handler understands a subset of these, the low-level instructions. To make
    it easier for the user, there are also a number of high-level
    instructions, which are translated to low-level instructions by
    systools:make_relup."
12. (Release Handling Instructions) "Residence module - The module where a
    process has its tail-recursive loop function(s). If these functions are
    implemented in several modules, all those modules are residence modules
    for the process."
13. (Release Handling Instructions) "Functional module - A module that is not
    a residence module for any process."
14. (Release Handling Instructions) "For a process implemented using an OTP
    behaviour, the behaviour module is the residence module for that process.
    The callback module is a functional module."
15. (load_module) "If a simple extension has been made to a functional module,
    it is sufficient to load the new version of the module into the system,
    and remove the old version. This is called simple code replacement..."
16. (update) "If a more complex change has been made, for example, a change to
    the format of the internal state of a gen_server, simple code replacement
    is not sufficient. Instead, it is necessary to: Suspend the processes
    using the module...; Ask them to transform the internal state format and
    switch to the new version of the module.; Remove the old version.; Resume
    the processes."
17. (update) "update with argument {advanced,Extra} is used when changing the
    internal state of a behaviour as described above. It causes behaviour
    processes to call the callback function code_change/3, passing the term
    Extra and some other information as arguments."
18. (update) "A process uses a module if the name is listed in Modules in the
    child specification for the process."
19. (update) "If Modules=dynamic, which is the case for event managers, the
    event manager process informs the release handler about the list of
    currently installed event handlers (gen_event)..."
20. (update) "The release handler suspends, asks for code change, and resumes
    processes by calling the functions sys:suspend/1,2, sys:change_code/4,5,
    and sys:resume/1,2, respectively."
21. (add_module and delete_module) "This instruction loads module Module. When
    running Erlang in embedded mode it is necessary to use this instruction.
    It is not strictly required when running Erlang in interactive mode,
    since the code server automatically searches for and loads unloaded
    modules."
22. (add_module and delete_module) "Any process, in any application, with
    Module as residence module, is killed when the instruction is evaluated.
    Therefore, the user must ensure that all such processes are terminated
    before deleting module Module to avoid a situation with failing
    supervisor restarts."
23. (restart_new_emulator) "This instruction is used when changing to a new
    version of the runtime system, or when any of the core applications
    Kernel, STDLIB, or SASL is upgraded. If a system reboot is needed for
    another reason, the restart_emulator instruction is to be used instead."
24. (restart_new_emulator) "The restart_new_emulator instruction must always
    be the first instruction in a relup."
25. (restart_new_emulator) "When the release handler encounters this
    instruction, it first generates a temporary boot file that starts the new
    versions of the runtime system and the core applications, and the old
    version of all other applications. Then it shuts down the current instance
    of the runtime system by calling init:reboot/0. All processes are
    terminated gracefully and the system is rebooted by the heart program,
    using the temporary boot file. After the reboot, the rest of the relup
    instructions are executed."
26. (restart_new_emulator warning) "This mechanism causes the new versions of
    the runtime system and core applications to run with the old version of
    other applications during startup. Thus, take extra care to avoid
    incompatibility."
27. (restart_new_emulator) "The new release version must be made permanent
    when the new runtime system is operational. Otherwise, the old version
    will be used if there is a new system reboot."
28. (restart_emulator) "A relup script can only contain one restart_emulator
    instruction, and it must always be placed at the end."
29. (Application Upgrade File) "UpFromVsn and DownToVsn can also be specified
    as regular expressions."
30. (Release Upgrade File) "This file does not need to be created manually. It
    can be generated by systools:make_relup/3,4. The relevant versions of the
    .rel file, .app files, and .appup files are used as input."
31. (Installing a Release) "To install the new version of the release in
    runtime, the release handler is used. This is a process belonging to the
    SASL application, which handles unpacking, installation, and removal of
    release packages."
32. (Installing a Release) "If an error occurs during the installation, the
    system is rebooted using the old version of the release. If installation
    succeeds, the system is afterwards using the new version of the release,
    but if anything happens and the system is rebooted, it starts using the
    previous version again."
33. (Installing a Release) "The system keeps information about which versions
    are old and permanent in the files $ROOT/releases/RELEASES and
    $ROOT/releases/start_erl.data."
34. (Updating Application Specifications) "When a new version of a release is
    installed, the application specifications are automatically updated for all
    loaded applications."
35. (Updating Application Specifications) "Specifically, the application
    configuration parameters are automatically updated according to (in
    increasing priority order): The data in the boot script, fetched from the
    new application resource file App.app; The new sys.config; Command-line
    arguments -App Par Val"
36. (Updating Application Specifications) "This means that parameter values
    set in the other system configuration files and values set using
    application:set_env/3 are disregarded."

## Version notes
- Page meta: Erlang System Documentation v29.0.2; OTP 29.0.2; major-vsn 29.
- ExDoc v0.40.3.
- Example `.rel` references: ERTS 14.2.5, kernel 9.2.4, stdlib 5.2.3, sasl
  4.2.1 (consistent with OTP 29).
- Copyright © 1996-2026 Ericsson AB.
- Source: github.com/erlang/otp @ OTP-29.0.2,
  system/doc/design_principles/release_handling.md.

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/system/code_loading.html#code-replacement — Code Replacement (Erlang Reference Manual)
- https://www.erlang.org/doc/system/release_structure.html — Releases (release structure / .rel)
- https://www.erlang.org/doc/system/release_structure.html#ch_rel — ch_rel example
- https://www.erlang.org/doc/system/create_target.html — System Principles / create target (first target system)
- https://www.erlang.org/doc/system/appup_cookbook.html — Appup Cookbook
- https://www.erlang.org/doc/system/appup_cookbook.html#int_state — internal state change
- https://www.erlang.org/doc/system/appup_cookbook.html#sup — supervisor change
- https://www.erlang.org/doc/apps/sasl/appup.html — appup (SASL): full instruction list + sync_nodes
- https://www.erlang.org/doc/apps/sasl/relup.html — relup (SASL): relup file syntax/contents
- https://www.erlang.org/doc/apps/sasl/release_handler.html — release_handler module (SASL)
- https://www.erlang.org/doc/apps/sasl/release_handler.html#which_releases/1 — which_releases(current)
- https://www.erlang.org/doc/apps/sasl/systools.html#make_relup/4 — systools:make_relup/3,4
- https://www.erlang.org/doc/apps/sasl/sasl_app.html — SASL app (start_prg config param)
- https://www.erlang.org/doc/apps/stdlib/gen_server.html — gen_server
- https://www.erlang.org/doc/apps/erts/erlang.html#apply/3 — erlang:apply/3
- https://www.erlang.org/doc/apps/erts/erl_cmd.html — erl command (ERTS)
- https://www.erlang.org/doc/apps/erts/init.html#reboot/0 — init:reboot/0
- https://www.erlang.org/doc/apps/kernel/heart.html — heart (Kernel)
- https://www.erlang.org/doc/apps/kernel/application.html#set_env/3 — application:set_env/3
- https://www.erlang.org/doc/system/release_handling.md — Markdown source of this page

### Skipped
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/release_handling.md#L1 — view source (GitHub)
- https://www.erlang.org/doc/search.html?v=29&q= — search
- https://www.erlang.org/doc/system/llms.txt — llms.txt index
- https://www.erlang.org/doc/system/Erlang%20System%20Documentation.epub — ePub download
- https://github.com/elixir-lang/ex_doc — ExDoc tooling
- https://www.erlang.org — Erlang home
- https://www.ericsson.com — Ericsson
- /assets/css/algolia-typeahead.css, /assets/js/algolia-typeahead.bundle.js, dist/html-erlang-KCHZLXSC.css, dist/sidebar_items-D241B7A5.js, docs_config.js, dist/html-Y2MUTVIN.js, assets/logo.png — site assets/scripts
- ../index.html — sidebar project root
