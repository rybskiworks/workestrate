# Crawl: kernel/logger.html (focused)
- seed_url: https://www.erlang.org/doc/apps/kernel/logger.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/logger.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: logger-and-config.md

## Purpose
`logger` is the API module for Logger, the standard logging facility in Erlang/OTP.
It implements the main API for logging. Log events are created via API functions
(`logger:error/1,2,3`, etc.) or via macros (`?LOG_ERROR`, ...). The Kernel app
installs one default handler (`default`, a `logger_std_h`) at system start which
prints to the terminal by default. Backend is configured via the configuration
functions in this module and the kernel `logger` config parameter.

## Levels (type + numeric)
```erlang
-type level() :: emergency | alert | critical | error | warning | notice | info | debug.
```
Severity ordering (most -> least severe): emergency > alert > critical > error >
warning > notice > info > debug. `all` and `none` are accepted by many functions
(e.g. `set_module_level`, `set_primary_config(level, _)`, `compare_levels`) as
level-like values but are not part of `level()` itself. Numeric level values are
not exposed on this page (handled internally; see `logger` source). Default
primary level is `info`.

## Macros
Defined in `logger.hrl`, included with `-include_lib("kernel/include/logger.hrl")`.
Each has two arities: `(StringOrReport[,Metadata])` and `(FunOrFormat,Args[,Metadata])`.
- `?LOG_EMERGENCY`
- `?LOG_ALERT`
- `?LOG_CRITICAL`
- `?LOG_ERROR`
- `?LOG_WARNING`
- `?LOG_NOTICE`
- `?LOG_INFO`
- `?LOG_DEBUG`
- `?LOG(Level, ...)` — Level taken from first argument.

All macros expand to a `logger` API call with the level taken from the macro name
(or first arg for `?LOG`). When a macro is used, Logger automatically inserts
location metadata: `mfa => {?MODULE, ?FUNCTION_NAME, ?FUNCTION_ARITY}`,
`file => ?FILE`, `line => ?LINE`. Macro log events are only created if the level
is equal to or below the configured log level.

## Handler API (add/remove/set config, handler config keys)
- `add_handler(HandlerId, Module, Config) -> ok | {error, term()}` —
  `HandlerId :: logger_handler:id()`, `Module :: module()`,
  `Config :: logger_handler:config()`. HandlerId must be unique and used in all
  subsequent calls referring to this handler.
- `remove_handler(HandlerId) -> ok | {error, term()}`.
- `add_handlers(Application) | add_handlers(HandlerConfig)` — reads the app's
  `logger` config param and starts configured handlers; intended to be called
  from `application:start/2` after handler processes are started.
- `set_handler_config(HandlerId, Config)` — overwrites entire handler config.
- `set_handler_config(HandlerId, Key, Value)` — add/update a single key. Keys:
  `level` (`level()|all|none`), `filter_default` (`log|stop`),
  `filters` (`[{filter_id(),filter()}]`), `formatter` (`{module(),formatter_config()}`),
  `config` (`term()` — handler-specific opaque data).
- `update_handler_config(HandlerId, Config|{Key,Value})` — merge semantics
  (equivalent to `maps:merge(Old, Config)`); added OTP 21.2 for the 3-arg form.
- `get_handler_config/0,1`, `get_handler_ids/0` — lookups.
- `update_formatter_config(HandlerId, FormatterConfig|{Key,Value})` — merges
  formatter config only.

Handler config keys (the `logger_handler:config()` map; full type on
`logger_handler` page): `level`, `filter_default`, `filters`, `formatter`,
`config` (handler-specific opaque data), plus `id`/`module` identifying fields.
For Kernel handlers, unspecified `config` sub-keys get default values.

## Filter API (primary + handler, filter_default)
A `filter()` is a fun `(LogEvent, filter_arg()) -> filter_return()` where
`filter_return() :: log_event() | stop | ignore` (i.e. `log`/`stop`/`ignore`).
- `add_primary_filter(FilterId, Filter)` / `remove_primary_filter(FilterId)`.
- `add_handler_filter(HandlerId, FilterId, Filter)` /
  `remove_handler_filter(HandlerId, FilterId)`.
- Filter fun return semantics:
  - `log_event()` (a.k.a. `log`) — filter passed; apply next filter, else forward.
  - `stop` — filter did not pass; log event immediately discarded.
  - `ignore` — filter has no knowledge; apply next filter; if none remain, the
    `filter_default` config decides (`log` = forward, `stop` = discard).
- `filter_default` is a key in both primary config and handler config:
  `log | stop`. Primary default is `log`.
- Built-in filters live in `logger_filters`.

## Module/application level API
- `set_module_level(Modules, Level)` — `Modules :: [module()]|module()`,
  `Level :: level()|all|none`. Module level overrides the primary level for log
  events originating from that module, but does NOT override any handler's level.
  Originating module is detected only if `mfa` metadata is present (auto-added by
  macros; must be added manually when calling API fns directly).
- `unset_module_level()` / `unset_module_level(Modules)`.
- `get_module_level()` / `get_module_level(Modules)` — returns `[{Module,Level}]`.
- `set_application_level(Application, Level)` — convenience calling
  `set_module_level/2` for each module of the application. (OTP 21.1)
- `unset_application_level(Application)`. (OTP 21.1)
- `set_primary_config(level, Level)` changes the global primary level;
  `set_handler_config(HandlerId, level, Level)` changes a handler's level.

## Formatter & report callback
- `formatter_config()` — configuration data for the formatter (see
  `logger_formatter`). Set on a handler via the `formatter` key as
  `{FormatterModule, FormatterConfig()}`.
- `report_cb()` contract:
  ```erlang
  -type report_cb() ::
      fun((report()) -> {io:format(), [term()]}) |
      fun((report(), report_cb_config()) -> unicode:chardata()).
  -type report_cb_config() ::
      #{depth := pos_integer()|unlimited,
        chars_limit := pos_integer()|unlimited,
        single_line := boolean()}.
  ```
  A report callback converts a `report()` (map or kv-list) to `{Format,Args}` or
  directly to chardata. Associated via the `report_cb` metadata key.
- `format_report(Report) -> {io:format(),[term()]}` — the DEFAULT report callback
  used by `logger_formatter` when no custom one is found. Produces `Key: Value`
  lines; strings printed with `~ts`, other terms with `~tp`; maps converted to
  kv-lists first.
- `legacy_mode` is NOT documented on this page (it belongs to `logger_formatter`).

## Built-in handlers (logger_std_h / logger_disk_log_h config keys, summary)
This page does NOT document the handler-specific `config` sub-keys (e.g. `file`,
`type`, `filesync_repeat_interval`, `max_no_files`, `standard_io`, etc.). Those
live on the dedicated pages:
- `logger_std_h` — standard handler; supports `file` output (e.g.
  `#{config => #{file => "path/to/file.log"}}`) and terminal output.
- `logger_disk_log_h` — disk_log-based handler.
Detailed config keys for both must be crawled from their own pages (see links).
The only `config` example on this page: `#{config => #{file => "path/to/file.log"}}`.

## Strict rules
- A log event is forwarded to a handler only if its level is <= the primary level
  AND <= that handler's level. Module level overrides primary (not handler).
- `set_handler_config/2` overwrites; `update_handler_config/2` merges. Same for
  primary (`set_primary_config/1` overwrite vs `update_primary_config/1` merge).
- If a known key is removed via `set_*_config`, the default value is used.
- Metadata merge precedence (highest wins): log-call metadata > process metadata
  > primary metadata > Logger-inserted defaults (`pid`, `gl`, `time`, and for
  macros `mfa`/`file`/`line`).
- Do not reuse reserved metadata keys (`pid`, `gl`, `time`, `mfa`, `file`, `line`,
  `domain`, `report_cb`) for custom data.
- `reconfigure/0` (OTP 24.2) is meant for build tools only, not during app
  lifetime (may drop log entries).
- `olp_config()` (proxy / overload protection) is DEPRECATED on this module;
  use `logger_handler:olp_config/0`.

## Verbatim quotes
- "level() :: emergency | alert | critical | error | warning | notice | info | debug."
- "Primary configuration data for Logger. The following default values apply:
  level => info filter_default => log filters => []"
- "The log level for a module overrides the primary log level of Logger for log
  events originating from the module in question. Notice, however, that it does
  not override the level configuration for any handler."
- "The originating module for a log event is only detected if the key mfa exists
  in the metadata ... When log macros are used, this association is automatically
  added to all log events. If an API function is called directly, without using a
  macro, the logging client must explicitly add this information if module levels
  shall have any effect."
- "values from the log call overwrite process metadata, which overwrites the
  primary metadata, which in turn overwrite values set by Logger."
- "Beware, that [reconfigure] is meant to be run only by the build tools, not
  manually during application lifetime, as this may cause missing log entries."

## Version notes
- Module introduced OTP 21.0 (most functions since OTP 21.0).
- `get_proxy_config/0`, `i/0`, `i/1`, `set_proxy_config/1`, `timestamp/0` since
  OTP 21.3.
- `set_application_level/2`, `unset_application_level/1` since OTP 21.1.
- `update_handler_config/3` since OTP 21.2.
- Primary `metadata` key added OTP 24.0.
- `reconfigure/0` since OTP 24.2.
- Page reflects OTP 29.0.2 / kernel 11.0.2.
- `handler_config()`, `handler_id()`, `olp_config()` types here are DEPRECATED
  in favour of `logger_handler:` equivalents.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/kernel/logger_handler.html — handler config type
  (config keys: level/filter_default/filters/formatter/config, olp_config).
- https://www.erlang.org/doc/apps/kernel/logger_std_h.html — std handler config
  keys (file, type, filesync_repeat_interval, max_no_files, ...).
- https://www.erlang.org/doc/apps/kernel/logger_disk_log_h.html — disk_log handler.
- https://www.erlang.org/doc/apps/kernel/logger_formatter.html — formatter config
  (template, legacy_mode, single_line, depth, ...).
- https://www.erlang.org/doc/apps/kernel/logger_filters.html — built-in filters.
- https://www.erlang.org/doc/apps/kernel/logger_chapter.html — Logger User's Guide.
- https://www.erlang.org/doc/apps/kernel/config.html — kernel config (logger param).

### Skipped
- ../../apps/erts/erlang.html, ../../apps/stdlib/io.html,
  ../../apps/stdlib/unicode.html, ../../index.html (out of scope for this crawl).
- GitHub source links (OTP-29.0.2/lib/kernel/src/logger.erl#L...).
