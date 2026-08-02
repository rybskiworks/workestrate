---
name: beam-logger-config
description: |
  Operational guide for the OTP Logger (OTP 21+) and the OTP `config(4)` file format.
  Load when configuring log levels, handlers, filters, formatters, structured logging,
  report callbacks, or `sys.config` / application env. Does NOT cover runtime
  debugging/tracing (see `beam-observability-debugging`) or application release
  lifecycle (see `beam-applications-releases`).
---

## Triggers

- Configuring OTP Logger levels, handlers, filters, or formatters.
- Writing structured log calls, reports, or report callbacks.
- Working with `sys.config`, `.config` files, or application env precedence.

## References

- `docs/beam/logger-and-config.md`
  - https://www.erlang.org/doc/apps/kernel/logger_chapter.html
  - https://www.erlang.org/doc/apps/kernel/logger.html
  - https://www.erlang.org/doc/apps/kernel/config.html

## Key Rules

- OTP Logger (OTP 21+) replaces `error_logger`. Pipeline:
  primary level (+ module levels) → primary filters → handler filters →
  handler callback `HModule:log/2`. All stages up to and including the handler
  callback run on the client process.
- 8 log levels (RFC 5424 / Syslog), highest→lowest severity: `emergency`,
  `alert`, `critical`, `error`, `warning`, `notice`, `info`, `debug`. Integer
  values are internal; API uses atoms. `all`/`none` are accepted by many
  functions but not part of `level()`. An event passes if its integer value is
  ≤ the configured level.
- Primary level default is `notice` (chapter); set via Kernel param `logger_level`
  or runtime `logger:set_primary_config(level, Level)`.
- Module level overrides primary per module with
  `logger:set_module_level(Module, Level)`. An event must pass BOTH primary and
  module level; module level does NOT override handler level. The originating
  module is detected only if `mfa` is in metadata — macros add it automatically;
  direct API calls do not. Reset with `logger:unset_module_level/1,0`.
- `logger:set_application_level/2` (OTP 21.1) is a convenience that calls
  `set_module_level/2` for each module of an application.
- Macros in `logger.hrl` (`-include_lib("kernel/include/logger.hrl")`):
  `?LOG_EMERGENCY`..`?LOG_DEBUG` and `?LOG`. Macros add `mfa`/`file`/`line`
  metadata and perform lazy evaluation (only evaluated if the primary level
  check passes).
- Functions `logger:Level/1,2,3` are shortcuts for `logger:log(Level, ...)`.
  Direct API calls do not add `mfa` automatically.
- Log message forms: format string + args; string; report (map or key-value list
  — preferred for structured logging); lazy fun (evaluated only if primary check
  passes).
- Report callback: pass `#{report_cb => Fun}` in metadata. `report_cb()` is
  `fun((report()) -> {io:format(), [term()]})` or
  `fun((report(), report_cb_config()) -> unicode:chardata())`. Config map keys:
  `depth`, `chars_limit`, `single_line`.
- Filters: `{FilterFun, Extra}` with arity-2 `FilterFun`. Returns `stop`
  (discard), `ignore` (leave decision to other filters), or the (possibly
  modified) log event. `filter_default` (`log`|`stop`, default `log`) decides
  when all filters return `ignore`. Primary:
  `add_primary_filter/2`/`remove_primary_filter/1`. Handler:
  `add_handler_filter/3`/`remove_handler_filter/2`. Built-in:
  `logger_filters:domain/2`, `level/2`, `progress/2`, `remote_gl/2`.
- Handlers: module exports `log(LogEvent, Config)`. Optional:
  `adding_handler/1`, `changing_config/3`, `filter_config/1`,
  `removing_handler/1`. Built-in: `logger_std_h` (default), `logger_disk_log_h`,
  `error_logger` (backwards compat, not started by default). Add/remove:
  `add_handler(HandlerId, Module, HandlerConfig)` / `remove_handler(HandlerId)`.
  Handler config keys: `level` (default `all`), `filters` (default `[]`),
  `filter_default` (default `log`), `formatter` (default
  `{logger_formatter, DefaultConfig}`), `config` (handler-specific).
- Kernel installs one handler named `default` (`logger_std_h`) at start. Disable
  with `{handler, default, undefined}` in the `logger` Kernel param.
- Formatters: `{FModule, FConfig}`; `FModule` exports
  `format(LogEvent, FConfig)`. Default `logger_formatter`; template tokens
  include `time`, `pid`, `msg`, etc.
- `set_handler_config/2` overwrites; `update_handler_config/2` merges. Same for
  primary config.
- SASL reports (OTP 21+): supervisor/crash reports are `error` level; progress
  reports are `info` level and not logged by default since primary default is
  `notice`. Carry `domain => [otp,sasl]`. Use `logger_sasl_compatible => true`
  for old behaviour.
- `config(4)` is a FILE FORMAT, not a callable module. A `.config` file contains
  a SINGLE Erlang term ending in `.` of shape `[{Application, [{Par, Val}]}]`.
  Named `Name.config`; in embedded mode exactly one `sys.config` in
  `$ROOT/releases/Vsn`. Values retrieved via `application:get_env/1,2`.
  Include-file mechanism: `[{App, [{Par,Val}]} | IncludeFile]` where
  `IncludeFile` is a string name. NO `${VAR}` interpolation, NO `sys.config.src`
  in OTP.
- Config precedence (lowest→highest): `.app` `env` defaults < config files
  (`-config`/`-configfd`, last wins) < command-line flags (`erts:erl(1)`,
  always override).
- `application:set_env/3` changes are disregarded during release upgrade unless
  the app exports `config_change/3`.
- `logger:compare_levels/2` returns `lt`/`eq`/`gt`. `logger:i/0` prints current
  config. `logger:get_primary_config/0` and `logger:get_handler_config/0,1`
  read current settings.
- Reserved metadata keys: `pid`, `gl`, `time`, `mfa`, `file`, `line`, `domain`,
  `report_cb` — do not reuse for custom data.

## Quick Commands

```erl
logger:set_primary_config(level, debug).
logger:set_module_level(my_mod, info).
logger:unset_module_level(my_mod).
?LOG_INFO(#{event => request, id => Id}, #{domain => [myapp]}).
logger:add_handler(myh, logger_std_h,
                   #{config => #{file => "app.log"}, level => info}).
logger:add_handler_filter(myh, myfilter, {fun(_,_) -> stop end, []}).
logger:i().
logger:compare_levels(error, warning).  %% lt
application:get_env(myapp, mypar).
file:consult("releases/Vsn/sys.config").
```

## Anti-patterns

- Using deprecated `error_logger`.
- String concatenation instead of structured reports/metadata.
- Setting a module level and forgetting to reset it.
- Using `application:set_env/3` for compile-time-read values.
- Running handlers without output filters (log floods).
- Using `io:format/2` instead of Logger.
- Expecting `${VAR}` or `sys.config.src` in OTP `config(4)`.
- Reusing reserved metadata keys.
- Calling API functions directly and expecting module-level filtering without
  `mfa` metadata.

## Related Skills

- `beam-observability-debugging`
- `beam-applications-releases`
