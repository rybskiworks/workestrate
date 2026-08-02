# Crawl: system/code_loading.html
- seed_url: https://www.erlang.org/doc/system/code_loading.html
- canonical_url: https://www.erlang.org/doc/system/code_loading.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: 29.0.2 (from page title "Erlang System Documentation v29.0.2")
- feeds_docs: code-loading-and-runtime.md, releases.md

## Purpose
Chapter "Compilation and Code Loading" in the Erlang System Documentation.
Explains how Erlang programs are compiled to BEAM object code and how that
object code is loaded into the runtime system, including the two code-loading
strategies (interactive vs embedded) and Erlang's module-level code
replacement mechanism (old/current code, two versions). It is the conceptual
narrative companion to the `code` module reference page in Kernel: this chapter
describes *what* happens and *why*, while `code.html` documents the API
functions (`code:purge/1`, `code:soft_purge/1`, `code:load_file/1`, etc.) used
to drive those mechanisms. It also points to System Principles for the boot
script / embedded loading story and to the `compile`, `make`, `erl`, and
`erlc` tooling for compilation.

## Key concepts (embedded vs interactive; code replacement; old/current; on_load)

### Compilation
- Erlang programs must be compiled to object code. The abstract machine is
  BEAM; object files use the `.beam` suffix. The compiler can emit a file or a
  binary loaded directly.
- Compiler lives in module `compile` (Compiler app). Shell shorthand `c(Module)`
  compiles + loads. Module `make` (Tools) provides make-like rebuilds.
- OS-level entry points: `erl -compile M1..Mn`, `erl -make`, and the `erlc`
  program (supports macros, include search paths, flags).

### Code loading
- Object code must be loaded into the runtime; handled by the **code server**
  (module `code` in Kernel).
- Two code-loading strategies:
  - **Interactive (default):** code is searched for in the code path and loaded
    on first reference (lazy).
  - **Embedded:** code is loaded at start-up according to a **boot script**
    (eager). Described in System Principles.

### Code replacement
- Erlang supports changing code in a running system; replacement is done at the
  **module level**.
- A module's code can exist in **two variants**: `current` and `old`.
  - First load -> code becomes `current`.
  - Loading a new instance -> previous instance becomes `old`, new instance
    becomes `current`.
  - Both old and current code are valid and can be evaluated concurrently.
  - **Fully qualified function calls** (`M:F/A`) always refer to *current*
    code. Old code keeps running via processes lingering in it.
  - Loading a *third* instance -> code server **purges** the old code and
    terminates any processes lingering in it; the third instance becomes
    `current` and the previously-current code becomes `old`.
- To switch a process from old to current code, it must make a **fully
  qualified function call** (e.g. `m:loop()`). This is the classic
  `code_switch` receive pattern.
- For code replacement of funs to work, use `fun Module:FunctionName/Arity`
  (not local funs).

### Running a function when a module is loaded (`-on_load`)
- The `-on_load(Name/0)` directive names a function run automatically when the
  module is loaded.
- The function need not be exported. It runs in a freshly spawned process that
  terminates as soon as the function returns.
- Return contract:
  - Return `ok` -> module becomes the new current code and becomes callable.
  - Any other return value, or an exception -> new code is **unloaded**. If the
    return value is not an atom, a warning error report is sent to the error
    logger.
- If current code already exists, it remains current and callable until the
  `on_load` function returns. If `on_load` fails, the existing current code (if
    any) stays current. If there is no current code, any process making an
    external call to the module before `on_load` finishes is **suspended** until
    it finishes.
- **Embedded mode ordering:** first all modules are loaded, *then* all
  `on_load` functions are called. The system is **terminated** unless all
  `on_load` functions return `ok`.
- Typical use: loading NIFs via `erlang:load_nif/2` inside the `on_load`
  function; on failure the module is unloaded and a warning report is emitted.

## Strict rules
- Code replacement is **module-level only**, not per-function/per-process.
- Only **two** versions of a module's code may coexist: `current` and `old`.
  A third load forces a purge of the old code and kills lingering processes.
- A process can only move from old code to current code by making a **fully
  qualified function call** (`M:F(...)`). Local calls stay in whatever code the
  process is currently executing.
- The `on_load` function **must** return `ok` for the module to become current
  and callable; any other return or exception unloads the new code.
- In embedded mode, **all** `on_load` functions must return `ok` or the entire
  system is terminated.
- For funs to participate in code replacement they must be written as
  `fun M:F/A`, not as local closures.
- The `on_load` function is not required to be exported.

## Examples

### Code-switch loop (old -> current via fully qualified call)
```erlang
-module(m).
-export([loop/0]).

loop() ->
    receive
        code_switch ->
            m:loop();
        Msg ->
            ...
            loop()
    end.
```
Send the message `code_switch` to the process; it then makes the fully
qualified call `m:loop()` and switches to current code. `m:loop/0` must be
exported.

### Compilation entry points
```erlang
compile:file(Module)
compile:file(Module, Options)
```
```shell
% erl -compile Module1...ModuleN
% erl -make
% erlc  File1.erl...FileN.erl
```

### on_load for NIF initialization
```erlang
-module(m).
-on_load(load_my_nifs/0).

load_my_nifs() ->
    NifPath = ...,    %Set up the path to the NIF library.
    Info = ...,       %Initialize the Info term
    erlang:load_nif(NifPath, Info).
```
If `erlang:load_nif/2` fails, the module is unloaded and a warning report is
sent to the error logger.

## Verbatim quotes
- "How code is compiled and loaded is not a language issue, but is
  system-dependent."
- "The code server loads code according to a code loading strategy, which is
  either interactive (default) or embedded. In interactive mode, code is
  searched for in a code path and loaded when first referenced. In embedded
  mode, code is loaded at start-up according to a boot script."
- "The code of a module can exist in two variants in a system: current and
  old."
- "Both old and current code are valid, and can be evaluated concurrently.
  Fully qualified function calls always refer to current code. Old code can
  still be evaluated because of processes lingering in the old code."
- "If a third instance of the module is loaded, the code server removes
  (purges) the old code and any processes lingering in it are terminated."
- "To change from old code to current code, a process must make a fully
  qualified function call."
- "For code replacement of funs to work, use the syntax
  `fun Module:FunctionName/Arity`."
- "The function must return `ok` if the module is to become the new current
  code for the module and become callable."
- "Returning any other value or generating an exception causes the new code to
  be unloaded."
- "In embedded mode, first all modules are loaded. Then all `on_load`
  functions are called. The system is terminated unless all of the `on_load`
  functions return `ok`."

## Version notes
- Page is part of Erlang System Documentation **v29.0.2** (ExDoc v0.40.3).
- **Change note (pre-OTP 19):** Before Erlang/OTP 19, if the `on_load` function
  failed, any previously current code would become old, essentially leaving the
  system without any working and reachable instance of the module. From OTP 19,
  on `on_load` failure the existing current code (if any) remains current.
- This chapter complements the `code` module page (`../apps/kernel/code.html`):
  the chapter gives the conceptual model (interactive vs embedded, old/current,
  purge-on-third-load, on_load semantics) while `code.html` provides the API
  (`code:purge/1`, `code:soft_purge/1`, `code:load_file/1`, `code:ensure_loaded/1`,
  code path manipulation, sticky directories, etc.). The chapter explicitly
  defers boot-script/embedded loading detail to System Principles
  (`system_principles.html#code_loading`).
- Note: this chapter does not itself document `code:purge`/`code:soft_purge`
  semantics, sticky directories, or `{atomic,...}`/`{error,...}` return values in
  detail — those belong to the `code` module reference. The chapter's purge
  coverage is limited to the conceptual statement that loading a third instance
  causes the code server to purge old code and terminate lingering processes.
  The `{atomic,...}`/`{error,...}` on_load return tuple phrasing in the task
  brief is not present in this chapter; the chapter specifies only `ok` vs
  non-`ok`/exception for `on_load`.

## Discovered links

### Relevant (crawl later)
- ../apps/kernel/code.html — `code` module reference (Kernel): purge/soft_purge, load_file, sticky dirs, code path. HIGH priority complement.
- ../system/system_principles.html — System Principles; boot scripts and embedded code loading (`#code_loading`). HIGH.
- ../apps/compiler/compile.html — `compile` module reference (Compiler app).
- ../apps/tools/make.html — `make` module reference (Tools app).
- ../apps/erts/erl_cmd.html — `erl` command flags (`-compile`, `-make`, `-mode`).
- ../apps/erts/erlc_cmd.html — `erlc` program reference.
- ../apps/erts/erlang.html — `erlang` module; `erlang:load_nif/2` anchor.
- distributed.html — Distributed Erlang (previous page).
- ports.html — Ports and Port Drivers (next page).

### Skipped
- /assets/css/algolia-typeahead.css, dist/html-erlang-KCHZLXSC.css (assets)
- ../index.html (system docs index — navigation only)
- code_loading.md (markdown source mirror of this page)
- https://github.com/erlang/otp/...source link
- https://github.com/elixir-lang/ex_doc, https://erlang.org, https://www.ericsson.com
- llms.txt, Erlang System Documentation.epub
- #compilation, #code-loading, #code-replacement, #running-a-function-when-a-module-is-loaded (in-page anchors)
