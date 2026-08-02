# Crawl: kernel/code.html
- seed_url: https://www.erlang.org/doc/apps/kernel/code.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/code.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: code-loading-and-runtime.md, releases.md

## Purpose
Interface to the Erlang **code server** process. The `code` module is the API
the runtime exposes for loading compiled (`.beam`) object code into a running
Erlang VM, for managing the **code path** (the ordered list of `ebin`
directories searched when a module is first referenced), for purging old code
during hot code upgrade, and for locating application resources (`priv`,
library dirs, archives).

The code server itself is a process; this module is its client API. The runtime
can run in two modes (set by `-mode` flag): **interactive** (default; modules
auto-loaded on first reference) and **embedded** (no auto-load; boot script must
load everything, as in OTP releases).

## Key functions (exact arities)
Path management:
- `add_path/1` — `add_path(Dir)` ≡ `add_pathz(Dir, nocache)` (append, no cache).
- `add_path/2` — `add_path(Dir, Cache)` ≡ `add_pathz(Dir, Cache)` (since OTP 26).
- `add_patha/1`, `add_patha/2` — prepend Dir to code path (since OTP 26 for /2).
- `add_pathz/1`, `add_pathz/2` — append Dir to code path (since OTP 26 for /2).
- `add_paths/1,2`, `add_pathsa/1,2`, `add_pathsz/1,2` — list variants.
- `del_path/1` — `del_path(NameOrDir)`; atom Name deletes `.../Name[-Vsn][/ebin]`; returns `true | false | {error, bad_name}`.
- `del_paths/1` — list variant (since OTP 26).
- `replace_path/2`, `replace_path/3` — replace `.../Name[-Vsn][/ebin]` with Dir (since OTP 26 for /3).
- `set_path/1` — `set_path(Path)` ≡ `set_path(Path, nocache)`.
- `set_path/2` — `set_path(Path, cache())` (since OTP 26).
- `get_path/0` — returns the current code path list.
- `clear_cache/0` — clears code path cache (since OTP 26).

Code loading:
- `load_file/1` — `load_file(Module)`; searches code path for `Module.beam`; returns `{module, Module} | {error, Reason}`.
- `load_abs/1` — `load_abs(Filename)`; absolute/relative filename, no code-path search; extension must be omitted.
- `load_binary/3` — `load_binary(Module, Filename, Binary)`; loads object code from a binary (for remote nodes); Filename only recorded, not opened.
- `ensure_loaded/1` — `ensure_loaded(Module)`; loads like `load_file/1` unless already loaded; in embedded mode returns `{error, embedded}`.
- `ensure_modules_loaded/1` — loads a list; unlike `ensure_loaded/1`, loads even in embedded mode (since OTP 19).
- `atomic_load/1` — atomically load a list of modules (since OTP 19).
- `prepare_loading/1` + `finish_loading/1` — two-phase atomic load to minimize inactive window (since OTP 19).

Purge / delete:
- `purge/1` — `purge(Module)`; removes old code, **kills** processes lingering in old code; returns `true` if any process was killed.
- `soft_purge/1` — `soft_purge(Module)`; removes old code **only if no processes linger**; returns `false` if lingering processes prevent purge.
- `delete/1` — `delete(Module)`; makes current code old (does not purge); returns `false` if old code already exists (must purge first) or Module not loaded.

Introspection:
- `is_loaded/1` — `{file, Loaded} | false`; Loaded is absolute filename, `preloaded`, or `cover_compiled`.
- `which/1` — absolute filename of loaded/first-found object code; `preloaded | cover_compiled | non_existing`.
- `where_is_file/1` — searches code path for arbitrary file; returns `non_existing | Absname`.
- `get_object_code/1` — `{Module, Binary, Filename} | error`; used to ship code to remote nodes (paired with `erpc:call(Node, code, load_binary, [Module, Filename, Binary])`).
- `all_loaded/0` — `[{Module, Loaded}]` for all loaded modules.
- `all_available/0` — `[{Module, Filename, Loaded}]` for loaded + would-be-loaded (since OTP 23).
- `module_status/0`, `module_status/1` — `not_loaded | loaded | removed | modified` (since OTP 20/23).
- `modified_modules/0` — loaded modules whose on-disk MD5 differs (since OTP 20).
- `objfile_extension/0` — `.beam` for official OTP.
- `get_mode/0` — `embedded | interactive` (since OTP R16B).
- `clash/0` — reports duplicate module names across code path to stdout.

Application / library dirs:
- `lib_dir/0` — `$OTPROOT/lib`.
- `lib_dir/1` — `lib_dir(Name)`; top directory of application Name; strips archive name; `{error, bad_name}` if not found.
- `lib_dir/2` — **DEPRECATED** (since OTP 27); use `filename:join(code:lib_dir(Name), SubDir)`.
- `priv_dir/1` — path to application's `priv` directory; `{error, bad_name}` if not found.
- `compiler_dir/0` — ≡ `code:lib_dir(compiler)`.
- `root_dir/0` — `$OTPROOT`.

Sticky directories:
- `stick_dir/1` — `stick_dir(Dir)`; returns `ok | error`.
- `unstick_dir/1` — `unstick_dir(Dir)`; returns `ok | error`.
- `is_sticky/1` — `is_sticky(Module)`; true if Module was loaded from a sticky dir.

Native coverage (since OTP 27):
- `coverage_support/0`, `get_coverage_mode/0`, `get_coverage_mode/1`, `set_coverage_mode/1`, `get_coverage/2`, `reset_coverage/1`.

Docs:
- `get_doc/1` — EEP 48 docs chunk (since OTP 23).
- `get_debug_info/1` — debug info for a module.

> NOTE: There is **no `compiler_options` function** on this page. The user-listed
> `compiler_options` does not exist in the `code` module; compiler options are
> set at compile time via the `compile` module (`../../apps/compiler/compile.html`)
> or via `erl` flags, not through `code`.

## The two-version rule (purge vs soft_purge)
A module can exist in **two variants simultaneously**: *current* code and *old*
code. When a module is loaded for the first time it becomes current. When a new
instance is loaded, the previous current becomes old (its export entries are
removed, so global calls into old code fail), and the new instance becomes
current. Both old and current code can execute concurrently — but exported
functions in old code are unavailable to global (external) calls; only
processes already lingering in old code keep executing it.

If a **third** instance is loaded, the code server must first purge the old
code; any processes lingering in old code are terminated, the third instance
becomes current, and the previously-current becomes old. This is the
"two-version" invariant: at most current + old; a third load forces a purge.

- `purge/1` — forcibly removes old code; **kills** lingering processes; returns
  `true` if any process had to be killed. Use when you must free the old slot to
  load a third version.
- `soft_purge/1` — removes old code **only if no processes linger**; returns
  `false` (without purging) if processes are still in old code. Safe: never
  kills. Use when you want to refuse the upgrade rather than terminate processes.
- `delete/1` — does not purge; just demotes current → old. Returns `false` if
  old code already exists (you must `purge/1` or `soft_purge/1` first to free
  the old slot before `delete/1` can succeed).

As of OTP 20.0, a process is only "lingering" if it has *direct references* to
the code (per `erlang:check_process_code/3`).

## Path management
The code path is an ordered list of directories searched sequentially (in
interactive mode) when a module is first referenced. Initial path = current
working directory + all `ebin` dirs under `$OTPROOT/lib`, choosing the highest
`-Vsn` among same-named dirs. `ERL_LIBS` adds more library roots (colon-separated
on Unix, semicolon on Windows); apps found there override standard OTP apps
**except Kernel and STDLIB**, which always come first.

- `add_patha` = prepend (a = "ahead"); `add_pathz` = append (z = "last"); `add_path` ≡ `add_pathz`.
- Path entries are **cached by default** (since OTP 26) except `.` and `-pa`/`-pz` dirs; disable with `-cache_boot_paths false` or `code:set_path(code:get_path())`.
- `del_path/1` accepts either an atom Name (resolves `.../Name[-Vsn][/ebin]`) or a full Dir string.
- `replace_path/2,3` swaps a named entry; used when adding a new library version to a running system.
- `set_path/1,2` replaces the entire path; returns `{error, bad_directory}` if any entry is invalid.

## Embedded vs interactive mode
Set by command-line flag `-mode` (e.g. `% erl -mode embedded`).

- **interactive** (default): only modules needed by the runtime are loaded at
  startup; all other code is **dynamically loaded on first reference**. The code
  server searches the code path and tries to load the module when a function in
  an unloaded module is called.
- **embedded**: modules are **not auto-loaded**; calling a function in an
  unloaded module raises an error. The boot script is expected to load all
  modules (as in OTP releases). Code can still be loaded later by explicitly
  calling the code server (`load_file/1`, `load_binary/3`, etc.).

`ensure_loaded/1` returns `{error, embedded}` in embedded mode for unloaded
modules; `ensure_modules_loaded/1` is the exception — it loads even in embedded
mode. `get_mode/0` returns the current mode atom. An external entity (e.g. an
IDE) adding code to a running node: in interactive mode just `add_path`; in
embedded mode must `load_binary/3`.

## Sticky directories
To prevent accidentally reloading modules that affect the runtime itself, the
directories `kernel`, `stdlib`, and `compiler` are **sticky by default**. A
reload request for a module residing in a sticky directory issues a warning and
is rejected (error reason `sticky_directory`). Disable globally with
`-nostick` flag. `stick_dir/1` / `unstick_dir/1` mark/unmark arbitrary
directories as sticky; `is_sticky/1` tests whether a loaded module came from a
sticky directory.

## Strict rules
- Module and application names are **atoms**; file and directory names are **strings**. Some functions accept both for backward compatibility, but a future release will likely require only the documented type.
- Functions fail with an **exception** for incorrect argument *type* (e.g. integer where atom expected); they return an `{error, Reason}` tuple when the type is correct but something else is wrong (e.g. non-existing directory to `set_path/1`).
- Code-loading functions return `{error, Reason}` with one of: `badfile`, `nofile`, `not_purged`, `on_load_failure`, `sticky_directory`.
- `load_file/1` fails if the module name embedded in the object code differs from the requested Module name; use `load_binary/3` to load object code under a different module name.
- `load_abs/1` Filename must **not** include the `.beam` extension (it is appended automatically).
- `lib_dir/2` is **deprecated** since OTP 27 (archive support is experimental); use `filename:join(code:lib_dir(Name), SubDir)`.
- `-code_path_choice` flag: default changed to `strict` in OTP 27 (the exact stated directory is used; archive fallback is not consulted); `relaxed` is scheduled for removal in OTP 28.

## Verbatim quotes
- "Interface to the Erlang code server process. This module contains the interface to the Erlang code server, which deals with the loading of compiled code into a running Erlang runtime system."
- "In interactive mode, which is default, only the modules needed by the runtime system are loaded during system startup. Other code is dynamically loaded when first referenced."
- "In embedded mode, modules are not auto-loaded. Trying to use a module that has not been loaded results in an error. This mode is recommended when the boot script loads all modules, as it is typically done in OTP releases."
- "The code for a module can exist in two variants in a system: current code and old code."
- "If a third instance of the module is loaded, the code server removes (purges) the old code and any processes lingering in it are terminated. Then the third instance becomes current and the previously current code becomes old."
- "Both old and current code for a module are valid, and can even be executed concurrently. The difference is that exported functions in old code are unavailable. Hence, a global call cannot be made to an exported function in old code, but old code can still be executed because of processes lingering in it."
- `purge/1`: "Purges the code for Module, that is, removes code marked as old. If some processes still linger in the old code, these processes are killed before the code is removed."
- `soft_purge/1`: "Purges the code for Module ... but only if no processes linger in it. Returns false if the module cannot be purged because of processes lingering in old code, otherwise true."
- `delete/1`: "Removes the current code for Module, that is, the current code for Module is made old. ... Returns true if successful, or false if there is old code for Module that must be purged first, or if Module is not a (loaded) module."
- "To prevent accidentally reloading of modules affecting the Erlang runtime system, directories kernel, stdlib, and compiler are considered sticky."
- `load_binary/3`: "Filename is only used by the code server to keep a record of from which file the object code for Module originates. Thus, Filename is not opened and read by the code server."
- `get_object_code/1`: "Returns the object code for module Module if found in the code path. ... This is useful if code is to be loaded on a remote node in a distributed system."

## Version notes
- Page version: OTP 29.0.2, kernel 11.0.2.
- Code path caching (`cache` arg on `add_path*`, `set_path/2`, `replace_path/3`, `clear_cache/0`): added OTP 26.
- `atomic_load/1`, `prepare_loading/1`, `finish_loading/1`, `ensure_modules_loaded/1`: since OTP 19.
- `all_available/0`, `get_doc/1`, `module_status/0`: since OTP 23.
- `module_status/1`, `modified_modules/0`: since OTP 20.
- `get_mode/0`: since OTP R16B.
- Native coverage (`coverage_support/0`, `set_coverage_mode/1`, `get_coverage/2`, `reset_coverage/1`, etc.): since OTP 27.
- `lib_dir/2` and `-code_path_choice` deprecated since OTP 27; `-code_path_choice` scheduled for removal in OTP 28.
- `purge/1` / `soft_purge/1` lingering semantics changed in OTP 20.0 (direct references only, per `erlang:check_process_code/3`).
- Page built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.

## Discovered links
### Relevant (crawl later)
- ../../system/code_loading.html — Compilation and Code Loading (Erlang Reference Manual); referenced for old/current code and on_load.
- ../../system/code_loading.html#on_load — on_load function semantics.
- ../../apps/erts/init.html — `init` module; interprets boot script; `-code_path_choice` strict/relaxed.
- ../../apps/erts/erl_prim_loader.html — low-level loader used to read files from archives.
- ../../apps/erts/erlang.html#check_process_code/3 — determines whether a process is lingering in old code (purge semantics).
- ../../apps/compiler/compile.html#line_coverage — `line_coverage` compile option for native coverage.
- ../../apps/erts/erl_cmd.html#%2BJPcover — `+JPcover` erl flag for coverage mode.
- ../../apps/stdlib/escript.html + ../../apps/stdlib/escript.html#extract/2 — `escript:extract/2` for reading archive data files (replaces deprecated `lib_dir/2`).
- ../../apps/stdlib/filename.html#join/2 — `filename:join/2` replacement for `lib_dir/2`.
- ../../apps/sasl/script.html — `script(4)` boot script format; preloaded modules.
- ../../apps/tools/cover.html — `cover` tool; uses native coverage when supported.
- eep48_chapter.html — EEP 48 documentation storage and format (Kernel User's Guide).
- https://www.erlang.org/eeps/eep-0048.html — EEP 48 spec.

### Skipped
- https://erlang.org, https://www.ericsson.com, https://github.com/elixir-lang/ex_doc — site/vendor homepages.
- ../../index.html — doc index root.
- code.html#error_reasons, code.md — same-page anchors / markdown mirror.
- dist/html-erlang-KCHZLXSC.css — ExDoc stylesheet.
- https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/code.erl#L... — per-function source links (40+); source is `code.erl` at OTP-29.0.2 tag.
- ../../apps/erts/erlang.html#t:* — built-in type anchors (atom/0, module/0, etc.); not crawl targets.
- file.html#t:filename/0, file.html#t:posix/0 — type anchors in `file` module.
