# Crawl: stdlib/proc_lib.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/proc_lib.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/proc_lib.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib v8.0.1)
- feeds_docs: proc-lib-and-sys.md

## Purpose

`proc_lib` provides functions for asynchronous and synchronous start of
processes adhering to the OTP design principles. It is the layer the OTP
standard behaviors (`gen_server`, `gen_statem`, `gen_event`, `supervisor`,
`supervisor_bridge`) are built on when starting new processes, and it is also
the recommended foundation for user-defined **special processes** that comply
with the OTP design principles (see `sys and proc_lib` in OTP Design
Principles).

When a process starts via `proc_lib`, useful information is initialized and
stored on the process dictionary:

- the registered name (or pid) of the **parent** process;
- the parent **ancestors**;
- information about the **function initially called** in the process.

This stored metadata is what makes crash reports, `initial_call/1`, and
`translate_initial_call/1` work — none of which raw `erlang:spawn*` provides.

## Key functions (exact arities)

### Spawning (asynchronous — like the BIFs but with proc_lib initialization)

- `spawn/1` — `spawn(Fun)` ≡ `spawn(erlang, apply, [Fun, []])`
- `spawn/2` — `spawn(Node, Fun)` ≡ `spawn(Node, erlang, apply, [Fun, []])`
- `spawn/3` — `spawn(Module, Function, Args)` ≡ `spawn(node(), M, F, A)`
- `spawn/4` — `spawn(Node, Module, Function, Args)` — spawns + initializes
- `spawn_link/1`, `spawn_link/2`, `spawn_link/3`, `spawn_link/4` — same shapes, link variant
- `spawn_opt/2` — `spawn_opt(Fun, SpawnOpts)`
- `spawn_opt/3` — `spawn_opt(Node, Fun, SpawnOpts)`
- `spawn_opt/4` — `spawn_opt(Module, Function, Args, SpawnOpts)`
- `spawn_opt/5` — `spawn_opt(Node, Module, Function, Args, SpawnOpts)` — spawns + initializes via `erlang:spawn_opt`

### Synchronous start (spawn + wait for the child to acknowledge)

- `start/3` — `start(Module, Function, Args)` ≡ `start(M, F, A, infinity)`
- `start/4` — `start(Module, Function, Args, Time)` ≡ `start(M, F, A, Time, [])`
- `start/5` — `start(Module, Function, Args, Time, SpawnOpts)` — synchronous start, no link/monitor
- `start_link/3`, `start_link/4`, `start_link/5` — same shapes; a link is atomically set on the spawned process
- `start_monitor/3` (OTP 23.0+), `start_monitor/4`, `start_monitor/5` — same shapes; a monitor is atomically set; returns `{Ret, Mon}`

### Startup handshake (called by the started child)

- `init_ack/1` — `init_ack(Ret)` ≡ `init_ack(Parent, Ret)` where `Parent` is the process that called `start/5`
- `init_ack/2` — `init_ack(Parent, Ret)` — tells Parent the process has initialized and started
- `init_fail/2` (OTP 26.0+) — `init_fail(Return, Exception)` ≡ `init_fail(Parent, Return, Exception)`
- `init_fail/3` (OTP 26.0+) — `init_fail(Parent, Return, Exception)` — tells Parent init failed and immediately raises `Exception`; the start function then returns `Return`

### Lifecycle / introspection

- `stop/1` — `stop(Process)` ≡ `stop(Process, normal, infinity)`
- `stop/3` — `stop(Process, Reason, Timeout)` — orders the process to exit with `Reason` and waits for it to terminate (based on the `terminate` system message; requires correct system-message handling)
- `hibernate/3` — `hibernate(Module, Function, Args)` — does the same as the `hibernate/3` BIF but preserves exception handling and logging on wake-up. **Always use this instead of the BIF for proc_lib-started processes.**
- `initial_call/1` — `initial_call(Process)` — extracts the initial call of a proc_lib-started process; returns `{Module, Function, Args} | false`. `Args` is now a list of placeholder atoms (`Argument__1`, `Argument__2`, ...) not the real arguments (memory + code-upgrade safety).
- `translate_initial_call/1` — translates the initial call to more useful info (e.g. for `gen_server` returns the callback module + `init`; for supervisors returns `supervisor` + callback module name, arity 1). Defaults to `{proc_lib,init_p,5}` when no info is found. Used by `c:i/0` and `c:regs/0`.

### Labelling (debugging aid)

- `set_label/1` (OTP 27.0+) — `set_label(Label)` — sets a label for the current process; any term, need not be unique or an atom; used in tools and crash reports to identify unregistered processes.
- `get_label/1` (OTP 27.0+) — `get_label(Pid)` — returns `undefined | Label` previously set with `set_label/1`.

### Crash report formatting (legacy error_logger interface)

- `format/1` — `format(CrashReport)` ≡ `format(CrashReport, latin1)`
- `format/2` (OTP R16B) — `format(CrashReport, Encoding)` — formats a crash report for a legacy `error_logger` handler; the event is `{error_report, GL, {Pid, crash_report, CrashReport}}`.
- `format/3` (OTP 18.1) — `format(CrashReport, Encoding, Depth)` — as `/2` with `Depth` limiting term printing via `io_lib:format("~P", [Term,Depth])`.

> `format/1,2,3` are **deprecated** for new Logger handlers: the formatting
> callback (`report_cb`) is included as metadata in the log event itself, so
> new Logger handlers do not need `format/...`. These functions exist for
> user-defined legacy `error_logger` event handlers.

## Strict rules (init_ack requirement; synchronous start; crash report format; what proc_lib adds over raw spawn)

### What proc_lib adds over raw `erlang:spawn*`

1. **Stored process metadata** — parent pid/registered name, ancestors, and the
   initial function are recorded on process start. This powers crash reports,
   `initial_call/1`, and `translate_initial_call/1`.
2. **Broadened "normal" termination** — a proc_lib process is considered to
   terminate *normally* not only for exit reason `normal`, but also for
   `shutdown` and `{shutdown, Term}`. `shutdown` is the reason used when an
   application (supervision tree) is stopped. This is what lets supervisors
   distinguish orderly shutdown from crashes.
3. **Automatic crash reports** — when a proc_lib process terminates abnormally
   (any reason other than `normal`, `shutdown`, or `{shutdown, Term}`), a crash
   report is generated and written to the terminal by the default logger
   handler set up by Kernel. The report contains the stored info (ancestors,
   initial function), the termination reason, and info about other processes
   that terminated as a result.
4. **No emulator error reports** — unlike plain Erlang, proc_lib processes do
   *not* generate the emulator's own error reports to the terminal. All
   exceptions are converted to exits which are ignored by the default logger
   handler. (Crash reporting is done by proc_lib/logger, not the emulator.)
5. **Synchronous start** — `start/3,4,5`, `start_link/3,4,5`,
   `start_monitor/3,4,5` spawn a process and *wait* for it to acknowledge via
   `init_ack/1,2` (or fail via `init_fail/2,3`). Raw `spawn` returns
   immediately with a pid and no knowledge of whether the child initialized.

### The `init_ack` requirement (synchronous start contract)

- `init_ack/1,2` **must only** be used by a process started by a
  `start[_link|_monitor]/3,4,5` function. It tells `Parent` the process has
  initialized itself and started; `Ret` is then returned by the start function.
- `init_ack/1` uses the parent value previously stored by the start function.
- If **neither** `init_ack/1,2` **nor** `init_fail/2,3` is called by the started
  process, the start function returns an error tuple when the started process
  exits, or when the start function time-out (if used) has passed.
- If `Time` is an integer, the start function waits `Time` ms for the child to
  call `init_ack/1,2` or `init_fail/2,3`; otherwise the process is killed and
  `{error, timeout}` is returned.

### The `init_fail` rule (preferred over `init_ack` for start failures)

- `init_fail/2,3` (OTP 26.0+) tells `Parent` that initialization failed and
  **immediately raises** `Exception` (a `{Class, Reason}` or
  `{Class, Reason, Stacktrace}` term; see `erlang:raise/3`). The start function
  then returns `Return`.
- **Warning:** do *not* use `init_ack` to signal a failed start — the start
  function could return before the failing process has exited, which may block
  VM resources required for a new start attempt to succeed. Use `init_fail/2,3`
  for that purpose.
- **Warning:** do *not* catch the exception raised by `init_fail`. A process
  started by `start[_link|_monitor]/3,4,5` should end in a value (ignored) or an
  exception handled by this module.

### Spawn-option restrictions on the synchronous starters

- The `start_spawn_option()` type is a **restricted** set of spawn options:
  `link | {priority, _} | {fullsweep_after, _} | {min_heap_size, _} |
  {min_bin_vheap_size, _} | {max_heap_size, _} | {message_queue_data, _}`.
- **`monitor` is not allowed** in `start/5`, `start_link/5`, `start_monitor/5`
  SpawnOpts — it causes `badarg`. (`start_monitor` sets up its own monitor
  atomically; `start_link` sets up its own link.)
- Passing `link` in `start/5` or `start_monitor/5` SpawnOpts sets a link, just
  like `start_link/3,4,5`.

### `start_link` exit-propagation rules

- If the started process is killed or crashes with a reason other than
  `normal`, the link will kill the calling process, so `start_link/5` does **not**
  return — unless the calling process traps exits. (E.g. on timeout the spawned
  process is killed and then the link may kill the caller.)
- When the caller traps exits and `start_link/5` returns due to the spawned
  process exiting (any error return), the function receives (consumes) the
  `'EXIT'` message — also when it times out and kills the spawned process.

### `start_monitor` DOWN-message rule

- `start_monitor/3,4,5` returns `{Ret, Mon}`. If it returns due to the spawned
  process exiting (any error value), a `'DOWN'` message **will** be delivered to
  the calling process — also when it times out and kills the spawned process.

### `stop/3` rules

- Returns `ok` if the process exits with the specified `Reason` within
  `Timeout` ms.
- On timeout: raises a `timeout` exception.
- If the process does not exist: raises a `noproc` exception.
- Implemented on top of the **`terminate` system message** — requires the
  process to handle system messages correctly (i.e. be `sys`-compatible).

### Crash report format

- The crash report is sent through `logger`. For legacy `error_logger`
  handlers the event is
  `{error_report, GL, {Pid, crash_report, CrashReport}}` where `GL` is the
  group leader pid of `Pid`.
- `format/1,2,3` format a `CrashReport` (a `[term()]`) into a `string()`.
  `format/2` adds `Encoding` (`latin1 | unicode | utf8`); `format/3` adds
  `Depth` (`unlimited | pos_integer()`) used as `io_lib:format("~P", [Term,Depth])`.
- New Logger handlers do **not** need `format/...` — the `report_cb`
  formatting callback is included as metadata in the log event itself.
- For how crash reports were logged prior to Erlang/OTP 21.0, see *SASL
  Error Logging* in the SASL User's Guide.

## The special-process contract

A **special process** is a user-defined process that complies with the OTP
design principles but is not one of the standard behaviors. `proc_lib` is the
foundation for writing one:

1. **Start via `proc_lib`** — use `spawn_opt/...` or (preferably, for
   synchronous start) `start_link/3,4,5` / `start_monitor/3,4,5` so the
   process gets proc_lib metadata (parent, ancestors, initial call) and the
   broadened normal-termination semantics (`normal | shutdown | {shutdown,Term}`).
2. **Acknowledge startup** — the child must call `init_ack/1,2` on success, or
   `init_fail/2,3` (OTP 26.0+) on failure. Using `init_ack` to signal failure
   is explicitly forbidden because it can leak VM resources.
3. **Be `sys`-compatible** — the process must handle **system messages**
   correctly. `stop/3` is implemented on the `terminate` system message, and
   `sys` debugging, tracing, and suspend/resume all depend on this. See
   `sys` and the *sys and proc_lib* section of OTP Design Principles.
4. **Hibernate via `proc_lib:hibernate/3`** — never the raw `hibernate/3` BIF,
   so exception handling and logging continue to work when the process wakes.
5. **Terminate through proc_lib's exit path** — end in a value (ignored) or an
   exception handled by this module, so crash reports are emitted correctly.

This contract is exactly what the standard behaviors implement internally,
which is why `gen_server`, `gen_statem`, `gen_event`, `supervisor`, and
`supervisor_bridge` are all built on `proc_lib`: it gives them synchronous
start, the startup handshake, crash reporting, ancestor tracking, and
`sys`-compatibility for free.

## Verbatim quotes

> Functions for asynchronous and synchronous start of processes adhering to the
> OTP design principles.

> This module is used to start processes adhering to the OTP Design Principles.
> Specifically, the functions in this module are used by the OTP standard
> behaviors (for example, gen_server and gen_statem) when starting new
> processes. The functions can also be used to start special processes,
> user-defined processes that comply to the OTP design principles.

> Some useful information is initialized when a process starts. The registered
> names, or the process identifiers, of the parent process, and the parent
> ancestors, are stored together with information about the function initially
> called in the process.

> While in "plain Erlang", a process is said to terminate normally only for exit
> reason normal, a process started using proc_lib is also said to terminate
> normally if it exits with reason shutdown or {shutdown,Term}. shutdown is the
> reason used when an application (supervision tree) is stopped.

> When a process that is started using proc_lib terminates abnormally (that is,
> with another exit reason than normal, shutdown, or {shutdown,Term}), a crash
> report is generated, which is written to terminal by the default logger
> handler setup by Kernel.

> Unlike in "plain Erlang", proc_lib processes will not generate error reports,
> which are written to the terminal by the emulator. All exceptions are
> converted to exits which are ignored by the default logger handler.

> The crash report contains the previously stored information, such as ancestors
> and initial function, the termination reason, and information about other
> processes that terminate as a result of this process terminating.

> This function must only be used by a process that has been started by a
> start[_link|_monitor]/3,4,5 function. It tells Parent that the process has
> initialized itself and started. (on `init_ack/2`)

> Do not use this function to return an error indicating that the process start
> failed. When doing so the start function can return before the failing
> process has exited, which may block VM resources required for a new start
> attempt to succeed. Use init_fail/2,3 for that purpose.

> This function must only be used by a process that has been started by a
> start[_link|_monitor]/3,4,5 function. It tells Parent that the process has
> failed to initialize, and immediately raises an exception according to
> Exception. The start function then returns Ret. (on `init_fail/3`)

> Do not consider catching the exception from this function. That would defeat
> its purpose. A process started by a start[_link|_monitor]/3,4,5 function
> should end in a value (that will be ignored) or an exception that will be
> handled by this module.

> Always use this function instead of the BIF for processes started using
> proc_lib functions. (on `hibernate/3`)

> Starts a new process synchronously. Spawns the process and waits for it to
> start. (on `start/5`, `start_link/5`, `start_monitor/5`)

> Using spawn option monitor is not allowed. It causes the function to fail with
> reason badarg. (on `start/5`, `start_link/5`, `start_monitor/5`)

> The implementation of this function is based on the terminate system message,
> and requires that the process handles system messages correctly. (on `stop/3`)

> By default, {proc_lib,init_p,5} is returned if no information about the
> initial call can be found. It is assumed that the caller knows that the
> process has been spawned with the proc_lib module. (on
> `translate_initial_call/1`)

> The list Args no longer contains the arguments, but the same number of atoms
> as the number of arguments; the first atom is 'Argument__1', the second
> 'Argument__2', and so on. The reason is that the argument list could waste a
> significant amount of memory, and if the argument list contained funs, it
> could be impossible to upgrade the code for the module. (on `initial_call/1`)

## Version notes

- Page targets **OTP 29.0.2**, stdlib **v8.0.1**.
- `format/2`: since OTP R16B.
- `format/3`: since OTP 18.1.
- `stop/1`, `stop/3`: since OTP 18.0.
- `start_monitor/3,4,5`: since OTP 23.0.
- `init_fail/2,3`: since OTP 26.0.
- `set_label/1`, `get_label/1`: since OTP 27.0.
- `format/1,2,3` are **deprecated** for new Logger handlers (the `report_cb`
  callback is included as log-event metadata). Legacy `error_logger` handlers
  may still use them. New logging API added in Erlang/OTP 21.0.
- Pre-21.0 crash-report logging is described in the SASL User's Guide
  (*SASL Error Logging*).
- `initial_call/1` no longer returns real argument terms (memory + code
  upgrade safety) — returns placeholder atoms `Argument__1..N`; funs are
  reported as `{Module, -Func/Arity-fun-N-, 0}`.

## Discovered links

### Relevant (crawl later)

- `sys.html` (stdlib) — the `sys` module; partner to `proc_lib` for special
  processes (system messages, debugging, `stop/3`'s `terminate` message).
- `../../apps/kernel/logger.html` — `logger`; handles crash reports, provides
  the `report_cb` metadata mechanism that supersedes `format/1,2,3`.
- `../../apps/sasl/error_logging.html` — SASL error logging (legacy crash
  report logging pre-OTP 21.0).
- `../../apps/erts/erlang.html` — `erlang` BIFs referenced: `spawn/1`,
  `spawn_link/1`, `spawn_opt/2`, `spawn_opt/4`, `hibernate/3`, `raise/3`, and
  the spawn-option / type definitions.
- `c.html` (stdlib) — `c:i/0` and `c:regs/0` consume `translate_initial_call/1`.

### Skipped

- Page-internal anchors (`#format/1`, `#start/5`, `#t:exception/0`, ...).
- `../../system/design_principles.html` — already crawled (01-design-principles.md).
- `../../system/spec_proc.html` — already crawled (10-spec-proc.md).
- `gen_server.html`, `gen_statem.html` (stdlib module docs) — already crawled
  (11-gen-server-module.md, 12-gen-statem-module.md).
- `../../index.html`, `llms.txt`, `stdlib.epub`, `proc_lib.md`, `proc_lib.html`
  (self / packaging).
- `https://github.com/erlang/otp/blob/OTP-29.0.2/lib/stdlib/src/proc_lib.erl#L*`
  source-line links (40+).
- `https://erlang.org`, `https://www.ericsson.com`, `https://github.com/elixir-lang/ex_doc`.
- CSS / asset links (`/assets/css/...`, `dist/html-erlang-...css`).
