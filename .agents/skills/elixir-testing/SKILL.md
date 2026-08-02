---
name: elixir-testing
description: |
  Operational guide for writing and running Elixir tests with ExUnit — assertions,
  callbacks, doctests, capture helpers, tags, async rules, mix test flags, coverage.
  Load when writing or reviewing tests or configuring test runs. Does NOT cover
  property-based testing (StreamData is a separate library) or Ecto sandbox setup.
---

# Testing with ExUnit

## Triggers

Load this skill when:

- Writing or reviewing tests.
- Configuring ExUnit.
- Using assertions, callbacks, capture helpers, doctests, or tags.
- Running `mix test` or setting up coverage.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/testing-exunit.md`
  - https://hexdocs.pm/ex_unit/ExUnit.html
  - https://hexdocs.pm/ex_unit/ExUnit.Case.html
  - https://hexdocs.pm/ex_unit/ExUnit.Assertions.html
  - https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html
  - https://hexdocs.pm/ex_unit/ExUnit.DocTest.html
  - https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html
  - https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html
  - https://hexdocs.pm/ex_unit/ExUnit.Formatter.html
  - https://hexdocs.pm/mix/Mix.Tasks.Test.html
  - https://hexdocs.pm/mix/Mix.Tasks.Test.Coverage.html
  - https://hexdocs.pm/elixir/Kernel.html#match?/2
- `docs/beam/validation.md` — shared BEAM runtime validation hooks (sys, trace, introspection, ETS/timer/NIF checks).

## Key Rules

- Start ExUnit in `test/test_helper.exs` with `ExUnit.start()`; configure via
  `ExUnit.configure/1` or options to `ExUnit.start/1`.
- Test modules: `use ExUnit.Case`; test files match `test/**/*_test.exs`.
- `async: true` (default `false`) — only safe when tests share no mutable state
  (no shared ETS, no DB without sandbox). Set per-module.
- Assertions: `assert`, `refute`, `assert_in_delta/3` (floats), `assert_raise/2`
  (exceptions), `assert_receive/3` & `assert_received/1` (messages, with
  timeout), `refute_receive`, `catch_exit/1`, `catch_error/1`, `catch_throw/1`,
  `flunk/1`.
- `match?/2` for pattern-based assertions: `assert match?({:ok, _}, result)`.
- Callbacks: `setup` (per-test), `setup_all` (per-module), `on_exit/1`
  (cleanup, runs after test even on failure). `start_supervised!/1` starts a
  process under ExUnit's supervisor (auto-stopped per test).
- Doctests: `doctest Module` in a test module; runs examples in
  `@doc`/`@moduledoc`; options `:only`/`:except`/`:import`/`:async`. Use `...>`
  for multiline continuation.
- Capture helpers: `ExUnit.CaptureIO.capture_io/1,2` (stdout/stderr),
  `ExUnit.CaptureLog.capture_log/2` (Logger output, with level filtering).
- Tags: `@tag :external`, `@tag timeout: 60_000`, `@moduletag`; `describe`
  blocks group tests; `:registered` tag for shared state.
- `mix test` flags: `--only tag`, `--exclude tag`, `--seed 0` (deterministic),
  `--trace`, `--cover`, `--failed` (rerun failures), `--stale`,
  `--max-failures N`, `--warnings-as-errors`, file:line filter
  (`mix test path/file.exs:42`).
- Coverage: `mix test --cover`; configure threshold in `test/test_helper.exs`
  or `project/0` (`test_coverage: [threshold: 90]`); `mix test.coverage` for
  reporting.
- `:capture_log` global option captures Logger output per-test (default false;
  enable in `ExUnit.start`).
- `:exclude` default tags (e.g. `:external`, `:integration`) set in
  `test_helper.exs`.

## Quick Commands

```bash
mix test                                  # run all tests
mix test --only pending                   # run tagged tests
mix test --seed 0                         # deterministic order
mix test --trace                          # verbose, show all
mix test --cover                          # coverage report
mix test path/to/file_test.exs:42         # run single test at line
mix test --warnings-as-errors             # treat compile warnings as errors
```

## Anti-patterns

- `async: true` with shared mutable state (DB without sandbox, shared ETS).
- `assert` on equality of floats without `assert_in_delta`.
- Not handling `{:error,_}` in `assert_receive` (use explicit pattern).
- Relying on test ordering (random by default; use `--seed 0` for
  reproducibility).
- `rescue _` in tests instead of `assert_raise`.
- Forgetting `on_exit` cleanup for started processes (use
  `start_supervised!`).
- Doctests with non-reproducible output (timestamps, random).

## Related Skills

- elixir-coding
- elixir-static-analysis
- elixir-docs-publishing
