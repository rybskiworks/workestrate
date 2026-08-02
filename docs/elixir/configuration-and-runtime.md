# Configuration and Runtime Behavior

## Purpose

This document is a multi-section guide covering configuration and runtime behavior in Elixir. The goal is to give AI agents a single reference for writing, reviewing, and debugging Elixir configuration code, from `Logger` and `Application` environment to the `Config`, `System`, and `IO` modules. It covers Logger, the Config module, config.exs vs runtime.exs, Application environment, the System module, and the IO module. Target audience: AI agents working on Elixir code in this repo.

## Sources used

- https://hexdocs.pm/logger/Logger.html (PRIMARY — v1.20.2)
- https://hexdocs.pm/logger/Logger.Formatter.html
- https://hexdocs.pm/logger/Logger.Backends.Console.html
- https://hexdocs.pm/logger/1.17.2/Logger.html
- https://hexdocs.pm/logger/1.15.0/Logger.html
- https://hexdocs.pm/logger/1.10.0/Logger.html
- https://www.erlang.org/doc/apps/kernel/logger.html (Erlang/OTP logger)
- https://hexdocs.pm/elixir/Config.html (PRIMARY — Config DSL)
- https://hexdocs.pm/elixir/Config.Provider.html
- https://hexdocs.pm/elixir/Config.Reader.html
- https://hexdocs.pm/elixir/Application.html (PRIMARY — application environment, compile_env/3, get_application)
- https://hexdocs.pm/elixir/System.html (PRIMARY — env, cmd, time, trap_signal, fetch_env!, EnvError)
- https://hexdocs.pm/elixir/System.EnvError.html
- https://hexdocs.pm/elixir/IO.html (PRIMARY — puts, inspect, binread/binwrite, chardata/iodata)
- https://hexdocs.pm/elixir/IO.ANSI.html (enabled?, format, syntax_colors)
- https://hexdocs.pm/elixir/Inspect.Opts.html (limit, syntax_colors, base)
- https://hexdocs.pm/mix/Mix.html (Build-time and Runtime configuration)
- https://hexdocs.pm/mix/Mix.Tasks.Release.html (config_providers, runtime.exs)
- https://hexdocs.pm/mix/Mix.Tasks.Release.html (:validate_compile_env)
- https://github.com/elixir-lang/elixir/blob/v1.11.0/CHANGELOG.md (runtime.exs introduction)
- https://hexdocs.pm/elixir/design-anti-patterns.html (using application configuration for libraries)
- https://hexdocs.pm/elixir/compatibility-and-deprecations.html (Application.get_env in module body deprecation)
- https://github.com/elixir-lang/elixir/blob/v1.7/CHANGELOG.md (__STACKTRACE__ introduction)
- https://github.com/elixir-lang/elixir/blob/v1.12/CHANGELOG.md (System.trap_signal/3, stacktrace hard-deprecation)

This page reflects Elixir Logger v1.20.2 docs (cross-checked against 1.15.0 and 1.17.2).
The Config Module and config.exs vs runtime.exs sections reflect Elixir Config docs (v1.20.2, cross-checked against 1.17.2 and the 1.11 CHANGELOG).
The Application Environment, System Module, and IO Module sections reflect Elixir v1.20.2 docs (cross-checked against v1.17.2 and v1.17.3).

## Logger

Elixir's [Logger](https://hexdocs.pm/logger/Logger.html) is a thin wrapper over Erlang/OTP's `:logger`. It translates Elixir log levels and terms into Erlang log events and supplies the formatting layer (`Logger.Formatter`). Since Elixir 1.15, the default destination is the Erlang `:logger_std_h` handler (the `:default` handler), not an Elixir-managed `GenEvent` backend.

From the [Logger overview](https://hexdocs.pm/logger/Logger.html):

> "The Logger module provides the main API for logging and is responsible for generating log events with the proper Elixir level and translating those events to Erlang/OTP ones."

This section focuses on practical agent usage: levels, configuration, compile-time purging, formatting, metadata, and migration from pre-1.15 backends.

### Log levels

Logger defines eight levels, ordered from most to least severe:

```text
:emergency > :alert > :critical > :error > :warning > :notice > :info > :debug
```

Use the level-specific macros whenever possible:

```elixir
require Logger

Logger.debug("debug detail")
Logger.info("info message")
Logger.notice("notice message")
Logger.warning("warning message")
Logger.error("error message")
Logger.critical("critical message")
Logger.alert("alert message")
Logger.emergency("emergency message")
```

Each macro accepts an optional second argument for metadata:

```elixir
Logger.info("user signed in", user_id: 42, request_id: "abc")
```

The general-purpose `Logger.log/3` exists for runtime level atoms, but the level macros are preferred:

> "The macros `debug/2`, `info/2`, ... are preferred over this macro as they can automatically eliminate the call to Logger altogether at compile time if desired."

```elixir
# Avoid:
Logger.log(:info, "message")

# Prefer:
Logger.info("message")
```

Two meta-levels are accepted in configuration: `:all` (equivalent to `:debug`, logs everything) and `:none` (logs nothing). The level type spec also retains the alias `:warn` for backwards compatibility, but `Logger.warn/2` is deprecated in favor of `Logger.warning/2`.

```elixir
# Avoid:
Logger.warn("old alias")

# Prefer:
Logger.warning("modern name")
```

With the legacy `Logger.Backends.Console`, only four levels reach the backend: `:debug`, `:info`, `:warning`, `:error`. In that path `:notice` maps to `:info`, `:warn` to `:warning`, and `:critical`/`:alert`/`:emergency` map to `:error`.

### Setting the level: compile time vs runtime

The primary level is configured under the `:logger` application:

```elixir
# config/config.exs or config/runtime.exs
config :logger, level: :warning
```

Change it at runtime with `Logger.configure/1`:

```elixir
Logger.configure(level: :debug)
Logger.level()  #=> :debug
```

`Logger.configure/1` accepts: `:level`, `:translator_inspect_opts`, `:sync_threshold`, `:discard_threshold`, `:truncate`, and `:utc_log`.

Logger supports scoped levels that override or combine with the primary level:

| Scope | Function | Behavior |
|---|---|---|
| Global | `config :logger, level: ...` / `Logger.configure/1` | Primary level for the whole node |
| Per-module | `Logger.put_module_level(SomeMod, :debug)` | Takes priority over the primary level |
| Per-application | `Logger.put_application_level(:my_app, :debug)` | Iterates all modules in the application |
| Per-process | `Logger.put_process_level(self(), :none)` | Works alongside the primary level; the higher (more restrictive) level wins |

```elixir
require Logger

# Per-module: priority over primary level
Logger.put_module_level(MyApp.Worker, :debug)

# Per-application
Logger.put_application_level(:my_app, :warning)

# Per-process: only pid 0 (self) is accepted
Logger.put_process_level(self(), :none)
```

Reset scoped levels with the matching `delete_*` functions:

```elixir
Logger.delete_module_level(MyApp.Worker)
Logger.delete_application_level(:my_app)
Logger.delete_process_level(self())
```

Use `Logger.compare_levels/2` to compare severity:

```elixir
Logger.compare_levels(:debug, :warning)  #=> :lt
Logger.compare_levels(:error, :info)     #=> :gt
Logger.compare_levels(:info, :info)      #=> :eq
```

### Compile-time purging

Use `:compile_time_purge_matching` to remove matching `Logger` calls entirely at compilation time. Purged calls have zero runtime overhead.

From [Logger.html](https://hexdocs.pm/logger/Logger.html):

> "It expects a list of keywords lists. Each keyword list contains a matching condition."

Special matching keys:

| Key | Meaning |
|---|---|
| `:level_lower_than` | Purge all messages with a lower (less severe) logger level |
| `:module` | Purge all messages from the matching module |
| `:function` | Purge all messages from the matching `'function/arity'` string |

```elixir
# config/config.exs
config :logger,
  compile_time_purge_matching: [
    [level_lower_than: :info]
  ]
```

Multiple rules are OR-ed:

```elixir
config :logger,
  compile_time_purge_matching: [
    [application: :foo],
    [module: Bar, function: "foo/3", level_lower_than: :error]
  ]
```

Critical caveat:

> "Remember that if you want to purge log calls from a dependency, the dependency must be recompiled."

That means changing purge rules alone will not affect already-compiled dependencies; force a rebuild with `mix deps.compile --force <dep>` or similar.

Two related compile-time options:

```elixir
# Force evaluation of log arguments even when the level suppresses the call.
# Useful in tests to catch String.Chars protocol errors.
config :logger, always_evaluate_messages: true

# Mix sets this automatically; it controls the :application metadata key.
config :logger, compile_time_application: :my_app
```

Important naming note: the official option is `:compile_time_purge_matching`. Names such as `:compile_time_purge_level` or `:compile_time_purge_log_level` do not appear in official Logger docs and are a common misconception.

```elixir
# Correct:
config :logger, compile_time_purge_matching: [[level_lower_than: :info]]

# Incorrect (not a real option):
# config :logger, compile_time_purge_level: :info
```

### Backends and the 1.15 handler migration

Before Elixir 1.15, Logger used Elixir-managed `GenEvent` backends:

```elixir
# Elixir <= 1.14 (legacy; still auto-mapped in 1.15+)
config :logger, backends: [:console]

config :logger, :console,
  level: :error,
  format: "$time $message $metadata",
  metadata: [:user_id],
  colors: [enabled: true]
```

`Logger.Backends.Console` is now deprecated. From [Logger.Backends.Console.html](https://hexdocs.pm/logger/Logger.Backends.Console.html):

> "This module is deprecated. Use LoggerBackends.Console from `:logger_backends` dependency."

Since Elixir 1.15, the default destination is the Erlang `:logger_std_h` handler. Configuration splits into `:default_handler` (handler-level options, primarily `:level`) and `:default_formatter` (formatting options).

From [Logger.html](https://hexdocs.pm/logger/Logger.html):

> "Previously, you would set:
> ```elixir
> config :logger, :console,
>   level: :error,
>   format: "$time $message $metadata"
> ```
> This is now equivalent to:
> ```elixir
> config :logger, :default_handler,
>   level: :error
>
> config :logger, :default_formatter,
>   format: "$time $message $metadata"
> ```
> All previous console configuration, except for `:level`, now go under `:default_formatter`."

Modern configuration looks like this:

```elixir
config :logger,
  default_handler: [],
  default_formatter: [
    format: "\n$time $metadata[$level] $message\n",
    metadata: [:user_id, :request_id],
    colors: [enabled: true],
    truncate: 8192
  ]
```

Boot-time options on `:logger` include:

| Option | Default | Purpose |
|---|---|---|
| `:default_handler` | handler config or `[]` | Erlang handler options; set to `false` to disable |
| `:default_formatter` | formatter config or `[]` | `Logger.Formatter` options |
| `:handle_otp_reports` | `true` | Route Erlang/OTP reports through Elixir Logger |
| `:handle_sasl_reports` | `false` | Route SASL reports |
| `:metadata` | `[]` | Global primary metadata merged into every event |

The legacy dynamic backend API is deprecated:

```elixir
# Avoid:
Logger.add_backend(Logger.Backends.Console)
Logger.remove_backend(Logger.Backends.Console)
Logger.configure_backend(Logger.Backends.Console, format: "...")

# Prefer (if you truly need the old backend):
# Add :logger_backends to deps and use LoggerBackends.add/2
```

For direct Erlang handler management, use `:logger` functions:

```elixir
:logger.add_handler(:my_handler, :logger_std_h, %{...})
:logger.remove_handler(:my_handler)
:logger.update_handler_config(:my_handler, :level, :warning)
```

### Default formatter

[Logger.Formatter](https://hexdocs.pm/logger/Logger.Formatter.html) turns log events into text. The default format string is:

```text
"\n$time $metadata[$level] $message\n"
```

Template tokens recognized by `Logger.Formatter.compile/1`:

| Token | Output |
|---|---|
| `$time` | Time the log was sent |
| `$date` | Date the log was sent |
| `$message` | The log message |
| `$level` | The log level |
| `$levelpad` | Padding to align level names |
| `$node` | Node that prints the message |
| `$metadata` | User metadata as `"key=val key2=val2 "` |

Compile a pattern for reuse:

```elixir
Logger.Formatter.compile("$time $metadata [$level] $message\n")
#=> [:time, " ", :metadata, " [", :level, "] ", :message, "\n"]
```

Use `Logger.Formatter.new/1` to build a `:logger`-compatible formatter config from keyword options:

```elixir
formatter = Logger.Formatter.new(
  format: "$time [$level] $message\n",
  metadata: [:request_id],
  colors: [enabled: true]
)
```

You can also supply a `{module, function}` tuple as the `:format` value. The function receives `(level, message, timestamp, metadata)` and must return `IO.chardata/0`:

```elixir
config :logger, :default_formatter,
  format: {MyConsoleLogger, :format}

defmodule MyConsoleLogger do
  @spec format(atom, IO.chardata(), Logger.Formatter.date_time_ms(), keyword()) :: IO.chardata()
  def format(level, message, _timestamp, metadata) do
    # Never raise inside a formatter; rescue internally if needed.
    "[#{level}] #{metadata[:request_id] || "-"} #{message}\n"
  end
end
```

### Truncation

The `:truncate` option controls how many bytes a formatted message may occupy. The default is **8192 bytes** across the documented versions (1.10, 1.15, 1.17.2, 1.20.2). Set `:infinity` to disable truncation.

```elixir
config :logger, :default_formatter, truncate: 8192

# Or disable entirely:
config :logger, :default_formatter, truncate: :infinity
```

Truncated messages receive the suffix `" (truncated)"`. Truncation is grapheme-cluster aware and never splits a code point. Note: the value 8088 sometimes appears online, but the correct default is 8192.

```elixir
require Logger

Logger.info(String.duplicate("x", 10_000))
# The printed message is truncated to ~8192 bytes plus " (truncated)"
```

### Colors

Color configuration lives under `:default_formatter` inside the `:colors` keyword list:

```elixir
config :logger, :default_formatter,
  colors: [
    enabled: true,
    debug: :cyan,
    info: :normal,
    warning: :yellow,
    error: :red
  ]
```

Defaults (v1.20.2):

| Level | Default color |
|---|---|
| `:debug` | `:cyan` |
| `:info` / `:notice` | `:normal` |
| `:warning` | `:yellow` |
| `:error` / `:critical` / `:alert` / `:emergency` | `:red` |

`:enabled` defaults to `IO.ANSI.enabled?/0`. Accepted color atoms come from `IO.ANSI` (e.g. `:red`, `:green`, `:yellow`, `:blue`, `:magenta`, `:cyan`, `:white`, `:normal`, `:bright`, `:underline`, `:reverse_video`).

Override color for a single message via the `:ansi_color` metadata key:

```elixir
Logger.info("highlighted", ansi_color: :magenta)
```

### Metadata

Metadata lets you attach structured context to log events. The crucial thing to remember: metadata is stored in the **process dictionary**. It is process-scoped and dies with the process.

```elixir
require Logger

Logger.metadata(request_id: "abc", user_id: 42)
Logger.metadata()  #=> [request_id: "abc", user_id: 42]

# Setting a key to nil removes it
Logger.metadata(user_id: nil)

# Replace all process metadata
Logger.reset_metadata(request_id: "def")

# Reset everything
Logger.reset_metadata()
```

A common mistake is treating `Logger.metadata/0` as global config. It only returns the current process's metadata.

Default metadata keys are auto-added by Logger and cannot be overridden by user metadata:

```text
:application, :mfa, :file, :line, :pid, :initial_call, :registered_name,
:domain, :crash_reason, :module, :function
```

(OTP 27+ also adds `:process_label` on Elixir 1.17.2+.)

Distinguish two kinds of metadata configuration:

```elixir
# 1. Global PRIMARY metadata: merged into every event.
config :logger, metadata: [:request_id, :user_id]

# 2. Per-formatter metadata: controls which keys the $metadata token prints.
config :logger, :default_formatter,
  metadata: [:request_id, :user_id]
```

Adding a key to the formatter's `:metadata` list does not add it to the event; the event must already contain the key (from `Logger.metadata/1` or global primary metadata).

### Async/sync and overload protection

Logger applies backpressure through `:sync_threshold` and `:discard_threshold`:

| Option | Default | Meaning |
|---|---|---|
| `:sync_threshold` | `20` | Message-queue size at which Logger switches from async to sync mode |
| `:discard_threshold` | `500` | Queue size at which Logger begins dropping messages directly in clients |
| `:discard_threshold_periodic_check` | `30_000` | Milliseconds between periodic checks |

Configure them the same way as the primary level:

```elixir
config :logger,
  sync_threshold: 20,
  discard_threshold: 500,
  discard_threshold_periodic_check: 30_000
```

In 1.15+ these options still pass through `Logger.configure/1`, but the underlying behavior lives in the Erlang `:logger` proxy.

Flush handlers synchronously when you need to guarantee logs have reached disk (useful in tests, avoid in production hot paths):

```elixir
Logger.flush()
```

> "This guarantees all logger handlers flush to disk or storage. This is useful for testing but it should be avoided in production…"

### Adding custom Erlang handlers (1.15+)

To add a non-default handler, declare it under your application's `:logger` config and attach it from `Application.start/2` with `Logger.add_handlers/1`.

File-rotation example using `:logger_std_h`:

```elixir
# config/runtime.exs
config :my_app, :logger, [
  {:handler, :file_log, :logger_std_h, %{
    config: %{
      file: ~c"system.log",
      filesync_repeat_interval: 5000,
      file_check: 5000,
      max_no_bytes: 10_000_000,
      max_no_files: 5,
      compress_on_rotate: true
    },
    formatter: Logger.Formatter.new()
  }}
]
```

```elixir
# lib/my_app/application.ex
defmodule MyApp.Application do
  use Application
  require Logger

  @impl true
  def start(_type, _args) do
    Logger.add_handlers(:my_app)

    children = [
      # ...
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: MyApp.Supervisor)
  end
end
```

For a fully custom handler, implement the `:logger_handler` behaviour:

```elixir
defmodule MyApp.CustomHandler do
  @behaviour :logger_handler

  @impl :logger_handler
  def log(event, _config) do
    IO.inspect(event)
  end
end
```

Then add it via `:logger.add_handler/3` or the `Logger.add_handlers/1` config tuple.

### Recommended modern configuration

A complete, modern `Logger` setup for a Mix project targeting Elixir 1.15+:

```elixir
# config/config.exs
import Config

config :logger,
  level: :info,
  sync_threshold: 20,
  discard_threshold: 500,
  truncate: 8192,
  metadata: [:request_id, :user_id],
  compile_time_purge_matching: [
    [level_lower_than: :info]
  ],
  default_handler: [],
  default_formatter: [
    format: "\n$time $metadata[$level] $message\n",
    metadata: [:request_id, :user_id],
    colors: [enabled: true],
    truncate: 8192
  ]
```

```elixir
# config/dev.exs
import Config

config :logger, level: :debug
```

```elixir
# config/test.exs
import Config

config :logger, level: :warning

# Evaluate log args in tests to catch formatter/protocol mistakes.
config :logger, always_evaluate_messages: true
```

```elixir
# config/runtime.exs
import Config

# Override with an env var at boot time, defaulting to :info.
config :logger, level: String.to_existing_atom(System.get_env("LOG_LEVEL", "info"))
```

## Config Module

The `Config` module provides the keyword-based DSL used inside every `config/*.exs` file. The DSL is brought into scope by `import Config`, the standard top-of-file directive (the older `use Mix.Config` was deprecated in Elixir 1.9 and is no longer available). From [Config](https://hexdocs.pm/elixir/Config.html):

> "A simple keyword-based configuration API."

> "`import Config` will import the functions `config/2`, `config/3`, `config_env/0`, `config_target/0`, and `import_config/1` to help you manage your configuration."

`Config` was introduced in Elixir 1.9 to replace `use Mix.Config`. Since Elixir 1.11 it also provides `config_env/0` and `config_target/0` (so config files can branch on environment without `Mix.env/0`, which is unavailable in releases), and since Elixir 1.18 it provides `read_config/1`. The full public surface in current Elixir is exactly these six: `config/2`, `config/3`, `config_env/0`, `config_target/0`, `import_config/1`, and `read_config/1`. Note that `Config` has no `env!/1` or `env!/2` — runtime environment-variable access lives in the `System` module (`System.fetch_env!/1`, `System.get_env/2`).

### `config/2` — flat application configuration

`config/2` sets a keyword list of options for a given application (the root key):

```elixir
import Config

config :my_app,
  key1: "value1",
  key2: "value2"
```

Keyword lists are **deep-merged** on repeated calls; non-keyword lists and scalar values are replaced. From [Config](https://hexdocs.pm/elixir/Config.html):

> "The given `opts` are merged into the existing configuration for the given `root_key`. Conflicting keys are overridden by the ones specified in `opts`, unless they are keywords, which are deep merged recursively."

```elixir
config :logger,
  level: :warn,
  backends: [:console]

config :logger,
  level: :info,
  truncate: 1024

# Final value for :logger:
# [level: :info, backends: [:console], truncate: 1024]
```

Read these values at runtime with `Application.fetch_env!/2`:

```elixir
"value1" = Application.fetch_env!(:my_app, :key1)
```

Note that `backends: [:console]` (a non-keyword list) is **replaced** wholesale by a later `backends:` value, not concatenated.

### `config/3` — sub-key configuration

`config/3` stores a value under a second-level key (conventionally a module name) inside the application's environment. It is the right choice when an application owns multiple sub-configurations addressed by a key — typically a Repo or endpoint module.

```elixir
config :my_app, MyApp.Repo,
  log_level: :warn,
  adapter: Ecto.Adapters.Postgres,
  metadata: [read_only: true]

config :my_app, MyApp.Repo,
  log_level: :info,
  pool_size: 10,
  metadata: [replica: true]
```

The two calls deep-merge, producing:

```elixir
Application.get_env(:my_app, MyApp.Repo)
#=> [
#=>   log_level: :info,
#=>   pool_size: 10,
#=>   adapter: Ecto.Adapters.Postgres,
#=>   metadata: [read_only: true, replica: true]
#=> ]
```

The crucial distinction between the two arities:

| Form | Stores | Read with |
|---|---|---|
| `config :app, key: value` | flat keyword list under `:app` | `Application.fetch_env!(:app, :key)` |
| `config :app, SubKey, opts` | `opts` under `:app` keyed by `SubKey` | `Application.get_env(:app, SubKey)` |

Using `config/2` when you meant `config/3` (or vice versa) is a common source of "why is my config nil" bugs, because the read path differs.

### `import_config/1`

`import_config/1` pulls in another config file, evaluated relative to the directory of the *current* file:

```elixir
# config/config.exs
import Config

import_config "#{config_env()}.exs"
```

From [Config](https://hexdocs.pm/elixir/Config.html):

> "Imports configuration from the given file. In case the file doesn't exist, an error is raised. If file is a relative, it will be expanded relatively to the directory the current configuration file is in."

Two important constraints:

1. `import_config/1` **cannot be used from `config/runtime.exs`**. From [Config](https://hexdocs.pm/elixir/Config.html):

   > "Note, however, some configuration files, such as `config/runtime.exs` does not support imports, as they are meant to be copied across systems."

2. Wildcard paths are no longer supported directly — expand them manually:

   ```elixir
   # Old (Mix.Config):
   # import_config "../apps/*/config/config.exs"

   # New (Config):
   for config <- "../apps/*/config/config.exs" |> Path.expand(__DIR__) |> Path.wildcard() do
     import_config config
   end
   ```

### `config_env/0` and `config_target/0`

These two macros (introduced in Elixir 1.11) report the environment and target the config file is running under. They are the **only** correct way to branch on environment inside any `config/*.exs` file, including `config/runtime.exs`. From [Config](https://hexdocs.pm/elixir/Config.html):

> "`config_env/0` — Returns the environment this configuration file is executed on. In Mix projects this function returns the environment this configuration file is executed on. In releases, the environment when `mix release` ran."

```elixir
if config_env() == :prod do
  config :my_app, :debug, false
end

if config_target() == :host do
  config :my_app, :debug, false
end
```

**Never call `Mix.env/0` or `Mix.target/0` from a config file.** Mix is a build tool and is not available inside releases. From [Mix](https://hexdocs.pm/mix/Mix.html):

> "This function should not be used at runtime in application code... Mix is a build tool and may not be available after the code is compiled (for example in a release)."

This is the single most common way `config/runtime.exs` breaks inside a release: a stray `Mix.env()` call.

### `read_config/1` (Elixir 1.18+)

`read_config/1` returns the configuration previously set for a root key **within the same config evaluation**, without going through the application environment. It is useful for sharing values across multiple config files. From [Config](https://hexdocs.pm/elixir/Config.html):

> "Reads the configuration for the given root key. This function only reads the configuration from a previous `config/2` or `config/3` call. If `root_key` points to an application, it does not read its actual application environment. ... If the `root_key` was not configured, it returns `nil`."

```elixir
# config/config.exs
config :my_app, foo: :bar

# config/dev.exs (imported by config/config.exs)
config :another_app, foo: read_config(:my_app)[:foo] || raise "missing parent configuration"
```

On Elixir versions earlier than 1.18, `read_config/1` is not available; use `Application.get_env/2` or restructure the config so each file sets its own values.

### `Config.Reader` — the loader engine

`Config.Reader` is the low-level API that Mix and releases use to evaluate `config/*.exs` files. `Config` is the DSL; `Config.Reader` is the loader that runs it. From [Config.Reader](https://hexdocs.pm/elixir/Config.Reader.html):

> "API for reading config files defined with `Config`."

Its public surface:

| Function | Since | Purpose |
|---|---|---|
| `read!/2` | 1.9 | Read and evaluate a config file; accepts `:imports`, `:env`, `:target` options |
| `read_imports!/2` | 1.9 | Like `read!/2` but also returns the list of imported paths |
| `eval!/3` | 1.11 | Evaluate config *contents* (a binary) as if from a given file |
| `merge/2` | 1.9 | Deep-merge two config keyword lists (later wins; recursive for keyword lists) |

```elixir
Config.Reader.merge([app: [k: :v1]], [app: [k: :v2]])
#=> [app: [k: :v2]]

Config.Reader.merge([app: [k: [v1: 1, v2: 2]]], [app: [k: [v2: :a, v3: :b]]])
#=> [app: [k: [v1: 1, v2: :a, v3: :b]]]
```

This `merge/2` is the same deep-merge rule used by `config/2` and `config/3`. When you need to merge config inside a custom provider, use `Config.Reader.merge/2` rather than rolling your own.

### `Config.Provider` — loading config at boot

`Config.Provider` is a behaviour for loading configuration **during system boot**, typically in releases. Providers run while only a minimal set of applications are started, before the project's applications launch. From [Config.Provider](https://hexdocs.pm/elixir/Config.Provider.html):

> "Specifies a provider API that loads configuration during boot. Config providers are typically used during releases to load external configuration while the system boots. This is done by starting the VM with the minimum amount of applications running, then invoking all of the providers, and then restarting the system."

The behaviour has two callbacks:

```elixir
@callback init(term()) :: state()
@callback load(config(), state()) :: config()
```

- `init/1` runs **on the machine where the release is assembled** (build time). It validates provider arguments and prepares state. The returned state must be serializable (atoms, strings, numbers, lists, maps, tuples) because it is written to a config file on disk.
- `load/2` runs **at boot** on the target machine. It receives the current config and the `init/1` state, reads its external source, and returns updated config. Merging must be done with `Config.Reader.merge/2`.

```elixir
defmodule MyApp.JsonConfigProvider do
  @behaviour Config.Provider

  @impl true
  def init(path) when is_binary(path), do: path

  @impl true
  def load(config, path) do
    {:ok, _} = Application.ensure_all_started(:jason)
    json = path |> File.read!() |> Jason.decode!()

    Config.Reader.merge(config, my_app: [some_value: json["my_app_some_value"]])
  end
end
```

Providers are wired into a release via the `:config_providers` option in `mix.exs`, as `{Module, opts}` tuples:

```elixir
# mix.exs
releases: [
  demo: [
    config_providers: [
      {MyApp.JsonConfigProvider, "/etc/my_app/config.json"}
    ]
  ]
]
```

Elixir ships with **exactly one** built-in provider, `Config.Reader`, which is what loads `config/runtime.exs` at boot. From [Config.Provider](https://hexdocs.pm/elixir/Config.Provider.html):

> "Elixir ships with one provider, called `Config.Reader`, which is capable of handling Elixir's built-in config files."

The `Config.Reader` provider accepts a path or a `{:system, var, path}` tuple (the only place the provider framework itself reads an environment variable — to *locate* a config file):

```elixir
config_providers: [
  {Config.Reader, {:system, "RELEASE_ROOT", "/extra_config.exs"}}
]
```

Names such as `Config.Provider.Elixir`, `Config.Provider.Env`, `Config.Provider.Toml`, or `Config.Provider.Path` are **not** part of Elixir core. If you need TOML, env-var, or vault-backed providers, use a third-party Hex package that implements the `Config.Provider` behaviour.

## config.exs vs runtime.exs

Elixir separates configuration into two phases: **build-time** (compile-time) configuration, evaluated whenever the code compiles, and **runtime** configuration, evaluated just before applications start. The phase a file runs in is determined by its name and location under `config/`.

The single most important rule:

> **Put values that must change per-deployment (database URLs, ports, secrets, feature flags read from the environment) in `config/runtime.exs`. Put everything static — including all configuration of dependencies — in `config/config.exs`.**

### `config/config.exs` — build-time

From [Mix](https://hexdocs.pm/mix/Mix.html) ("Build-time configuration"):

> "Whenever you invoke a `mix` command, Mix loads the configuration in `config/config.exs`, if said file exists. It is common for the `config/config.exs` file itself to import other configuration based on the current `MIX_ENV`, such as `config/dev.exs`, `config/test.exs`, and `config/prod.exs`, by calling `Config.import_config/1`:
>
> ```elixir
> import Config
> import_config "#{config_env()}.exs"
> ```
>
> We say `config/config.exs` and all imported files are build-time configuration as they are evaluated whenever you compile your code."

`config/config.exs` (and the `dev.exs`/`test.exs`/`prod.exs` files it imports) are evaluated on the **build host** at compile time. Their results are baked into the compiled `.app` files and bytecode under `_build/`.

Reading environment variables here is almost always a mistake. From [Mix](https://hexdocs.pm/mix/Mix.html):

> "In other words, if your configuration does something like:
>
> ```elixir
> import Config
> config :my_app, :secret_key, System.fetch_env!("MY_APP_SECRET_KEY")
> ```
>
> The `:secret_key` key under `:my_app` will be computed on the host machine before your code compiles. This can be an issue if the machine compiling your code does not have access to all environment variables used to run your code, as loading the config above will fail due to the missing environment variable. Furthermore, even if the environment variable is set, changing the environment variable will require a full recompilation of your application by calling `mix compile --force` (otherwise your project won't start)."

Consequences:

- The value is frozen into the BEAM; the runtime host's environment is irrelevant.
- The build host must have the env var set, or compilation fails.
- Changing the env var requires `mix compile --force`.

```elixir
# config/config.exs — AVOID reading env vars here
import Config

config :my_app, :secret_key, System.fetch_env!("MY_APP_SECRET_KEY")  # wrong phase
```

This file's job is static, compile-time settings: configuring dependencies (e.g. `config :logger, ...`), compile-time feature flags, and per-environment *compile-time* overrides via `import_config "#{config_env()}.exs"`.

A useful fact about dependencies: when your project depends on a library, **the library's own `config/config.exs` is not evaluated**. From [Config](https://hexdocs.pm/elixir/Config.html) ("Avoid application environment in libraries"):

> "Note that the `config/config.exs` of a library is not evaluated when the library is used as a dependency, as configuration is always meant to configure the current project."

So *your* `config/config.exs` is what configures your dependencies.

### `config/runtime.exs` — runtime

`config/runtime.exs` was introduced in **Elixir 1.11** as a unified runtime-configuration file. It runs just before applications start, in **all** environments and under **both** Mix and releases. From [Config](https://hexdocs.pm/elixir/Config.html):

> "For runtime configuration, you can use the `config/runtime.exs` file. It is executed right before applications start in both Mix and releases (assembled with `mix release`)."

From the Elixir 1.11 release notes:

> "Elixir v1.11 addresses this issue by introducing a new configuration file, called `config/runtime.exs`. This new configuration file is loaded exactly before your application starts, when the code is already fully compiled. It is loaded in development, test, and production, regardless if you are using Mix or releases. Therefore it provides a unified API for runtime configuration in Elixir."

This is the correct place to read environment variables:

```elixir
# config/runtime.exs
import Config

config :my_app, :secret_key, System.fetch_env!("MY_APP_SECRET_KEY")

config :my_app, MyApp.Repo,
  url: System.get_env("DATABASE_URL", "ecto://localhost/my_app_dev"),
  pool_size: String.to_integer(System.get_env("POOL_SIZE", "10"))
```

From [Mix.Tasks.Release](https://hexdocs.pm/mix/Mix.Tasks.Release.html) ("Runtime configuration"):

> "Your `config/runtime.exs` file needs to follow three important rules:
>
> - It MUST `import Config` at the top instead of the deprecated `use Mix.Config`
> - It MUST NOT import any other configuration file via `import_config`
> - It MUST NOT access `Mix` in any way, as `Mix` is a build tool and it is not available inside releases"

Use `config_env/0` (not `Mix.env/0`) to branch per environment inside `runtime.exs`:

```elixir
# config/runtime.exs
import Config

if config_env() == :prod do
  config :my_app, MyAppWeb.Endpoint,
    server: true,
    url: [host: System.get_env("PHX_HOST"), port: 443]
end

config :my_app, :secret_key, System.fetch_env!("SECRET_KEY")
```

For values that should be optional, prefer `System.get_env("VAR", "default")` (returns a default). For values that are required and should fail loudly at boot, prefer `System.fetch_env!("VAR")` (raises `System.EnvError` if unset). Remember the return value is always a string — cast explicitly (`String.to_integer/1`, `String.to_existing_atom/1`, etc.).

### Side-by-side comparison

| Property | `config/config.exs` (+ `dev.exs`/`test.exs`/`prod.exs`) | `config/runtime.exs` |
|---|---|---|
| Phase | Build-time (on host, each compile) | Runtime (just before apps start) |
| Introduced | Elixir 1.0+ (`use Mix.Config`); `import Config` since 1.9 | **Elixir 1.11** |
| Runs in `iex -S mix` | Yes, when compiling | Yes, on every start |
| Runs in `mix <task>` | Yes | Yes |
| Runs in release boot | No (baked in at `mix release`) | Yes |
| Read env vars? | **Avoid** — value baked into BEAM; needs `mix compile --force` to change | **Yes — the entire point** |
| `import_config` allowed? | Yes | **No** (explicitly forbidden) |
| `Mix.env()` / `Mix.target()` | Avoid; use `config_env/0` / `config_target/0` | **Forbidden** — Mix unavailable in releases |
| Typical contents | Dependency config, static settings, compile-time flags | DB URLs, ports, secrets, host names, per-deploy overrides |
| Predecessor | `use Mix.Config` (deprecated 1.9) | `config/releases.exs` (soft-deprecated 1.11) |

A common misconception: `config/dev.exs`, `config/test.exs`, and `config/prod.exs` are **not** runtime variants. They are compile-time files imported by `config/config.exs`. There is exactly one runtime file, `config/runtime.exs`, and it runs in every environment — branch inside it with `if config_env() == :prod do ... end`.

### Releases and `:config_providers`

When you build a release with `mix release`, `config/runtime.exs` is copied into the release and executed early in the boot process. From [Mix.Tasks.Release](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "If a `config/runtime.exs` exists, it will be copied to your release and executed early in the boot process, when only Elixir and Erlang's main applications have been started."

The runtime file is itself loaded by the built-in `Config.Reader` provider. Additional `Config.Provider` modules can be wired in via the `:config_providers` option in `mix.exs` to load config from external sources (a TOML/JSON file, a secret vault, a distributed config service) at boot. See the **Config Module** section above for the provider behaviour and the `{Module, opts}` tuple form.

Related release options (set in the `releases` key of `mix.exs`):

| Option | Default | Purpose |
|---|---|---|
| `:config_providers` | `[]` | List of `{Module, opts}` config-provider tuples |
| `:reboot_system_after_config` | `false` (true only with legacy `releases.exs`) | Reboot the VM after config so `runtime.exs` can configure `:kernel`/`:stdlib`; needed for low-level VM config |
| `:validate_compile_env` | `true` | At boot, compare runtime config against values frozen at compile time via `Application.compile_env/3`; mismatch fails the boot |

### The `Application.compile_env/3` boundary

In rare cases a module attribute must depend on a config value at compile time (e.g. a library that embeds a value into compiled code). Reading `Application.fetch_env!/2` in a module body produces a warning and fails if the application isn't yet loaded. Use `Application.compile_env/3` instead. From [Application](https://hexdocs.pm/elixir/Application.html) ("Compile-time environment"):

> "This allows Elixir to track when configuration values change between compile time and runtime."

```elixir
defmodule MyApp.DBClient do
  # Frozen at compile time; default required.
  @db_host Application.compile_env(:my_app, :db_host, "db.local")

  def start_link, do: SomeLib.DBClient.start_link(host: @db_host)
end
```

`compile_env/3` records the value used at compile time. Because `:validate_compile_env` defaults to `true` in releases, **the release will fail to boot** if the runtime value of `:db_host` differs from the compile-time value. This makes `compile_env` a deliberate, auditable choice — not a default.

The bang variant `Application.compile_env!/2` raises on missing keys, and both accept a path list (`[:key, :foo, :bar]`) for nested traversal. For everything else, prefer reading the application environment **at runtime** (inside a function body) with `Application.fetch_env!/2` or `Application.get_env/3`.

## Application Environment

The `Application` module manages the application environment: a per-application key-value store that is loaded from the `.app` resource file and updated by `config/*.exs` files. It is the canonical place for runtime configuration of your own application, but it is also a global state surface that libraries should avoid.

From [Application](https://hexdocs.pm/elixir/Application.html):

> "The application environment is one of the ways to configure applications in Elixir."

### Reading values: get_env, fetch_env, and fetch_env!

`Application.get_env/3` is the permissive reader:

```elixir
Application.get_env(:my_app, :db_host)
#=> nil

Application.get_env(:my_app, :db_host, "localhost")
#=> "localhost"
```

Its full signature is `get_env(app, key, default \\ nil)`. It returns the stored value or the default; it never raises and never returns `:error`. This also means it cannot distinguish between "the key is missing" and "the key is set to `nil`". For that distinction, use `Application.fetch_env/2`:

```elixir
Application.fetch_env(:my_app, :db_host)
#=> {:ok, "db.local"}

Application.fetch_env(:my_app, :missing_key)
#=> :error
```

`Application.fetch_env!/2` is the loud reader for required keys. It raises `ArgumentError` (not `KeyError`) when the key is unavailable:

```elixir
Application.fetch_env!(:my_app, :missing_key)
#=> ** (ArgumentError) could not fetch application environment :missing_key for application :my_app because configuration at :missing_key was not set
```

If the application itself is not loaded or configured, the message instead says `because the application was not loaded nor configured`. Many examples online claim this raises `KeyError`; it does not. It is `ArgumentError`.

Recommended read pattern:

| Requirement | Function | Reason |
|---|---|---|
| Optional with default | `get_env/3` | Simplest, never raises |
| Optional but must branch on presence | `fetch_env/2` | Distinguishes missing from `nil` |
| Required | `fetch_env!/2` | Fails early with a clear error |

### Writing values: put_env and delete_env

`Application.put_env/4` stores a value at runtime:

```elixir
Application.put_env(:my_app, :db_host, "db.local")
#=> :ok
```

The signature is `put_env(app, key, value, opts \\ [])`. Options are `:timeout` (default `5000`) and `:persistent` (boolean). The `:persistent` option matters when the application is reloaded:

From [Application](https://hexdocs.pm/elixir/Application.html):

> "`:persistent` ... guarantees parameters set with this function will not be overridden by the ones defined in the application resource file on load."

The opposite case is just as important:

From [Application](https://hexdocs.pm/elixir/Application.html):

> "If `put_env/4` is called before the application is loaded, the application environment values specified in the `.app` file will override the ones previously set."

So runtime mutation before the application starts is fragile. Use `:persistent` when you need a runtime override to survive application reload.

A second, stronger warning from the docs:

From [Application](https://hexdocs.pm/elixir/Application.html):

> "Do not use this function to change environment variables read via `Application.compile_env/2`."

`compile_env` freezes a value at compile time; mutating it at runtime is a bug and may cause a release boot failure when `:validate_compile_env` is true.

`Application.delete_env/3` removes a key:

```elixir
Application.delete_env(:my_app, :temporary_key)
#=> :ok
```

It accepts the same `:timeout` and `:persistent` options as `put_env/4`. There is no `delete_env!/3`; if you need to assert the key existed before deletion, read it first with `fetch_env!/2`.

### Compile-time environment: compile_env/3 and compile_env!/2

Since Elixir 1.10, `Application.compile_env/3` and `Application.compile_env!/2` read configuration at compile time. They are macros, so they cannot be called inside a function body.

From [Application](https://hexdocs.pm/elixir/Application.html) ("Compile-time environment"):

> "Similar to `get_env/3`, except it must be used to read values at compile time. This allows Elixir to track when configuration values change between compile time and runtime."

Use them for module attributes that must embed a config value:

```elixir
defmodule MyApp.DBClient do
  @db_host Application.compile_env(:my_app, :db_host, "db.local")

  def host, do: @db_host
end
```

`compile_env/3` takes a default; `compile_env!/2` raises `ArgumentError` if the key is missing. Both accept a path list for nested traversal:

```elixir
@timeout Application.compile_env(:my_app, [:http, :timeout], 5000)
```

Important: `compile_env` does **not** use the default declared in `mix.exs`'s `def application do [env: [...]] end` block. From [Application](https://hexdocs.pm/elixir/Application.html):

> "`compile_env` expects the default value to be given as an argument, instead of using the `def application` function of your `mix.exs`."

And in general, compile-time configuration should be rare:

From [Application](https://hexdocs.pm/elixir/Application.html):

> "In any case, compile-time environments should be avoided. Whenever possible, reading the application environment at runtime should be the first choice."

### Where values come from: .app files and runtime.exs

Application env has two sources:

1. **The `.app` resource file** is built from `def application do [env: [...]] end` in `mix.exs`. From [Application](https://hexdocs.pm/elixir/Application.html):

> "Mix will use this configuration to create an application resource file, which is a file called `APP_NAME.app`."

2. **`config/runtime.exs`** runs before applications start and its `config` calls override `.app` env at boot. See the **config.exs vs runtime.exs** section above.

`Application.get_all_env/1` returns the entire environment for an application as a keyword list:

```elixir
Application.get_all_env(:my_app)
#=> [db_host: "db.local", http: [timeout: 5000]]
```

`Application.get_application/1` takes a module and returns its owning application (or `nil`). There is no `get_application/0`:

```elixir
Application.get_application(MyApp.DBClient)
#=> :my_app
```

### Decision rule: module body vs function body

A clear decision table prevents the most common environment mistakes:

| Location | Use | Why |
|---|---|---|
| Module body (module attributes) | `Application.compile_env/3` | Value is frozen at compile time; enables compile_env validation |
| Function body, required key | `Application.fetch_env!/2` | Fails loudly if misconfigured |
| Function body, optional key | `Application.get_env/3` with default, or `fetch_env/2` + pattern match | Graceful fallback |

Since Elixir 1.14, calling `Application.get_env/3` or `Application.fetch_env!/2` in a module body emits a compiler warning. From [compatibility-and-deprecations](https://hexdocs.pm/elixir/compatibility-and-deprecations.html):

> "Application.fetch_env!/2 is discouraged in the module body, use Application.compile_env/3 instead"

Use `compile_env` for module attributes.

### Libraries and global state

Library authors should not use application env as their primary configuration mechanism. From [design-anti-patterns](https://hexdocs.pm/elixir/design-anti-patterns.html):

> "Library authors should avoid using the application environment to configure their library. The reason is exactly that the application environment is a global state..."

Pass configuration explicitly through `start_link/1` options or function arguments. Application env is appropriate for the end application, not for libraries that may be used with different settings by different consumers.

### Releases and `:validate_compile_env`

Releases compare runtime config against compile-time values by default. The `:validate_compile_env` option defaults to `true`. From [Mix.Tasks.Release](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "If `true`, the release will validate the application environment at runtime against the values set at compile time."

If you used `Application.compile_env/3` to freeze `:db_host` as `"db.local"` and `config/runtime.exs` later sets it to `"db.prod"`, the release fails to boot. Do not disable `:validate_compile_env` to hide such mismatches; fix the configuration design instead.

### Common mistakes

- Using `Application.fetch_env!/2` in a module body instead of `Application.compile_env/3`.
- Expecting `put_env/4` values to survive application reload without `:persistent: true`.
- Reading or writing another application's env (cross-app coupling).
- Believing `Application.get_application/0` or `Application.delete_env!/3` exist.
- Calling `put_env` on a key that was read with `compile_env` at compile time.
- Disabling `:validate_compile_env` to silence a legitimate compile-time/runtime mismatch.

## System Module

The `System` module provides the Elixir interface to the operating system and the Erlang VM: environment variables, OS commands, system time, process control, and VM introspection. It is the right place to read deployment-specific values at runtime.

From [System](https://hexdocs.pm/elixir/System.html):

> "Convenience functions for getting and setting time, environment variables, and reading system information."

### Environment variables: get_env, fetch_env, put_env, delete_env

All `System` environment-variable functions use **binary strings** for both keys and values, not charlists.

`System.get_env/0` returns the entire OS environment as a string-keyed map:

```elixir
System.get_env()
#=> %{"HOME" => "/home/user", "PATH" => "/usr/bin"}
```

`System.get_env/1` returns a single value or `nil`:

```elixir
System.get_env("HOME")
#=> "/home/user"
```

`System.get_env/2` returns the value or a default:

```elixir
System.get_env("PORT", "4000")
#=> "4000"
```

Critical caveat: the default must be a binary string or `nil`. A non-string default raises `FunctionClauseError`:

```elixir
System.get_env("PORT", 4000)
#=> ** (FunctionClauseError) no function clause matching in System.get_env/2
```

It does not return the default verbatim regardless of type. This is a common misconception.

`System.fetch_env/1` and `System.fetch_env!/1` were added in Elixir 1.9. The non-bang variant returns `{:ok, value}` or `:error`; the bang variant raises `System.EnvError`:

```elixir
System.fetch_env!("MISSING")
#=> ** (System.EnvError) could not fetch environment variable "MISSING" because it is not set
```

`System.EnvError` is a dedicated exception:

```elixir
%System.EnvError{env: "MISSING"}
#=> %System.EnvError{env: "MISSING"}
```

In very old Elixir versions (around 1.12) the bang variant raised `ArgumentError`; since then it raises `System.EnvError`.

`System.put_env/2` and `System.put_env/1` mutate the **entire BEAM/OS process environment**, not just the current Elixir process:

```elixir
System.put_env("MY_VAR", "value")
System.put_env(%{"A" => "1", "B" => "2"})
```

`put_env/2` raises `ArgumentError` if the variable name contains `=`.

`System.delete_env/1` removes a variable by wrapping `:os.unsetenv/1`:

```elixir
System.delete_env("MY_VAR")
```

### Subprocesses: cmd/3

`System.cmd/3` runs an external program through a Port:

```elixir
System.cmd("echo", ["hello"])
#=> {"hello\n", 0}
```

The signature is `cmd(command, args, opts) :: {output, exit_status}`. Both `command` and `args` must be binaries; otherwise `ArgumentError` is raised. It raises on a missing command with `:enoent` rather than returning an error tuple.

Useful options:

| Option | Meaning |
|---|---|
| `:into` | Collect output into this collectable (default `""`) |
| `:lines` | Return output line-by-line (since 1.15) |
| `:cd` | Working directory for the command |
| `:arg0` | Override `argv[0]` |
| `:stderr_to_stdout` | Merge stderr into stdout |
| `:use_stdio` | Connect stdio (default `true`) |
| `:parallelism` | Use scheduler parallelism (default `false`) |
| `:env` | List of `{"VAR", "value"}` binary tuples |

Important: the `:env` option takes binary tuples, not charlists. Elixir converts internally, but you should pass binaries:

```elixir
System.cmd("env", [], env: [{"MY_VAR", "value"}])
```

For untrusted input, prefer `System.cmd/3` with an explicit argument list over a shell. Shells are injection vectors; an explicit argument list is not.

### Identifying the runtime: build_info, version, and otp_release

`System.build_info/0` returns exactly five string keys:

```elixir
System.build_info()
#=> %{
#=>   build: "1.20.2 (compiled with Erlang/OTP 27)",
#=>   date: "2024-...",
#=>   revision: "abc123",
#=>   version: "1.20.2",
#=>   otp_release: "27"
#=> }
```

The exact keys are `:build`, `:date`, `:revision`, `:version`, and `:otp_release`. There is no `:elixir`, `:erlang`, or `:options` key. Do not look for them.

`System.version/0` returns the Elixir version string. There is no `System.elixir_version/0`:

```elixir
System.version()
#=> "1.20.2"
```

`System.otp_release/0` (since 1.3) returns the Erlang/OTP release string:

```elixir
System.otp_release()
#=> "27"
```

`System.get_pid/0` is deprecated; it returns the OS PID as a string. Prefer `System.pid/0` (since 1.9):

```elixir
System.pid()
#=> "12345"
```

`System.argv/0` returns the list of command-line arguments passed to the VM; `System.argv/1` replaces it (useful when spawning scripts):

```elixir
System.argv()
#=> ["--port", "4000"]
```

`System.endianness/0` returns the runtime byte order (`:little` or `:big`), while `System.compiled_endianness/0` returns the byte order at compile time. There is no `System.endianness/1`:

```elixir
System.endianness()
#=> :little
```

### Time functions (and which to choose)

`System` exposes three different clocks. Picking the wrong one is a common source of benchmark and timestamp bugs.

| Function | Guarantees | Use for |
|---|---|---|
| `System.monotonic_time/0,1` | Never goes backwards; arbitrary epoch | Durations, benchmarks, timeouts |
| `System.os_time/0,1` | Raw OS wall clock; can jump (NTP, DST) | Wall-clock timestamps with `DateTime` |
| `System.system_time/0,1` | VM-adjusted wall clock; can go backwards in time warps | Wall-clock timestamps when you want the BEAM view |

`System.monotonic_time/1` accepts `:second`, `:millisecond`, `:microsecond`, `:nanosecond`, or a positive integer. It is monotonic but "not strictly monotonically increasing" — equal values are allowed.

From [System](https://hexdocs.pm/elixir/System.html):

> "The current time can be adjusted forwards or backwards in time... Therefore, if you want to keep track of time differences, use `monotonic_time/0` instead."

`System.time_unit/0` is a type (`@type time_unit()`), not a function. Use `System.convert_time_unit/3` to convert between units:

```elixir
System.convert_time_unit(1_000_000_000, :nanosecond, :millisecond)
#=> 1000
```

`System.unique_integer/0,1` returns a unique integer across the BEAM lifetime. Modifiers `:positive` and `:monotonic` can be combined:

```elixir
System.unique_integer([:positive, :monotonic])
```

`:monotonic` guarantees strictly increasing values even across processes.

`System.schedulers/0` returns the configured number of scheduler threads; `System.schedulers_online/0` (both since 1.3) returns the number currently online and available to this BEAM.

### Signals and process control

`System.trap_signal/3` and `System.untrap_signal/2` (since 1.12) register callbacks for OS signals:

```elixir
System.trap_signal(:sigterm, fn -> IO.puts("graceful shutdown") end)
```

The signal type is limited to these atoms:

```elixir
:sigabrt, :sigalrm, :sigchld, :sighup, :sigquit, :sigstop, :sigterm,
:sigtstp, :sigusr1, :sigusr2
```

There is no `:sigint`. Ctrl+C is not trappable through this API.

`trap_signal/3` returns `{:ok, id}` or `{:error, :already_registered | :not_sup}`. The default VM traps are `:sigstop` (graceful stop), `:sigquit` (halt), and `:sigusr1` (halt with status 1). Multiple traps for the same signal compose by prepending: later traps run first, all from a single process.

From [System](https://hexdocs.pm/elixir/System.html):

> "Avoid setting traps in libraries. ... extremely discouraged for libraries to set their own traps."

Use traps only in applications, scripts, or deployment tooling.

`System.stop/1` (since 1.5) performs a graceful shutdown: all applications are taken down smoothly, code is unloaded, and ports are closed before the system terminates. It returns `:ok` and is asynchronous. A binary argument causes a crash dump and exits with status 1.

`System.halt/1` terminates immediately with no graceful shutdown. It accepts an integer exit code, `:abort` for a core dump, or a binary for a crash dump plus status 1. Its return type is `no_return`.

`System.no_halt/0` and `System.no_halt/1` (since 1.9) prevent the VM from halting when the main script finishes, which is useful for long-running scripts.

### Stacktrace

`System.stacktrace/0` is still present in Elixir v1.20.2 but is deprecated. It was soft-deprecated in 1.7 when `__STACKTRACE__` was introduced and hard-deprecated in 1.12. It always returns `[]` in current Elixir.

```elixir
System.stacktrace()
#=> []
```

It was never removed. Inside `rescue` or `catch`, use `__STACKTRACE__` instead. This is a frequent misconception.

### Common mistakes

- Using `System.os_time/1` for durations or benchmarks (use `monotonic_time`).
- Believing `System.stacktrace/0` was removed; it is deprecated and returns `[]`.
- Passing a non-string default to `System.get_env/2`, causing `FunctionClauseError`.
- Passing charlists to `System.cmd/3`'s `:env` option; use binary tuples.
- Trapping signals inside a library.
- Expecting `System.build_info/0` to contain `:elixir` or `:erlang` keys.
- Using the deprecated `System.get_pid/0` instead of `System.pid/0`.

## IO Module

The `IO` module is Elixir's interface to input/output devices. It handles UTF-8 chardata, raw bytes, ANSI escape sequences, and the common debug helper `IO.inspect/2`. For production OTP code, `Logger` is almost always the better choice.

From [IO](https://hexdocs.pm/elixir/IO.html):

> "Functions to handle input/output (IO)."

### Writing and inspecting

`IO.puts/2` writes chardata followed by a newline and returns `:ok`:

```elixir
IO.puts("hello")
#=> hello
#=> :ok
```

The default device is `:stdio`; the signature is `puts(device \\ :stdio, item)`.

`IO.write/2` is the same but without the trailing newline:

```elixir
IO.write("hello")
#=> hello:ok
```

`IO.inspect/2` is the debug tap: it prints and returns the original value unchanged, so it fits in pipelines:

```elixir
1..100
|> IO.inspect(label: "before")
|> Enum.take(3)
|> IO.inspect(label: "after")
#=> before: 1..100
#=> after: [1, 2, 3]
```

The signature is `inspect(item, opts \\ []) :: item`; `IO.inspect/3` takes an explicit device as the first argument. By default it enables pretty printing with width 80.

`IO.inspect` options come from `Inspect.Opts`. Important options:

| Option | Default | Meaning |
|---|---|---|
| `:label` | nil | Prefix label for the output |
| `:limit` | 200 (since 1.20.0; was 50) | Max items to print |
| `:printable_limit` | 4096 | Max printable code points |
| `:width` | 80 | Line width; 0 forces one item per line |
| `:pretty` | false | Pretty printing; true in `IO.inspect` |
| `:base` | :decimal | :decimal, :binary, :octal, :hex |
| `:structs` | true | Print structs as structs |
| `:binaries` | :infer | How to print binaries |
| `:charlists` | :infer | How to print charlists |
| `:safe` | true | When false, inspect failures raise instead of being wrapped in `Inspect.Error` |
| `:syntax_colors` | from `IO.ANSI.syntax_colors/0` | Color mapping |
| `:custom_options` | [] | Since 1.9 |
| `:inspect_fun` | `Inspect.Opts.default_inspect_fun()` | Since 1.9 |

Important naming correction: the inspect color option is `:syntax_colors`, not `:colors`. `Logger.Formatter` uses `:colors`; `IO.inspect` and `Inspect.Opts` use `:syntax_colors`.

```elixir
IO.inspect(1..100, label: "a wonderful range")
#=> a wonderful range: 1..100
```

### Reading: gets, getn, read

`IO.gets/2` reads a line and returns the data, `:eof`, or `{:error, reason}`:

```elixir
IO.gets("Name: ")
#=> "Alice\n"
```

`IO.getn/2` and `IO.getn/3` read a count of bytes (or Unicode codepoints on Unicode devices). The default count is 1. There is no "until delimiter" mode; for line-delimited input, use `gets`.

`IO.read/2` reads in `:line` mode, an integer byte count, or `:eof` mode (since 1.13). Integer mode is preferred for non-textual input:

```elixir
IO.read(:stdio, :line)
IO.read(:stdio, 1024)
```

All three can return `:eof` or `{:error, reason}` on failure.

### Binary (raw-byte) IO

`IO.binread/2`, `IO.binwrite/2`, and `IO.binstream/2` operate on raw bytes / `iodata`. They bypass chardata conversion and are Unicode-unsafe.

From [IO](https://hexdocs.pm/elixir/IO.html):

> "do not use this function on IO devices in Unicode mode."

Because `:stdio` is Unicode by default, do not use `IO.binwrite/2` on `:stdio`. Use `IO.write/2` for UTF-8 text and `IO.binwrite/2` only for binary devices or raw byte streams.

`IO.stream/2` and `IO.stream/0` (since 1.12) operate on UTF-8 characters; `IO.binstream/2` operates on raw bytes.

### IO.chardata vs IO.iodata

This distinction is the source of many encoding bugs.

`IO.iodata/0` is a binary or list of bytes (0..255) or nested iodata. `iolist/0` is the same but cannot be a top-level binary. In iodata, integers are raw bytes.

`IO.chardata/0` is a string or list of Unicode code points (0..0x10FFFF) or nested chardata. In chardata, integers are Unicode code points.

From [IO](https://hexdocs.pm/elixir/IO.html):

> "the only difference is that integers in IO data represent bytes while integers in chardata represent Unicode code points."

The practical gotcha:

```elixir
# Raises: π is not a byte
IO.iodata_to_binary(["The symbol for pi is: ", ?π])
#=> ** (ArgumentError) argument error

# Works: π is a valid code point
IO.chardata_to_string(["The symbol for pi is: ", ?π])
#=> "The symbol for pi is: π"
```

`IO.iodata_to_binary/1` is Unicode-unsafe; it treats integers as raw bytes. Use `IO.chardata_to_string/1` for text.

Other helpers:

| Function | Purpose |
|---|---|
| `IO.iodata_length/1` | Byte length of iodata |
| `IO.iodata_to_binary/1` | Convert iodata to binary |
| `IO.iodata_empty?/1` | Check emptiness (since 1.20.0) |
| `IO.chardata_to_string/1` | Convert chardata to string |

### IO devices and ANSI

`:stdio` is a shortcut for Erlang's `:standard_io`, which maps to the current process group leader. `:stderr` is a shortcut for `:standard_error`. The `device()` type is `atom() | pid()`. `StringIO` is a registered process useful in tests.

From [IO](https://hexdocs.pm/elixir/IO.html) ("IO devices"):

> "`:stdio` is a shortcut for Erlang's `:standard_io`. It maps to `Process.group_leader/0`. `:stderr` is a shortcut for `:standard_error`."

`IO.ANSI` provides ANSI escape sequences. `IO.ANSI.enabled?/0` reports whether ANSI output is enabled:

```elixir
IO.ANSI.enabled?()
#=> false
```

It reads `:ansi_enabled` from the `:elixir` application. The default is `false` unless both stdout and stderr are detected as terminals. To force colors, configure:

```elixir
config :elixir, :ansi_enabled, true
```

Important correction: there is no `IO.ANSI.enable/0`. Only `enabled?/0` exists.

`IO.ANSI` color functions are arity-0 only. Each returns a string escape sequence:

```elixir
IO.ANSI.red()
#=> "\e[31m"
```

There is no `IO.ANSI.red/1` that wraps and resets text. To wrap text manually:

```elixir
IO.ANSI.blue_background() <> "Example" <> IO.ANSI.reset()
```

`IO.ANSI.format/1` and `IO.ANSI.format/2` convert named atoms to actual codes and append `reset/0` after each conversion:

```elixir
IO.ANSI.format(["Hello, ", :red, :bright, "world!"], true)
#=> "Hello, \e[31m\e[1mworld!\e[0m"
```

The second argument is a plain boolean, not `:enabled`, `:disabled`, or `:auto`. The arity-1 form defaults to `IO.ANSI.enabled?/0`. Use `IO.ANSI.format_fragment/2` to avoid the trailing reset.

`IO.ANSI.syntax_colors/0` (since 1.14) returns the default color mapping:

```elixir
IO.ANSI.syntax_colors()
#=> [atom: :cyan, boolean: :magenta, charlist: :yellow, nil: :magenta, number: :yellow, string: :green]
```

It is overridable via `config :elixir, :ansi_syntax_colors, ...`. The old name `default_colors/0` does not exist.

Cursor control functions include `cursor/2`, `cursor_up/1`, `cursor_down/1`, `cursor_left/1`, `cursor_right/1`, `home/0`, `clear/0`, and `clear_line/0`. Line 0 and column 0 refer to the top-left corner.

### Why production code should prefer Logger over IO

`IO.puts` and `IO.inspect` are fine for scripts and one-off debugging, but OTP applications should use `Logger`:

| Capability | Logger | IO |
|---|---|---|
| Levels | 8 levels + `:all`/`:none` | None |
| Filtering | Per-level, per-module, per-process, per-application | None |
| Blocking | Async with backpressure | Synchronous |
| Overload protection | `sync_threshold`, `discard_threshold` | None |
| Structured reports | Metadata + reports | Plain text |
| Multiple outputs | Multiple handlers | One device |
| Compile-time purging | `:compile_time_purge_matching` | Always emitted |
| Lazy evaluation | `Logger.debug(expensive())` skipped if level too high | Always evaluates |

The last point matters for expensive debug expressions. With `Logger.debug/1`, the argument is not evaluated if the level is above `:debug`. With `IO.puts`, it is always evaluated. For telemetry and production traces, use `Logger.debug` or `dbg`; reserve `IO.inspect` for local debugging.

### Common mistakes

- Using `IO.binwrite/2` on `:stdio` (Unicode mode).
- Confusing `IO.iodata` bytes with `IO.chardata` codepoints.
- Calling `IO.iodata_to_binary/1` on chardata containing non-ASCII codepoints.
- Believing `IO.ANSI.red/1` or `IO.ANSI.enable/0` exist.
- Passing `:colors` instead of `:syntax_colors` to `IO.inspect`.
- Relying on `IO.puts` in production instead of `Logger`.
- Passing `:enabled` or `:auto` to `IO.ANSI.format/2`; it expects a boolean.

## Review checklist

- [ ] Level macros (`Logger.debug/2`, `Logger.info/2`, `Logger.warning/2`, `Logger.error/2`, etc.) are used instead of `Logger.log/3`.
- [ ] `Logger.warn/2` is not used; `Logger.warning/2` is used instead.
- [ ] Logger configuration uses `:default_handler` and `:default_formatter` for Elixir 1.15+, not the legacy `:console` backend.
- [ ] The legacy `Logger.Backends.Console` and `Logger.add_backend/2` API are avoided unless a repo policy explicitly requires legacy compat.
- [ ] Compile-time purging uses the correct option name `:compile_time_purge_matching`, not `:compile_time_purge_level`.
- [ ] The default truncation value is stated as 8192 bytes, not 8088.
- [ ] Metadata is treated as process-scoped (process dictionary), not global.
- [ ] Global primary metadata (`config :logger, metadata: ...`) is distinguished from formatter metadata (`config :logger, :default_formatter, metadata: ...`).
- [ ] Custom formatters handle errors internally and never raise.
- [ ] `Logger.flush/0` is used only in tests or startup/teardown, not in production hot paths.

## Implementation checklist

- [ ] Require `Logger` at the top of modules that log.
- [ ] Choose the minimum log level per environment (`:debug` in dev, `:info`/`:warning` in prod, `:warning` in test).
- [ ] Configure `:default_handler` for level and `:default_formatter` for format/colors/metadata/truncate.
- [ ] Add shared context metadata (e.g. `request_id`, `user_id`) near the entry point of each request/job/process.
- [ ] Use `compile_time_purge_matching` to strip noisy debug logs from release builds, with awareness that dependencies must be recompiled.
- [ ] Add custom Erlang handlers via `config :my_app, :logger, [...]` plus `Logger.add_handlers(:my_app)` in `Application.start/2`.
- [ ] Set `:truncate` intentionally; use `:infinity` only when you have another size limit downstream.
- [ ] Document repo decisions on legacy backend compatibility, structured logging, and log level policy.

## Validation hooks

- `mix format --check-formatted` — catches formatting issues in `.ex` and `.exs` files.
- `mix compile --force` — deprecation warnings for `Logger.warn/2`, `Logger.add_backend/2`, `Logger.Backends.Console`, and similar legacy APIs surface at compile time.
- `mix test` — can assert on logs with `ExUnit.CaptureLog` or verify `Logger.flush/0` drains queued messages.
- `:logger` application environment can be inspected at runtime: `Application.get_env(:logger, :default_formatter)`.
- There is no compiler enforcement that you pick the "right" log level or avoid `Logger.log/3`; those are conventions and performance choices.

## Examples

### Custom formatter module

```elixir
defmodule MyApp.JsonFormatter do
  @spec format(atom, IO.chardata(), Logger.Formatter.date_time_ms(), keyword()) :: IO.chardata()
  def format(level, message, timestamp, metadata) do
    # Keep the formatter crash-free; rescue and return a fallback line.
    try do
      event = %{
        level: level,
        time: format_time(timestamp),
        message: IO.iodata_to_binary(message),
        metadata: Enum.into(metadata, %{})
      }

      [Jason.encode!(event), "\n"]
    rescue
      _ -> "[#{level}] #{IO.iodata_to_binary(message)}\n"
    end
  end

  defp format_time({date, {hour, minute, second, millisecond}}) do
    # Build an ISO-8601-ish timestamp.
    date_part = Calendar.ISO.date_to_string(date)
    time_part = :io_lib.format("~2..0B:~2..0B:~2..0B.~3..0B", [hour, minute, second, millisecond])
    "#{date_part} #{time_part}"
  end
end

# config/config.exs
config :logger, :default_formatter,
  format: {MyApp.JsonFormatter, :format}
```

### Metadata in a Phoenix-style request context

```elixir
defmodule MyAppWeb.Plugs.RequestLogger do
  require Logger

  def init(opts), do: opts

  def call(conn, _opts) do
    request_id = List.first(Plug.Conn.get_req_header(conn, "x-request-id")) || generate_id()

    Logger.metadata(
      request_id: request_id,
      user_id: conn.assigns[:current_user_id],
      remote_ip: format_ip(conn.remote_ip)
    )

    conn
    |> Plug.Conn.put_resp_header("x-request-id", request_id)
    |> register_after_send(fn _conn -> Logger.reset_metadata() end)
  end

  defp generate_id, do: Base.encode16(:crypto.strong_rand_bytes(8), case: :lower)
  defp format_ip(ip), do: ip |> :inet.ntoa() |> to_string()
  defp register_after_send(conn, fun), do: Plug.Conn.register_before_send(conn, fun)
end
```

### Runtime level bump in IEx

```elixir
# In an iex -S mix session:
Logger.level()              #=> :info
Logger.configure(level: :debug)
Logger.debug("now visible")

# Per-process suppression:
Logger.put_process_level(self(), :error)
Logger.info("hidden in this process")
Logger.delete_process_level(self())
```

### Per-module debugging

```elixir
# Enable debug logs only from one module while the rest of the system stays at :warning.
Logger.put_module_level(MyApp.Scheduler, :debug)
# ... exercise the scheduler ...
Logger.delete_module_level(MyApp.Scheduler)
```

## Common mistakes

- Using `Logger.warn/2` instead of `Logger.warning/2` (`warn` is deprecated).
- Using `Logger.log/3` for a static level instead of the level-specific macro, losing compile-time purge benefits.
- Writing `:compile_time_purge_level` instead of `:compile_time_purge_matching`.
- Believing the default truncation is 8088 bytes; it is 8192 bytes.
- Treating `Logger.metadata/0` as global metadata rather than process-scoped metadata from the process dictionary.
- Setting `:metadata` under `:default_formatter` and expecting it to add keys to the event; it only controls what `$metadata` prints.
- Forgetting that purging logs from a dependency requires recompiling that dependency.
- Raising exceptions inside a custom `{module, function}` formatter, which can crash the logger handler.
- Calling `Logger.flush/0` in production hot paths.
- Keeping `config :logger, :console, ...` on Elixir 1.15+ without understanding the `:default_handler`/`:default_formatter` split.

## Strict vs contextual guidance

### Strict

- `Logger.warn/2` is deprecated; use `Logger.warning/2`. The compiler emits a deprecation warning.
- `Logger.Backends.Console` is deprecated; new code should use `:default_handler`/`:default_formatter` or the `:logger_backends` library.
- `Logger.add_backend/2`, `Logger.remove_backend/2`, and `Logger.configure_backend/2` are deprecated.
- `Logger.disable/1`, `Logger.enable/1`, and `Logger.enabled?/1` are deprecated.
- The compile-time purge option is `:compile_time_purge_matching`; other spellings are not recognized.
- Module/alias names must start uppercase; function names must start lowercase or underscore. These are parser rules, not Logger-specific.

### Conventions (not enforced by the compiler)

- Prefer level-specific macros over `Logger.log/3`.
- Keep log levels per environment consistent: `:debug` in dev, `:info`/`:warning` in prod, `:warning` in test.
- Use process metadata for request/job context, not module attributes or global variables.
- Treat `Logger.flush/0` as a test/startup helper, not a production tool.
- Use `compile_time_purge_matching` to remove noisy logs from release builds.
- Reserve `IO.puts` and `IO.inspect` for scripts and one-off debugging; production code should use `Logger`.

### Contextual tradeoffs

- Structured JSON logging requires a custom formatter and increases dependency surface (e.g. `Jason`). Choose it when log aggregation requires it.
- Legacy `:console` backend compatibility is acceptable if the repo must support Elixir < 1.15, but document that decision and plan migration.
- `:infinity` truncation is fine for small-scale scripts; production services should keep a finite limit or enforce one downstream.
- Per-module and per-process levels are powerful debugging tools but can surprise operators if left enabled in production; reset them explicitly.

## Policy decisions for individual repos

- Minimum Elixir version targeted (determines whether `:default_handler`/`:default_formatter` or legacy `:console` config is appropriate).
- Whether to retain legacy `Logger.Backends.Console` compatibility for older Elixir versions.
- Default log level per environment and whether to allow runtime override via environment variables.
- Whether to adopt structured JSON logging versus plain text logs.
- Which metadata keys are required (`request_id`, `user_id`, `trace_id`, etc.) and where they are injected.
- Whether to use `compile_time_purge_matching` in release builds and how to force dependency recompilation in CI.
- Whether to enable colors in production logs or restrict them to dev/test.
- Whether `mix format --check-formatted` and compile-time deprecation checks should block CI.

## Related docs

- Sibling Elixir corpus doc: `docs/elixir/naming-conventions.md`.
- Related Elixir corpus docs: `docs/elixir/language-fundamentals.md`, `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.

## Related BEAM guidance

- `../beam/logger-and-config.md` — OTP Logger handler/filter/formatter model and `sys.config` semantics that Elixir's `Config`/`Logger` build on.
- `../beam/applications.md` — OTP application environment and start phases that `Application.get_env/put_env` and `config/runtime.exs` resolve to.
- `../beam/runtime-environment.md` — VM startup, `vm.args`, and runtime environment behavior that `config/runtime.exs` and `Config.Provider` hook into.

## Related skills

- None defined yet.
