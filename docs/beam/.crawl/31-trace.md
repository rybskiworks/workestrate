# Crawl: kernel/trace.html
- seed_url: https://www.erlang.org/doc/apps/kernel/trace.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: runtime-debugging.md, observability

## Purpose
The `trace` module is the Erlang trace interface — a front-end to the runtime's
trace points (function calls, message send/receive, GC, scheduling, process/port
events). It is the modern, session-isolated successor to the older
`erlang:trace/3`, `erlang:trace_pattern/3`, and `erlang:trace_info/2` BIFs,
introduced in OTP 27.0. It is intended both for direct use and as a building
block for debugging/profiling tools. For debugging Erlang code the docs recommend
`dbg`; for profiling, `tprof`.

## Key functions (exact arities)
- `session_create(Name, Tracer, Opts) -> session()` — OTP 27.0. Create an isolated
  trace session. `Name :: atom()`, `Tracer :: pid() | port() | {module(), term()}`,
  `Opts :: []` (must be empty). Returns opaque handle `{session_strong_ref(),
  session_weak_ref()}`. Dropping/GC of the strong handle destroys the session.
- `session_destroy(Session) -> true | false` — OTP 27.0. Cleanup all settings on
  processes, ports, functions. Already-sent trace messages are NOT cleaned up.
- `session_info(PidPortFuncEvent) -> Res` — OTP 27.0. Returns weak session handles
  affecting a pid/port/function/event; `all` returns all active sessions.
- `process(Session, Procs, How, FlagList) -> integer()` — OTP 27.0. Turn on/off
  trace flags for processes. `Procs :: pid() | all | existing | new`,
  `How :: boolean()`. Returns count of matched processes (0 for `new`).
- `port(Session, Ports, How, FlagList) -> integer()` — OTP 27.0. Same shape for ports.
- `function(Session, MFA, MatchSpec, FlagList) -> non_neg_integer()` — OTP 27.0.
  Enable/disable call tracing for functions. `MFA :: trace_pattern_mfa() | on_load`.
  Returns number of matching functions (0 for `on_load`).
- `send(Session, MatchSpec, FlagList) -> non_neg_integer()` — OTP 27.0. Set trace
  pattern for message sending. `FlagList :: []`. Match on `[Receiver, Msg]`.
- `recv(Session, MatchSpec, FlagList) -> non_neg_integer()` — OTP 27.0. Set trace
  pattern for message receiving. `FlagList :: []`. Match on `[Node, Sender, Msg]`.
- `info(Session, PidPortFuncEvent, Item) -> Res` — OTP 27.0. Return trace info about
  a port/process/function/event. Items: `flags | tracer | traced | match_spec |
  meta | meta_match_spec | call_count | call_time | call_memory | all`.
- `system(Session, Event, Value) -> ok` — OTP 28.0. Enable/disable monitoring of
  system events (`long_gc`, `long_message_queue`, `long_schedule`, `large_heap`,
  `busy_port`, `busy_dist_port`). Session tracer MUST be a process.
- `delivered(Session, Tracee) -> reference()` — OTP 27.0. Equivalent to
  `erlang:trace_delivered(Tracee)` within the session.

## trace flags + trace_pattern match specs
`trace_flag() :: trace_info_flag() | all | cpu_timestamp`
`trace_info_flag() :: arity | call | exiting | garbage_collection |
  monotonic_timestamp | procs | ports | 'receive' | return_to | running |
  running_procs | running_ports | send | set_on_first_link | set_on_first_spawn |
  set_on_link | set_on_spawn | silent | strict_monotonic_timestamp | timestamp`

Process/port flags (subset):
- `send` / `'receive'` — message send/receive tracing (limited by `send/3`,`recv/3`).
- `call` — function call tracing (combined with `function/4`). Tags: `call`,
  `return_from`. Use `{return_trace}`/`{exception_trace}` match-spec actions for
  return values.
- `return_to` — only with `call`; only for `local`-traced functions. One message
  per chain of tail calls (preserves tail-recursion properties).
- `silent` — with `call`; inhibits `call`/`return_from`/`return_to` messages but
  match specs still execute. Toggle via `{silent, Bool}` match-spec action. Enables
  tracing many/all processes then activating selectively via match spec.
- `procs` — spawn/spawned/exit/register/unregister/link/unlink/getting_linked/
  getting_unlinked.
- `ports` — open/closed/register/unregister/getting_linked/getting_unlinked.
- `running` / `running_procs` / `running_ports` / `exiting` — scheduling events
  (`in`/`out`, `in_exiting`/`out_exiting`/`out_exited`).
- `garbage_collection` — `gc_minor_start`, `gc_max_heap_size`, `gc_minor_end`
  (also `gc_major_start`/`gc_major_end`).
- `set_on_spawn` / `set_on_first_spawn` / `set_on_link` / `set_on_first_link` —
  inheritance of trace flags to spawned/linked processes.
- `arity` — with `call`; emits `{M,F,Arity}` instead of `{M,F,Args}`.
- Timestamps: `timestamp` (wall, `erlang:now/0` form), `cpu_timestamp` (global,
  `Procs==all` only, not all platforms, may go backwards across cores),
  `monotonic_timestamp` (`erlang:monotonic_time(nanosecond)`),
  `strict_monotonic_timestamp` (`{monotonic_time(nanosecond),
  unique_integer([monotonic])}`). Precedence: `timestamp` >
  `strict_monotonic_timestamp` > `monotonic_timestamp`; all remembered so disabling
  the highest re-enables the next.

`trace_pattern_flag() :: global | local | meta | call_count | call_time | call_memory`
- `global` (default if FlagList empty) — only exported functions, only global calls.
- `local` — all call types; enables `return_to`.
- `meta` — meta-tracing; traces ALL processes regardless of process trace flags
  (fixed `[call, timestamp]`); `{return_trace}` works.
- `call_count` / `call_time` / `call_memory` — counter-based profiling; no process
  trace flags needed; `restart`/`pause` MatchSpec controls counters.
- `global` cannot combine with the others; enabling one disables the others for
  the matching function set.

`trace_match_spec() :: [{[term()] | '_' | match_variable(), [term()], [term()]}]`
- `function/4` MatchSpec forms: `true` (enable, remove spec), `false` (disable),
  `MatchExpression` (empty list == `true`), `restart`, `pause` (counters only).
- MFA wildcards: `{M,F,'_'}`, `{M,'_','_'}`, `{'_','_','_'}`. `{M,'_',Arity}` NOT allowed.
- `on_load` atom — apply to all functions in newly loaded modules.
- `send/3` matches on `[Receiver, Msg]`; `recv/3` matches on `[Node, Sender, Msg]`.
  Both default to `true` (all messages). `recv` match spec CANNOT use `caller`,
  `is_seq_trace`, `get_seq_token`, `set_seq_token`, `enable_trace`, `disable_trace`,
  `trace`, `silent`, `process_dump`. `send` match spec CANNOT use `caller`.
- `{message}` action with non-boolean value adds extra element to trace message
  (before timestamp if present).
- Failures: `badarg` (invalid args), `system_limit` (excessive match-spec nesting
  → scheduler stack exhaustion; configurable via `+sss`/`erl_cmd sched_thread_stack_size`).

## Relationship to dbg / seq_trace
- `trace` module introduced OTP 27.0 as the session-isolated successor to the
  static single-node-session BIFs `erlang:trace/3`, `erlang:trace_pattern/3`,
  `erlang:trace_info/2`. Old BIFs operate on ONE static trace session per node,
  so different users/tools interfere. New module uses dynamically created,
  isolated sessions; `session_destroy/1` cleans up everything in one call.
- Mapping table (old → new): `erlang:trace(Pid,...)`→`process(S,Pid,...)`;
  `erlang:trace(processes,...)`→`process(S,all,...)`; `existing_processes`→
  `process(S,existing,...)`; `new_processes`→`process(S,new,...)`; ports analog
  via `port/4`; `erlang:trace(all,...)`→both `process(S,all,...)` and `port(S,all,...)`;
  `erlang:trace_pattern(MFA,...)`→`function(S,MFA,...)`; `erlang:trace_pattern(send,...)`
  →`send(S,...)`; `erlang:trace_pattern('receive',...)`→`recv(S,...)`;
  `erlang:trace_info(...)`→`info(S,...)`.
- Options `{tracer,T}`, `{tracer,M,S}`, `{meta,T}`, `{meta,M,S}` are NOT allowed in
  the new module — the tracer is fixed at session creation. Default tracer is never
  the calling process.
- `dbg` (runtime_tools) is RECOMMENDED for debugging Erlang code; `tprof` (tools)
  for profiling. `trace` module is the lower-level building block.
- `ttb` (observer) and `dbg` are the path for REMOTE node tracing — `trace` module
  operates on LOCAL node only (traced entities AND tracer must be local).
- `seq_trace` (sequential trace) is a SEPARATE mechanism; the `recv` match spec
  explicitly forbids `is_seq_trace`/`get_seq_token`/`set_seq_token` guard functions,
  indicating seq_trace tokens are a distinct subsystem not managed by this module.
  Trace token/session semantics here refer to the `session()` handle
  (`{session_strong_ref(), session_weak_ref()}`), NOT seq_trace tokens.

## Performance caveats
- Untargeted tracing is expensive: tracing `all` processes/ports sends a trace
  message for every matching event. The `silent` flag is the documented mechanism
  to trace many/all processes cheaply then activate selectively via match-spec
  `{silent, Bool}` — "giving a high degree of control of which functions with which
  arguments that trigger the trace."
- `cpu_timestamp` is global, not supported on all platforms, and most OSes do not
  synchronize the value across cores — "time can seem to go backwards."
- Match specs with excessive nesting raise `system_limit` due to scheduler stack
  exhaustion.
- System monitor (`system/3`): "If the session tracer process gets so large that it
  itself starts to cause system monitor messages when garbage collecting, the
  messages enlarge the process message queue and probably make the problem worse.
  Keep the tracer process neat and do not set the system monitor limits too tight."
- `long_message_queue`: use a much smaller `Disable` than `Enable` to avoid being
  flooded with monitor messages.
- `long_schedule`: 1 ms is a good max for a driver callback or NIF; <100 ms is
  "possible"/"normal" on time-sharing systems; longer indicates swapping or
  misbehaving NIF/driver → bad resource utilization and overall performance.
- Meta-tracing traces ALL processes (fixed flags `[call, timestamp]`) — broader
  blast radius than per-process call tracing.
- If the tracing process/port dies or the tracer module returns `remove`, flags
  are silently removed.

## Strict rules
- `trace` module is LOCAL-node only. For remote nodes use `dbg` or `ttb`.
- A session MUST be created with `session_create/3` before any other call.
- `function/4` call tracing requires ALSO setting the `call` flag via `process/4`
  on the target processes — call tracing only fires when a traced process calls a
  traced function.
- `send/3` and `recv/3` require the `send`/`'receive'` flag set via `process/4` or
  `port/4`. Their `FlagList` MUST be `[]`.
- `return_to` only works with `local`-traced functions and the `call` flag.
- `global` cannot combine with `local`/`meta`/`call_count`/`call_time`/`call_memory`.
  Disabling trace must use the matching flag type (e.g. disable `local` with `local`).
- `cpu_timestamp` only allowed with `Procs == all`.
- `system/3` requires the session tracer to be a local process (else `badarg`).
- `session_destroy/1` does not retract already-sent trace messages.
- Wildcard `{Module,'_',Arity}` is NOT allowed in `function/4` MFA.
- Match specs cannot be partially changed; replace wholesale (retrieve via `info/3`).

## Verbatim quotes
- "The functions in this module can be used directly, but can also be used as
  building blocks to build more sophisticated debugging or profiling tools. For
  debugging Erlang code it is recommended to use dbg and for profiling to use tprof."
- "All tracing is done within a trace session. Trace sessions can be created and
  destroyed dynamically. Each session has its own tracer that will receive all
  trace messages. Several sessions can exist at the same time without interfering
  with each other."
- "The functions in this module only operates on the local node. ... To trace remote
  nodes use dbg or ttb."
- "This trace module was introduced in OTP 27.0. The interface and semantics are
  similar to the older functions erlang:trace/3 , erlang:trace_pattern/3 , and
  erlang:trace_info/2 . The main difference is the old functions operate on a
  single static trace session per node. That could impose the problem that different
  users and tools would interfere with each other's trace settings."
- "The silent trace flag facilitates setting up a trace on many or even all
  processes in the system. The trace can then be activated and deactivated using
  the match specification function {silent,Bool} , giving a high degree of control
  of which functions with which arguments that trigger the trace."
- "Using call and return_to trace together makes it possible to know exactly in
  which function a process executes at any time."
- "For tail calls, only one trace message is sent per chain of tail calls, so the
  properties of tail recursiveness for function calls are kept while tracing with
  this flag."
- "If the session tracer process gets so large that it itself starts to cause
  system monitor messages when garbage collecting, the messages enlarge the
  process message queue and probably make the problem worse. Keep the tracer
  process neat and do not set the system monitor limits too tight."
- "1 ms is considered a good maximum time for a driver callback or a NIF."
- "Options {tracer,T} , {tracer,M,S} , {meta,T} , and {meta,M,S} are therefore not
  allowed, and the default tracer is never the calling process."

## Version notes
- Module introduced OTP 27.0 (all functions except `system/3`).
- `system/3` introduced OTP 28.0.
- Page metadata: OTP 29.0.2, kernel 11.0.2. Built with ExDoc v0.40.3.
- `session_weak_ref()` from `session_info/1` does not prevent session destruction
  when last strong handle is GC'd.
- `call_memory` accumulation stops at the next memory-traced function (nested
  attribution rule).

## Discovered links
### Relevant (crawl later)
- ../../apps/runtime_tools/dbg.html — `dbg` debugger (recommended debugging front-end)
- ../../apps/observer/ttb.html — `ttb` trace tool for remote/multi-node tracing
- ../../apps/tools/tprof.html — `tprof` profiler (recommended for profiling)
- ../../apps/erts/match_spec.html — Match Specifications in Erlang (referenced repeatedly)
- ../../apps/erts/erl_tracer.html — `erl_tracer` tracer module callback interface
- ../../apps/erts/erlang.html#trace/3 — legacy `erlang:trace/3` BIF
- ../../apps/erts/erlang.html#trace_pattern/3 — legacy `erlang:trace_pattern/3` BIF
- ../../apps/erts/erlang.html#trace_info/2 — legacy `erlang:trace_info/2` BIF
- ../../apps/erts/erlang.html#trace_delivered/1 — `erlang:trace_delivered/1`
- ../../apps/erts/time_correction.html#erlang-monotonic-time — monotonic time semantics
- ../../apps/erts/erl_cmd.html#sched_thread_stack_size — scheduler stack size (+sss)

### Skipped
- In-page anchors (#module-trace-sessions, #process_trace_messages_*, #port_trace_messages_*, #t:trace_flag/0, etc.) — internal to this page.
- GitHub source links (https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/trace.erl#L*) — source, not docs.
- kernel.epub, trace.md, trace.html — self/download variants.
- Type-reference links to basic types (#t:atom/0, #t:pid/0, etc.) — primitive.
