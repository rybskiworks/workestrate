# Logger and Config

## Purpose

This document defines BEAM/OTP guidance for the OTP Logger (Kernel, OTP 21+) and the OTP configuration file format (`config(4)`). It covers log levels, primary vs module level, handlers, filters, formatters, macros, structured logging/report callbacks, `compare_levels`, and the `sys.config` / `Name.config` term format. Future agents who write, review, refactor, debug, or validate logging or configuration code should follow these rules so behavior is consistent, predictable, and aligned with the official Erlang/OTP documentation.

This doc is BEAM-common, not Elixir-specific. Examples are in Erlang. Logger lives in the `kernel` application.

## Sources used

- `.crawl/20-logger-chapter.md` — https://www.erlang.org/doc/apps/kernel/logger_chapter.html (PRIMARY — Logger architecture, levels, primary vs module level, handlers, filters, formatters, macros, report callbacks). **Correction note:** the seed URL `https://www.erlang.org/doc/system/logger_chapter.html` returns **404**; the docs were restructured under `/doc/apps/kernel/`. The canonical URL is `https://www.erlang.org/doc/apps/kernel/logger_chapter.html`.
- `.crawl/21-logger-module.md` — https://www.erlang.org/doc/apps/kernel/logger.html (PRIMARY — `logger` module API, `level()` type, handler/filter API, `set_module_level`, `compare_levels`, report_cb contract)
- `.crawl/19-config.md` — https://www.erlang.org/doc/apps/kernel/config.html (PRIMARY — `config(4)` file format, `sys.config`, include-file mechanism, merge/override precedence)

This page reflects Erlang/OTP 29.0.2 semantics. Logger was introduced in OTP 21.0, replacing the `error_logger`-based logging model.

## Core guidance

### What Logger is

From `logger_chapter.html`:

> "Erlang provides a standard API for logging through Logger, which is part of the Kernel application. Logger consists of the API for issuing log events, and a customizable backend where log handlers, filters and formatters can be plugged in."

> "By default, the Kernel application installs one log handler at system start. This handler is named `default`."

A **log event** = `{level, message, metadata}`. The backend forwards events: first through **primary filters** (level check + filter functions), then per handler through **handler filters** (level check + filter functions), then to the **handler callback** (`HModule:log/2`).

> "Everything up to and including the call to the handler callbacks is executed on the client process, that is, the process where the log event was issued."

### Log levels (8 levels, RFC 5424 / Syslog)

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

From `logger.html`:

> "level() :: emergency | alert | critical | error | warning | notice | info | debug."

Integer values are internal only; the API always uses atoms. `all` and `none` are accepted by many functions but are not part of `level()` itself. An event passes the level check if its integer value is **less than or equal to** the configured level (equally or more severe). Compare two levels with `logger:compare_levels/2`.

### Primary vs module level

- **Primary log level** — gate for the whole system. Default = **`notice`** (per `logger_chapter.html`; `logger.html` states default `info` for the primary config map). Set at startup via Kernel param `logger_level`; at runtime via `logger:set_primary_config(level, Level)`.
- **Module level** — overrides the primary level **per module**. Set with `logger:set_module_level(Module, Level)`.

> "The primary log level can be overridden by a log level configured per module."

> "The log level for a module overrides the primary log level of Logger for log events originating from the module in question. Notice, however, that it does not override the level configuration for any handler."

> "The originating module for a log event is only detected if the key `mfa` exists in the metadata ... When log macros are used, this association is automatically added to all log events. If an API function is called directly, without using a macro, the logging client must explicitly add this information if module levels shall have any effect."

**Correction note:** The chapter (`logger_chapter.html`) documents `set_module_level/2` as the per-module API. There is **no `set_application_level`** described in the chapter. (The `logger` module API page, `logger.html`, does expose `set_application_level/2` since OTP 21.1 as a convenience that calls `set_module_level/2` for each module of an application, but it is not covered in the chapter.)

### Macros vs functions

Macros defined in `logger.hrl`, included via `-include_lib("kernel/include/logger.hrl")`: `?LOG_EMERGENCY`, `?LOG_ALERT`, `?LOG_CRITICAL`, `?LOG_ERROR`, `?LOG_WARNING`, `?LOG_NOTICE`, `?LOG_INFO`, `?LOG_DEBUG` (and `?LOG`).

> "The difference between using the macros and the exported functions is that macros add location (originator) information to the metadata, and performs lazy evaluation by wrapping the logger call in a case statement, so it is only evaluated if the log level of the event passes the primary log level check."

Functions: `logger:Level/1,2,3` are shortcuts for `logger:log(Level, ...)`. When using API functions directly (not macros), `mfa` metadata is not auto-added, so module-level filtering has no effect unless you add it manually.

### Log message forms and structured logging

- format string + args: `logger:error("The file does not exist: ~ts",[Filename])`
- string: `logger:notice("Something strange happened!")`
- **report** (map or key-value list) — preferred for structured logging:

> "A report, which is either a map or a key-value list, is the preferred way to log using Logger as it makes it possible for different backends to filter and format the log event as it needs to."

- **lazy fun** — evaluated only if primary level check passes.

### Report callbacks

A report can be accompanied by a `report_cb` in metadata. Contract (from `logger.html`):

```erlang
-type report_cb() ::
    fun((report()) -> {io:format(), [term()]}) |
    fun((report(), report_cb_config()) -> unicode:chardata()).
-type report_cb_config() ::
    #{depth := pos_integer()|unlimited,
      chars_limit := pos_integer()|unlimited,
      single_line := boolean()}.
```

Example:
```erlang
logger:debug(#{got => connection_request, id => Id, state => State},
             #{report_cb => fun(R) -> {"~p",[R]} end}).
```

### Filters

A filter = `{FilterFun, Extra}` where `FilterFun` is arity-2, called as `FilterFun(LogEvent, Extra)`. Return values:
- `stop` — event immediately discarded. If primary, no handler filters/callbacks run. If handler filter, only that handler is skipped.
- `ignore` — filter did not recognize the event; leaves the decision to other filters.
- the (possibly modified) log event — next filter receives the modified event.

> "filter_default is by default set to log, meaning that if all existing filters ignore a log event, Logger forwards the event to the handler callback."

- Primary filters: `logger:add_primary_filter/2`, `logger:remove_primary_filter/1`.
- Handler filters: `logger:add_handler_filter/3`, `logger:remove_handler_filter/2`.
- `filter_default` (`log | stop`) is a key in both primary config and handler config. Primary default is `log`.
- Built-in filters (`logger_filters`): `domain/2`, `level/2`, `progress/2`, `remote_gl/2`.

### Handlers

A handler module must export `log(LogEvent, Config) -> term()`. Optional callbacks: `adding_handler/1`, `changing_config/3`, `filter_config/1`, `removing_handler/1`.

Built-in handlers:
- **`logger_std_h`** — the default OTP handler. Multiple instances; each writes to terminal or file.
- **`logger_disk_log_h`** — like `logger_std_h` but uses `disk_log` as destination (wrap logs).
- **`error_logger`** — backwards compatibility only; not started by default.

Add/remove: `logger:add_handler(HandlerId, Module, HandlerConfig)`, `logger:remove_handler(HandlerId)`.

Handler config keys: `id` (auto), `module` (auto), `level` (default `all`), `filters` (default `[]`), `filter_default` (default `log`), `formatter` (default `{logger_formatter, DefaultFormatterConfig}`), `config` (handler-specific).

### Formatters

Formatter info = `{FModule, FConfig}`; `FModule` must export `format(LogEvent, FConfig) -> FormattedLogEntry`.

> "If no formatter information is specified for a handler, Logger uses `logger_formatter` as default."

Template tokens include `time`, `pid`, `msg`, etc. Example: `#{template => [time," ",pid," ",msg,"\n"]}`. See the `logger_formatter` manual page for the full token list.

### Default handler / default formatter

Kernel installs one handler named **`default`** at start: a `logger_std_h` writing to the terminal. Default formatter is `logger_formatter`; the default handler is started with `legacy_header => true`. Disable the default handler at startup: `{handler, default, undefined}` in the `logger` Kernel param.

### SASL reports (OTP 21.0+)

> "As of Erlang/OTP 21.0, the concept of SASL reports is removed... Supervisor reports and crash reports are issued as error level log events... Progress reports are issued as info level log events, and since the default primary log level is notice, these are not logged by default."

All SASL reports carry `domain => [otp,sasl]` metadata. Set `logger_sasl_compatible => true` for old behaviour.

### The OTP config file format (`config(4)`)

From `config.html`:

> "A *configuration file* contains values for configuration parameters for the applications in the system."

A `.config` file contains a **single Erlang term** ending in `.` with shape:

```erlang
[{Application1, [{Par11, Val11}, ...]},
 ...
 {ApplicationN, [{ParN1, ValN1}, ...]}].
```

- `Application = atom()` — application name.
- `Par = atom()` — name of a configuration parameter.
- `Val = term()` — value of a configuration parameter.

The file is named `Name.config` (any `Name`). In embedded mode exactly one system configuration file is assumed, named `sys.config`, located in `$ROOT/releases/Vsn`. Values are retrieved via `application:get_env/1,2`.

**Correction note (important):** `config(4)` is a **file-format reference**, NOT a module with callable functions. There is **no `Config.Reader`**, **no `${VAR}` environment-variable interpolation**, and **no `sys.config.src`** on the official OTP `config.html` page. Those concepts belong to Elixir releases / `config_provider`s and rebar3/distillery build-time sources, not to the OTP kernel config file described here. The only environment-like reference on the page is the literal `$ROOT` placeholder denoting the OTP root installation directory (used to describe the `sys.config` location), not a runtime expansion mechanism.

### Config merge and override precedence

Precedence (lowest → highest):

1. **Application resource files** (`app(4)`, the `.app` files) — baseline.
2. **Configuration files / file descriptors** via `-config` / `-configfd` — read in command-line order; **last wins**.
3. **Command-line flags** (`erts:erl(1)`) — always override config-file values.

> "Configuration parameter values in a configuration file or file descriptor override the values in the application resource files (see `app(4)`)."

> "The values in the configuration file are always overridden by command-line flags (see `erts:erl(1)`)."

### Include-file mechanism

A `sys.config` (or `-configfd` configuration) may include other `.config` files:

```erlang
[{Application, [{Par, Val}]} | IncludeFile].
```

`IncludeFile = string()` — name of a `.config` file (extension can be omitted). Merging: new parameters are added and existing parameter values are overwritten. Missing/erroneous include file at startup → runtime aborts; at release install → file ignored, error returned.

## Practical rules

1. Use macros (`?LOG_ERROR`, etc.) over direct API calls to get location metadata and lazy evaluation.
2. Use reports (maps or kv-lists) over format strings for structured, backend-filterable logging.
3. Set the primary level to `notice` in production; lower per module with `set_module_level` for verbose subsystems.
4. Do not reuse reserved metadata keys (`pid`, `gl`, `time`, `mfa`, `file`, `line`, `domain`, `report_cb`).
5. Use `compare_levels/2` to compare severities programmatically; never compare level atoms directly.
6. Put config defaults in `.app` `env`; override via `sys.config` or CLI flags.
7. Use exactly one `sys.config` in embedded/release mode; use the include-file mechanism for composition.
8. Do not expect `${VAR}` expansion in OTP config files — it does not exist in `config(4)`.

## Review checklist

- [ ] Logger macros used (not bare API calls) where location metadata matters.
- [ ] Primary level set deliberately (default `notice`).
- [ ] Handler levels set deliberately (default `all`).
- [ ] No reserved metadata keys reused for custom data.
- [ ] `sys.config` is a single Erlang term of shape `[{App, [{Par,Val}]}]`.
- [ ] No `${VAR}` interpolation or `sys.config.src` expected in OTP config.
- [ ] Default handler disabled only when a replacement is configured.

## Implementation checklist

- [ ] `-include_lib("kernel/include/logger.hrl")` in modules using macros.
- [ ] Report callbacks obey `depth`, `chars_limit`, `single_line` (arity-2 form).
- [ ] Custom handlers export `log/2` (and optional `adding_handler/1`, `removing_handler/1`).
- [ ] Formatter set to `{logger_formatter, Config}` or a custom module exporting `format/2`.
- [ ] Config file term terminates with `.`.

## Runtime / debugging checklist

- [ ] `logger:get_primary_config/0` — primary level, filters, metadata.
- [ ] `logger:get_handler_config/0,1` — handler config.
- [ ] `logger:get_handler_ids/0` — installed handler ids.
- [ ] `logger:get_module_level/0,1` — per-module overrides.
- [ ] `logger:compare_levels/2` — compare two levels.
- [ ] `logger:i/0` — print Logger configuration overview.
- [ ] `application:get_env(App, Par)` — retrieve configured values.

## Validation hooks

- Verify a log event reaches a handler: set primary + handler level, issue `?LOG_INFO`, confirm output.
- Verify filter behavior: add a `stop` filter, confirm event is discarded.
- Verify config precedence: set in `.app`, override in `sys.config`, override on CLI; confirm `get_env` at each layer.
- Verify `sys.config` parses: `file:consult("releases/Vsn/sys.config")`.

## Examples

### Add a file handler at info level

```erlang
logger:set_primary_config(level, info).
logger:set_handler_config(default, level, notice).
Config = #{config => #{file => "./info.log"}, level => info},
logger:add_handler(myhandler, logger_std_h, Config).
logger:add_handler_filter(myhandler, stop_non_info,
    {fun logger_filters:level/2, {stop, neq, info}}).
```

### Kernel config: default handler to file

```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{config => #{file => "log/erlang.log"}}}]}]}].
```

### Kernel config: two handlers (errors + debug)

```erlang
[{kernel,
  [{logger,
    [{handler, default, logger_std_h,
      #{level => error, config => #{file => "log/erlang.log"}}},
     {handler, info, logger_std_h,
      #{level => debug, config => #{file => "log/debug.log"}}}
    ]}]}].
```

### Structured logging with report callback

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

### Config file with includes

```
sys.config:
["/home/user/myconfig1"
 {myapp,[{par1,val1},{par2,val2}]},
 "/home/user/myconfig2"].
```

## Common mistakes

1. **Using API functions instead of macros and expecting module-level filtering.** Fix: macros auto-add `mfa` metadata; API calls do not, so `set_module_level` has no effect unless you add `mfa` manually.
2. **Reusing reserved metadata keys.** Fix: do not use `pid`, `gl`, `time`, `mfa`, `file`, `line`, `domain`, `report_cb` for custom data.
3. **Expecting `${VAR}` expansion in `sys.config`.** Fix: OTP `config(4)` has no env-var interpolation; that is an Elixir/rebar3 concept. Pre-expand values at build time if needed.
4. **Expecting `sys.config.src` to be an OTP concept.** Fix: `sys.config.src` is a rebar3/distillery build-time source; OTP uses `sys.config`.
5. **Looking for a `Config.Reader` module in OTP.** Fix: `Config.Reader` is an Elixir concept; OTP config is a file format consumed by `-config`/`-configfd`.
6. **Setting module level expecting it to override handler level.** Fix: module level overrides primary level only, not handler level.
7. **Using multiple `.config` files in release mode.** Fix: use exactly one `sys.config`; use the include-file mechanism for composition.
8. **Letting filters/handlers crash on bad input.** Fix: Logger removes a crashing filter/handler and prints an error; validate input in your filter/handler.

## Strict vs contextual guidance

### Strict

- A log event is forwarded to a handler only if its level is ≤ the primary level AND ≤ that handler's level. Module level overrides primary (not handler).
- `set_handler_config/2` overwrites; `update_handler_config/2` merges. Same for primary config.
- Metadata merge precedence (highest wins): log-call metadata > process metadata > primary metadata > Logger-inserted defaults.
- A `.config` file is a single Erlang term terminated by `.`.
- Config-file values override `.app` resource files but are always overridden by command-line flags.
- Across multiple `-config`/`-configfd` sources, last wins (command-line order).

### Convention

- Use macros over direct API calls for location metadata and lazy evaluation.
- Use reports (maps/kv-lists) over format strings for structured logging.
- Default primary level `notice`; lower per module for verbose subsystems.
- Put config defaults in `.app` `env`; override via `sys.config` or CLI flags.

### Contextual

- Exact primary/handler/module levels per environment.
- Whether to use `logger_std_h`, `logger_disk_log_h`, or custom handlers.
- Formatter template and config.
- Whether to use the include-file mechanism for config composition.

### Policy

- Whether `error_logger` legacy compatibility is enabled (`logger_sasl_compatible`).
- Default log levels per environment (dev/staging/prod).
- Whether custom handlers/formatters are permitted.
- Config override strategy (`sys.config` only vs CLI flags in production).

## Policy decisions for individual repos

1. **Default log levels.** What primary/handler levels per environment (dev/staging/prod)?
2. **Macro vs API policy.** Are bare `logger:error/1,2,3` calls permitted, or are macros required?
3. **Structured logging.** Are reports (maps/kv-lists) mandatory, or are format strings allowed?
4. **Handler strategy.** Is `logger_std_h` to file the standard, or are custom handlers permitted?
5. **Config composition.** Is the include-file mechanism used, or is `sys.config` flat?
6. **SASL compatibility.** Is `logger_sasl_compatible => true` set for legacy behaviour?
7. **Env-var expansion.** If `${VAR}`-style expansion is needed, where is it done (build tool, not OTP runtime)?

## Related docs

- [applications](applications.md)
- [releases](releases.md)
- [runtime-debugging](runtime-debugging.md)
- [proc-lib-and-sys](proc-lib-and-sys.md)

## Related skills

- `beam-logger-config`
