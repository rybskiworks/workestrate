# Crawl: observer/ttb.html
- seed_url: https://www.erlang.org/doc/apps/observer/ttb.html
- canonical_url: https://www.erlang.org/doc/apps/observer/ttb.html
- family: Erlang/OTP module docs (app = observer, v2.19)
- fetch: 200
- otp_version: OTP 29.0.2
- feeds_docs: runtime-debugging.md

## Purpose
ttb (Trace Tool Builder) is "a base for building trace tools for distributed
systems." It wraps Erlang/OTP's low-level tracing (the `dbg` module in
Runtime_Tools) with conveniences for distributed tracing: a file trace port
started on every node, automatic collection ("fetch") of logs to a trace
control node, a history buffer of every ttb call (for reproducible
configurations), configuration-file save/restore, sequential-trace
integration, and offline formatting of the binary trace logs.

Explicit warning from the page:
> "When using ttb, do not use module dbg in application Runtime_Tools in
> parallel."

## Key functions (exact arities)
Present on this page (OTP 29.0.2 / observer 2.19):

- `tracer/0` — `Equivalent to tracer(node()).`
- `tracer/1` — `tracer(shell | dbg | nodes())`; shortcuts for common settings.
  - `shell` => `tracer(node(),[{file, {local, "ttb"}}, shell])`
  - `dbg`  => `tracer(node(),[{shell, only}])`
  - `Nodes` => `tracer(Nodes,[])`
- `tracer/2` — `tracer(Nodes, Opts)`. Starts a file trace port on all nodes
  and points the system tracer for sequential tracing to the same port.
- `p/2` — `p(Item, Flags)`. Sets trace flags on processes/ports; flag
  `timestamp` is always turned on. `MatchDesc` is the same as returned from
  `dbg:p/2`.
- `tp/2,3,4` — set trace patterns on **global** function calls. Equivalent to
  `tpl/4` family. All calls stored in history.
- `tpl/2,3,4` — `tpl(Module, Function, Arity, MatchSpec)`. Set trace patterns
  on **local and global** function calls. Used with trace flags `call`,
  `send`, `'receive'`.
- `tpe/2` — `tpe(Event, MatchSpec)` (since OTP 19.0). Set trace patterns on
  **messages** (`send` / `'receive'`).
- `ctp/0,1,2,3` — clear trace patterns on local and global function calls.
- `ctpl/0,1,2,3` — clear trace patterns on local function calls.
- `ctpg/0,1,2,3` — clear trace patterns on global function calls.
- `ctpe/1` — clear trace patterns on messages (since OTP 19.0).
- `start_trace/4` — `start_trace(Nodes, Patterns, FlagSpec, TracerOpts)` (since
  OTP R15B). One-command shortcut: each tuple in `Patterns` is converted to a
  list and passed to `ttb:tpl/2,3,4`.
- `stop/0` — `Equivalent to stop([]).`
- `stop/1` — `stop(Opts :: stop_opts())`. Stops tracing on all nodes; fetches
  logs/`.ti` files to the trace control node into
  `ttb_upload_FileName-Timestamp`; saves history to `ttb_last_config`.
- `format/1` — `format(Files)`. `Equivalent to format(Files, []).`
- `format/2` — `format(Files, Options)`. Reads binary trace log(s); logs
  processed in timestamp order unless `disable_sort`.
- `get_et_handler/0` — returns the `et` handler for use with `format/2` or
  `tracer/2` (since OTP R15B).
- `write_trace_info/2` — `write_trace_info(Key, Info)`. Adds `Data` to the
  `ValueList` for `Key` in the `.ti` file; all such info is included in the
  call to the format handler.
- `seq_trigger_ms/0` — `Equivalent to seq_trigger_ms(all).`
- `seq_trigger_ms/1` — returns a match spec that turns on sequential tracing
  with the given `Flags`; usable as the last arg to `tp`/`tpl`.
- `list_history/0`, `run_history/1` — history buffer of all ttb calls.
- `list_config/1`, `run_config/1,2`, `write_config/2,3` — configuration-file
  save/restore of trace setups.

Match-spec shortcuts accepted by `tp`/`tpl`:
- `return` — for `[{'_',[],[{return_trace}]}]` (report return value)
- `caller` — for `[{'_',[],[{message,{caller}}]}]` (report calling function)
- `{codestr, Str}` — for `dbg:fun2ms/1` arguments passed as strings

### Requested-but-absent on this page
The crawl brief listed several functions that do **not** appear in the OTP 29
`observer` ttb module page. Recorded here so the gap is explicit:
- `start_trace/3` — only `start_trace/4` is exported (the 3-arg form is not
  present; the brief's "start_trace/3,4" collapses to `/4` only).
- `trace_pattern/2` — not exported by ttb; pattern setting is via
  `tp/tpl/tpe` (ttb has no function literally named `trace_pattern`).
- `set_ts/3` — not present.
- `get_all_ts/0` — not present.
- `stop/2` — only `stop/0` and `stop/1` are exported; the "fetch"/"return"
  behaviour is via `stop/1` options `nofetch`, `{fetch, Dir}` (note: page
  documents `{fetch, Dir}` in prose though the `stop_opt()` type lists
  `{fetch_dir, file:filename()}`), `format`, `return_fetch_dir`.
- `ddll/...` — not present (no `ddll` function family in this module).

## start_trace + tracing-to-disk flow
The end-to-end distributed trace-to-disk flow as automated by ttb:

1. **Start tracer(s) on all nodes** — `ttb:tracer(Nodes, Opts)` starts a file
   trace port on every specified node and points the system tracer for
   sequential tracing to the same port. The `Filename` is prefixed with the
   node name (default `ttb`). Wrap logs via `{wrap, Filename, Size, Count}`
   (defaults `Size=128*1024`, `Count=8`). For diskless nodes, run ttb from an
   external "trace control node" with disk access and use
   `Client = {local, File}` so all trace info is sent to the control node.
2. **Set trace patterns** — `ttb:tpl/2,3,4` (local+global), `ttb:tp/2,3,4`
   (global only), `ttb:tpe/2` (messages). These are "equivalent to the
   corresponding functions in module dbg, but all calls are stored in the
   history."
3. **Select trace targets** — `ttb:p(Item, Flags)` sets trace flags on
   processes/ports; `timestamp` is always forced on. Registered names apply on
   all active nodes. Issuing `p/2` also starts the `timer` if
   `{timer, TimerSpec}` was given to `tracer/2`.
4. **(Optional one-shot)** — `ttb:start_trace/4` collapses steps 1–3 into one
   call: each tuple in `Patterns` is converted to a list and passed to
   `ttb:tpl/2,3,4`, then `ttb:p(all, call)` is issued. The page's equivalence:
   ```
   ttb:start_trace([Node, OtherNode],
     [{mod, foo, []}, {mod, bar, 2}],
     {all, call},
     [{file, File}, {handler,{fun myhandler/4, S}}]).
   ```
   is equivalent to `tracer/2` + `tpl(mod,foo,[])` + `tpl(mod,bar,2,[])` +
   `p(all, call)`.
5. **Stop & fetch** — `ttb:stop/1` stops tracing on all nodes and sends logs
   + `.ti` files to the trace control node, stored in
   `ttb_upload_FileName-Timestamp` (Timestamp `yyyymmdd-hhmmss`). Even
   same-machine logs are moved into this directory. History is saved to
   `ttb_last_config`. Stop options:
   - `nofetch` — do not collect logs after stop.
   - `{fetch, Dir}` — specify fetch directory (must not already exist).
   - `format` — format logs after stop; all logs in fetch dir are merged.
   - `return_fetch_dir` — return `{stopped, Dir}` instead of `stopped`
     (implies fetch).

Operational `tracer/2` options that affect the disk flow: `{file, Client}`,
`{wrap,...}` size limits, `queue_size` (ip trace driver queue, see
`dbg:trace_port/2`), `flush` (periodic `dbg:flush_trace_port/1`),
`overload_check` (disables tracing on a node when `Module:Function(check)`
returns true), `resume`/`{resume, MSec}` (autoresume: remote nodes reconnect
to the controlling node after restart; requires Runtime_Tools started;
autostart info stored in `ttb_autostart.bin` unless overridden by env var
`ttb_autostart_module`), `timer` (auto-stop after MSec, StopOpts passed to
`stop/1`), `process_info` (replace Pid with `{Pid,ProcessInfo,Node}`).

## Formatting trace logs offline
`ttb:format/1,2` reads the binary trace log(s) produced by the file trace
port and presents them. Logs are processed in timestamp order unless
`disable_sort` is given. Format options (`format_opt()`):
- `{out, standard_io | file:filename()}` — output destination; the format
  handler receives this fd as its first parameter (ignored for the et
  handler).
- `{handler, format_handler()}` where `format_handler() =
  {format_fun(), InitialState}` and `format_fun()` is
  `fun((Fd, Trace, TraceInfo, State) -> NewState)`. The state returned from
  each call is passed to the next call, even across log files.
- `disable_sort`.

Handler choices:
- Default handler — presents each trace message as a text line.
- `ttb:get_et_handler()` — uses `et_viewer` in application ET for graphical
  presentation; ttb supplies filters selectable from the et_viewer Filters
  menu. Example: `ttb:format(Dir, [{handler, ttb:get_et_handler()}])`.
- Custom `{Function, InitialState}` — `Function` called for each trace
  message; receives `TraceInfo` (the `{Key,ValueList}` tuples accumulated via
  `write_trace_info/2` in the `.ti` file).

Wrap logs: format one wrap log by specifying the exact file name, or the
whole set by specifying the name with `*` instead of the wrap count (see
User's Guide `ttb_ug.html#format`).

`stop/1` option `format` triggers formatting immediately after fetch, merging
all logs in the fetch directory.

## Relationship to dbg / seq_trace
- **dbg (Runtime_Tools):** ttb's `tp/tpl/tpe/ctp/ctpl/ctpg/ctpe` are
  "equivalent to the corresponding functions in module dbg, but all calls
  are stored in the history." `p/2`'s `MatchDesc` is the same as returned from
  `dbg:p/2`. Match-spec shortcuts (`return`, `caller`, `{codestr, Str}`) map
  to `dbg:fun2ms/1`-style specs. The ip trace driver and
  `dbg:flush_trace_port/1` are reused. **Critical rule:** do not use `dbg` in
  parallel with ttb — ttb owns the tracer state.
- **seq_trace:** `tracer/2` "points the system tracer for sequential tracing
  to the same port" as the file trace port — i.e. the system tracer for
  sequential tracing is automatically initiated by ttb when a trace port is
  started with `ttb:tracer/0,1,2`. `seq_trigger_ms/0,1` returns a match spec
  that turns on sequential tracing with given flags; used as the last arg to
  `tp`/`tpl`, the traced item becomes a sequential-trace trigger: when called
  on a process with trace flag `call` set, the process is "contaminated" with
  a `seq_trace` token. Possible `SeqTraceFlag` values come from `seq_trace`.

## Strict rules
- **Never use `dbg` in parallel with ttb** — ttb manages tracer state;
  concurrent `dbg` calls produce inconsistent tracing state.
- `p/2` always forces the `timestamp` trace flag on.
- `{fetch, Dir}` directory must not already exist (an error is thrown).
- `overload_check`: once a node is overloaded, `ttb:p/2` and the
  `tp/2,3,4` family cannot be issued (would lead to inconsistent tracing
  state across nodes).
- `flush` option is not allowed with `{file, {local, File}}` tracing.
- `resume` requires Runtime_Tools started on traced nodes (must be in `.boot`
  scripts for embedded Erlang); otherwise resume manually via `rpc:call/4`.
  Custom autostart module (env `ttb_autostart_module`) must implement
  `write_config/1`, `read_config/0`, `delete_config/0` with crash-safe
  storage.
- `Out` is ignored when the et format handler is used.
- Wrap-log formatting: exact filename for one log, `*` for the whole set.

## Verbatim quotes
- "A base for building trace tools for distributed systems."
- "When using ttb, do not use module dbg in application Runtime_Tools in
  parallel."
- "Starts a file trace port on all specified nodes and points the system
  tracer for sequential tracing to the same port."
- "These functions are equivalent to the corresponding functions in module
  dbg, but all calls are stored in the history. The history buffer makes it
  easy to create configuration files; the same trace environment can be set up
  many times, for example, to compare two test runs."
- "tp - Sets trace patterns on global function calls. tpl - Sets trace
  patterns on local and global function calls. tpe - Sets trace patterns on
  messages."
- "Sets the specified trace flags on the specified processes or ports. Flag
  timestamp is always turned on."
- "The system tracer for sequential tracing is automatically initiated by ttb
  when a trace port is started with ttb:tracer/0,1,2."
- "Stops tracing on all nodes. Logs and trace information files are sent to
  the trace control node and stored in a directory named
  ttb_upload_FileName-Timestamp."
- "Reads the specified binary trace log(s). The logs are processed in the
  order of their time stamps as long as option disable_sort is not
  specified."
- "File .ti contains {Key,ValueList} tuples. This function adds Data to the
  ValueList associated with Key. All information written with this function
  is included in the call to the format handler."

## Version notes
- Page metadata: OTP 29.0.2, observer v2.19.
- `start_trace/4`, `get_et_handler/0`: since OTP R15B.
- `ctpe/1`, `tpe/2`: since OTP 19.0.
- Source: `lib/observer/src/ttb.erl` at tag `OTP-29.0.2`.
- Built with ExDoc v0.40.3.
- Note on app placement: the seed URL is under `/doc/apps/observer/`; ttb
  lives in the `observer` application (not `tools` / `runtime_tools`). The
  page explicitly references `dbg` and `seq_trace` as belonging to
  `runtime_tools` and `kernel` respectively.

## Discovered links

### Relevant (crawl later)
- ../../apps/runtime_tools/dbg.html — dbg module (ttb's underlying tracer;
  parallel-use forbidden; provides `p/2`, `trace_port/2`, `fun2ms/1`,
  `flush_trace_port/1`, `match_spec()` type)
- ../../apps/runtime_tools/dbg.html#trace_port/2 — ip trace driver / file
  trace port construction reused by ttb
- ../../apps/runtime_tools/dbg.html#fun2ms/1 — match-spec shortcut source
- ../../apps/kernel/seq_trace.html — seq_trace module (sequential tracing
  flags, token contamination semantics)
- ../../apps/erts/match_spec.html — "Match Specifications in Erlang" (ERTS
  User's Guide) — the match_spec language used by tp/tpl/tpe
- ttb_ug.html#format — ttb User's Guide, formatting section (wrap-log
  formatting examples)

### Skipped
- #... anchor links (in-page function/type anchors)
- ../../apps/erts/erlang.html#t:* — primitive type definitions
- ../../apps/kernel/file.html#t:fd/0, #t:filename/0
- ../../apps/kernel/rpc.html#call/4
- ../../apps/stdlib/timer.html#t:time/0
- ../../index.html, llms.txt, observer.epub, ttb.md, dist/*.css,
  /assets/css/algolia-typeahead.css
- https://github.com/erlang/otp/blob/OTP-29.0.2/lib/observer/src/ttb.erl#L*
  (source line anchors)
- https://erlang.org, https://www.ericsson.com,
  https://github.com/elixir-lang/ex_doc
