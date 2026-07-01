# BEAM / OTP Guidance Index

## Purpose

This corpus is a project-independent, BEAM-common reference for the Erlang/OTP runtime and its standard behaviours. It is Erlang-native: every document is sourced from the official Erlang/OTP documentation (OTP 29.0.2), live-fetched via curl (HTTP 200 or verified redirect) from erlang.org, with verbatim quotations so claims can be audited. It is NOT Elixir- or Gleam-specific. The 22 topic docs were written strictly from 47 crawled source files persisted in `docs/beam/.crawl/01–47`.

The intended audience is future AI agents that will write, review, refactor, debug, validate, or scaffold BEAM/OTP (Erlang) code. Each file states its purpose, lists its crawl sources, provides core guidance with inline citations, decision tables/checklists, common mistakes, strict-vs-contextual guidance, and a "Policy decisions for individual repos" section. Because the corpus is project-independent, it stops short of mandating repo-specific choices; those are enumerated per file and consolidated in the Open policy decisions section below.

## How to use this corpus

### Reading paths

Agents should not read all 22 files linearly; consult the Recommended reading paths section to find the 2–4 files most relevant to the current task.

### When to consult which docs

- **New to BEAM/OTP** → start with `overview.md`, then `otp-behaviours.md` and `supervision.md`.
- **Writing OTP behaviours** → `otp-behaviours.md`, then the specific behaviour doc (`gen-server.md`, `gen-statem.md`, `gen-event.md`, `supervision.md`).
- **Reviewing a diff** → `supervision.md`, the behaviour doc matching the change, `common-mistakes.md`, and `validation.md`.
- **Debugging a crash or hang** → `links-monitors-and-exits.md`, `processes-and-messages.md`, `runtime-debugging.md`, `proc-lib-and-sys.md`.
- **Building or upgrading releases** → `applications.md` and `releases.md`.
- **Configuring logging** → `logger-and-config.md`.
- **Working with ETS** → `ets-data.md`.
- **Distribution** → `distribution.md`.
- **NIFs / FFI** → `nifs.md` and `ports-io.md`.
- **Binaries / bit syntax** → `binaries.md`.
- **Filesystem / OS environment** → `runtime-environment.md`.

### Conventions used in every file

1. **Purpose** — what the file covers and who it is for.
2. **Sources used** — the crawl files and canonical erlang.org URLs consulted, with OTP version notes.
3. **Core guidance** — verbatim quotations from the official docs, with inline citations.
4. **Practical rules** — concrete decision rules and API contracts.
5. **Review / Implementation / Runtime-debugging checklists** — copy-pasteable checklists.
6. **Common mistakes** — anti-patterns and traps.
7. **Strict vs contextual guidance** — what is mandatory vs what depends on context.
8. **Policy decisions for individual repos** — open questions each repo must answer.
9. **Related docs / Related skills** — cross-references.

## Relationship to Erlang, Elixir, and Gleam

BEAM is the shared runtime source of truth. This corpus documents the Erlang/OTP layer directly. The sibling corpora `docs/elixir/` and `docs/gleam/` reference this corpus for shared runtime concepts (processes, signals, supervision, distribution, ETS, NIFs, timers, ports). The three corpora do not depend on each other; an Elixir or Gleam agent consults `docs/beam/` when language-level docs are insufficient and the underlying Erlang/OTP runtime semantics are needed.

Consumer corpora (siblings that reference this corpus): [`docs/elixir/`](../elixir/index.md), [`docs/gleam/`](../gleam/index.md). See the cross-corpus index [`docs/languages.md`](../languages.md) for the top-level map across all three corpora.

## Generated files

### overview.md

- **Path:** `docs/beam/overview.md`
- **Purpose:** Establish the BEAM/OTP mental model and design principles: how Erlang/OTP structures code into processes, modules, and directories via supervision trees, behaviours, applications, and releases. The conceptual entry point for the `docs/beam/` corpus.
- **Main topics:**
  - Supervision tree model (workers vs supervisors)
  - behaviours generic/specific split and `-behaviour/1`
  - the four standard behaviours
  - applications as process + directory structure
  - library applications
  - minimal system = Kernel + STDLIB
  - releases and release handling
- **When a future agent should read it:** Consult first as the conceptual entry point before any BEAM/OTP work; read when establishing how an application is structured into supervision trees, behaviours, applications, and releases.
- **Related skills:** `beam-supervision`, `beam-gen-server`, `beam-applications-releases`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/refactoring.md`

### processes-and-messages.md

- **Path:** `docs/beam/processes-and-messages.md`
- **Purpose:** Define the BEAM process abstraction: creation (`spawn`/`spawn_opt` + options), message passing (`!`/`send`/`receive`/selective/`after`), registration, process flags, process states, reductions/preemption, and signal ordering.
- **Main topics:**
  - Process creation via `spawn`/`spawn_link`/`spawn_monitor`/`spawn_opt`
  - signals as universal async mechanism and per-sender ordering
  - `send`/`receive` semantics and priority messages (OTP 28+)
  - registration
  - process flags (`trap_exit`, `priority`, `message_queue_data`, `max_heap_size`, `async_dist`)
  - process states, exit reasons, reductions, hibernation
  - process dictionary
- **When a future agent should read it:** Consult when writing or reviewing code that spawns processes, uses message passing/receive, registers names, sets process flags, or relies on signal ordering.
- **Related skills:** `beam-processes`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/debugging.md`, `docs/beam/workflows/runtime-diagnosis.md`

### links-monitors-and-exits.md

- **Path:** `docs/beam/links-monitors-and-exits.md`
- **Purpose:** Define link/monitor/alias mechanics, exit-signal propagation, `trap_exit` reception rules, `DOWN` messages, and the distinction between `exit/1`, `exit/2`, and `exit_signal/2,3`. Failure semantics for OTP supervision trees depend on these primitives.
- **Main topics:**
  - Links (bidirectional, unique, `noproc` exception vs signal)
  - monitors (unidirectional, `monitor/3` alias option, `demonitor/2` `[flush]`)
  - process aliases
  - exit signals and the `link` flag
  - three exit-reception cases
  - `trap_exit`
  - `kill` from link vs explicit `exit_signal/2`
  - exception classes and stacktrace
- **When a future agent should read it:** Consult when designing or reviewing failure propagation, link/monitor/alias usage, `trap_exit` policy, `exit/1` vs `exit_signal/2` choice, or race-free client/server reply patterns.
- **Related skills:** `beam-processes`, `beam-errors-failures`, `beam-supervision`
- **Related workflow files:** `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`

### otp-behaviours.md

- **Path:** `docs/beam/otp-behaviours.md`
- **Purpose:** Explain the generic/specific behaviour split, the `-behaviour` and `-callback` attributes, the four standard behaviours, and the special-process contract (`proc_lib` + `sys`) that lets a hand-written process comply with OTP design principles without a standard behaviour.
- **Main topics:**
  - Generic/specific split and `-behaviour/1`
  - four standard behaviours
  - special processes (`sys` + `proc_lib` contract, system messages)
  - why prefer standard behaviours
  - `-callback`/`-optional_callbacks`
  - `proc_lib` start, `init_ack`/`init_fail`, `sys:handle_system_msg/6`
- **When a future agent should read it:** Consult when choosing between a standard behaviour and a hand-written special process, declaring user-defined behaviours, or implementing the `system_*` callback contract.
- **Related skills:** `beam-gen-server`, `beam-gen-statem`, `beam-supervision`, `beam-processes`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/refactoring.md`

### gen-server.md

- **Path:** `docs/beam/gen-server.md`
- **Purpose:** Source-verified guidance for the OTP `gen_server` behaviour. Reproduces the callback contract, return-tuple shapes, and supporting types verbatim from the official Erlang/OTP documentation.
- **Main topics:**
  - Client-server model
  - full callback signatures (`init/1`, `handle_call/3`, `handle_cast/2`, `handle_info/2`, `handle_continue/2`, `terminate/2`, `code_change/3`, `format_status/1,2`)
  - `action/0` type
  - return-tuple shapes
  - naming/registration
  - `start`/`start_link`/`start_monitor`
  - `trap_exit` not automatic
  - `{error,Reason}` from init (OTP 26)
  - `stop/1,2,3`
- **When a future agent should read it:** Consult whenever writing, reviewing, refactoring, debugging, or validating a `gen_server` callback module.
- **Related skills:** `beam-gen-server`, `beam-supervision`, `beam-errors-failures`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/refactoring.md`, `docs/beam/workflows/validation.md`

### gen-statem.md

- **Path:** `docs/beam/gen-statem.md`
- **Purpose:** Source-verified guidance for the `gen_statem` behaviour (OTP 29.0.2 / stdlib v8.0.1). `gen_statem` is the generic state machine behaviour that replaces `gen_fsm` and should be used for new code.
- **Main topics:**
  - When to use over `gen_server`
  - callback modes (`state_functions` vs `handle_event_function`)
  - `state_enter`
  - event types
  - three timeout kinds
  - transition actions
  - `{reply,From,Reply}` vs `reply/1,2`
  - `code_change/4`
  - `gen_fsm` differences
- **When a future agent should read it:** Consult when implementing, reviewing, or migrating to `gen_statem`.
- **Related skills:** `beam-gen-statem`, `beam-supervision`, `beam-errors-failures`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/refactoring.md`

### gen-event.md

- **Path:** `docs/beam/gen-event.md`
- **Purpose:** BEAM/OTP guidance for the `gen_event` behaviour, grounded in the Erlang/OTP "Event Handling" system doc and the STDLIB `gen_event` module reference. Covers event managers, handlers, notify, and the callback contract.
- **Main topics:**
  - Event manager vs handler (no process isolation)
  - `start_link/1` vs `start/1`
  - `add_handler/3` vs `add_sup_handler/3`
  - `notify/2`/`sync_notify/2`
  - `call/3`/`delete_handler/3`
  - handler callback contract
  - `handle_info/2`/`code_change/3`
  - no `get_state` for managers (use `sys:get_status/1`)
- **When a future agent should read it:** Consult when implementing or reviewing event managers/handlers.
- **Related skills:** `beam-gen-server`, `beam-supervision`, `beam-errors-failures`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/implementation.md`

### supervision.md

- **Path:** `docs/beam/supervision.md`
- **Purpose:** BEAM/OTP supervision guidance in Erlang/OTP terminology, grounded in the official Erlang/OTP 29.0.2 documentation. Covers supervisor flags, child specs, restart strategies, and auto-shutdown.
- **Main topics:**
  - `init/1` return shape
  - `supervisor_flags` map (strategy, intensity, period, hibernate_after, auto_shutdown)
  - `child_spec` map
  - strategies (`one_for_one`/`one_for_all`/`rest_for_one`/`simple_one_for_one`)
  - restart values
  - shutdown values
  - `auto_shutdown` and `significant`
  - intensity/period
  - start/stop ordering
- **When a future agent should read it:** Consult when designing, reviewing, or debugging supervision trees, child specs, restart/shutdown strategies, or auto-shutdown.
- **Related skills:** `beam-supervision`, `beam-gen-server`, `beam-errors-failures`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`, `docs/beam/workflows/refactoring.md`, `docs/beam/workflows/validation.md`

### applications.md

- **Path:** `docs/beam/applications.md`
- **Purpose:** BEAM/OTP guidance for Erlang/OTP applications: the `.app` resource file, the `application` callback module, the application controller, application environment, start phases, included applications, and restart types.
- **Main topics:**
  - `.app` resource file term format
  - required vs optional keys
  - application callback (`start/2`, `stop/1`, `prep_stop/1`, `start_phase/3`, `config_change/3`)
  - StartType values
  - application controller/master
  - env API and precedence
  - restart types
- **When a future agent should read it:** Consult when writing/reviewing `.app` files, application callback modules, env precedence, or start types.
- **Related skills:** `beam-applications-releases`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/validation.md`

### releases.md

- **Path:** `docs/beam/releases.md`
- **Purpose:** BEAM/OTP guidance for releases and release handling: boot scripts, embedded vs interactive mode, `.appup`/`relup` instructions, `release_handler` sequence, code replacement, `on_load`, target systems, `heart`, and the Appup Cookbook recipes.
- **Main topics:**
  - Boot scripts
  - embedded vs interactive mode
  - code replacement (old/current code, purge/soft_purge)
  - `on_load`
  - `.appup` structure and instruction set
  - `update` mechanics
  - `relup` generation
  - `release_handler` sequence
  - `config_change`
  - target systems and `heart`
  - appup cookbook recipes (functional vs residence modules, supervisor/gen_server updates)
- **When a future agent should read it:** Consult when building, upgrading, downgrading, or debugging OTP releases.
- **Related skills:** `beam-applications-releases`
- **Related workflow files:** `docs/beam/workflows/validation.md`

### logger-and-config.md

- **Path:** `docs/beam/logger-and-config.md`
- **Purpose:** BEAM/OTP guidance for the OTP Logger (Kernel, OTP 21+) and the OTP configuration file format (`config(4)`). Covers levels, handlers, filters, formatters, macros, structured logging, and `sys.config`.
- **Main topics:**
  - Logger architecture (API + backend, primary/handler filters)
  - 8 log levels
  - `compare_levels/2`
  - macros vs functions
  - report callbacks
  - filters
  - handlers (`logger_std_h`, `logger_disk_log_h`)
  - formatters
  - `config(4)` format and merge/override precedence
  - correction notes (no `${VAR}` interpolation, no `Config.Reader` in OTP)
- **When a future agent should read it:** Consult when writing/reviewing logging or configuration code.
- **Related skills:** `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/runtime-diagnosis.md`

### proc-lib-and-sys.md

- **Path:** `docs/beam/proc-lib-and-sys.md`
- **Purpose:** `proc_lib` and `sys` are the two STDLIB modules that form the foundation for OTP-compliant processes that are not one of the standard behaviours. `proc_lib` provides start; `sys` is the interface to system messages.
- **Main topics:**
  - Special process definition
  - what `proc_lib` adds over raw `spawn*`
  - `init_ack`/`init_fail`, `start_spawn_option`, `start_link`/`start_monitor` rules, `stop/1,3`, `initial_call`, `set_label`/`get_label`
  - `sys` rules (default timeout, debug structure, `change_code` requires suspend, `get_state`/`replace_state` debugging-only)
  - system-message protocol
  - `system_*` callbacks
- **When a future agent should read it:** Consult when implementing or reviewing a hand-written special process, user-defined behaviour, or any `proc_lib`/`sys` integration.
- **Related skills:** `beam-gen-server`, `beam-supervision`, `beam-errors-failures`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/debugging.md`, `docs/beam/workflows/refactoring.md`, `docs/beam/workflows/runtime-diagnosis.md`, `docs/beam/workflows/validation.md`

### runtime-debugging.md

- **Path:** `docs/beam/runtime-debugging.md`
- **Purpose:** Runtime debugging on the BEAM spans three layers: the `sys` protocol, the `trace` module (OTP 27.0), and the introspection BIFs (`process_info`, `system_info`, `statistics`, `memory`). Includes the `dbg` interactive tracer and the `ttb` Trace Tool Builder.
- **Main topics:**
  - `sys` module (system messages, `get_state`/`get_status`/`replace_state`, suspend/resume/terminate/change_code, debug control)
  - `trace` module (session lifecycle, process/port/function/send/recv tracing, flags)
  - `dbg` text trace facility (tracer, `p/2`, `tp`/`tpl`/`tpe`, match specs, built-in aliases)
  - `ttb` Trace Tool Builder (distributed tracing, `start_trace/4`, `format/2`, history/config)
  - introspection BIFs (`process_info/1,2` items, `system_info/1`, `statistics/1`, `memory/0`)
  - reduction counting
  - Observer/crash-dump coverage gap
- **When a future agent should read it:** Consult when debugging or validating at runtime via `sys`, `trace`, `dbg`, `ttb`, or introspection BIFs.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/debugging.md`, `docs/beam/workflows/runtime-diagnosis.md`, `docs/beam/workflows/validation.md`

### ports-io.md

- **Path:** `docs/beam/ports-io.md`
- **Purpose:** Ports are the BEAM's basic mechanism for communicating with the external world: a byte-oriented interface to an external program in a separate OS process. Covers the owner/connected-process model, port signals, BIFs, and linked-in drivers.
- **Main topics:**
  - Ports as processes
  - owner/connected process
  - `open_port` forms and options
  - port signals and equivalent BIFs
  - linked-in drivers vs port drivers vs plain ports
  - fault-isolation warning
  - contrast with `os:cmd/1` (see runtime-environment.md)
- **When a future agent should read it:** Consult when implementing or reviewing external-program communication via ports.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/debugging.md`

### distribution.md

- **Path:** `docs/beam/distribution.md`
- **Purpose:** Distributed Erlang is a set of runtime systems ("nodes") communicating over TCP/IP. Covers node naming, cookie authentication, EPMD, `net_kernel`, distributed message passing, `global`, `erpc` vs `rpc`, hidden nodes, TLS distribution, and alternative distribution carriers.
- **Main topics:**
  - Nodes, names, EPMD
  - cookie security model
  - `net_kernel` (start/stop/monitor_nodes/net_ticktime)
  - `net_ticktime` failure detection
  - distributed message passing and transitive connections
  - hidden nodes and dynamic names
  - TLS distribution
  - `global` registration and resolver policies
  - `erpc` vs `rpc`
  - alternative distribution carriers (`-proto_dist`, distribution controller, non-blocking-tick caveat)
- **When a future agent should read it:** Consult when designing/reviewing distributed Erlang.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`, `docs/beam/workflows/runtime-diagnosis.md`, `docs/beam/workflows/validation.md`

### timers.md

- **Path:** `docs/beam/timers.md`
- **Purpose:** BEAM timers come in two layers: the `timer` module (STDLIB) and the BIF timers (`erlang:send_after/3`, `erlang:start_timer/3`), which are more efficient and preferred at scale. Covers time-correction and time-warp-mode guarantees.
- **Main topics:**
  - `timer` module (one-shot/interval send/apply/exit/kill, `sleep`, `tc`)
  - BIF timers (`send_after/3,4`, `start_timer/3,4`, `cancel_timer`, `read_timer`)
  - timer-module bottleneck caveat
  - accuracy and monotonic time
  - time correction, time warp modes, and the `now/0` deprecation
- **When a future agent should read it:** Consult when implementing or reviewing timer-based logic.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/runtime-diagnosis.md`

### nifs.md

- **Path:** `docs/beam/nifs.md`
- **Purpose:** A NIF library contains native (C) implementations of some functions of an Erlang module. Covers `ERL_NIF_INIT` and lifecycle callbacks, dirty schedulers, resource objects, environments, cardinal safety rules, versioning, and ports-vs-NIFs guidance.
- **Main topics:**
  - What NIFs are and when to use
  - `ERL_NIF_INIT` + lifecycle callbacks
  - dirty schedulers (CPU vs IO)
  - resource objects
  - environments and term construction
  - cardinal rules (latency, crash=VM crash, lifetime)
  - versioning/static NIFs
  - ports vs NIFs
- **When a future agent should read it:** Consult when implementing or reviewing NIFs.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`, `docs/beam/workflows/runtime-diagnosis.md`, `docs/beam/workflows/validation.md`

### ets-data.md

- **Path:** `docs/beam/ets-data.md`
- **Purpose:** ETS (Erlang Term Storage) is the interface to Erlang's built-in term storage BIFs: dynamic tables of tuples with constant-time access. Covers table creation, CRUD, match/select, ownership/heir/give_away, `ordered_set` quirks, and `safe_fixtable`. Contrasts ETS with DETS (disk-based) and Mnesia (transactional/distributed).
- **Main topics:**
  - `new/2` types/protection/options
  - CRUD and traversal
  - `match`/`match_object`/`select` + match specs and `fun2ms`
  - ownership/heir/`give_away`
  - `ordered_set` quirks
  - `safe_fixtable`
  - DETS vs ETS (disk-backed, 2 GB limit, no `ordered_set`, repair on open)
  - Mnesia overview (transactional, distributed, ETS/DETS-backed)
- **When a future agent should read it:** Consult when implementing or reviewing ETS tables, or choosing between ETS/DETS/Mnesia.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/runtime-diagnosis.md`

### common-mistakes.md

- **Path:** `docs/beam/common-mistakes.md`
- **Purpose:** Catalogue the BEAM/Erlang pitfalls enumerated in the Erlang/OTP "Common Caveats" chapter and the "Secure Coding" guidelines, each with the problem and recommended practice.
- **Main topics:**
  - `timer` module bottleneck
  - `++` copies left operand
  - accidental copying when spawning/sending funs
  - atom exhaustion
  - `length/1` is O(n)
  - `setelement/3` copies
  - `size/1` too generic
  - NIFs — too much or too little work
  - secure-coding pitfalls (`binary_to_term` on untrusted input, code injection via `eval`/`apply`, cookie/distribution security, input validation)
- **When a future agent should read it:** Consult when reviewing or auditing BEAM code for performance/correctness/security pitfalls.
- **Related skills:** `beam-processes`, `beam-observability-debugging`
- **Related workflow files:** `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`

### validation.md

- **Path:** `docs/beam/validation.md`
- **Purpose:** Cross-cutting BEAM validation hooks: compile-time gates, runtime sanity checks, introspection assertions, and observability monitors that catch regressions early. Synthesizes validation hooks from the crawled corpus into a single checklist.
- **Main topics:**
  - Compile-time gates (gap — not crawled)
  - runtime sanity via `sys`
  - process introspection
  - system introspection
  - trace-based validation
  - ETS checks
  - timer checks
  - NIF checks
  - distribution checks
  - common-caveat guards
  - secure-coding validation hooks
- **When a future agent should read it:** Consult when building or running a validation suite before merge/release.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`, `beam-logger-config`
- **Related workflow files:** `docs/beam/workflows/validation.md`

### binaries.md

- **Path:** `docs/beam/binaries.md`
- **Purpose:** Bit syntax + binary representations + efficient building. Source-verified guidance for constructing and matching binaries/bitstrings (segment syntax, type/signedness/endianness/unit specifiers, defaults, construction/matching rules) and for the internal binary representations (refc/heap binaries, sub-binaries, match contexts), the append optimization, iolist/iodata building, and the circumstances that force copying.
- **Main topics:**
  - Bit syntax construction & matching (`<<E1,...>>`, `Value:Size/TypeSpecifierList`)
  - segment Type (integer/float/binary/bitstring/utf8/utf16/utf32), Signedness, Endianness, Unit
  - defaults (type/size/unit/signedness/endianness)
  - construction rules (`badarg`, alignment, parenthesizing expressions)
  - matching rules (Size as guard expression, OTP 23+; rest of binary/bitstring)
  - binary representations (refc binaries, heap binaries ≤64 bytes, sub-binaries, match contexts)
  - append optimization (first segment, 2x/256-byte growth)
  - iolist/iodata building; prepend-vs-append cost
  - circumstances that force copying (message send, ETS insert, `port_command`, `enif_inspect_binary`)
- **When a future agent should read it:** Consult when constructing or matching binaries, building iolists, or reviewing binary-handling code for correctness or performance.
- **Related skills:** `beam-processes`, `beam-observability-debugging`, `beam-errors-failures`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/runtime-diagnosis.md`

### runtime-environment.md

- **Path:** `docs/beam/runtime-environment.md`
- **Purpose:** File + os runtime modules. Source-verified guidance for the two Kernel modules that interface the BEAM to the host operating system: `file` (filesystem open/read/write/close, modes, metadata, raw vs file-server, atomicity caveats, filename encoding) and `os` (environment variables, `cmd/1,2`, process id, OS time/perf counters, platform type/version, signal handling).
- **Main topics:**
  - `file` module: `read_file`/`write_file`, `open/2` modes (`read`/`write`/`append`/`exclusive`/`raw`/`binary`/`delayed_write`/`read_ahead`/`compressed`/`zstd`/`encoding`/`sync`/`ram`/`directory`), `read`/`read_line`/`write`, `list_dir`, `make_dir`/`del_dir`, `delete`, `copy`, `rename`, `read_file_info`, `get_cwd`, `datasync`/`sync`
  - file-server atomicity caveats; `raw` mode limitations; `delayed_write` late-error semantics
  - filename encoding (`native_name_encoding/0`, `+fnl`/`+fnu`)
  - `os` module: `env/0`/`getenv/1,2`/`putenv/2`/`unsetenv/1`, `find_executable/1,2`, `getpid/0`, `perf_counter/0,1`, `system_time/0,1`, `timestamp/0`, `type/0`, `version/0`, `set_signal/2`
  - `cmd/1,2` caveats (buffers all output, shell subprocess, `max_size`, `exception_on_failure`) and `cmd` vs ports guidance
  - OS time vs Erlang monotonic time (do not use `os:system_time` for intervals)
- **When a future agent should read it:** Consult when implementing or reviewing filesystem access, environment-variable handling, OS command execution, or OS time/identity queries.
- **Related skills:** `beam-observability-debugging`, `beam-processes`, `beam-errors-failures`, `beam-applications-releases`
- **Related workflow files:** `docs/beam/workflows/implementation.md`, `docs/beam/workflows/code-review.md`, `docs/beam/workflows/debugging.md`, `docs/beam/workflows/runtime-diagnosis.md`

## Recommended reading paths

### New to BEAM/OTP

1. `overview.md` — supervision trees, behaviours, applications, releases.
2. `otp-behaviours.md` — the generic/specific split and the four standard behaviours.
3. `supervision.md` — supervisor flags, child specs, restart strategies.
4. `processes-and-messages.md` — spawn, send/receive, registration, process flags.
5. `common-mistakes.md` — the recurring performance/correctness traps.

### Writing OTP code

1. `otp-behaviours.md` — choose the right behaviour.
2. The specific behaviour doc: `gen-server.md`, `gen-statem.md`, `gen-event.md`, or `supervision.md`.
3. `proc-lib-and-sys.md` — only if writing a special process.
4. `applications.md` — package callbacks into an application.
5. `common-mistakes.md` — avoid the known traps.

### Reviewing BEAM/OTP code

1. `supervision.md` — child specs, restart/shutdown strategies, intensity/period.
2. The behaviour doc matching the change (`gen-server.md`, `gen-statem.md`, etc.).
3. `links-monitors-and-exits.md` — failure propagation and `trap_exit` policy.
4. `common-mistakes.md` — performance pitfalls.
5. `binaries.md` — binary-handling and copying pitfalls.
6. `validation.md` — gates to run before merge.

### Debugging a crash or hang

1. `links-monitors-and-exits.md` — exit reasons, `trap_exit`, `DOWN`, exit-signal propagation.
2. `processes-and-messages.md` — process states, mailboxes, process flags.
3. `runtime-debugging.md` — `sys`, `trace`, `dbg`, `ttb`, introspection BIFs.
4. `proc-lib-and-sys.md` — `sys` debugging of special processes.
5. `logger-and-config.md` — Logger configuration for environment-specific issues.

### Releases and deployment

1. `applications.md` — `.app` resource, application callback, env precedence.
2. `releases.md` — boot scripts, code replacement, `release_handler`, `heart`, appup cookbook.
3. `logger-and-config.md` — `sys.config` composition.
4. `validation.md` — release sanity checks.

### Runtime/operational diagnosis

1. `runtime-debugging.md` — `sys`, `trace`, `process_info`/`system_info`/`statistics`/`memory`.
2. `processes-and-messages.md` — mailbox growth, reductions, scheduling.
3. `distribution.md` — node connectivity, `net_ticktime`, `global`.
4. `ets-data.md` — table fixation and memory.
5. `timers.md` — timer-module bottleneck.
6. `binaries.md` — refc-binary retention and binary memory growth.
7. `runtime-environment.md` — filesystem and OS-environment diagnostics.

## Skill derivation map

Skills (opencode skill packages) will be derived from this corpus. The docs are the source of truth; skills encode the actionable procedures agents follow. The mapping below uses the current skill names. NOTE: skills are scheduled to be repaired to align with the new doc names in the next step; several docs currently carry a generic skill list pending that repair.

| Skill | Source docs |
|---|---|
| `beam-supervision` | `overview.md`, `links-monitors-and-exits.md`, `otp-behaviours.md`, `gen-server.md`, `gen-statem.md`, `gen-event.md`, `supervision.md`, `proc-lib-and-sys.md` |
| `beam-gen-server` | `overview.md`, `otp-behaviours.md`, `gen-server.md`, `gen-statem.md`, `gen-event.md`, `supervision.md`, `proc-lib-and-sys.md` |
| `beam-gen-statem` | `otp-behaviours.md`, `gen-statem.md` |
| `beam-processes` | `processes-and-messages.md`, `links-monitors-and-exits.md`, `otp-behaviours.md`, `common-mistakes.md`, `runtime-debugging.md`, `ports-io.md`, `distribution.md`, `timers.md`, `nifs.md`, `ets-data.md`, `binaries.md`, `runtime-environment.md`, `validation.md` |
| `beam-errors-failures` | `links-monitors-and-exits.md`, `gen-server.md`, `gen-statem.md`, `gen-event.md`, `supervision.md`, `proc-lib-and-sys.md`, `runtime-debugging.md`, `ports-io.md`, `distribution.md`, `timers.md`, `nifs.md`, `ets-data.md`, `binaries.md`, `runtime-environment.md`, `validation.md` |
| `beam-applications-releases` | `overview.md`, `applications.md`, `releases.md`, `runtime-debugging.md`, `ports-io.md`, `distribution.md`, `timers.md`, `nifs.md`, `ets-data.md`, `runtime-environment.md`, `validation.md` |
| `beam-logger-config` | `logger-and-config.md`, `runtime-debugging.md`, `ports-io.md`, `distribution.md`, `timers.md`, `nifs.md`, `ets-data.md`, `validation.md` |
| `beam-observability-debugging` | `processes-and-messages.md`, `gen-server.md`, `gen-statem.md`, `gen-event.md`, `supervision.md`, `proc-lib-and-sys.md`, `runtime-debugging.md`, `ports-io.md`, `distribution.md`, `timers.md`, `nifs.md`, `ets-data.md`, `binaries.md`, `runtime-environment.md`, `validation.md`, `common-mistakes.md` |

## Workflow map

Workflow docs live under `docs/beam/workflows/`. BEAM's skill surface is now: `constraint-beam-*` (4 cross-language invariants) + `workflow-beam-runtime-diagnosis` (the one cross-language workflow). BEAM has NO implementation/code-review/refactoring/debugging/validation workflow packages — those belong to the concrete languages (Elixir/Gleam); BEAM is the shared runtime reference + invariants layer they consult. The workflow files below currently reference some pre-consolidation doc names; those references will be updated to the canonical 22 in a follow-up.

| Workflow file | Purpose |
|---|---|
| `docs/beam/workflows/index.md` | Index for the workflow directory; maps task types to workflows, docs, and skills. |
| `docs/beam/workflows/runtime-diagnosis.md` | Step-by-step procedure for diagnosing live-runtime/operational problems (memory leaks, scheduler pressure, crash dumps). Hands off to a concrete language's debugging workflow when a code defect is found. |

## Open policy decisions

Each topic file ends with a "Policy decisions for individual repos" section. Below is a consolidated summary grouped by theme. Each consuming repo should record its answers in a project-level CONTRIBUTING.md.

### Behaviours and special processes

- Whether non-behaviour special processes are permitted and under what review gate. (`overview.md`, `otp-behaviours.md`, `proc-lib-and-sys.md`)
- Whether user-defined (non-standard) behaviours are permitted and their review gate. (`otp-behaviours.md`)
- Whether all processes must use `gen_server`/`gen_statem`/`gen_event`/`supervisor`, or special processes are allowed. (`otp-behaviours.md`)
- Required `system_*` callback coverage (e.g. mandating `system_code_change/4`). (`otp-behaviours.md`)
- Preferred start function for special processes (`start_link` vs `start_monitor` vs `spawn_link`). (`proc-lib-and-sys.md`)
- Whether `init_fail/2,3` (OTP 26+) and `set_label`/`get_label` (OTP 27+) baselines are adopted. (`proc-lib-and-sys.md`)
- Whether `:sys` debug options are permitted in production code. (`gen-statem.md`, `proc-lib-and-sys.md`)
- Default start function (`start_link/3,4`) and registration (`{local,Name}`) for supervised servers. (`gen-server.md`)
- Whether `handle_info/2` and `format_status/1` are mandatory. (`gen-server.md`)
- Whether to hibernate idle servers and at what threshold. (`gen-server.md`)
- Preferred `gen_statem` callback mode and whether `state_enter` is default. (`gen-statem.md`)
- `gen_event` manager start mode, naming, handler supervision, and handler restoration on restart. (`gen-event.md`)

### Process flags and traps

- Whether `trap_exit` is permitted outside supervisors/bridges. (`links-monitors-and-exits.md`)
- Default `max_heap_size` per process class. (`processes-and-messages.md`)
- Whether `async_dist` is enabled for latency-sensitive senders. (`processes-and-messages.md`)
- Whether priority messaging is permitted at all. (`processes-and-messages.md`)
- Whether `exit/2` is banned in favor of `exit_signal/2`. (`links-monitors-and-exits.md`)
- Default monitoring strategy (monitor vs link) for client/server calls. (`links-monitors-and-exits.md`)

### Supervision and restart strategy

- Default intensity/period per supervisor class (top/intermediate/leaf). (`supervision.md`)
- Restart defaults per worker class (`permanent`/`transient`/`temporary`). (`supervision.md`)
- Auto-shutdown policy for non-top supervisors. (`supervision.md`)
- Whether `shutdown => infinity` for supervisor children is enforced by static analysis. (`supervision.md`)
- Where `significant => true` is permitted and who approves. (`supervision.md`)
- `brutal_kill` usage conditions and approval. (`supervision.md`)
- `modules` key completeness requirements. (`supervision.md`)
- Supervisor registration conventions. (`supervision.md`)
- Whether `simple_one_for_one` is permitted in new code. (`supervision.md`)

### Applications

- Default start type (`temporary` vs `permanent`). (`applications.md`)
- `set_env` policy (runtime permitted? `{persistent,true}`?). (`applications.md`)
- Whether `included_applications` and `start_phases` are permitted. (`applications.md`)
- `env` override strategy (`sys.config` only vs CLI flags). (`applications.md`)
- `maxT` usage. (`applications.md`)

### Release handling

- Release-handling strategy: SASL `release_handler`/appup/relup vs full restart. (`overview.md`, `releases.md`)
- Whether `heart` is mandatory in production. (`releases.md`)
- Whether every `.appup` must provide downgrade instructions. (`releases.md`)
- `code_change/3` discipline requirements. (`releases.md`)
- Config file strategy (`sys.config` sole source vs includes). (`releases.md`)
- `on_load`/NIF initializer policy. (`releases.md`)

### Logging and configuration

- Default log levels per environment (dev/staging/prod). (`logger-and-config.md`)
- Macro vs API policy. (`logger-and-config.md`)
- Whether structured logging (reports) is mandatory. (`logger-and-config.md`)
- Handler strategy (`logger_std_h` to file vs custom). (`logger-and-config.md`)
- Config composition (include-file mechanism vs flat `sys.config`). (`logger-and-config.md`)
- SASL compatibility and env-var expansion location. (`logger-and-config.md`)

### NIFs and FFI

- Whether NIFs are permitted at all and the review bar. (`nifs.md`, `common-mistakes.md`)
- Maximum regular-NIF duration before dirty is mandatory. (`nifs.md`)
- Whether dirty CPU/IO schedulers are sized for the workload. (`nifs.md`)
- Required `ERL_NIF_MAJOR_VERSION` baseline. (`nifs.md`)
- Whether linked-in drivers are permitted. (`ports-io.md`)

### ETS and storage

- Default protection level (`protected` vs `private`). (`ets-data.md`)
- Whether `heir` is mandatory for long-lived tables. (`ets-data.md`)
- Whether `write_concurrency, auto` is the default. (`ets-data.md`)
- Match-spec nesting depth limits. (`ets-data.md`)
- When DETS or Mnesia is preferred over ETS. (`ets-data.md`)

### Ports and I/O

- Default framing (`{packet,4}` vs `stream`). (`ports-io.md`)
- Whether port owners must be supervised and trap exits. (`ports-io.md`)
- `os:cmd/1` vs ports for one-shot shell commands. (`ports-io.md`, `runtime-environment.md`)

### Distribution

- Whether TLS distribution is mandatory. (`distribution.md`)
- Default `net_ticktime`/`net_tickintensity`. (`distribution.md`)
- Required `erpc` timeout slack. (`distribution.md`)
- Whether `global` or a custom registry is used for singletons. (`distribution.md`)
- Whether hidden nodes are used for O&M/inspection. (`distribution.md`)
- Whether alternative distribution carriers (`-proto_dist`) are permitted. (`distribution.md`)

### Timers

- Threshold above which BIF timers are mandatory. (`timers.md`)
- Whether `timer:send_interval` is permitted in hot paths. (`timers.md`)
- Default time unit for `tc/*`. (`timers.md`)
- Whether absolute-monotonic deadlines are standard for request timeouts. (`timers.md`)
- Whether `timer:` timer-managing calls are banned via lint. (`common-mistakes.md`)
- Required time-warp-mode (`+C`) and time-correction (`+c`) baseline. (`timers.md`)

### Binaries and data representation

- Default binary type for protocol framing (`binary` vs `bitstring`). (`binaries.md`)
- iolist/iodata vs flat binary policy for accumulation. (`binaries.md`)
- Refc-binary retention policy and sub-binary GC expectations. (`binaries.md`)
- Whether prepend-to-binary patterns are banned via lint. (`binaries.md`)

### Runtime environment (file/os)

- File mode policy (`raw` vs file-server) and when `delayed_write` is permitted. (`runtime-environment.md`)
- `os:cmd/1,2` policy vs ports for external commands. (`runtime-environment.md`)
- OS time source policy (monotonic vs `os:system_time`) for intervals. (`runtime-environment.md`)
- Filename-encoding baseline (`+fnl` vs `+fnu`) and raw-filename handling. (`runtime-environment.md`)

### Debugging and observability

- Default `sys` timeout for production debug probes. (`runtime-debugging.md`)
- Whether `trace:system/3` monitors are always-on in staging. (`runtime-debugging.md`)
- Thresholds for `long_schedule`/`large_heap` monitors. (`runtime-debugging.md`)
- Whether `process_info` is permitted in non-debug modules. (`runtime-debugging.md`)
- Whether `dbg` (interactive) vs `ttb` (scripted) is the staging default. (`runtime-debugging.md`)

### Validation and CI

- Whether `sys:get_state` assertions are permitted in integration tests. (`validation.md`)
- `trace:system/3` thresholds for staging monitors. (`validation.md`)
- Required `erlang:load_nif/2` assertion in every NIF module test. (`validation.md`)
- Atom-count alarm threshold relative to `atom_limit`. (`validation.md`, `common-mistakes.md`)
- Whether `erpc` results must be checked per-node. (`validation.md`)
- Whether `list_to_atom`/`binary_to_atom` are banned in CI. (`common-mistakes.md`)
- Whether `binary_to_term/2` must always pass `[safe]` on untrusted input. (`common-mistakes.md`, `validation.md`)
- Atom-table limit tuning (`+t`). (`common-mistakes.md`)
