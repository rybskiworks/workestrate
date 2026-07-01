---
name: elixir-config
description: |
  Operational guide for Elixir application configuration — config.exs vs runtime.exs,
  Application env patterns, Logger configuration, System module usage. Load when
  configuring applications, managing runtime config, or setting up Logger. Does NOT
  cover project structure (see elixir-project-setup) or OTP supervision.
---

# Configuration and Runtime

## Triggers

Load this skill when:

- Writing `config/config.exs` or `config/runtime.exs`.
- Reading application environment.
- Configuring Logger.
- Using `System` for env vars/signals.
- Configuring releases with `Config.Provider`.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/configuration-and-runtime.md`
  - https://hexdocs.pm/logger/Logger.html
  - https://hexdocs.pm/logger/Logger.Formatter.html
  - https://hexdocs.pm/logger/Logger.Backends.Console.html
  - https://hexdocs.pm/elixir/Config.html
  - https://hexdocs.pm/elixir/Config.Provider.html
  - https://hexdocs.pm/elixir/Config.Reader.html
  - https://hexdocs.pm/elixir/Application.html
  - https://hexdocs.pm/elixir/System.html
  - https://hexdocs.pm/elixir/System.EnvError.html
  - https://hexdocs.pm/elixir/IO.html
  - https://hexdocs.pm/elixir/IO.ANSI.html
  - https://hexdocs.pm/elixir/Inspect.Opts.html
  - https://hexdocs.pm/mix/Mix.Tasks.Release.html
  - https://hexdocs.pm/elixir/design-anti-patterns.html
  - https://hexdocs.pm/elixir/compatibility-and-deprecations.html
- `docs/beam/logger-and-config.md` — OTP Logger architecture, levels, handlers, filters, `config(4)` format.
- `docs/beam/applications.md` — `.app` resource file, application callback, env precedence, start types.
- `docs/beam/runtime-environment.md` — `file`/`os` modules, env vars, `cmd/1,2` caveats, OS time.

## Key Rules

- `config/config.exs` (compile-time, `use Mix.Config` replaced by
  `import Config` since Elixir 1.11) — runs at compile, baked into release. Use
  for static config.
- `config/runtime.exs` (since Elixir 1.11) — runs at boot, evaluated by
  `Config.Provider` in releases. Use for env-var-dependent / secret config.
  NOT available in `config.exs`.
- `Config` DSL: `config :app, key: value`, `config :app, Key, value`,
  `import_config "other.exs"`.
- Application env: `Application.get_env/3` (returns default if absent),
  `Application.fetch_env!/2` (raises `System.EnvError` if absent),
  `Application.get_all_env/1`, `Application.compile_env/3` (compile-time read,
  validated at boot via `:validate_compile_env`).
- `Application.compile_env/3` for values read at compile time (prevents runtime
  changes from silently affecting compiled code). `Application.get_env/3` in
  module body is deprecated.
- `System.get_env/1` (returns `nil` if absent), `System.get_env/1, default`,
  `System.fetch_env!/1` (raises `System.EnvError`), `System.cmd/3`,
  `System.time/1`, `System.monotonic_time/1`, `System.schedulers_online/0`,
  `System.trap_signal/3`.
- Logger levels (most->least severe): `:emergency > :alert > :critical >
  :error > :warning > :notice > :info > :debug`. Use level macros
  (`Logger.info/2`) over `Logger.log/3` (compile-time elimination).
- Logger config: `:level` (runtime via `Logger.configure/1`),
  `:compile_time_purge_matching` (drop log calls at compile time),
  `:default_handler`/`:default_formatter` (since 1.15), metadata via
  `:metadata`.
- Since Elixir 1.15: Logger uses Erlang `:logger` `:default` handler (not
  `GenEvent` backends). `Logger.Backends.Console` is deprecated -> use
  `:logger_backends` dependency or `:default` handler.
- `Logger.Formatter` tokens: `$time`, `$message`, `$metadata`, `$level`,
  `$node`; config `:format`, `:metadata`, `:colors`, `:truncate`.
- Anti-pattern (from `design-anti-patterns.html`): using
  `Application.get_env` in library module bodies — pass config as function
  arguments instead.
- `Config.Provider` for release-time config injection; `runtime.exs` is the
  simplest provider.
- `IO.puts`/`IO.inspect` for output; chardata/iodata for efficient
  concatenation; `IO.binread`/`IO.binwrite` for raw bytes.

## Quick Commands

```bash
mix run --no-halt                         # start app
MIX_ENV=prod mix release                  # build prod release
_release/bin/app eval "Application.get_env(:app, :key)"
_release/bin/app start                    # start release
```

## Anti-patterns

- `Application.get_env/3` in module body (deprecated; use `compile_env/3` or
  pass as arg).
- Secrets in `config.exs` (use `runtime.exs` + env vars).
- Using `Logger.Backends.Console` (deprecated since 1.15).
- `Logger.log(:info, ...)` instead of `Logger.info/2` (loses compile-time
  elimination).
- Library reading its own `Application.get_env` at compile time without
  `compile_env`.
- Hardcoding env-specific values instead of using `System.fetch_env!/1`.

## Related Skills

- elixir-project-setup
- elixir-otp
- elixir-error-handling
