# Crawl: system/appup_cookbook.html
- seed_url: https://www.erlang.org/doc/system/appup_cookbook.html
- canonical_url: https://www.erlang.org/doc/system/appup_cookbook.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: 29.0.2 (Erlang System Documentation v29.0.2)
- feeds_docs: releases.md

## Purpose

The Appup Cookbook provides ready-to-use `.appup` file examples for the typical
cases of runtime upgrades/downgrades performed by the OTP release handler. It is
a recipe catalog: for each common change scenario it gives the exact appup
instruction(s) and, where relevant, the callback function the developer must
implement. It assumes familiarity with Release Handling and the OTP design
principles (supervisor, gen_server, gen_statem, gen_event, gen_fsm behaviours).

The central distinction it enforces is between:
- **Functional module** — a module containing only functions (no resident
  process). Simple code replacement suffices.
- **Residence module** — a module whose code is executing inside a long-lived
  process (a behaviour process or a "special process"). Simple replacement is
  *not* sufficient; synchronized code replacement is required so the process
  switches to the new code and, if needed, transforms its internal state.

In an OTP-structured system all processes except system processes and special
processes reside in one of the STDLIB behaviours; upgrading those behaviours
normally requires a runtime system restart, so OTP gives no support for changing
residence modules except for special processes.

## Upgrade patterns (each: scenario → appup instruction recipe)

### Changing a functional module

- **Scenario:** a functional module changed (new function added, bug fixed) but
  no resident process state is involved.
- **Recipe:** simple code replacement with `load_module`.
- **appup:**
  ```erlang
  {"2",
   [{"1", [{load_module, m}]}],
   [{"1", [{load_module, m}]}]
  }.
  ```
- **Note:** a callback module is a functional module; for pure code extension
  the same `load_module` recipe applies (e.g. adding `available/0` to `ch3`).

### Adding/deleting a module

- **Scenario:** a new functional module `m` is introduced in version "2" (and
  removed on downgrade).
- **Recipe:** `add_module` on upgrade, `delete_module` on downgrade.
- **appup:**
  ```erlang
  {"2",
   [{"1", [{add_module, m}]}],
   [{"1", [{delete_module, m}]}]
  }.
  ```
- **Module dependencies:** if module `m1` calls a new function in `ch3`, then
  `ch3` must be loaded *before* `m1` on upgrade (and the reverse on downgrade).
  Express this with the `DepMods` element:
  ```erlang
  {load_module, Module, DepMods}
  {update, Module, {advanced, Extra}, DepMods}
  ```
  `DepMods` is the list of modules that `Module` depends on. `systools` knows
  the up/down direction and generates a correct `relup` (loads `ch3` before
  `m1` when upgrading, `m1` before `ch3` when downgrading). When the dependent
  modules live in the same application, list both in one `.appup` in load order.

### Changing a supervisor

The supervisor behaviour supports changing its internal state — restart
strategy, max restart frequency, and existing child specifications — but child
add/delete is *not* automatic; explicit `apply` instructions are required.

#### Changing properties (restart strategy, intensity)
- **Recipe:** synchronized replacement using the special supervisor update
  instruction. The new callback module is loaded first (both up and down), then
  the new `init/1` return value is applied.
- **Instruction:** `{update, Module, supervisor}`
- **appup (change `ch_sup` from `one_for_one` to `one_for_all`):**
  ```erlang
  {"2",
   [{"1", [{update, ch_sup, supervisor}]}],
   [{"1", [{update, ch_sup, supervisor}]}]
  }.
  ```

#### Changing child specifications
- Same instruction `{update, ch_sup, supervisor}`. Changes do **not** affect
  existing child processes (e.g. changing the start function only affects future
  restarts). The child spec **id cannot be changed**. Changing the `Modules`
  field can affect release handling itself, since it identifies which processes
  are affected by synchronized code replacement.

#### Adding and deleting child processes
- New child specs are auto-added but **not** auto-deleted; children are not
  auto-started/terminated — use `apply` instructions. **Order matters.**
- **appup (add child `m1` on upgrade, delete on downgrade):**
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
- If module `m1` is newly introduced it must also be loaded/deleted:
  ```erlang
  {"2",
   [{"1",
     [{add_module, m1},
      {update, ch_sup, supervisor},
      {apply, {supervisor, restart_child, [ch_sup, m1]}}
     ]}],
   [{"1",
     [{apply, {supervisor, terminate_child, [ch_sup, m1]}},
      {apply, {supervisor, delete_child, [ch_sup, m1]}},
      {update, ch_sup, supervisor},
      {delete_module, m1}
     ]}]
  }.
  ```
- The supervisor must be registered (here `ch_sup`) for direct script access;
  otherwise a helper function finding the pid must be called via `apply`.

### Changing a gen_server (code_change state migration)

- **Scenario:** the internal state format must change (e.g. `Chs` → `{Chs,N}`).
  Simple code replacement is insufficient; the process must transform its state
  via the `code_change/3` callback before switching to the new module version.
  This is **synchronized code replacement**.
- **Recipe:** `{update, Module, {advanced, Extra}}`. The `{advanced, Extra}`
  tuple tells the affected processes to perform a state transformation before
  loading the new version, by calling `code_change/3`.
- **appup (add counter `N` to `ch3` state):**
  ```erlang
  {"2",
   [{"1", [{update, ch3, {advanced, []}}]}],
   [{"1", [{update, ch3, {advanced, []}}]}]
  }.
  ```
- **Callback to implement:**
  ```erlang
  -module(ch3).
  ...
  -export([code_change/3]).
  ...
  code_change({down, _Vsn}, {Chs, N}, _Extra) ->
      {ok, Chs};
  code_change(_Vsn, Chs, _Extra) ->
      {ok, {Chs, 0}}.
  ```
- **Semantics:** first arg is `{down, Vsn}` on downgrade, else `Vsn` on
  upgrade. `Vsn` comes from the *original* module version (the one upgrading
  from / downgrading to), defined by the `vsn` module attribute; absent `vsn`,
  it is the beam checksum (an uninteresting huge integer, usually ignored).
  `Extra` is passed through unchanged to `code_change/3`.

### Changing code a process is running

Covers two sub-cases: **special processes** (user-defined residence modules) and
**non-Erlang code** (port programs). Both require synchronized replacement.

#### Special process (residence module change)
- **Scenario:** a special process's residence module changes. Simple replacement
  is insufficient; the process must make a fully-qualified call to its loop
  function to switch to the new code → synchronized replacement.
- **Rule:** the user-defined residence module name(s) **must** be listed in the
  `Modules` part of the child spec, or the release handler cannot find the
  process.
- **Recipe:** `{update, Module, {advanced, Extra}}` — makes the special process
  call `system_code_change/4`, which the user must implement.
- **Child spec example:**
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
  Args: (1) internal `State` from
  `sys:handle_system_msg(Request, From, Parent, Module, Deb, State)`; (2) module
  name; (3) `Vsn` or `{down, Vsn}` (as for `code_change/3`); (4) `Extra`. If
  only code was extended, return state unchanged; if state changed, transform
  here and return `{ok, Chs2}`.

#### Non-Erlang code (port program)
- Application-dependent; OTP provides no special support. Pattern: the
  controlling `gen_server` implements `code_change/3` that closes the old port
  and opens a new one (optionally fetching data to preserve first).
- **appup:**
  ```erlang
  ["2",
   [{"1", [{update, portc, {advanced,port}}]}],
   [{"1", [{update, portc, {advanced,port}}]}]
  ].
  ```
- Include the `priv` dir (where the C program lives) in the release tarball:
  `systools:make_tar("my_release", [{dirs,[priv]}]).`

## Strict rules

1. **Residence vs functional:** simple `load_module` only works for functional
   modules. Any module whose code runs inside a live process (behaviour process
   or special process) requires synchronized replacement (`update` with
   `{advanced, Extra}` or `supervisor`).
2. **Child spec `Modules` field is mandatory** for special processes — the
   release handler uses it to locate affected processes during synchronized
   replacement. Omitting it breaks release handling.
3. **Child spec `id` cannot be changed** via supervisor update.
4. **Order of instructions matters** — especially for supervisor child
   add/delete and module add/delete. On upgrade: load module → update supervisor
   → start child. On downgrade: terminate child → delete child spec → update
   supervisor → delete module.
5. **`DepMods` direction:** `systools` auto-handles up/down ordering; list the
   dependency, not the manual order, in `DepMods`.
6. **`vsn` attribute** determines the `Vsn` passed to `code_change` /
   `system_code_change`; without it the beam checksum is used.
7. **Supervisor must be registered** to be addressed directly by `apply`
   scripts; otherwise write a helper to locate the pid.
8. **Application spec / config changes** need *no* appup instructions —
   application specs are auto-updated before the relup script runs; config can
   also go in `sys.config`.
9. **Adding/removing applications** needs no `.appup` — `add_application` /
   `remove_application` are auto-generated by comparing `.rel` files.
10. **Included applications** have no dedicated release-handling instructions;
    the `.relup` must be hand-written (load/unload application + supervisor
    child manipulation).
11. **Runtime restart instructions:** `restart_new_emulator` (for ERTS/Kernel/
    STDLIB/SASL upgrades, auto-added, runs before all other instructions) and
    `restart_emulator` (runs after all other instructions). If only a restart
    is needed, a minimal hand-written `.relup` with `restart_emulator` suffices
    and no `.appup` files are required.

## Examples

- **Functional module change:** `[{load_module, m}]` both directions.
- **Callback module extension (add `available/0` to `ch3`):** `[{load_module, ch3}]`.
- **gen_server state migration (add counter `N`):** `[{update, ch3, {advanced, []}}]`
  with `code_change/3` returning `{ok, {Chs, 0}}` (upgrade) / `{ok, Chs}` (down).
- **Cross-application dependency (`m1` depends on `ch3`):**
  `[{load_module, m1, [ch3]}]` in `myapp.appup`; `[{load_module, ch3}]` in
  `ch_app.appup`.
- **Same-app dependency:** list both in load order in one `.appup`.
- **Supervisor restart-strategy change:** `[{update, ch_sup, supervisor}]`.
- **Add+delete supervisor child `m1`:** upgrade =
  `[update supervisor, apply restart_child]`; downgrade =
  `[apply terminate_child, apply delete_child, update supervisor]`.
- **Add+delete child with new module `m1`:** upgrade =
  `[add_module m1, update supervisor, apply restart_child]`; downgrade =
  `[apply terminate_child, apply delete_child, update supervisor, delete_module m1]`.
- **Add a functional module:** upgrade `[{add_module, m}]`, downgrade `[{delete_module, m}]`.
- **Restart application (alternative to supervisor child update):**
  `[{restart_application, ch_app}]` both directions.
- **Application spec change:** empty instructions `[[]]` both directions.
- **Special process `ch4`:** `[{update, ch4, {advanced, []}}]` with
  `system_code_change/4`.
- **Port program `portc`:** `[{update, portc, {advanced,port}}]` with
  `code_change/3` closing/reopening the port; ship `priv` via
  `systools:make_tar("my_release", [{dirs,[priv]}])`.
- **Included application via application restart:** hand-written `.relup`
  using `load`/`remove`/`purge`/`application,load`/`application,start` instead
  of auto-generated start/stop.
- **Included application via supervisor change:** hand-written `.relup`
  combining `suspend`/`load`/`code_change`/`resume`/`restart_child` (upgrade)
  and the reverse `terminate_child`/`delete_child`/`suspend`/`load`/`code_change,
  down`/`resume`/`remove`/`purge`/`application,unload` (downgrade).
- **Emulator-only restart:** `[{[], [restart_emulator]}]` both directions.

## Verbatim quotes

> "This section includes examples of .appup files for typical cases of
> upgrades/downgrades done in runtime."

> "In a system implemented according to the OTP design principles, all
> processes, except system processes and special processes, reside in one of the
> behaviours supervisor, gen_server, gen_statem, gen_event, or gen_fsm. These
> belong to the STDLIB application and upgrading/downgrading normally requires a
> runtime system restart. Thus, OTP provides no support for changing residence
> modules except in the case of special processes."

> "In this case, simple code replacement is not sufficient. The process must
> explicitly transform its state using the callback function code_change/3
> before switching to the new version of the callback module. Thus, synchronized
> code replacement is used."

> "The third element of the update instruction is a tuple {advanced,Extra},
> which says that the affected processes are to do a state transformation before
> loading the new version of the module."

> "The first argument is {down,Vsn} if there is a downgrade, or Vsn if there is
> an upgrade. The term Vsn is fetched from the 'original' version of the module,
> that is, the version you are upgrading from, or downgrading to."

> "m1 is said to be dependent on ch3. In a release handling instruction, this is
> expressed by the DepMods element ... systools knows the difference between up-
> and downgrading and generates a correct relup, where ch3 is loaded before m1
> when upgrading, but m1 is loaded before ch3 when downgrading."

> "When a new version of a residence module for a special process is loaded, the
> process must make a fully qualified call to its loop function to switch to the
> new code. Thus, synchronized code replacement must be used."

> "The name(s) of the user-defined residence module(s) must be listed in the
> Modules part of the child specification for the special process. Otherwise the
> release handler cannot find the process."

> "The supervisor behaviour supports changing the internal state, that is,
> changing the restart strategy and maximum restart frequency properties, as well
> as changing the existing child specifications. Child processes can be added or
> deleted, but this is not handled automatically."

> "The changes do not affect existing child processes. ... The id of the child
> specification cannot be changed. Changing the Modules field of the child
> specification can affect the release handling process itself, as this field is
> used to identify which processes are affected when doing a synchronized code
> replacement."

> "New child specifications are automatically added, but not deleted. Child
> processes are not automatically started or terminated, this must be done using
> apply instructions. ... The order of the instructions is important."

> "When adding or removing an application, no .appup file is needed. When
> generating relup, the .rel files are compared and the add_application and
> remove_application instructions are added automatically."

> "When installing a release, the application specifications are automatically
> updated before evaluating the relup script. Thus, no instructions are needed
> in the .appup file."

> "The release handling instructions for adding, removing, and restarting
> applications apply to primary applications only. There are no corresponding
> instructions for included applications. ... a .relup file can be manually
> created."

> "restart_new_emulator — Intended when ERTS, Kernel, STDLIB, or SASL is
> upgraded. It is automatically added when the relup file is generated by
> systools:make_relup/3,4. It is executed before all other upgrade instructions."

> "restart_emulator — Used when a restart of the runtime system is required after
> all other upgrade instructions are executed."

## Version notes

- Page title: "Appup Cookbook — Erlang System Documentation v29.0.2".
- Built with ExDoc v0.40.3 for the Erlang programming language.
- Copyright © 1996-2026 Ericsson AB.
- The page references `gen_statem` (modern state machine behaviour) alongside
  the legacy `gen_fsm`, consistent with OTP 29 where `gen_fsm` is deprecated in
  favour of `gen_statem`.
- `restart_new_emulator` is described as auto-added by `systools:make_relup/3,4`,
  reflecting the modern release-handling API.
- No behavioural change vs. earlier OTP versions is flagged on the page itself;
  the recipes are stable across recent OTP releases.

## Discovered links

### Relevant (crawl later)
1. https://www.erlang.org/doc/system/release_handling.html — Release Handling (parent topic; appup instructions, relup, restart_emulator_instr, restart_new_emulator_instr anchors)
2. https://www.erlang.org/doc/system/gen_server_concepts.html — gen_server Behaviour (ch3 example origin)
3. https://www.erlang.org/doc/system/sup_princ.html — Supervisor Behaviour (ch_sup example origin)
4. https://www.erlang.org/doc/system/spec_proc.html — sys and proc_lib (ch4 special process example origin)
5. https://www.erlang.org/doc/system/secure_coding.html — Secure Coding Guidelines (next page)
6. https://www.erlang.org/doc/system/index.html — Erlang System Documentation index
7. https://www.erlang.org/doc/apps/stdlib/gen_server.html — STDLIB gen_server reference (code_change/3, #c:code_change/3)
8. https://www.erlang.org/doc/apps/stdlib/supervisor.html — STDLIB supervisor reference
9. https://www.erlang.org/doc/apps/stdlib/gen_statem.html — STDLIB gen_statem reference
10. https://www.erlang.org/doc/apps/stdlib/gen_event.html — STDLIB gen_event reference
11. https://www.erlang.org/doc/apps/stdlib/gen_fsm.html — STDLIB gen_fsm reference (legacy)
12. https://www.erlang.org/doc/apps/stdlib/sys.html — STDLIB sys reference (#handle_system_msg/6)

### Skipped
- https://erlang.org (site root, non-doc)
- /assets/css/algolia-typeahead.css (asset)
- In-page fragment anchors (#changing-a-functional-module, #module-dependencies, #changing-a-supervisor, #changing-properties, #changing-child-specifications, #adding-and-deleting-child-processes, #changing-internal-state, #changing-code-for-a-special-process, #changing-a-residence-module, #changing-a-callback-module, #adding-or-deleting-a-module, #starting-or-terminating-a-process, #adding-or-removing-an-application, #restarting-an-application, #changing-an-application-specification, #changing-application-configuration, #changing-included-applications, #application-restart, #supervisor-change, #changing-non-erlang-code, #runtime-system-restart-and-upgrade) — same-page navigation
