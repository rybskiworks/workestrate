# Crawl: system/logger_chapter.html
- seed_url: https://www.erlang.org/doc/system/logger_chapter.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/logger_chapter.html
- family: Erlang/OTP system docs
- fetch: 200 (seed `/doc/system/` path returns 404; canonical is `/doc/apps/kernel/logger_chapter.html`)
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: logger-and-config.md

## Purpose
Erlang/OTP's standard logging chapter. Describes **Logger** (part of the Kernel application), the standard API for issuing log events plus a customizable backend into which log handlers, filters and formatters can be plugged. Logger replaces the legacy `error_logger`-based logging as of OTP 21.0. The Kernel application installs one handler named **`default`** at system start (a `logger_std_h` instance writing to the terminal by default), which receives runtime/behaviour/application log events.

## Key concepts (logger architecture, levels, primary vs module, handlers, filters, formatters)

### Conceptual flow
A **log event** = `{level, message, metadata}`. The backend forwards events from the API:
1. through **primary filters** (level check + zero or more filter functions);
2. then, for each handler, through that handler's **handler filters** (level check + filter functions);
3. if all pass, the event is forwarded to the **handler callback** (`HModule:log/2`), which formats and prints.

Everything up to and including the handler callback call executes on the **client process** (the process that issued the event). Handlers are called in sequence in undefined order. Handlers may optionally spawn their own process.

### Log levels (RFC 5424 / Syslog, 8 levels)
| Level | Integer | Description |
|---|---|---|
| `emergency` | 0 | system is unusable |
| `alert` | 1 | action must be taken immediately |
| `critical` | 2 | critical conditions |
| `error` | 3 | error conditions |
| `warning` | 4 | warning conditions |
| `notice` | 5 | normal but significant conditions |
| `info` | 6 | informational messages |
| `debug` | 7 | debug-level messages |

- Integer values are internal only; the API always uses atoms.
- An event passes the level check if its integer value is **less than or equal to** the configured level (i.e. equally or more severe).
- Compare two levels with `logger:compare_levels/2`.

### Primary vs module level
- **Primary log level** (`level` key of primary config): gate for the whole system. Events less severe than this are immediately discarded before primary filters. Default = **`notice`**. Set at startup via Kernel param `logger_level`; at runtime via `logger:set_primary_config(level, Level)`.
- **Module level** overrides the primary level **per module** (e.g. to allow more verbose logging from one part of the system). Set with `logger:set_module_level(Module, Level)`. Configurable at startup via `{module_level, Level, [Module]}` in the `logger` Kernel param.
- (Note: the task brief referenced `set_application_level/2`; the actual API exposed by this chapter is `set_module_level/2`. There is no `set_application_level/2` in the Logger chapter.)

### Logger API — macros vs functions
- Macros defined in `logger.hrl`, included via `-include_lib("kernel/include/logger.hrl")`. Macros: `?LOG_EMERGENCY`, `?LOG_ALERT`, `?LOG_CRITICAL`, `?LOG_ERROR`, `?LOG_WARNING`, `?LOG_NOTICE`, `?LOG_INFO`, `?LOG_DEBUG` (and `?LOG`).
- Functions: `logger:Level/1,2,3` are shortcuts for `logger:log(Level, Arg1[, Arg2[, Arg3]])`.
- Difference: macros add **location (originator) metadata** and perform **lazy evaluation** — the logger call is wrapped in a `case` so the message is only built if the event passes the primary level check.

### Log message forms
- format string + args: `logger:error("The file does not exist: ~ts",[Filename])`
- string: `logger:notice("Something strange happened!")`
- **report** (map or key-value list) — preferred, enables backend-specific filtering/formatting: `?LOG_ERROR(#{ user => joe, filename => Filename, reason => enoent })`
- **lazy fun** — evaluated only if primary level check passes; returns string | report | `{Format, Args}`.

### Report callbacks (structured logging)
A report can be accompanied by a `report_cb` in metadata — a convenience fun the formatter can use to convert the report:
- arity 1: `fun((logger:report()) -> {io:format(),[term()]})`
- arity 2: `fun((logger:report(), logger:report_cb_config()) -> unicode:chardata())` — must obey `depth`, `chars_limit`, and `single_line` from the config map.

Example:
```erlang
logger:debug(#{got => connection_request, id => Id, state => State},
             #{report_cb => fun(R) -> {"~p",[R]} end})
```

### Metadata
Three ways to add custom metadata:
1. **Primary metadata** — base metadata for all events. Startup: Kernel param `logger_metadata`. Runtime: `logger:set_primary_config/1`, `logger:update_primary_config/1`. Default `#{}`.
2. **Process metadata** — `logger:set_process_metadata/1`, `logger:update_process_metadata/1`; applies to all events issued on that process.
3. **Per-event metadata** — last arg to the macro/API, e.g. `?LOG_ERROR("Connection closed",#{context => server})`.

### Filters
- A filter = `{FilterFun, Extra}` where `FilterFun` is arity-2, called as `FilterFun(LogEvent, Extra)`.
- Return values:
  - `stop` — event immediately discarded. If primary, no handler filters/callbacks run. If handler filter, only that handler is skipped (event still goes to other handlers).
  - `ignore` — filter did not recognize the event; leaves the decision to other filters.
  - the (possibly modified) log event — next filter receives the modified event; the last filter's return value is what the handler callback receives.
- `filter_default` (`log` | `stop`) decides behaviour when all filters return `ignore` or no filters exist. Default = **`log`**.
- Primary filters: `logger:add_primary_filter/2`, `logger:remove_primary_filter/1`. Also via Kernel `logger` param.
- Handler filters: `logger:add_handler_filter/3`, `logger:remove_handler_filter/2`. Also set in handler config at `add_handler/3`.
- Inspection: `logger:get_config/0`, `logger:get_primary_config/0`, `logger:get_handler_config/1`. Filters listed in application order.
- Built-in filters (`logger_filters`):
  - `logger_filters:domain/2` — filter on `domain` metadata field.
  - `logger_filters:level/2` — filter on log level.
  - `logger_filters:progress/2` — stop/allow supervisor + application_controller progress reports.
  - `logger_filters:remote_gl/2` — stop/allow events whose group leader is on a remote node.

### Handlers
- A handler module must export `log(LogEvent, Config) -> term()`.
- Optional callbacks: `adding_handler/1`, `changing_config/3`, `filter_config/1`, `removing_handler/1` (see `logger_handler`).
- Multiple instances of the same callback module allowed, identified by unique handler ids.
- Built-in handlers:
  - **`logger_std_h`** — the default OTP handler. Multiple instances; each writes to terminal or file.
  - **`logger_disk_log_h`** — like `logger_std_h` but uses `disk_log` as destination (wrap logs).
  - **`error_logger`** — backwards compatibility only; not started by default; auto-started on first `error_logger:add_report_handler/1,2`. Old STDLIB/SASL `error_logger` event handlers are not added in OTP 21.0+.
- Add/remove: `logger:add_handler(HandlerId, Module, HandlerConfig)`, `logger:remove_handler(HandlerId)`.
- Handler config keys: `id` (auto), `module` (auto), `level` (default `all`), `filters` (default `[]`), `filter_default` (default `log`), `formatter` (default `{logger_formatter, DefaultFormatterConfig}`), `config` (handler-specific).
- `level` and `filters` are enforced by Logger before forwarding to the handler; `formatter` and handler-specific options are left to the handler implementation.

### Formatters
- Formatter info = `{FModule, FConfig}`; `FModule` must export `format(LogEvent, FConfig) -> FormattedLogEntry`.
- Set as part of handler config at add time; change at runtime via `logger:set_handler_config(HandlerId, formatter, {Module, FConfig})` (overwrite) or `logger:update_formatter_config/2,3` (modify only FConfig).
- Optional callback `check_config(FConfig)` validates formatter config when set/modified.
- **Default formatter = `logger_formatter`** (used when no formatter info is specified). Template tokens include `time`, `pid`, `msg`, etc. (e.g. `#{template => [time," ",pid," ",msg,"\n"]}`). See `logger_formatter` manual page for full token list and defaults.

### Default handler / default formatter
- Kernel installs one handler named **`default`** at start: a `logger_std_h` writing to the terminal.
- Default formatter is `logger_formatter`; the default handler is started with `legacy_header => true` so OTP log events look like the old `error_logger_tty_h`/`error_logger_file_h` output.
- Disable default handler at startup: `{handler, default, undefined}` in the `logger` Kernel param.

## Strict rules (level semantics; handler/filter evaluation order; default handler)
1. **Level semantics**: an event passes a level check iff its integer value ≤ the configured level (equally or more severe). Integers are internal only; API uses atoms. Use `logger:compare_levels/2` to compare severities.
2. **Evaluation order**: API → (module level overrides primary level) → primary level check → primary filter functions → [per handler] handler level check → handler filter functions → handler callback `log/2`. A primary `stop` discards for all handlers; a handler `stop` skips only that handler.
3. **Filter chaining**: a filter returning a (modified) log event passes the modified event to the next filter; the last filter's return value is what the handler callback receives. `ignore` defers to other filters; `filter_default` decides when all ignore / none exist (default `log`).
4. **Default handler**: Kernel starts `default` (`logger_std_h`, terminal) at boot; default primary level `notice`; default handler level `all`; default formatter `logger_formatter` with `legacy_header => true`.
5. **Client-process execution**: everything up to and including the handler callback runs on the client process; whether other processes are involved is handler-specific.
6. **Error handling**: Logger does limited input validation; it does NOT evaluate report callbacks or validate format strings/args. Filters/handlers must not crash on bad input. If one does, Logger removes that filter/handler and prints a short error to terminal plus a `debug` event with details.
7. **SASL reports (OTP 21.0+)**: supervisor/crash reports are `error` level; progress reports are `info` level (not logged by default since primary level is `notice`). All SASL reports carry `domain => [otp,sasl]` metadata. Set `logger_sasl_compatible => true` for old behaviour.

## Examples (adding a handler/filter, structured logging, report callback)

### Add a handler to log info events to file
```erlang
%% Lower primary level, or per-module:
logger:set_primary_config(level, info).
logger:set_module_level(mymodule, info).

%% Keep default handler at notice (no info to terminal):
logger:set_handler_config(default, level, notice).

%% Add a file handler at info level:
Config = #{config => #{file => "./info.log"}, level => info},
logger:add_handler(myhandler, logger_std_h, Config).

%% Add a filter so the file handler only gets info events:
logger:add_handler_filter(myhandler, stop_non_info,
    {fun logger_filters:level/2, {stop, neq, info}}).
```

### Kernel config: default handler to file + single-line formatter
```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{config => #{file => "log/erlang.log"}}}]}]}].
```
```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{formatter => {logger_formatter, #{single_line => true}}}}]}]}].
```

### Kernel config: template with pid token
```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{formatter => {logger_formatter,
                      #{template => [time," ",pid," ",msg,"\n"]}}}}]}]}].
```

### Kernel config: two handlers (errors to erlang.log, all to debug.log)
```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{level => error, config => #{file => "log/erlang.log"}}},
     {handler, info, logger_std_h,
      #{level => debug, config => #{file => "log/debug.log"}}}
    ]}]}].
```

### Structured logging (report) + report callback
```erlang
?LOG_ERROR(#{ user => joe, filename => Filename, reason => enoent }).

logger:debug(#{got => connection_request, id => Id, state => State},
             #{report_cb => fun(R) -> {"~p",[R]} end}).
```

### Minimal custom handler
```erlang
-module(myhandler1).
-export([log/2]).
log(LogEvent, #{formatter := {FModule, FConfig}}) ->
    io:put_chars(FModule:format(LogEvent, FConfig)).
```

### Custom handler with gen_server (file, one process)
```erlang
-module(myhandler2).
-export([adding_handler/1, removing_handler/1, log/2]).
-export([init/1, handle_call/3, handle_cast/2, terminate/2]).
adding_handler(Config) ->
    MyConfig = maps:get(config,Config,#{file => "myhandler2.log"}),
    {ok, Pid} = gen_server:start(?MODULE, MyConfig, []),
    {ok, Config#{config => MyConfig#{pid => Pid}}}.
removing_handler(#{config := #{pid := Pid}}) ->
    gen_server:stop(Pid).
log(LogEvent,#{config := #{pid := Pid}} = Config) ->
    gen_server:cast(Pid, {log, LogEvent, Config}).
init(#{file := File}) ->
    {ok, Fd} = file:open(File, [append, {encoding, utf8}]),
    {ok, #{file => File, fd => Fd}}.
handle_cast({log, LogEvent, Config}, #{fd := Fd} = State) ->
    do_log(Fd, LogEvent, Config), {noreply, State}.
do_log(Fd, LogEvent, #{formatter := {FModule, FConfig}}) ->
    io:put_chars(Fd, FModule:format(LogEvent, FConfig)).
```

### Overload protection config (logger_std_h)
```erlang
logger:add_handler(my_standard_h, logger_std_h,
    #{config => #{file => "./system_info.log",
                  sync_mode_qlen => 100,
                  drop_mode_qlen => 1000,
                  flush_qlen => 2000}}).
```
Defaults: `sync_mode_qlen=10`, `drop_mode_qlen=200`, `flush_qlen=1000`; burst `burst_limit_enable=true`, `burst_limit_max_count=500`, `burst_limit_window_time=1000`ms; `overload_kill_enable=false`, `overload_kill_qlen=20000`, `overload_kill_mem_size=3000000`, `overload_kill_restart_after=5000`. Constraint: `sync_mode_qlen =< drop_mode_qlen =< flush_qlen` and `drop_mode_qlen > 1`.

## Verbatim quotes
- "Erlang provides a standard API for logging through Logger, which is part of the Kernel application. Logger consists of the API for issuing log events, and a customizable backend where log handlers, filters and formatters can be plugged in."
- "By default, the Kernel application installs one log handler at system start. This handler is named default."
- "A log event consists of a log level, the message to be logged, and metadata."
- "The Logger backend forwards log events from the API, first through a set of primary filters, then through a set of secondary filters attached to each log handler."
- "a log event passes the log level check if the integer value of its log level is less than or equal to the currently configured log level. That is, the check passes if the event is equally or more severe than the configured level."
- "The primary log level can be overridden by a log level configured per module."
- "The difference between using the macros and the exported functions is that macros add location (originator) information to the metadata, and performs lazy evaluation by wrapping the logger call in a case statement, so it is only evaluated if the log level of the event passes the primary log level check."
- "A report, which is either a map or a key-value list, is the preferred way to log using Logger as it makes it possible for different backends to filter and format the log event as it needs to."
- "The filter function can return stop, ignore or the (possibly modified) log event."
- "If stop is returned, the log event is immediately discarded. If the filter is primary, no handler filters or callbacks are called. If it is a handler filter, the corresponding handler callback is not called, but the log event is forwarded to filters attached to the next handler, if any."
- "filter_default is by default set to log, meaning that if all existing filters ignore a log event, Logger forwards the event to the handler callback."
- "If no formatter information is specified for a handler, Logger uses logger_formatter as default."
- "Everything up to and including the call to the handler callbacks is executed on the client process, that is, the process where the log event was issued."
- "If a filter or handler still crashes, Logger will remove the filter or handler in question from the configuration, and print a short error message to the terminal."
- "As of Erlang/OTP 21.0, the concept of SASL reports is removed... Supervisor reports and crash reports are issued as error level log events... Progress reports are issued as info level log events, and since the default primary log level is notice, these are not logged by default."

## Version notes
- Page meta: **OTP 29.0.2**, kernel **11.0.2**. Built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- Logger was introduced in **Erlang/OTP 21.0**, replacing the `error_logger`-based logging model. The "SASL reports" concept was removed in 21.0 (supervisor/crash → `error`; progress → `info`).
- `error_logger` API still exists for legacy code but is deprecated and will be removed in a later release; calls are forwarded to Logger as `logger:log(Level, Report, Metadata)`.
- Seed URL `https://www.erlang.org/doc/system/logger_chapter.html` returns **404** — the docs were restructured under `/doc/apps/kernel/`. The canonical URL is `https://www.erlang.org/doc/apps/kernel/logger_chapter.html`.
- Source markdown: https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/doc/guides/logger_chapter.md

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/apps/kernel/logger.html — `logger` module API (macros, `log/2,3`, `compare_levels/2`, `set_module_level`, `set_primary_config`, `add_handler`, filters types)
- https://www.erlang.org/doc/apps/kernel/logger_formatter.html — default formatter, template tokens, config
- https://www.erlang.org/doc/apps/kernel/logger_filters.html — built-in filters (`domain/2`, `level/2`, `progress/2`, `remote_gl/2`)
- https://www.erlang.org/doc/apps/kernel/logger_std_h.html — default std handler + overload config
- https://www.erlang.org/doc/apps/kernel/logger_disk_log_h.html — disk_log handler
- https://www.erlang.org/doc/apps/kernel/logger_handler.html — handler callback module (`log/2`, `adding_handler/1`, `changing_config/3`, `filter_config/1`, `removing_handler/1`)
- https://www.erlang.org/doc/apps/kernel/logger_cookbook.html — Logging Cookbook (next page)
- https://www.erlang.org/doc/apps/kernel/kernel_app.html — kernel(6) app config (`logger_level`, `logger_metadata`, `logger_sasl_compatible`)
- https://www.erlang.org/doc/apps/kernel/disk_log.html — disk_log module
- https://www.erlang.org/doc/apps/kernel/error_logger.html — legacy error_logger API
- https://www.erlang.org/doc/apps/sasl/error_logging.html — SASL error logging (old behaviour)

### Skipped
- https://www.erlang.org/doc/apps/kernel/kernel.epub — ePub download (binary)
- https://www.erlang.org/doc/apps/kernel/logger_chapter.md — source markdown mirror (already captured)
- In-page anchors (#filters, #formatters, #logger-api, #primary-logger-configuration, #kernel-configuration-parameters, #logger-proxy, #backwards-compatibility-with-error_logger, #logger_level, #logger_parameter, #logger_sasl_compatible) — same page
- GitHub source link, ExDoc/search/settings chrome
