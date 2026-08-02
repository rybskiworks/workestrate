# Testing with ExUnit

## Purpose

Provide repo-independent guidance for configuring, organizing, and running Elixir tests with ExUnit. This document is a multi-part ExUnit reference; it covers ExUnit startup, configuration, async execution, formatters, seed/ordering, `mix test` integration, `test_helper.exs` patterns, assertions, callbacks, capture helpers, doctests, tags, and `Mix.Tasks.Test`.

## Sources used

- https://hexdocs.pm/ex_unit/ExUnit.html (PRIMARY)
- https://hexdocs.pm/ex_unit/ExUnit.Case.html
- https://hexdocs.pm/ex_unit/ExUnit.Assertions.html
- https://hexdocs.pm/ex_unit/ExUnit.AssertionError.html
- https://hexdocs.pm/elixir/Kernel.html#match?/2
- https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html
- https://hexdocs.pm/ex_unit/ExUnit.DocTest.html
- https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html
- https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html
- https://hexdocs.pm/logger/Logger.html
- https://hexdocs.pm/ex_unit/ExUnit.Formatter.html
- https://hexdocs.pm/mix/Mix.Tasks.Test.html
- https://hexdocs.pm/mix/Mix.Tasks.Test.Coverage.html
- https://hexdocs.pm/ex_unit/ExUnit.html#configure/1
- https://hexdocs.pm/elixir/Module.html#register_attribute/3
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/mix.exs (application env defaults)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/callbacks.ex (ExUnit.Callbacks source)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_io.ex (ExUnit.CaptureIO source)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_log.ex (ExUnit.CaptureLog source)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_server.ex (level-filtering source)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/runner.ex (capture_log tag wiring source)
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/doc_test.ex (ExUnit.DocTest source)
- https://hexdocs.pm/stream_data/ (property-based testing is a separate library)

This page reflects Elixir v1.20.2 docs.

## ExUnit Overview and Configuration

### What ExUnit is

From [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Unit testing framework for Elixir."

ExUnit ships with Elixir as a separate OTP application named `:ex_unit`. You do not add it as a dependency; you start it from `test/test_helper.exs` and write test modules that `use ExUnit.Case`.

From [ExUnit.html "Integration with Mix"](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Mix is the project management and build tool for Elixir. Invoking `mix test` from the command line will run the tests in each file matching the pattern `*_test.exs` found in the `test` directory of your project."

### Starting ExUnit: `ExUnit.start/0` and `ExUnit.start/1`

The only public start function is `ExUnit.start(options \\ [])` — spec `@spec start(configure_opts()) :: :ok`. Call it as `ExUnit.start()` or `ExUnit.start(keyword)`. There is **no public `ExUnit.start/2`**; the module's `start/2` is the private OTP `Application` callback and is marked `@doc false`.

From [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Starts ExUnit and automatically runs tests right before the VM terminates. It accepts a set of `options` to configure ExUnit (the same ones accepted by `configure/1`). If you want to run tests manually, you can set the `:autorun` option to `false` and use `run/0` to run tests."

`ExUnit.start/1` performs three steps:

1. `Application.ensure_all_started(:ex_unit)` — starts the `:ex_unit` OTP application.
2. `configure(options)` — writes each `{k, v}` into the `:ex_unit` application environment.
3. If `:autorun` is `true` (the default), it installs a `System.at_exit/1` hook that runs the suite before the VM terminates, and sets `:autorun` to `false` so repeated calls do not stack hooks.

The minimum `test_helper.exs` is:

```elixir
# test/test_helper.exs
ExUnit.start()
```

From [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Mix will load the `test_helper.exs` file before executing the tests. It is not necessary to `require` the `test_helper.exs` file in your test files."

A more typical helper enables common options:

```elixir
# test/test_helper.exs
ExUnit.start(
  capture_log: true,
  exclude: [external: true],
  formatters: [ExUnit.CLIFormatter, MyApp.TestFormatter]
)
```

### Configuring ExUnit: `ExUnit.configure/1`

`ExUnit.configure(options)` has spec `@spec configure(configure_opts()) :: :ok`. It writes each option into the `:ex_unit` application env via `Application.put_env(:ex_unit, k, v)`.

Convention: configure before starting, or pass options directly to `ExUnit.start/1`. Both approaches write to the same application environment.

```elixir
ExUnit.configure(exclude: [external: true])
ExUnit.start()
```

Equivalent to:

```elixir
ExUnit.start(exclude: [external: true])
```

### Running tests manually

When `autorun: false`, use `ExUnit.run/0` to run tests. `run(additional_modules \\ [])` returns a map with `:total`, `:failures`, `:excluded`, `:skipped`, etc. From v1.14 it accepts an optional list of extra modules.

```elixir
ExUnit.start(autorun: false)
# load or define additional test modules if needed
result = ExUnit.run()
```

For asynchronous execution, `async_run/0` (since v1.12.0) returns a `Task.t()`, and `await_run(task)` returns the result. From the docs: "Starts tests asynchronously while test cases are still loading."

### Configuration options

The full public typespec from [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html) is:

```elixir
@type configure_opts() :: [
  {:assert_receive_timeout, non_neg_integer()}
  | {:autorun, boolean()}
  | {:capture_log, boolean() | [level: Logger.level()]}
  | {:colors, enabled: boolean(), success: atom(), invalid: atom(), skipped: atom(),
     failure: atom(), error_info: atom(), extra_info: atom(),
     location_info: [atom()], diff_insert: atom(), diff_insert_whitespace: IO.ANSI.ansidata(),
     diff_delete: atom(), diff_delete_whitespace: IO.ANSI.ansidata()}
  | {:exclude, atom() | [atom() | {atom(), any()}]}
  | {:exit_status, non_neg_integer()}
  | {:failures_manifest_path, String.t()}
  | {:formatters, [module()]}
  | {:include, atom() | [atom() | {atom(), any()}]}
  | {:max_cases, pos_integer()}
  | {:max_failures, pos_integer() | :infinity}
  | {:only_test_ids, [test_id()]}
  | {:rand_algorithm, atom()}
  | {:refute_receive_timeout, non_neg_integer()}
  | {:seed, non_neg_integer()}
  | {:slowest, non_neg_integer()}
  | {:slowest_modules, non_neg_integer()}
  | {:stacktrace_depth, non_neg_integer()}
  | {:timeout, pos_integer()}
  | {:trace, boolean()}
  | {:test_location_relative_path, String.t()}
  | {atom(), term()}
]
```

Defaults come from the application env block in [`lib/ex_unit/mix.exs`](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/mix.exs):

```elixir
env: [
  # Calculated on demand
  # max_cases: System.schedulers_online * 2,
  # seed: rand(),
  assert_receive_timeout: 100,
  autorun: true,
  capture_log: false,
  colors: [],
  exclude: [],
  exit_status: 2,
  formatters: [ExUnit.CLIFormatter],
  include: [],
  max_failures: :infinity,
  rand_algorithm: :exsss,
  refute_receive_timeout: 100,
  slowest: 0,
  slowest_modules: 0,
  stacktrace_depth: 20,
  timeout: 60_000,
  trace: false,
  after_suite: [],
  repeat_until_failure: 0,
  dry_run: false
]
```

#### Option reference

| Option | Default | Description |
|---|---|---|
| `:assert_receive_timeout` | `100` (ms) | The timeout used by `assert_receive` in milliseconds. |
| `:autorun` | `true` | Whether ExUnit runs automatically on VM exit. Set to `false` to run manually with `ExUnit.run/0`. |
| `:capture_log` | `false` | Whether to capture log messages and print them only on failure. Can be `true`, `false`, or `[level: LEVEL]`. Overridable per test via `@tag capture_log: false`. |
| `:colors` | `[]` | Keyword list of color options. Sub-keys default lazily: `:enabled` → `IO.ANSI.enabled?/0`; `:success` → `:green`; `:invalid` → `:yellow`; `:skipped` → `:yellow`; `:failure` → `:red`; `:error_info` → `:red`; `:extra_info` → `:cyan`; `:location_info` → `[:bright, :black]`; `:diff_insert` → `:green`; `:diff_delete` → `:red`. |
| `:exclude` | `[]` | Tags or `{tag, value}` pairs that skip matching tests by default. |
| `:exit_status` | `2` | Exit status used when the suite fails. |
| `:failures_manifest_path` | unset | Path to a file that stores failures between runs for `--failed`. |
| `:formatters` | `[ExUnit.CLIFormatter]` | Modules that receive ExUnit events and print results. |
| `:include` | `[]` | Tags or `{tag, value}` pairs that keep only matching tests. |
| `:max_cases` | `System.schedulers_online() * 2` | Maximum number of test modules running concurrently. Computed on demand. |
| `:max_failures` | `:infinity` | Stop evaluating tests after this many failures. |
| `:only_test_ids` | unset | List of `{module_name, test_name}` tuples limiting which tests run; typically used by Mix. |
| `:rand_algorithm` | `:exsss` | Algorithm used to generate the test seed. Available since v1.16.0; before that it was hard-coded to `:exs1024`. |
| `:refute_receive_timeout` | `100` (ms) | The timeout used by `refute_receive` in milliseconds. |
| `:seed` | random per run | Integer seed for randomizing test order. A seed of `0` disables randomization per file. |
| `:slowest` | `0` | Print timing for the N slowest tests. Forces trace mode. Disabled by default. |
| `:slowest_modules` | `0` | Print timing for the N slowest test modules. Disabled by default. |
| `:stacktrace_depth` | `20` | Stacktrace depth used in formatting and reporters. |
| `:timeout` | `60_000` (ms) | Default timeout for tests in milliseconds. |
| `:trace` | `false` | Trace mode: sets `:max_cases` to `1`, prints each test, and ignores timeouts. |
| `:test_location_relative_path` | unset | Path prefix used when printing test locations; typically used by Mix for umbrellas. |

Additional internal/convention-only keys visible in the env block include `:after_suite`, `:repeat_until_failure`, and `:dry_run`. The public `configure_opts/0` type ends with `{atom(), term()}` to allow extension, but prefer documented options.

### Async testing

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "`:async` - configures tests in this module to run concurrently with tests in other modules. Tests in the same module never run concurrently (with the exception of tests run via the `:parameterize` option - see below). It should be enabled only if tests do not change any global state. **Defaults to `false`**."

Rules:

- Tests in different modules marked `async: true` run concurrently in separate processes.
- Tests within the same module run serially.
- Async tests MUST NOT change global state (ETS tables not owned by the test, the filesystem, environment variables, global process registration, etc.).
- `:max_cases` is the upper bound on concurrently-running test modules.

`:group` (since v1.18.0) serializes modules in the same group:

> "configures the group this module belongs to. Tests in the same group never run concurrently. Tests from different groups (or with no groups) can run concurrently when `async: true` is given. By default, this module belongs to no group (defaults to `nil`)."

`:parameterize` (since v1.18.0):

> "a list of maps to parameterize tests. If both `:async` and `:parameterize` are given, the different parameters run concurrently."

`:register: false` prevents the module from being registered in the ExUnit server, so it will not run with the suite.

```elixir
defmodule MyApp.CacheTest do
  use ExUnit.Case, async: true

  test "stores and retrieves values" do
    # runs concurrently with other async modules
  end
end
```

### Test type overview

The ExUnit building blocks are covered in later sections. Briefly:

- `ExUnit.Case` — `use ExUnit.Case`, `test/1`, `test/3`, `describe/2`, registered attributes, `register_test/6`. Also imports `ExUnit.Assertions`, `ExUnit.Callbacks`, `ExUnit.DocTest`, and itself.
- `ExUnit.Assertions` — `assert/1,2`, `refute/1,2`, `assert_in_delta/4`, `refute_in_delta/4`, `assert_raise/2,3`, `assert_receive/3`, `assert_received/2`, `refute_receive/3`, `refute_received/2`, `catch_error/1`, `catch_exit/1`, `catch_throw/1`, `flunk/1`.
- `ExUnit.Callbacks` — `setup/1,2`, `setup_all/1,2`, `on_exit/2`, `start_supervised/2`, `start_supervised!/2`, `start_link_supervised!/2`, `stop_supervised/1`, `stop_supervised!/1`.
- `ExUnit.DocTest` — `doctest/2`, `doctest_file/2`; auto-imported with `ExUnit.Case`.
- `ExUnit.CaptureIO` — `capture_io/1,2,3`, `with_io/1,2,3`.
- `ExUnit.CaptureLog` — `capture_log/2`, `with_log/2`.
- `ExUnit.CaseTemplate` — `using/2` macro for shared case templates.

Property-based testing is **not** part of ExUnit; use the separate [`StreamData`](https://hexdocs.pm/stream_data/) library.

### Formatters

From [ExUnit.Formatter.html](https://hexdocs.pm/ex_unit/ExUnit.Formatter.html):

> "Formatters are `GenServer`s specified during ExUnit configuration that receive a series of events as casts."

A formatter is a plain `GenServer` exporting `init/1` and `handle_cast/2`. There is **no formal `ExUnit.Formatter` behaviour** and **no `__format__` callback**. Events cast to formatters include:

- `{:suite_started, opts}`
- `{:suite_finished, times_us}`
- `{:module_started, test_module}`
- `{:module_finished, test_module}`
- `{:test_started, test}`
- `{:test_finished, test}`
- `{:sigquit, [test | test_module]}`
- `:max_failures_reached`

The deprecated `{:case_started, _}` / `{:case_finished, _}` events should be ignored.

The only formatter that ships with ExUnit is `ExUnit.CLIFormatter` (its module docs are `@moduledoc false`). There is no built-in JSON formatter.

Configure formatters via the `:formatters` option or with `mix test --formatter MyFormatter`:

```elixir
# test/test_helper.exs
ExUnit.start(formatters: [ExUnit.CLIFormatter, MyApp.JsonFormatter])
```

Custom formatter skeleton:

```elixir
defmodule MyApp.JsonFormatter do
  use GenServer

  def init(opts), do: {:ok, opts}

  def handle_cast({:test_finished, test}, state) do
    # collect or print results
    {:noreply, state}
  end

  def handle_cast(_event, state), do: {:noreply, state}
end
```

### Capturing logs with `:capture_log`

From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

> "Captures Logger messages generated when evaluating `fun`. Returns the binary which is the captured output."

The global default is `capture_log: false`. Enable globally with:

```elixir
ExUnit.start(capture_log: true)
```

Or with a specific level:

```elixir
ExUnit.start(capture_log: [level: :warning])
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "ExUnit can optionally suppress printing of log messages that are generated during a test. Log messages generated while running a test are captured and only if the test fails are they printed to aid with debugging. You can opt into this behaviour for individual tests by tagging them with `:capture_log` or enable log capture for all tests in the ExUnit configuration: `ExUnit.start(capture_log: true)`. This default can be overridden by `@tag capture_log: false` or `@moduletag capture_log: false`."

Per-test capture:

```elixir
defmodule MyApp.LogTest do
  use ExUnit.Case

  @tag capture_log: true
  test "emits expected warning" do
    # ...
  end
end
```

Note: `ExUnit.CaptureIO` captures writes to the process group leader (`IO.puts`, `IO.write`). `ExUnit.CaptureLog` captures `Logger` messages. They are different helpers.

### Seed and test ordering

From [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html):

> "`:seed` - an integer seed value to randomize the test suite. This seed is also mixed with the test module and name to create a new unique seed on every test, which is automatically fed into the `:rand` module. This provides randomness between tests, but predictable and reproducible results. A `:seed` of `0` will disable randomization and the tests in each file will always run in the order that they were defined in"

Set a fixed seed for reproducibility:

```elixir
ExUnit.start(seed: 12345)
```

Or from the command line:

```bash
mix test --seed 12345
mix test --seed 0
```

The default seed is computed lazily from `System.system_time()` modulo 1,000,000.

### `Mix.Tasks.Test` integration

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "This task starts the current application, loads up `test/test_helper.exs` and then, requires all files matching the `test/**/*_test.exs` pattern in parallel."

`test_helper.exs` MUST call `ExUnit.start()` (or `ExUnit.start/1`). Mix does not start ExUnit for you.

#### Common `mix test` flags

| Flag | Effect |
|---|---|
| `--seed N` | Seeds the random number generator. `--seed 0` disables randomization within a file. |
| `--trace` | Detailed reporting; sets `--max-cases 1` and ignores timeouts. |
| `--max-cases N` | Maximum number of test modules running concurrently. Defaults to twice the number of cores. |
| `--max-failures N` | Stop after N failures. Runs all tests if omitted. |
| `--only filter` | Run only matching tests, e.g. `--only ci` or `--only async:true`. Fails if no tests run. |
| `--exclude filter` | Skip matching tests. May be given multiple times. |
| `--include filter` | Include matching tests. May be given multiple times. |
| `--stale` | Run only tests referencing modules changed since the last `--stale` run. |
| `--failed` | Run only tests that failed last time. |
| `--cover` | Run the coverage tool. |
| `--timeout N` | Set the test timeout. |
| `--color` / `--no-color` | Enable or disable ANSI colors. |
| `--preload-modules` | Preload all modules defined in applications. |
| `--raise` | Raise immediately if the suite fails instead of continuing other Mix tasks. |
| `--no-start` | Do not start applications after compilation. |
| `--warnings-as-errors` | Treat compilation warnings from loading the test suite as errors. |
| `--formatter Module` | Use a formatter module in addition to defaults. |
| `--slowest N` | Print the N slowest tests; sets `--trace` and `--preload-modules`. |
| `--slowest-modules N` (v1.17.0) | Print the N slowest modules. |
| `--repeat-until-failure N` (v1.17.0) | Repeat the suite until it fails; useful for flaky-test debugging. |
| `--breakpoints` (v1.17.0) | Set a breakpoint at the beginning of every test; sets `--trace`. |
| `--dry-run` (v1.19.0) | Print which tests would run without running them. |
| `--listen-on-stdin` | Run tests, then listen on stdin for changes. |
| `--max-requires N` | Maximum number of test files to compile in parallel. |
| `--partitions N` | Split tests across partitions; requires `MIX_TEST_PARTITION`. |
| `--exit-status N` | Alternate exit status on failure (default 2). |

#### Tags and filters

From [Mix.Tasks.Test.html "Tags and filters"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

```elixir
# Exclude all external tests from running
ExUnit.configure(exclude: [external: true])
```

Then run them with `mix test --include external:true`. The shorthand `--only external` is equivalent to `--include external --exclude test`. Line-specific runs look like:

```bash
mix test test/some/file_test.exs:12
mix test test/some/file_test.exs:12:24
```

#### Mix project configuration

From [Mix.Tasks.Test.html "Configuration"](https://hexdocs.pm/mix/Mix.Tasks.Test.html), the relevant project keys are:

| Key | Default | Purpose |
|---|---|---|
| `:test_paths` | `["test"]` if the directory exists, else `[]` | Directories scanned for tests. |
| `:test_pattern` | `"*.{ex,exs}"` (since v1.19.0) | Pattern for test files. |
| `:test_load_filters` | `[&String.ends_with?(&1, "_test.exs")]` | Filters applied after matching `:test_pattern`. |
| `:test_ignore_filters` | ignores `_helper.exs` and `:elixirc_paths` | Filters that exclude non-test files. |
| `:test_coverage` | unset | Coverage tool configuration. |
| `:test_elixirc_options` | unset | Compiler options used when loading tests. |

### `test_helper.exs` and `config/test.exs` patterns

Standard boilerplate:

```elixir
# test/test_helper.exs
ExUnit.start()
```

With common options:

```elixir
# test/test_helper.exs
ExUnit.start(
  assert_receive_timeout: 200,
  capture_log: true,
  exclude: [external: true],
  formatters: [ExUnit.CLIFormatter, MyApp.CustomFormatter],
  seed: 12345
)
```

You can also configure ExUnit via `config/test.exs`. This is an idiomatic working pattern — it writes the same `:ex_unit` application env that `ExUnit.configure/1` uses — but it is **not documented on hexdocs as an official ExUnit API**. Treat it as a working idiom:

```elixir
# config/test.exs
import Config

config :ex_unit,
  assert_receive_timeout: 200,
  capture_log: true,
  formatters: [ExUnit.CLIFormatter, MyApp.CustomFormatter],
  exclude: [external: true]
```

If both `config/test.exs` and `test/test_helper.exs` set the same key, the later call wins (usually the helper, because it runs after the config is loaded).

## ExUnit.Case

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Helpers for defining test cases."

`ExUnit.Case` is the module you `use` in every test module. It registers the module with the ExUnit server, imports the assertion/callback/doctest macros, and exposes the `test/1`, `test/2`, `test/3`, and `describe/2` macros used to define tests.

### `use ExUnit.Case` and what it injects

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "When you `use ExUnit.Case`, it will import the functionality from `ExUnit.Assertions`, `ExUnit.Callbacks`, `ExUnit.DocTest`, and this module itself."

Concretely, `__using__/1` imports:

- `ExUnit.Callbacks` — `setup/1,2`, `setup_all/1,2`, `on_exit/2`, `start_supervised/2`, etc.
- `ExUnit.Assertions` — `assert`, `refute`, `assert_raise`, `assert_receive`, etc.
- `ExUnit.Case` — **only** `describe/2`, `test/1`, `test/2`, `test/3`. The `register_*` functions and `register_test/6` are **not** imported; call them fully qualified as `ExUnit.Case.register_...`.
- `ExUnit.DocTest` — `doctest/2`, `doctest_file/2`.

It also registers the module with the ExUnit server and installs a `@before_compile` hook that validates tags and compiles the test cases. Two compile-time invariants to keep in mind:

- Set `@tag`, `@describetag`, and `@moduletag` **after** `use ExUnit.Case`, not before (setting them before raises a compile error).
- `:async` must be a boolean and `:parameterize` must be `nil` or a list of maps.

#### Options accepted by `use ExUnit.Case`

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

| Option | Default | Since | Description |
|---|---|---|---|
| `:async` | `false` | — | Run tests in this module concurrently with tests in other modules. Enable only if tests do not change any global state. |
| `:group` | `nil` | v1.18.0 | Group this module belongs to. Tests in the same group never run concurrently; different groups (or no group) can run concurrently when `async: true`. |
| `:parameterize` | `nil` | v1.18.0 | A list of maps merged into the test context, running the same tests once per map. Different parameters run concurrently when `async: true` is also given. |
| `:register` | `true` | — | When `false`, the module is not registered with the ExUnit server and will not run with the suite. |

An unknown option emits a compile warning rather than being silently ignored.

### Defining tests: `test/1`, `test/2`, and `test/3`

There are two `test` macros. `test/1` defines a **not implemented** test.

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Provides a convenient macro that allows a test to be defined with a string, but not yet implemented. The resulting test will always fail and print a 'Not implemented' error message. The resulting test case is also tagged with `:not_implemented`."

```elixir
test "TODO: revisit this behavior"
```

`test/3` defines a real test. Its second argument is a pattern matched against the test context.

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Defines a test with `message`. The test may also define a pattern, which will be matched against the test context. For more information on contexts, see `ExUnit.Callbacks`."

The signature is `test(message, var \\ quote do _ end, contents)`. The `test/2` form is simply `test/3` with `var` defaulted. The rendered test name is `"test <message>"` (or `"test <describe> <message>"` inside a `describe` block); ExUnit atomizes that name for the generated function.

### Context

Every test receives a context map. Callbacks merge into it and the test pattern-matches on it.

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

```elixir
defmodule KVTest do
  use ExUnit.Case

  setup do
    {:ok, pid} = KV.start_link()
    {:ok, pid: pid}
  end

  test 'stores key-value pairs', %{pid: pid} = _context do
    assert KV.put(pid, :hello, :world) == :ok
    assert KV.get(pid, :hello) == :world
  end
end
```

See the later ExUnit.Callbacks section for the full merging rules.

### Grouping tests with `describe/2`

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Describes tests together.
>
> Every describe block receives a name which is used as prefix for upcoming tests. Inside a block, `ExUnit.Callbacks.setup/1` may be invoked and it will define a setup callback to run only for the current block. The describe name is also added as a tag, allowing developers to run tests for specific blocks."

Signature: `describe(message, list)` — available since v1.3.0.

```elixir
defmodule StringTest do
  use ExUnit.Case, async: true

  describe "String.downcase/1" do
    test "with ascii characters" do
      assert String.downcase("HELLO") == "hello"
    end

    test "with Unicode" do
      assert String.downcase("HÉLLÒ") == "héllò"
    end
  end
end
```

Describe blocks cannot be nested. From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Note describe blocks cannot be nested. Instead of relying on hierarchy for composition, developers should build on top of named setups."

> "By forbidding hierarchies in favor of named setups, it is straightforward for the developer to glance at each describe block and know exactly the setup steps involved."

The supported pattern for what would otherwise be nested describes is shared named setups:

```elixir
defmodule UserManagementTest do
  use ExUnit.Case, async: true

  describe "when user is logged in and is an admin" do
    setup [:log_user_in, :set_type_to_admin]
    # ...
  end

  describe "when user is logged in and is a manager" do
    setup [:log_user_in, :set_type_to_manager]
    # ...
  end

  defp log_user_in(_context), do: :ok
  defp set_type_to_admin(_context), do: :ok
  defp set_type_to_manager(_context), do: :ok
end
```

A `setup/1` written inside a `describe` runs only for tests in that block, and the describe name is available in the context under the `:describe` key.

### Registered attributes

ExUnit provides three attribute registers with different scopes, plus `register_test/6` for building custom test macros. Registered values surface in the test context under `context.registered.<name>`.

#### `register_attribute/3` — per test

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a new attribute to be used during `ExUnit.Case` tests.
>
> The attribute values will be available through `context.registered`. Registered values are cleared after each `test/3` similar to `@tag`.
>
> This function takes the same options as `Module.register_attribute/3`."

Signature: `register_attribute(env, name, opts \\ [])` — `@spec register_attribute(env(), atom(), register_attribute_opts()) :: :ok`. Available since v1.3.0. `env()` is `module() | Macro.Env.t()`; opts are `[accumulate: boolean(), persist: boolean()]`.

```elixir
defmodule MyTest do
  use ExUnit.Case

  ExUnit.Case.register_attribute(__MODULE__, :fixtures, accumulate: true)

  @fixtures :user
  @fixtures {:post, insert: false}
  test "using custom attribute", context do
    assert context.registered.fixtures == [{:post, insert: false}, :user]
  end

  test "custom attributes are cleared per test", context do
    assert context.registered.fixtures == []
  end
end
```

#### `register_describe_attribute/3` — per describe block

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a new describe attribute to be used during `ExUnit.Case` tests.
>
> The attribute values will be available through `context.registered`. Registered values are cleared after each `describe/2` similar to `@describetag`.
>
> This function takes the same options as `Module.register_attribute/3`."

Signature: `register_describe_attribute(env, name, opts \\ [])` — available since v1.10.0.

```elixir
defmodule MyTest do
  use ExUnit.Case

  ExUnit.Case.register_describe_attribute(__MODULE__, :describe_fixtures, accumulate: true)

  describe "using custom attribute" do
    @describe_fixtures :user
    @describe_fixtures {:post, insert: false}

    test "has attribute", context do
      assert context.registered.describe_fixtures == [{:post, insert: false}, :user]
    end
  end

  describe "custom attributes are cleared per describe" do
    test "doesn't have attributes", context do
      assert context.registered.describe_fixtures == []
    end
  end
end
```

#### `register_module_attribute/3` — whole module

`register_module_attribute/3` (since v1.10.0) registers an attribute that is **not** cleared between tests; it persists across all tests in the module. Use it for module-wide fixtures shared by every test.

#### `register_test/6` — custom test macros

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a function to run as a test for this module.
>
> This is used by third-party projects to implement macros like `property/3` that works like `test` but instead defines a property. See `test/3` implementation for an example of invoking this function.
>
> The test type will be converted to a string and pluralized for display. You can use `ExUnit.plural_rule/2` to set a custom pluralization."

Signature: `register_test(mod, file, line, test_type, name, tags)` — available since v1.11.0. The older `register_test/4` is deprecated in favor of `/6`. Internally, `test/3` calls `ExUnit.Case.register_test(mod, file, line, :test, message, [])`.

### Tags: `@tag`, `@moduletag`, `@describetag`

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "By tagging a test, the tag value can be accessed in the context, allowing the developer to customize the test."

Tags are set per test (`@tag`), per describe block (`@describetag`), or per module (`@moduletag`):

```elixir
defmodule ApiTest do
  use ExUnit.Case
  @moduletag :external

  describe 'makes calls to the right endpoint' do
    @describetag :endpoint
    # ...
  end
end
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "If you are setting a `@moduletag` or `@describetag` attribute, you must set them after your call to `use ExUnit.Case` otherwise you will see compilation errors.
>
> If the same key is set via `@tag`, the `@tag` value has higher precedence.
>
> The `setup_all` blocks only receive tags that are set using `@moduletag`."

Tag forms: `@tag key: value` sets `key` to `value`; `@tag :key` is shorthand for `@tag key: true`. If a key is set more than once, the last value wins. Precedence for the same key is `@moduletag` < `@describetag` < `@tag`.

#### Known tags

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html), reserved tags **set automatically by ExUnit** (do not set these yourself):

| Tag | Meaning |
|---|---|
| `:async` | Whether the test case is in async mode. |
| `:file` | File in which the test was defined. |
| `:line` | Line on which the test was defined. |
| `:module` | Module in which the test was defined. |
| `:registered` | Values from `register_attribute/3` and friends. |
| `:test` | The test name. |
| `:test_group` | The group the test belongs to. |
| `:test_pid` | PID of the testing process. |
| `:test_type` | Test type used when printing results (`:test`, `:doctest`, or a custom type from `register_test/6`). |
| `:describe` | The describe block the test belongs to (if any). |
| `:describe_line` | Line the describe block begins on (if any); usable for line-based runs. |
| `:doctest` | The module/file being doctested (if a doctest). |
| `:doctest_data` | Extra doctest metadata (e.g. `:end_line`) for reflection. |
| `:doctest_line` | Line the doctest was defined on (if a doctest). |

Tags that **customize behavior**:

| Tag | Since | Effect |
|---|---|---|
| `:capture_log` | — | Capture log messages during the test and print them only on failure. Set via `@tag`/`@moduletag`; overrides the global `:capture_log` default. |
| `:skip` | — | Skip the test with the given reason. |
| `:timeout` | — | Custom test timeout in ms (default `60_000`). Accepts `:infinity`. |
| `:tmp_dir` | v1.11.0 | Create a unique temp dir for the test and expose its path via the context. |

#### Filters

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Tags can also be used to identify specific tests, which can then be included or excluded using filters. The most common functionality is to exclude some particular tests from running, which can be done via `ExUnit.configure/1`."

```elixir
# Exclude all external tests from running
ExUnit.configure(exclude: [external: true])
```

> "Keep in mind that all tests are included by default, so unless they are excluded first, the `include` option has no effect."

Filters can match a tag regardless of value (`exclude: :os`) or a specific value (`include: [os: :unix]`), and may be given multiple times. See the Overview section and `mix help test` for the command-line equivalents.

### Concurrency options: `:async`, `:group`, `:parameterize`

See also the Overview section's "Async testing" discussion of `:max_cases` and the global-state rules.

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "`:async` - configures tests in this module to run concurrently with tests in other modules. Tests in the same module never run concurrently (with the exception of tests run via the `:parameterize` option - see below). It should be enabled only if tests do not change any global state. Defaults to `false`."

> "`:group` (since v1.18.0) - configures the **group** this module belongs to. Tests in the same group never run concurrently. Tests from different groups (or with no groups) can run concurrently when `async: true` is given. By default, this module belongs to no group (defaults to `nil`)."

> "`:parameterize` (since v1.18.0) - a list of maps to parameterize tests. If both `:async` and `:parameterize` are given, the different parameters run concurrently. See the 'Parameterized tests' section below for more information."

#### Parameterized tests

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Sometimes you want to run the same tests but with different parameters. In ExUnit, it is possible to do so by passing a `:parameterize` key to `ExUnit.Case`. The value must be a list of maps which will be the parameters merged into the test context."

```elixir
use ExUnit.Case,
  async: true,
  parameterize:
    for(kind <- [:unique, :duplicate],
        partitions <- [1, 8],
        do: %{kind: kind, partitions: partitions})

test 'starts a registry', %{kind: kind, partitions: partitions} do
  # ...
end
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Use parameterized tests with care:
>
> - Although parameterized tests run concurrently when `async: true` is also given, abuse of parameterized tests may make your test suite slower
> - If you use parameterized tests and then find yourself adding conditionals in your tests to deal with different parameters, then parameterized tests may be the wrong solution to your problem. Consider creating separated tests and sharing logic between them using regular functions"

### Process model

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "1. First, all `ExUnit.Callbacks.setup_all/1` callbacks run in a single process, sequentially, in the order they were defined.
>
> 2. Then, a new process is spawned for the test itself. In this process, first all `ExUnit.Callbacks.setup/1` callbacks run, in the order they were defined. Then, the test itself is executed.
>
> 3. After the test exits, a new process is spawned to run all `ExUnit.Callbacks.on_exit/2`, in the reverse order they were defined."

This per-test process model is why `start_supervised/2` (from `ExUnit.Callbacks`) is the recommended way to start processes: anything it starts is automatically terminated via `on_exit` after the test.

### Importing vs using

The ExUnit docs document `use ExUnit.Case` as the entry point; there is **no documented recommendation** to `import ExUnit.Case` instead. Because `__using__/1` imports only `describe/2` and `test/1,2,3` from `ExUnit.Case`, a bare `import ExUnit.Case` would give you the `test`/`describe` macros but **not** `assert`, `setup`, or `doctest`. In practice, always use `use ExUnit.Case` (or `use MyCase` for a shared template) so the full set of test macros is available and the module is registered with ExUnit.

### Shared templates: `ExUnit.CaseTemplate` (data_case / conn_case)

When several test modules share setup or helper functions, define a case template instead of repeating `use ExUnit.Case`.

From [ExUnit.CaseTemplate.html](https://hexdocs.pm/ex_unit/ExUnit.CaseTemplate.html):

> "Defines a module template to be used throughout your test suite.
>
> This is useful when there are a set of setup callbacks or a set of functions that should be shared between test modules."

> "When you `use ExUnit.CaseTemplate`, it will import the functionality from `ExUnit.Assertions`, `ExUnit.Callbacks`, and this module itself. It will also define a `__using__` callback, so the module itself can be used as a template instead of `ExUnit.Case`."

```elixir
defmodule MyCase do
  use ExUnit.CaseTemplate

  setup do
    IO.puts("This will run before each test that uses this case")
  end
end

defmodule MyTest do
  use MyCase, async: true

  test "truth" do
    assert true
  end
end
```

#### The `using/2` macro

`using/2` lets the template inject code (aliases, imports) into every module that calls `use MyCase`:

```elixir
defmodule MyCase do
  use ExUnit.CaseTemplate

  using do
    quote do
      alias MyApp.FunModule
    end
  end
end
```

It can also receive the options passed to `use MyCase`:

```elixir
defmodule MyCase do
  use ExUnit.CaseTemplate

  using options do
    quote do
      if unquote(options)[:import_helpers] do
        import MyApp.TestHelpers
      end
    end
  end
end

defmodule SomeTestCase do
  use MyCase, async: true, import_helpers: true
  # ...
end
```

From [ExUnit.CaseTemplate.html](https://hexdocs.pm/ex_unit/ExUnit.CaseTemplate.html):

> "The options that you pass to `use MyCase` get also passed to `use ExUnit.Case` under the hood. This means you can do things like `use MyCase, async: true`. You can also access this options in `using/2`."

#### `DataCase` and `ConnCase`

`DataCase` and `ConnCase` are not part of ExUnit; they are **project-specific case templates** that Phoenix/Ecto generators scaffold on top of `ExUnit.CaseTemplate`. The pattern, derived from the mechanism above, is:

- `use ExUnit.CaseTemplate` in `test/support/data_case.ex` (or `conn_case.ex`).
- A `using do ... end` block that injects the application's data-layer aliases and imports (e.g. `alias MyApp.Repo`, `import Ecto.Query`, data factory helpers).
- A `setup` block that tags the test for sandbox mode and checks out an Ecto sandbox connection, so each test runs in a database transaction rolled back on exit; for `ConnCase`, the setup also builds a `Plug.Conn` through the endpoint.
- Test modules then do `use MyApp.DataCase` (or `use MyApp.ConnCase, async: true`), inheriting both the setup and the injected aliases.

The exact contents are generated by the Phoenix/Ecto generators and vary by version; consult the [Phoenix guides](https://hexdocs.pm/phoenix/) and [Ecto SQL docs](https://hexdocs.pm/ecto_sql/) for the current skeleton. The ExUnit-level contract documented here — `use ExUnit.CaseTemplate` plus `using/2` plus `setup/1` — is what `DataCase` and `ConnCase` are built on.

### Test module organization

From [ExUnit.html "Integration with Mix"](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Invoking `mix test` from the command line will run the tests in each file matching the pattern `*_test.exs` found in the `test` directory of your project."

Conventions reinforced by Mix and the ExUnit examples:

- One test file per source module, suffixed `_test.exs`, under `test/`. Mix discovers tests by the `*_test.exs` pattern; a file not matching it is not run.
- One test module per file, named `<Subject>Test` (e.g. `MyApp.UserTest` for `MyApp.User`), beginning with `use ExUnit.Case`.
- Group related tests with `describe/2`, one describe per public function or per behavior.
- Write test names as descriptive sentences (`test "returns an error when the input is empty"`), not as identifiers.
- Share setup and helpers across files via `ExUnit.CaseTemplate` rather than copy-paste.

### Examples

#### Minimal test module

```elixir
defmodule MyApp.MathTest do
  use ExUnit.Case, async: true

  test "adds two numbers" do
    assert 1 + 1 == 2
  end
end
```

#### `describe` with scoped setup and tags

```elixir
defmodule MyApp.ApiTest do
  use ExUnit.Case
  @moduletag :external

  describe "GET /users" do
    @describetag :read
    setup [:stub_users]

    test "returns the list of users", %{users: users} do
      assert length(users) == 3
    end
  end

  defp stub_users(_context), do: {:ok, users: [%{id: 1}, %{id: 2}, %{id: 3}]}
end
```

#### Custom describe-scoped attribute

```elixir
defmodule MyApp.FixtureTest do
  use ExUnit.Case

  ExUnit.Case.register_describe_attribute(__MODULE__, :roles, accumulate: true)

  describe "admin actions" do
    @roles :admin
    @roles :superuser

    test "sees both roles", context do
      assert context.registered.roles == [:superuser, :admin]
    end
  end
end
```

#### Shared case template

```elixir
defmodule MyApp.DataCase do
  use ExUnit.CaseTemplate

  using do
    quote do
      alias MyApp.Repo
      import Ecto.Query
    end
  end
end

defmodule MyApp.UserTest do
  use MyApp.DataCase, async: true

  test "counts users" do
    assert Repo.aggregate(MyApp.User, :count) == 0
  end
end
```

### Common mistakes

- Calling `register_attribute/3`, `register_describe_attribute/3`, or `register_test/6` unqualified after `use ExUnit.Case` and expecting them imported — they are not imported; call them as `ExUnit.Case.register_...`.
- Setting `@tag`/`@describetag`/`@moduletag` before `use ExUnit.Case`; it raises a compile error.
- Trying to nest `describe/2` blocks; describe cannot be nested — use named setups instead.
- Expecting tests inside the same module to run concurrently; only different modules (or different `:parameterize` maps) run concurrently.
- Marking a module `async: true` when it changes global state; see the Overview section's async rules.
- Using `:parameterize` and then branching inside tests on the parameter value; the docs recommend separate tests sharing helper functions instead.
- Assuming `DataCase`/`ConnCase` ship with ExUnit; they are Phoenix/Ecto-generated templates built on `ExUnit.CaseTemplate`.
- Using a bare `import ExUnit.Case` that omits `assert`/`setup`/`doctest`; use `use ExUnit.Case` instead.

Sources: https://hexdocs.pm/ex_unit/ExUnit.Case.html, https://hexdocs.pm/ex_unit/ExUnit.CaseTemplate.html, https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html, https://hexdocs.pm/ex_unit/ExUnit.html

## ExUnit.Assertions

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "This module contains a set of assertion functions that are imported by default into your test cases.
>
> In general, a developer will want to use the general `assert` macro in tests. This macro introspects your code and provides good reporting whenever there is a failure. For example, `assert some_fun() == 10` will fail (assuming `some_fun()` returns `13`):"

```
Comparison (using ==) failed in:
code:  assert some_fun() == 10
left:  13
right: 10
```

> "This module also provides other convenience functions like `assert_in_delta` and `assert_raise` to easily handle other common cases such as checking a floating-point number or handling exceptions."

The assertions are imported automatically by `use ExUnit.Case` (see the ExUnit.Case section). You never need to `import ExUnit.Assertions` explicitly in a test module.

Note that ExUnit deliberately does **not** provide named `assert_equal`/`refute_equal` or `assert_throw`/`assert_catch` functions. Equality is expressed with the introspecting `assert/1` macro (`assert left == right`), throws/exits/errors are caught with `catch_throw/1`, `catch_exit/1`, `catch_error/1`, and raised exceptions are asserted with `assert_raise/2,3`. The `assert/1` macro's expression introspection is what gives equality assertions their rich diff output, so a separate `assert_equal` is unnecessary.

### Macros vs functions

Most assertions are **macros** because they need to rewrite or introspect the assertion expression to produce good error messages. A few are plain **functions**.

| Assertion | Kind |
|---|---|
| `assert/1` | macro |
| `assert/2` | function |
| `refute/1` | macro |
| `refute/2` | function |
| `assert_receive/3` | macro |
| `assert_received/2` | macro |
| `refute_receive/3` | macro |
| `refute_received/2` | macro |
| `catch_error/1`, `catch_exit/1`, `catch_throw/1` | macro |
| `assert_raise/2`, `assert_raise/3` | function |
| `assert_in_delta/3,4`, `refute_in_delta/3,4` | function |
| `flunk/0,1` | function |

### `assert/1` — the introspecting assertion macro

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts its argument is a truthy value.
>
> `assert` introspects the underlying expression and provides good reporting whenever there is a failure. For example, if the expression uses the comparison operator, the message will show the values of the two sides. The assertion"

```elixir
assert 1 + 2 + 3 + 4 > 15
```

fails with:

```
Assertion with > failed
code:  assert 1 + 2 + 3 + 4 > 15
left:  10
right: 15
```

`assert/1` recognizes a fixed set of operators and rewrites each into a structured `left`/`right` error. The recognized operators are `==`, `!=`, `===`, `!==`, `<`, `>`, `<=`, `>=`, `=~`, and `in` (membership). Any non-operator truthy/falsy expression is checked for truthiness directly. For equality failures the formatter additionally produces a structured diff of the two compound values; for the other comparisons (`<`, `>`, `=~`, `in`, …) the message is `"Assertion with <operator> failed"`.

### Match assertions in `assert/1`

`assert/1` also handles a match form, binding variables that escape into the surrounding scope.

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Similarly, if a match expression is given, it will report any failure in terms of that match. Given"

```elixir
assert [1] = [2]
```

> "you'll see:"

```
match (=) failed
code:  assert [1] = [2]
left: [1]
right: [2]
```

Important truthiness caveat from [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Keep in mind that `assert` does not change its semantics based on the expression. In other words, the expression is still required to return a truthy value. For example, the following will fail:"

```elixir
assert nil = some_function_that_returns_nil()
```

> "Even though the match works, `assert` still expects a truth value. In such cases, simply use `==/2` or `match?/2`."

Because `assert pattern = expression` binds the matched variables, it is generally preferred over `assert match?(pattern, expression)`, which does not bind variables (see `match?/2` below). If the pattern needs guards, `assert/1` cannot express them directly; use `match?/2`:

> "If you need more complex pattern matching using guards, you need to use `match?/2`:"

```elixir
assert match?([%{id: id} | _] when is_integer(id), records)
```

### `assert/2` — assert with a custom message

`assert/2` is a plain function that asserts a value is truthy and displays a given message otherwise. It does **not** introspect expressions — it only checks truthiness.

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts `value` is truthy, displaying the given `message` otherwise."

```elixir
assert false, "it will never be true"

assert x == :foo, "expected x to be foo"

assert match?({:ok, _}, x), "expected x to match {:ok, _}"
```

Use `assert/2` when you want a custom failure message; use `assert/1` when you want automatic introspection.

### `refute/1` and `refute/2`

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "A negative assertion, expects the expression to be `false` or `nil`."

Like `assert/1`, `refute/1` introspects recognized operators and produces `"Refute with <operator> failed"` messages. It does **not** invert a match expression. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Keep in mind that `refute` does not change the semantics of the given expression. In other words, the following will fail:
>
> "

```elixir
refute {:ok, _} = some_function_that_returns_error_tuple()
```

> "The code above will fail because the `=` operator always fails when the sides do not match and `refute/2` does not change it.
>
> The correct way to write the refutation above is to use `match?/2`:"

```elixir
refute match?({:ok, _}, some_function_that_returns_error_tuple())
```

```elixir
refute age < 0
```

`refute/2` is the function form. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts `value` is `nil` or `false` (that is, `value` is not truthy)."

```elixir
refute true, "This will obviously fail"
```

### `match?/2` (from `Kernel`, not ExUnit)

`match?/2` is part of `Kernel`, not `ExUnit.Assertions`. It is auto-imported and available in tests without qualification. From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html#match?/2):

> "A convenience macro that checks if the result of `expression` matches `pattern`."

The common test idiom is `assert match?(pattern, value)` or `refute match?(pattern, value)`. `assert/1` and `refute/1` have a dedicated clause that intercepts `match?` inside them to produce a richer message (`"match (match?) failed"` / `"match (match?) succeeded, but should have failed"`) with the formatted pattern as `left` and the actual value as `right`. Two behavioral differences to remember:

- `assert pattern = value` binds variables; `assert match?(pattern, value)` does **not** bind variables — use the match form when you need the matched values.
- `match?` supports guards (`match?({:ok, n} when n > 0, result)`); the plain `assert pattern = value` match form does not.

### `flunk/1` — unconditional failure

From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Fails with a message."

Its specs are `@spec flunk :: no_return` and `@spec flunk(String.t()) :: no_return`; the default message is `"Flunked!"`. `flunk/1` is a common building block — `assert/2`, `assert_receive`, and others call it internally to report failures — and it raises `ExUnit.AssertionError`.

```elixir
flunk("This should raise an error")
```

### `assert_raise/2` and `assert_raise/3`

`assert_raise` asserts that an exception is raised when evaluating a function, and returns the rescued exception struct. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts the `exception` is raised during `function` execution. Returns the rescued exception, fails otherwise."

```elixir
assert_raise ArithmeticError, fn ->
  1 + "test"
end

assert_raise RuntimeError, fn ->
  raise "assertion will pass due to this raise"
end
```

The three-argument form also checks the message, which may be an exact `String` or a `Regex`. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts the `exception` is raised during `function` execution with the expected `message`, which can be a `Regex` or an exact `String`. Returns the rescued exception, fails otherwise."

```elixir
assert_raise ArithmeticError, "bad argument in arithmetic expression", fn ->
  1 / 0
end

assert_raise RuntimeError, ~r/^today's lucky number is 0\.\d+!$/, fn ->
  raise "today's lucky number is #{:rand.uniform()}!"
end
```

If no exception is raised, `assert_raise` fails; if a different exception is raised, it reports the mismatch; if the message does not match (in the `/3` form), it reports expected vs. actual message.

### `catch_error/1`, `catch_exit/1`, `catch_throw/1`

These macros assert that an expression throws, exits, or raises, and **return the caught value** so it can be used in a further assertion. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts `expression` will throw a value.
>
> Returns the thrown value or fails otherwise."

```elixir
assert catch_throw(throw(1)) == 1
```

> "Asserts `expression` will exit.
>
> Returns the exit status/message of the current process or fails otherwise."

```elixir
assert catch_exit(exit(1)) == 1
```

> "Asserts `expression` will cause an error.
>
> Returns the error or fails otherwise."

```elixir
assert catch_error(error(1)) == 1
```

To assert on exits from **linked** processes (not the test process itself), trap exits and assert on the `:EXIT` message with `assert_receive`:

```elixir
Process.flag(:trap_exit, true)
pid = spawn_link(fn -> Process.exit(self(), :normal) end)
assert_receive {:EXIT, ^pid, :normal}
```

#### `catch_*` vs `assert_raise`

- `assert_raise/2,3` is purpose-built for raised exceptions and additionally supports `Regex`/exact-string message matching.
- `catch_error/1`, `catch_exit/1`, `catch_throw/1` cover all three control-flow channels (`error`, `:exit`, `:throw`) and return the caught value rather than asserting a specific message up front. Prefer `assert_raise` when testing `raise`, and `catch_exit`/`catch_throw` for `exit/1` and `throw/1`.

### `assert_in_delta/3,4` and `refute_in_delta/3,4`

`assert_in_delta` checks that two numbers differ by no more than `delta`. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts that `value1` and `value2` differ by no more than `delta`.
>
> This difference is inclusive, so the test will pass if the difference and the `delta` are equal."

```elixir
assert_in_delta 1.1, 1.5, 0.2
assert_in_delta 10, 15, 2
assert_in_delta 10, 15, 5
```

`refute_in_delta` is the negation but with an important boundary difference. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts `value1` and `value2` are not within `delta`.
>
> This difference is exclusive, so the test will fail if the difference and the delta are equal.
>
> If you supply `message`, information about the values will automatically be appended to it."

```elixir
refute_in_delta 1.1, 1.2, 0.2
refute_in_delta 10, 11, 2
```

| Assertion | Check | Boundary |
|---|---|---|
| `assert_in_delta a, b, d` | `abs(a - b) <= d` | inclusive (`<=`) |
| `refute_in_delta a, b, d` | `abs(a - b) < d` fails | exclusive (`<`) |

A negative `delta` raises an `ArgumentError`. The optional `message` is the fourth argument and is appended with value details when supplied. (There is no `assert_in_delta/5` or `refute_in_delta/5`; the API is `/3` and `/4` in v1.20.2.)

### `assert_receive/3` and `assert_received/2`

`assert_receive` waits for a message matching a pattern to arrive in the current process mailbox within a timeout. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts that a message matching `pattern` was or is going to be received within the `timeout` period, specified in milliseconds.
>
> Unlike `assert_received`, it has a configurable timeout. The default timeout duration is determined by the `assert_receive_timeout` option, which can be set using `ExUnit.configure/1`. This option defaults to 100 milliseconds.
>
> The `pattern` argument must be a match pattern.
>
> Flunks with `failure_message` if a message matching `pattern` is not received. If a message matches, it is also removed from the process mailbox."

```elixir
assert_receive :hello

# with a larger timeout
assert_receive :hello, 20_000

# matching specific patterns
assert_receive {:hello, _}

x = 5
assert_receive {:count, ^x}
```

`assert_received` checks the mailbox immediately, with a timeout of `0` (no waiting). From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts that a message matching `pattern` was received and is in the current process' mailbox.
>
> The `pattern` argument must be a match pattern.
>
> Flunks with `failure_message` if a message matching `pattern` was not received. If a message matches, it is also removed from the process mailbox.
>
> Timeout is set to `0`, so there is no waiting time."

```elixir
send(self(), :hello)
assert_received :hello

send(self(), :bye)
assert_received :hello, "Oh No!"
** (ExUnit.AssertionError) Oh No!
```

### `refute_receive/3` and `refute_received/2`

`refute_receive` asserts that no matching message arrives within the timeout. From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts that a message matching `pattern` was not received (and won't be received) within the `timeout` period, specified in milliseconds.
>
> The `pattern` argument must be a match pattern. Flunks with `failure_message` if a message matching `pattern` is received."

```elixir
refute_receive :bye

# with an explicit timeout
refute_receive :bye, 1000
```

`refute_received` checks the mailbox immediately (timeout `0`). From [ExUnit.Assertions.html](https://hexdocs.pm/ex_unit/ExUnit.Assertions.html):

> "Asserts a message matching `pattern` was not received (i.e. it is not in the current process' mailbox).
>
> The `pattern` argument must be a match pattern. Flunks with `failure_message` if a message matching `pattern` was received.
>
> Timeout is set to `0`, so there is no waiting time."

```elixir
send(self(), :hello)
refute_received :bye

send(self(), :hello)
refute_received :hello, "Oh No!"
** (ExUnit.AssertionError) Oh No!
```

Behavioral note: `refute_receive` waits the **full** timeout before succeeding, so messages left in the mailbox from earlier setup will cause a matching `refute_receive` to fail. Consume or flush known messages first.

### Assertion error messages and `ExUnit.AssertionError`

Every assertion failure raises an `ExUnit.AssertionError`. From [ExUnit.AssertionError.html](https://hexdocs.pm/ex_unit/ExUnit.AssertionError.html):

> "Raised to signal an assertion error.
>
> This is used by macros such as `ExUnit.Assertions.assert/1`."

The struct carries the data the formatter uses to render the diff. The `t()` type (since v1.16.0) is:

```elixir
@type t :: %__MODULE__{
        left: any,
        right: any,
        message: any,
        expr: any,
        args: any,
        doctest: any,
        context: any
      }
```

| Field | Purpose |
|---|---|
| `:left` | Left-hand side of a comparison/match (often the actual value). |
| `:right` | Right-hand side of a comparison/match (often the expected value). |
| `:message` | Human-readable failure message. |
| `:expr` | Quoted form of the original assertion expression, so the failing source code is reproduced. |
| `:args` | Generated argument vars for call arguments (e.g. `assert f(a, b) == x`). |
| `:doctest` | Doctest metadata when the assertion originates from a doctest. |
| `:context` | Diff context — `:==`, `:===`, `{:match, pins}`, or `{:mailbox, pins, mailbox}` — which drives formatter behavior. |

Fields default to the sentinel `:ex_unit_no_meaningful_value`, exposed via `ExUnit.AssertionError.no_value/0`, which tells the formatter to omit a field.

### Configuration options relevant to assertions

Two ExUnit options govern the message-assertion timeouts (see the Overview section's option reference for the full list). From [ExUnit.html](https://hexdocs.pm/ex_unit/ExUnit.html):

> "`:assert_receive_timeout` - the timeout to be used on `assert_receive` calls in milliseconds, defaults to `100`;
>
> `:refute_receive_timeout` - the timeout to be used on `refute_receive` calls in milliseconds, defaults to `100`;"

```elixir
ExUnit.start(assert_receive_timeout: 200, refute_receive_timeout: 200)
```

### Common mistakes

- Writing `refute {:ok, _} = ...` expecting it to refute a match; `refute` does not invert `=`. Use `refute match?({:ok, _}, ...)`.
- Writing `assert pattern when guard = value`; guards are not allowed in a plain match assertion. Use `assert match?(pattern when guard, value)`.
- Using `assert match?(pattern, value)` and then trying to use variables bound by `pattern`; `match?` does not bind variables — use `assert pattern = value`.
- Expecting `refute_in_delta a, b, d` to pass when `abs(a - b) == d`; it is exclusive, so it fails at the boundary.
- Forgetting that `assert_receive`/`assert_received` remove the matched message from the mailbox, so a second assertion for the same message fails.
- Leaving messages in the mailbox from setup that then cause `refute_receive` to fail; consume or flush them first.
- Reaching for a named `assert_equal`/`assert_catch`; ExUnit has neither — use `assert ==` and the `catch_*` macros.
- Treating `assert/2` as introspecting; it only checks truthiness and prints your message. Use `assert/1` for introspection.
- Assuming `assert_in_delta`/`refute_in_delta` have a 5-arity form; in v1.20.2 they are `/3` and `/4` (optional message).

Sources: https://hexdocs.pm/ex_unit/ExUnit.Assertions.html, https://hexdocs.pm/ex_unit/ExUnit.AssertionError.html, https://hexdocs.pm/elixir/Kernel.html#match?/2, https://hexdocs.pm/ex_unit/ExUnit.html

## ExUnit.Callbacks

From [ExUnit.Callbacks.html](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html):

> "Defines ExUnit callbacks.
>
> This module defines the `setup/1`, `setup/2`, `setup_all/1`, and `setup_all/2` macros, as well as process lifecycle and management functions, such as `on_exit/2`, `start_supervised/2`, `stop_supervised/1` and `start_link_supervised!/2`.
>
> The setup callbacks may be used to define test fixtures and run any initialization code which help bring the system into a known state. They are defined via macros and each one can optionally receive a map with test state and metadata, usually referred to as the `context`. Optionally, the context to be used in the tests can be extended by the setup callbacks by returning a properly structured value (see below).
>
> The `setup_all` callbacks are invoked only once per module, before any test is run. All `setup` callbacks are run before each test. No callback is run if the test case has no tests or all tests have been filtered out.
>
> `setup` and `setup_all` callbacks can be defined by either a block, an atom naming a local function, a `{module, function}` tuple, or a list of atoms/tuples.
>
> Both can opt to receive the current context by specifying it as a parameter if defined by a block. Functions used to define a test setup must accept the context as a single argument.
>
> A test module can define multiple `setup` and `setup_all` callbacks, and they are invoked in order of appearance.
>
> `start_supervised/2` is used to start processes under a supervisor. The supervisor is linked to the current test process. The supervisor as well as all child processes are guaranteed to terminate before any `on_exit/2` callback runs.
>
> `on_exit/2` callbacks are registered on demand, usually to undo an action performed by a setup callback. `on_exit/2` may also take a reference, allowing the callback to be overridden in the future. A registered `on_exit/2` callback will always run, while failures in `setup` and `setup_all` will stop all remaining setup callbacks from executing.
>
> Finally, `setup_all` callbacks run in a separate process per module, while all `setup` callbacks run in the same process as the test itself. `on_exit/2` callbacks always run in a separate process, as implied by their name. The test process always exits with reason `:shutdown`, which means any process linked to the test process will also exit, although asynchronously. Therefore it is preferred to use `start_supervised/2` to guarantee all supervised processes have fully terminated before the next test starts."

### What `ExUnit.Callbacks` provides

`ExUnit.Callbacks` is imported automatically by `use ExUnit.Case` (see the ExUnit.Case section). You never need to `import ExUnit.Callbacks` in a test module. The public surface is:

| Function / macro | Kind | Since |
|---|---|---|
| `setup/1`, `setup/2` | macro | pre-existing |
| `setup_all/1`, `setup_all/2` | macro | pre-existing |
| `on_exit/2` (and the `on_exit/1` default) | function | pre-existing |
| `start_supervised/2` | function | v1.5.0 |
| `start_supervised!/2` | function | v1.6.0 |
| `start_link_supervised!/2` | function | v1.14.0 |
| `stop_supervised/1` | function | v1.5.0 |
| `stop_supervised!/1` | function | v1.10.0 |

The macros carry no user-facing `@spec` on the hexdocs page; the supervised helpers do.

### Context and valid return values

Every `setup` and `setup_all` callback can optionally receive a map of test state and metadata called the `context`, and can extend that context by returning a properly structured value. From [ExUnit.Callbacks.html "Context"](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#module-context):

> "`setup_all` or `setup` may return one of:
>
> - the atom `:ok`
> - a keyword list or map
> - a tuple in the shape of `{:ok, keyword() | map()}`
>
> If a keyword list or map is returned, it will be merged into the current context and will be available in all subsequent `setup_all`, `setup`, and the `test` itself.
>
> Returning anything else from `setup_all` will force all tests to fail, while a bad response from `setup` causes the current test to fail."

Accepted return shapes are therefore interchangeable — all of the following are equivalent for a `setup`/`setup_all` callback (from the module's inline examples):

```elixir
setup do
  # Any of these four shapes is valid and merges :hello => "world":
  #
  #     :ok                                        # context unchanged
  #     {:ok, hello: "world"}                      # {:ok, keyword}
  #     {:ok, %{hello: "world"}}                   # {:ok, map}
  #     %{hello: "world"}                          # bare map
  #
  [hello: "world"]                                  # bare keyword list
end
```

The context starts from the test's built-in tags (`:module`, `:file`, `:line`, `:test`, `:async`, `:registered`, `:describe`, `:test_pid`, etc., plus any `@tag`/`@describetag`/`@moduletag` values — see the ExUnit.Case section's "Known tags"). Setup callbacks read those tags via their context argument and merge additional keys that subsequent callbacks and the test itself can see:

```elixir
defmodule ExampleTagModificationTest do
  use ExUnit.Case

  setup %{login_as: username} do
    {:ok, current_user: username}
  end

  @tag login_as: "max"
  test "tags modify context", context do
    assert context[:login_as] == "max"
    assert context[:current_user] == "max"
  end
end
```

Two important caveats:

- `setup_all` blocks only receive tags set via `@moduletag` (not `@tag` or `@describetag`, which are per-test).
- When multiple `setup` (or multiple `setup_all`) callbacks merge the same key, they are invoked "in order of appearance" and each return value is merged into the running context; the result is that a later callback's value for a key wins over an earlier one. (The docs state the ordering and the merge explicitly; the "later wins" rule is the natural consequence of sequential `Map`/keyword merging and is not called out verbatim.)

### `setup/1` and `setup/2`

From [setup/1](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup/1):

> "Defines a callback to be run before each test in a case.
>
> Accepts one of these:
>
> - a block
> - an atom naming a local function
> - a `{module, function}` tuple
> - a list of atoms and `{module, function}` tuples
>
> Can return values to be merged into the context, to set up the state for tests. For more details, see the "Context" section shown above.
>
> `setup/1` callbacks are executed in the same process as the test process."

```elixir
defp clean_up_tmp_directory(context) do
  # perform setup
  :ok
end

setup :clean_up_tmp_directory

setup [:clean_up_tmp_directory, :another_setup]

setup do
  [conn: Plug.Conn.build_conn()]
end

setup {MyModule, :my_setup_function}
```

`setup/2` is identical except its first argument is bound to the context. From [setup/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup/2):

> "Defines a callback to be run before each test in a case.
>
> This is similar to `setup/1`, but the first argument is the context. The `block` argument can only be a block.
>
> For more details, see the "Context" section shown above."

```elixir
setup context do
  [conn: Plug.Conn.build_conn()]
end
```

`setup` runs **once per test**, in the same process as the test itself, before the test body.

### `setup_all/1` and `setup_all/2`

From [setup_all/1](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup_all/1):

> "Defines a callback to be run before all tests in a case.
>
> Accepts one of these:
>
> - a block
> - an atom naming a local function
> - a `{module, function}` tuple
> - a list of atoms and `{module, function}` tuples
>
> Can return values to be merged into the `context`, to set up the state for tests. For more details, see the "Context" section shown above.
>
> `setup_all/1` callbacks are executed in a separate process than tests. All `setup_all/1` callbacks are executed in order in the same process."

```elixir
# One-arity function name
setup_all :clean_up_tmp_directory

# A module and function
setup_all {MyModule, :my_setup_function}

# A list of one-arity functions and module/function tuples
setup_all [:clean_up_tmp_directory, {MyModule, :my_setup_function}]

defp clean_up_tmp_directory(_context) do
  # perform setup
  :ok
end

# A block
setup_all do
  [conn: Plug.Conn.build_conn()]
end
```

From [setup_all/1](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup_all/1):

> "The context returned by `setup_all/1` will be available in all subsequent `setup_all`, `setup`, and the `test` itself. For instance, the `conn` from the previous example can be accessed as:
>
> ```elixir
> test "fetches current users", %{conn: conn} do
>   # ...
> end
> ```"

`setup_all(context, block)` is the 2-arity form: identical but with the context bound as the first argument and a block as the second. From [setup_all/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup_all/2):

> "Defines a callback to be run before all tests in a case.
>
> Similar as `setup_all/1` but also takes a context. The second argument must be a block. See the "Context" section in the module documentation."

```elixir
setup_all _context do
  [conn: Plug.Conn.build_conn()]
end
```

#### `on_exit` registered inside `setup_all`

`on_exit/2` callbacks registered inside a `setup_all/1` callback are **not** per-test; they run once, after all tests in the module, in a single dedicated process, in the reverse order of their `setup_all/1` registration. From [setup_all/1 "On-Exit Handlers"](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup_all/1):

> "On-exit handlers that you register inside `setup_all/1` callbacks are executed at once after all tests in the module have been run. They are all executed *in the same process*, which is a separate process dedicated to running these handlers. These handlers are executed in the reverse order of their respective `setup_all/1` callbacks."

```elixir
setup_all do
  Database.create_table_for(__MODULE__)

  on_exit(fn ->
    Database.drop_table_for(__MODULE__)
  end)

  :ok
end
```

From [setup_all/1 "Handlers"](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#setup_all/1):

> "The handler in the example above will be executed only once, after running all tests in the module."

### When to use `setup` vs `setup_all`

- `setup_all` runs **once per module**, before any test, in a dedicated process. Use it only for expensive, read-only, or genuinely shared fixtures whose state does not change between tests (e.g. compiling a list of static fixtures, creating a database schema). Anything it returns is available to every `setup` and every test in the module.
- `setup` runs **once per test**, in the test's own process. This is the default: fresh state per test, isolated failures, and safe parallelism in `async: true` modules. Prefer per-test `setup` for anything that mutates state.

From [ExUnit.Callbacks.html](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html):

> "The `setup_all` callbacks are invoked only once per module, before any test is run. All `setup` callbacks are run before each test. No callback is run if the test case has no tests or all tests have been filtered out."

> "A test module can define multiple `setup` and `setup_all` callbacks, and they are invoked in order of appearance."

Rule of thumb: default to `setup`. Reach for `setup_all` only when a fixture is genuinely module-wide and either expensive to rebuild or intentionally shared. Because `setup_all` mutates state outside any individual test's supervision tree, it is easy to introduce cross-test coupling — if two tests need different fixture state, give each its own `setup` instead.

### Named setups: `setup [:foo, :bar]` and `{Module, :fun}`

Both `setup` and `setup_all` accept a list of named callbacks (atoms or `{module, function}` tuples). The named functions receive the context as their single argument and may return any of the valid context shapes:

```elixir
defmodule ExampleContextTest do
  use ExUnit.Case

  setup [:step1, :step2, :step3, {OtherModule, :step4}]

  defp step1(_context), do: [step_one: true]
  defp step2(_context), do: {:ok, step_two: true} # return values with shape of {:ok, keyword() | map()} allowed
  defp step3(_context), do: :ok # Context not modified

  test "context was modified", context do
    assert context[:step_one] == true
    assert context[:step_two] == true
  end
end
```

Named setups are the idiomatic replacement for nested `describe` blocks (which ExUnit forbids). Each `describe` can declare its own list of named setups so that a reader can see at a glance what fixtures a block depends on:

```elixir
defmodule UserManagementTest do
  use ExUnit.Case, async: true

  describe "when user is logged in and is an admin" do
    setup [:log_user_in, :set_type_to_admin]
    # ...
  end

  describe "when user is logged in and is a manager" do
    setup [:log_user_in, :set_type_to_manager]
    # ...
  end

  defp log_user_in(_context), do: :ok
  defp set_type_to_admin(_context), do: :ok
  defp set_type_to_manager(_context), do: :ok
end
```

From [ExUnit.Case.html `describe/2`](https://hexdocs.pm/ex_unit/ExUnit.Case.html#describe/2):

> "Inside a block, `ExUnit.Callbacks.setup/1` may be invoked and it will define a setup callback to run only for the current block."

A named setup is a plain private function, so it can be unit-tested and reused across multiple `describe` blocks or even multiple modules (via a shared helper module and the `{Module, :fun}` form).

### `on_exit/1` and `on_exit/2`

`on_exit/2` registers a callback that runs once the test exits. The 1-arity form is the default-argument overload (`name_or_ref = make_ref()`). From [on_exit/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#on_exit/2):

> "Registers a callback that runs once the test exits.
>
> `callback` is a function that receives no arguments and runs in a separate process than the caller. Its return value is irrelevant and is discarded.
>
> `on_exit/2` is usually called from `setup/1` and `setup_all/1` callbacks, often to undo the action performed during the setup.
>
> However, `on_exit/2` may also be called dynamically. An "ID" (the `name_or_ref` argument) can be used to guarantee that the callback will be invoked only once. ExUnit uses this term to identify an `on_exit/2` handler: if you want to override a previous handler, for example, use the same `name_or_ref` across multiple `on_exit/2` calls.
>
> If `on_exit/2` is called inside `setup/1` or inside a test, it's executed in a blocking fashion after the test exits and *before running the next test*. This means that no other test from the same test case will be running while the `on_exit/2` callback for a previous test is running. `on_exit/2` is executed in a different process than the test process. On the other hand, if `on_exit/2` is called inside a `setup_all/1` callback then `callback` is executed after running *all tests* (see `setup_all/1` for more information)."

Spec: `@spec on_exit(term(), (-> term())) :: :ok`.

```elixir
setup do
  File.write!("fixture.json", "{}")
  on_exit(fn -> File.rm!("fixture.json") end)
end
```

The `name_or_ref` allows overriding a previously registered handler by reusing the same ID:

```elixir
setup do
  on_exit(:drop_table, fn ->
    Database.drop_table()
  end)
end

test "a test that shouldn't drop the table" do
  on_exit(:drop_table, fn -> :ok end)
end
```

From [on_exit/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#on_exit/2):

> "Relying too much on overriding callbacks like this can lead to test cases that are hard to understand and with too many layers of indirection. However, it can be useful in some cases or for library authors, for example."

The defining property of `on_exit/2` is that it **always runs**, even if the test fails or raises, and even if a later setup callback fails. Contrast this with `setup`/`setup_all`: a failure in either short-circuits all remaining setup callbacks. From [ExUnit.Callbacks.html](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html):

> "A registered `on_exit/2` callback will always run, while failures in `setup` and `setup_all` will stop all remaining setup callbacks from executing."

### Callback execution order and the process model

ExUnit uses several processes per test case. The canonical statement of ordering and process boundaries is in [ExUnit.Case.html "Process Architecture"](https://hexdocs.pm/ex_unit/ExUnit.Case.html#module-process-architecture):

> "1. First, all `ExUnit.Callbacks.setup_all/1` callbacks run in a single process, sequentially, in the order they were defined.
>
> 2. Then, a new process is spawned for the test itself. In this process, first all `ExUnit.Callbacks.setup/1` callbacks run, in the order they were defined. Then, the test itself is executed.
>
> 3. After the test exits, a new process is spawned to run all `ExUnit.Callbacks.on_exit/2`, in the reverse order they were defined."

The per-test lifecycle, from [ExUnit.Callbacks.html](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html):

> "Here is a rundown of the life cycle of the test process:
>
> 1. the test process is spawned
> 2. it runs `setup/2` callbacks
> 3. it runs the test itself
> 4. the test process exits with reason `:shutdown`
> 5. it stops all supervised processes synchronously
> 6. `on_exit/2` callbacks are executed in a separate process"

Summary:

| Step | Process | Order |
|---|---|---|
| `setup_all` | one process per module | in definition order, once per module, before any test |
| `setup` | the test process | in definition order, once per test, before the test body |
| test body | the test process | once per test |
| supervised children | the test process's supervisor | stopped synchronously after the test exits |
| `on_exit` (from `setup`/test) | a separate process | in reverse definition order, blocking before the next test |
| `on_exit` (from `setup_all`) | a separate per-module process | in reverse definition order, once, after all tests |

Because the test process exits with reason `:shutdown`, anything it links to also exits asynchronously — which is exactly why `start_supervised/2` exists (see below): it guarantees supervised processes have fully terminated before the next test starts.

### Starting supervised processes

`start_supervised/2` starts a child under the test's own supervisor. The supervisor is linked to the test process and — critically — it and its children are guaranteed to terminate before any `on_exit/2` callback runs. This is the recommended way to start processes from a setup callback, in preference to a manual `start_link`.

From [start_supervised/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#start_supervised/2):

> "Starts a child process under the test supervisor.
>
> It expects a child specification or a module, similar to the ones given to `Supervisor.start_link/2`. For example, if your application starts a supervision tree by running:
>
> ```elixir
> Supervisor.start_link([MyServer, {OtherSupervisor, ...}], ...)
> ```
>
> You can start those processes under test in isolation by running:
>
> ```elixir
> start_supervised(MyServer)
> start_supervised({OtherSupervisor, :initial_value})
> ```
>
> A keyword list can also be given if there is a need to change the child specification for the given child process:
>
> ```elixir
> start_supervised({MyServer, :initial_value}, restart: :temporary)
> ```
>
> See the `Supervisor` module for a discussion on child specifications and the available specification keys.
>
> The started process is not linked to the test process and a crash will not necessarily fail the test. To start and link a process to guarantee that any crash would also fail the test use `start_link_supervised!/2`.
>
> This function returns `{:ok, pid}` in case of success, otherwise it returns `{:error, reason}`.
>
> The advantage of starting a process under the test supervisor is that it is guaranteed to exit before the next test starts. Therefore, you don't need to remove the process at the end of your tests via `stop_supervised/1`. You only need to use `stop_supervised/1` if you want to remove a process from the supervision tree in the middle of a test, as simply shutting down the process would cause it to be restarted according to its `:restart` value.
>
> Finally, since Elixir v1.17.0, the test supervisor has both `$ancestors` and `$callers` key in its process dictionary pointing to the test process. This means developers can invoke `Process.get(:\"$callers\", [])` in their `start_link` function and forward it to the spawned process, which may set `Process.put(:\"$callers\", callers)` during its initialization. This may be useful in projects who track process ownership during tests. You can learn more about these keys in the `Task` module."

Spec:

```elixir
@spec start_supervised(
  Supervisor.child_spec() | module() | {module(), term()},
  child_spec_overrides()
) :: Supervisor.on_start_child()
```

`start_supervised!/2` (since v1.6.0) is the bang form. From [start_supervised!/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#start_supervised!/2):

> "Same as `start_supervised/2` but returns the PID on success and raises if not started properly."

```elixir
@spec start_supervised!(
  Supervisor.child_spec() | module() | {module(), term()},
  child_spec_overrides()
) :: pid()
```

`start_link_supervised!/2` (since v1.14.0) additionally links the started process to the test process, so a crash in the child fails the test. From [start_link_supervised!/2](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#start_link_supervised!/2):

> "Same as `start_supervised!/2` but links the started process to the test process.
>
> If the process that was started crashes, the crash is propagated to the test process, failing the test and printing the cause of the crash.
>
> Note that if the started process terminates before it is linked to the test process, this function will exit with reason `:noproc`."

#### To link or not to link

From [start_link_supervised!/2 "To link or not to link"](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#start_link_supervised!/2):

> "When using `start_link_supervised!/2`, the test process will be linked to the spawned processes. When the test process exits, it exits with reason `:shutdown`, and the crash signal propagates to all linked processes virtually simultaneously, which can lead to processes terminating in an unpredictable order if they are not trapping exits. This is particularly problematic when you have processes that the test starts with `start_link_supervised!/2` and that depend on each other.
>
> If you need guaranteed shutdown order, use `start_supervised!/2`. This way the test process exiting does not affect the started processes, and they will be shut down *by the test supervisor* in reverse order, ensuring graceful termination."

Practical guidance:

- Default to `start_supervised/2` (or `start_supervised!/2` if you want a crash on start failure). The started process is **not** linked to the test process, so an unrelated crash in the child does not automatically fail the test — but the test supervisor guarantees the child is gone before the next test.
- Use `start_link_supervised!/2` when you specifically want a crash in the child to fail the test. Be aware that linked children terminate together when the test process exits with `:shutdown`, so termination order is not guaranteed for interdependent children.
- You do **not** need to call `stop_supervised/1` at the end of a test — the test supervisor tears children down automatically. Call it only when you need to remove a child *mid-test* (e.g. to assert on its exit), because a plain `Process.exit/2` or `GenServer.stop/2` would just cause the supervisor to restart it according to its `:restart` policy.

### `stop_supervised/1` and `stop_supervised!/1`

These remove a child started via `start_supervised/2` by its child-spec `:id` (which defaults to the module name). From [stop_supervised/1](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#stop_supervised/1):

> "Stops a child process started via `start_supervised/2`.
>
> This function expects the `id` in the child specification. For example:
>
> ```elixir
> {:ok, _} = start_supervised(MyServer)
> :ok = stop_supervised(MyServer)
> ```
>
> It returns `:ok` if there is a supervised process with such `id`, `{:error, :not_found}` otherwise."

```elixir
@spec stop_supervised(id :: term()) :: :ok | {:error, :not_found}
```

`stop_supervised!/1` (since v1.10.0) raises instead of returning `{:error, :not_found}`. From [stop_supervised!/1](https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html#stop_supervised!/1):

> "Same as `stop_supervised/1` but raises if it cannot be stopped."

```elixir
@spec stop_supervised!(id :: term()) :: :ok
```

### Async safety for callbacks

The rules for callbacks under `async: true` are the same as for the tests themselves: the whole module runs concurrently with other async modules, so no test (or its `setup`) may change global state. From [ExUnit.Case.html `:async`](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "`:async` - configures tests in this module to run concurrently with tests in other modules. Tests in the same module never run concurrently (with the exception of tests run via the `:parameterize` option - see below). It should be enabled only if tests do not change any global state. Defaults to `false`."

In practice "global state" means anything not owned by the per-test process or its test supervisor: application environment (`Application.put_env/3`), ETS tables not created by the test, files on disk under a shared path, the system clock, globally registered processes/names, network endpoints, and any shared cache. Because each test runs in its own process with its own test supervisor, anything started via `start_supervised/2` is isolated per test and safe under `async: true`.

Two process-boundary consequences worth remembering:

- `setup_all` runs in a separate per-module process and persists across all tests in the module; if it mutates shared global state, the module cannot safely be `async: true`. Prefer `setup` for anything that touches the outside world.
- `on_exit` and `setup_all` callbacks run outside the test process, so the global `:capture_log` default does not silence their log output. From [ExUnit.html `:capture_log`](https://hexdocs.pm/ex_unit/ExUnit.html#configure/1):

> "Note that `on_exit` and `setup_all` callbacks may still log, as they run outside of the testing process. To silent those, you can use `ExUnit.CaptureLog.capture_log/2` or consider disabling logging altogether."

### Common mistakes

- Starting processes with a bare `GenServer.start_link/2` or `Supervisor.start_link/2` from `setup` instead of `start_supervised/2`; the manual process is not tied to the test supervisor and can leak into the next test.
- Calling `stop_supervised/1` at the end of every test "to clean up"; the test supervisor already stops the child synchronously, so this is redundant unless you need to stop it mid-test.
- Using `setup_all` for state that differs between tests, then being surprised that the first test's mutations bleed into later tests (the `setup_all` state is shared and lives in a separate process).
- Expecting a failing `setup` callback to still run later `setup` callbacks; it does not — a setup failure short-circuits the remaining setups (only `on_exit/2` is guaranteed to run).
- Assuming `on_exit/2` registered inside `setup` runs immediately at the end of `setup`; it runs after the test exits, in a separate process, blocking before the next test.
- Believing `setup_all` sees `@tag` or `@describetag` values; it only sees `@moduletag` values.
- Returning an invalid value (e.g. `{:error, ...}` or a bare `pid`) from `setup`/`setup_all`; this fails the test (or the whole module, for `setup_all`).
- Marking a module `async: true` while its `setup_all` or `setup` mutates application env, shared ETS, or files in a fixed path.
- Using `start_link_supervised!/2` for a tree of interdependent processes and depending on a specific teardown order; the linked processes exit together on `:shutdown` in unspecified order. Use `start_supervised!/2` when you need guaranteed reverse-order shutdown.

### Examples

#### `setup` returning a context value, with a paired `on_exit`

```elixir
defmodule AssertionTest do
  use ExUnit.Case, async: true

  # "setup_all" is called once per module before any test runs
  setup_all do
    IO.puts("Starting AssertionTest")

    # Context is not updated here
    :ok
  end

  # "setup" is called before each test
  setup do
    IO.puts("This is a setup callback for #{inspect(self())}")

    on_exit(fn ->
      IO.puts("This is invoked once the test is done. Process: #{inspect(self())}")
    end)

    # Returns extra metadata to be merged into context.
    # Any of the following would also work:
    #
    #     {:ok, %{hello: "world"}}
    #     {:ok, [hello: "world"]}
    #     %{hello: "world"}
    #
    [hello: "world"]
  end

  # Same as above, but receives the context as argument
  setup context do
    IO.puts("Setting up: #{context.test}")

    # We can simply return :ok when we don't want to add any extra metadata
    :ok
  end

  # Setups can also invoke a local or imported function that returns a context
  setup :invoke_local_or_imported_function

  test "always pass" do
    assert true
  end

  test "uses metadata from setup", context do
    assert context[:hello] == "world"
    assert context[:from_named_setup] == true
  end

  defp invoke_local_or_imported_function(context) do
    [from_named_setup: true]
  end
end
```

#### Starting a process under the test supervisor

```elixir
defmodule MyApp.RegistryTest do
  use ExUnit.Case, async: true

  setup do
    # Starts under the test supervisor; the child is guaranteed to
    # terminate before the next test, so no manual cleanup is needed.
    {:ok, registry} = start_supervised(MyApp.Registry)
    {:ok, registry: registry}
  end

  test "stores and retrieves values", %{registry: registry} do
    assert MyApp.Registry.put(registry, :hello, :world) == :ok
    assert MyApp.Registry.get(registry, :hello) == :world
  end
end
```

#### Inspecting a tag inside `setup` and branching

```elixir
defmodule ExampleTagModificationTest do
  use ExUnit.Case

  setup %{login_as: username} do
    {:ok, current_user: username}
  end

  @tag login_as: "max"
  test "tags modify context", context do
    assert context[:login_as] == "max"
    assert context[:current_user] == "max"
  end
end
```

#### Mid-test teardown via `stop_supervised!`

```elixir
defmodule MyApp.RegistryRestartTest do
  use ExUnit.Case

  setup do
    {:ok, _} = start_supervised({MyApp.Registry, name: :test_registry})
    :ok
  end

  test "registry survives a restart", context do
    # ... some assertions ...

    # Remove the child mid-test (a plain stop would just let the
    # supervisor restart it, per its :restart policy).
    :ok = stop_supervised(MyApp.Registry)
    {:ok, _} = start_supervised({MyApp.Registry, name: :test_registry})

    # ... more assertions ...
  end
end
```

Sources: https://hexdocs.pm/ex_unit/ExUnit.Callbacks.html, https://hexdocs.pm/ex_unit/ExUnit.Case.html, https://hexdocs.pm/ex_unit/ExUnit.html, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/callbacks.ex

## CaptureIO and CaptureLog

ExUnit ships two unrelated capture helpers that are easy to confuse:

- `ExUnit.CaptureIO` captures writes to **IO devices** (`IO.puts`, `IO.write`, `IO.inspect`, `IO.gets`, prompts) by temporarily swapping the process's group leader (or another named device) for a `StringIO` buffer.
- `ExUnit.CaptureLog` captures **`Logger` messages** by installing a `:logger` handler through the internal `ExUnit.CaptureServer` and routing matching events into a per-capture `StringIO` buffer.

The `@tag capture_log` tag (and the global `:capture_log` option) is a third, higher-level mechanism: it wraps every test body in `ExUnit.CaptureLog.with_log/2` so log output is suppressed on success and printed only on failure. It does **not** let test code assert on log content — for that you call `ExUnit.CaptureLog` directly.

Both modules are imported on demand (`import ExUnit.CaptureIO` / `import ExUnit.CaptureLog`); they are not auto-imported by `use ExUnit.Case`.

### Public API at a glance

| Module | Functions | Returns |
|---|---|---|
| `ExUnit.CaptureIO` | `capture_io/1,2,3`, `with_io/1,2,3` | `capture_io` → captured `String.t()`; `with_io` → `{result, captured_output}` |
| `ExUnit.CaptureLog` | `capture_log/2`, `with_log/2` | `capture_log` → captured `String.t()`; `with_log` → `{result, captured_log}` |

There is **no `ExUnit.CaptureLog.log/2`**. The public surface of `ExUnit.CaptureLog` is only `capture_log/2` and `with_log/2` (this matches the Overview summary above). To *emit* a log message at an arbitrary level, use `Logger.log/3` from the `Logger` module; to *capture* one, use `capture_log/2` / `with_log/2`.

### ExUnit.CaptureIO

From [ExUnit.CaptureIO.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html):

> "By default, `capture_io` replaces the `Process.group_leader/0` of the current process with a `StringIO` process when evaluating the given function. [...] Capturing the group leader of the current process is safe to run concurrently, under `async: true` tests."

#### Signatures

```elixir
capture_io(fun)                                  :: String.t()
capture_io(device_or_input_or_opts, fun)         :: String.t()
capture_io(device, input_or_opts, fun)           :: String.t()

with_io(fun)                                     :: {any(), String.t()}
with_io(device_or_input_or_opts, fun)            :: {any(), String.t()}
with_io(device, input_or_opts, fun)              :: {any(), String.t()}
```

Where `device` is an atom (`:stdio`, `:standard_io`, `:stderr`, `:standard_error`) or a pid; `input_or_opts` is a binary input string or a keyword list of options. The full option type is:

```elixir
@type capture_io_opts :: [
  input: String.t(),
  capture_prompt: boolean(),
  encoding: :unicode | :latin1
]
```

`capture_io/*` returns only the captured output and discards the function's result; `with_io/*` returns both as a 2-tuple `{result, captured_output}`. Internally `capture_io/*` delegates to `with_io/*` and returns just the captured string.

From [ExUnit.CaptureIO.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html):

```elixir
defmodule AssertionTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  test "example" do
    assert capture_io(fn -> IO.puts("a") end) == "a\n"
  end

  test "another example" do
    assert with_io(fn ->
      IO.puts("a")
      IO.puts("b")
      2 + 2
    end) == {4, "a\nb\n"}
  end
end
```

#### Device argument

| First argument | What is captured |
|---|---|
| *(omitted)* / `:stdio` / `:standard_io` | Output to the current process's group leader (standard output). The default. Per-process, fully async-safe; `==` assertions are safe. |
| `:stderr` / `:standard_error` | Output to standard error. Shared across all tests — safe to run concurrently but content from other tests may be captured; use `=~`, never `==`. |
| `pid` | Output from a process whose group leader is that `pid` (since v1.17.0). **Not** async-safe; tests capturing a specific pid cannot run concurrently. |

#### Options

| Option | Default | Description |
|---|---|---|
| `:input` | `""` | A string fed to the device as stdin, consumed by `IO.gets`/`IO.read`. |
| `:capture_prompt` | `true` | Whether prompts passed to `IO.get*` are captured into the output. Ignored for devices other than `:stdio`. |
| `:encoding` (v1.10.0) | `:unicode` | Encoding of the IO device; allowed values are `:unicode` and `:latin1`. |

#### Feeding stdin with `:input`

From [ExUnit.CaptureIO.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html):

```elixir
iex> capture_io("this is input", fn ->
...>   input = IO.gets("> ")
...>   IO.write(input)
...> end) == "> this is input"
true

iex> capture_io([input: "this is input", capture_prompt: false], fn ->
...>   input = IO.gets("> ")
...>   IO.write(input)
...> end) == "this is input"
true
```

`capture_io("this is input", fun)` is sugar for `capture_io(:stdio, [input: "this is input"], fun)`.

#### Capturing stderr

There is no `:capture_device` option; pass `:stderr` (or `:standard_error`) as the device argument:

```elixir
import ExUnit.CaptureIO

assert capture_io(:stderr, fn -> IO.write(:stderr, "boom") end) =~ "boom"
```

From [ExUnit.CaptureIO.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html):

> "`:stderr`, `:standard_error` — captures all IO to standard error (represented internally by an Erlang process named `:standard_error`). This is safe to run concurrently but it will capture the output of any other test writing to the same named device."

> "Note it is fine to use `==` with `:stdio`. However, `:stderr` is shared across all tests, so you will want to use `=~` instead of `==` for assertions on `:stderr` if your tests are async. In particular, avoid empty captures on `:stderr` with async tests: otherwise, if the standard error of any other test is captured, the test will fail."

#### Limitations and what `capture_io` does NOT capture

`capture_io` only intercepts IO that flows through the captured device's `StringIO` (i.e. through `:io` / the Erlang group-leader protocol). It does **not** capture:

- Output written by NIFs or ports directly to file descriptors 1/2, bypassing `:io`.
- Output from another process unless you pass that process's group-leader pid explicitly (since v1.17.0); a process whose group leader was set elsewhere is not captured by a default `capture_io`.
- `Logger` messages — those go to `ExUnit.CaptureLog`, not `capture_io`.

Two nesting/concurrency rules from the docs:

> "If capturing a named device asynchronously, an input can only be given to the first capture. Any further capture that is given to a capture on that device will raise an exception and would indicate that the test should be run synchronously."

> "Similarly, once a capture on a named device has begun, the encoding on that device cannot be changed in a subsequent concurrent capture. An error will be raised in this case."

### ExUnit.CaptureLog

From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

> "Captures Logger messages generated when evaluating `fun`. Returns the binary which is the captured output. The captured log messages will be formatted using `Logger.default_formatter/1` by default. Any option, besides `:level` or `:formatter`, will be forwarded as an override to the default formatter."

#### Signatures

```elixir
capture_log(opts \\ [], fun)  :: String.t()
with_log(opts \\ [], fun)     :: {result, log :: String.t()} when result: any
```

```elixir
@type capture_log_opts :: [
  level: Logger.level() | nil,
  formatter: {module(), term()} | nil
]
```

As with the IO helpers, `capture_log/2` returns only the captured log string (it internally delegates to `with_log/2` and discards the result), while `with_log/2` returns `{result, captured_log}`. From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

```elixir
defmodule AssertionTest do
  use ExUnit.Case
  import ExUnit.CaptureLog
  require Logger

  test "example" do
    {result, log} =
      with_log(fn ->
        Logger.error("log msg")
        2 + 2
      end)

    assert result == 4
    assert log =~ "log msg"
  end
end
```

`require Logger` is needed before calling `Logger.error/1` and friends, because the Logger macros are compile-time expanded.

#### Options

| Option | Default | Description |
|---|---|---|
| `:level` | `nil` (= capture all levels) | Threshold filter: messages **at or above** this level are captured; messages below it are ignored. |
| `:formatter` | `Logger.default_formatter/1` | A `{module, config}` tuple overriding the default formatter. |

Other keywords (e.g. `:colors`, `:metadata`, `:format`) are not consumed by `CaptureLog` itself — they are forwarded as overrides to `Logger.default_formatter/1`. There is no `:filter` or `:group_leader` option on `ExUnit.CaptureLog`.

#### Level filtering semantics

`:level` is a **threshold**, not an exact match. Messages whose level is **at or above** the configured level are captured. Logger levels are ordered most-severe to least-severe:

```
:emergency > :alert > :critical > :error > :warning > :notice > :info > :debug
```

From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

> "For example, if the log level is set to `:error`, then any message with the lower level will be ignored. The default level is `nil`, which will capture all messages."

So `capture_log([level: :error], fun)` captures `:emergency`, `:alert`, `:critical`, and `:error`, but not `:warning`, `:notice`, `:info`, or `:debug`.

```elixir
log =
  capture_log([level: :error], fn ->
    Logger.debug("ignored")
    Logger.error("captured")
  end)

assert log =~ "captured"
refute log =~ "ignored"
```

#### Interaction with the global `Logger.level/0`

From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

> "Note that this setting does not override the overall `Logger.level/0` value. Therefore, if `Logger.level/0` is set to a higher level than the one configured in this function, no message will be captured. The behaviour is undetermined if async tests change `Logger` level."

In practice: if your config sets `config :logger, level: :warning`, a `Logger.debug/1` call inside `capture_log([level: :debug], ...)` will still not appear, because Logger itself drops it before `CaptureLog` ever sees it. Do not change `Logger` level from async tests.

### The `capture_log` tag vs the `ExUnit.CaptureLog` module

These two are independent and complementary:

- **`@tag capture_log: ...`** (and the global `ExUnit.start(capture_log: ...)` / `ExUnit.configure(capture_log: ...)` option) wraps the whole test body in `ExUnit.CaptureLog.with_log/2`. Logs are captured while the test runs; on **failure** the formatter prints them under `"The following output was logged:"`; on **success** they are discarded. This is purely for keeping passing-test output quiet — it gives test code no handle on the captured text.
- **`ExUnit.CaptureLog.capture_log/2` / `with_log/2`** let you capture logs and **assert on their content** (`log =~ "..."`). Use these when a test cares about *what* was logged.

Use the **tag** to suppress noise; use the **module** to assert on log output.

#### Tag forms and global configuration

| Form | Effect |
|---|---|
| `@tag capture_log: true` (or `@tag :capture_log`) | Wrap this test in `with_log/2`; show logs only on failure. |
| `@tag capture_log: false` | Disable capture for this test, even if `capture_log` is enabled globally. |
| `@tag capture_log: [level: :debug]` | Capture this test with the given opts list, passed straight to `with_log/2`. |
| `@moduletag capture_log: ...` | Same, applied to every test in the module. |

Globally, via `test_helper.exs`:

```elixir
# Off by default; enable for the whole suite
ExUnit.start(capture_log: true)

# Or capture with a specific level for the whole suite
ExUnit.start(capture_log: [level: :warning])
```

The upstream default is `capture_log: false` (see the Overview section's option reference, sourced from the ExUnit application env). The global option accepts only a boolean or `[level: LEVEL]` — its type is `boolean() | [{:level, Logger.level()}]`. Per-test `@tag`/`@moduletag` overrides take precedence over the global default.

From [ExUnit.html `configure/1`](https://hexdocs.pm/ex_unit/ExUnit.html#configure/1):

> "`:capture_log` - if ExUnit should default to keeping track of log messages and print them on test failure. Can be overridden for individual tests via `@tag capture_log: false`. This can also be configured to a specific level with `capture_log: [level: LEVEL]`, to capture all logs but only keep those above `LEVEL`."

#### Caveat: `setup_all` and `on_exit`

Because the tag wraps only the test body, logs emitted from `setup_all` or `on_exit` (which run outside the test process) are **not** captured by the tag. From [ExUnit.html `:capture_log`](https://hexdocs.pm/ex_unit/ExUnit.html#configure/1):

> "Note that `on_exit` and `setup_all` callbacks may still log, as they run outside of the testing process. To silent those, you can use `ExUnit.CaptureLog.capture_log/2` or consider disabling logging altogether."

To suppress that noise entirely, disable the default console handler in `config/test.exs`:

```elixir
# config/test.exs
import Config
config :logger, :default_handler, false
```

### Common patterns

#### Asserting on stdout

```elixir
import ExUnit.CaptureIO

test "prints a greeting" do
  assert capture_io(fn -> MyApp.greet("world") end) == "Hello, world!\n"
end

# When you also need the function's return value:
test "returns :ok and prints a report" do
  {result, output} = with_io(fn -> MyApp.run() end)
  assert result == :ok
  assert output =~ "expected line"
end
```

#### Feeding stdin

```elixir
import ExUnit.CaptureIO

test "echoes the entered name" do
  output =
    capture_io([input: "Ada\n"], fn ->
      MyApp.prompt_for_name()
    end)

  assert output =~ "Ada"
end
```

#### Asserting on stderr (async-safe)

```elixir
import ExUnit.CaptureIO

test "writes a warning to stderr" do
  # :stderr is shared across tests under async: use =~, never ==
  assert capture_io(:stderr, fn -> MyApp.warn("disk almost full") end) =~ "disk almost full"
end
```

#### Asserting on Logger output

```elixir
import ExUnit.CaptureLog
require Logger

test "logs an error on failure" do
  log =
    capture_log(fn ->
      MyApp.handle(:error)
    end)

  assert log =~ "error"
end

test "logs at the configured threshold" do
  log =
    capture_log([level: :warning], fn ->
      Logger.debug("ignored")
      Logger.warning("deprecation coming")
    end)

  assert log =~ "deprecation"
  refute log =~ "ignored"
end
```

### Async safety

| Capture | Async-safe? | Assertion advice |
|---|---|---|
| `capture_io` default (`:stdio`, current process group leader) | **Yes** — per-process group-leader swap | `==` is safe |
| `capture_io(:stderr, ...)` (named device) | Safe to run concurrently, but **shared** across tests | Use `=~`; never `==`; never assert on empty stderr |
| `capture_io(pid, ...)` for another process (v1.17.0) | **No** — tests capturing a pid cannot run concurrently | Mark `async: false` |
| `capture_log/2` / `with_log/2` | Yes — per-capture `StringIO`, but sibling tests' logs may interleave | Use `=~` |

From [ExUnit.CaptureLog.html](https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html):

> "Note that when the async is set to true on `use ExUnit.Case`, messages from other tests might be captured. This is OK as long you consider such cases in your assertions, typically by using the `=~` operator to perform partial matches."

Recommendation: default to `async: true` with the `capture_log` tag enabled; use `capture_io`/`capture_log` for assertions, and prefer `=~` for any capture that touches a shared device or the shared Logger handler.

### Common mistakes

- Calling `ExUnit.CaptureLog.log/2` — there is no such function. Use `capture_log/2` / `with_log/2` to capture, or `Logger.log/3` to emit.
- Expecting `capture_io` to capture `Logger` output; it captures IO-device writes only. Use `ExUnit.CaptureLog` for `Logger`.
- Expecting the `@tag capture_log: true` tag to let the test assert on the captured log; it only suppresses-on-success / shows-on-failure. Call `ExUnit.CaptureLog` inside the test to assert on content.
- Using `==` on `:stderr` captures under `async: true`; `:stderr` is shared, so partial matches with `=~` are required.
- Asserting on an empty `:stderr` capture under async; another test's stderr can leak in and make the assertion spuriously pass or fail.
- Passing `:level` to `capture_log` and expecting an exact-level match; it is a threshold (at-or-above), not equality.
- Setting `capture_log([level: :debug], ...)` and expecting `Logger.debug/1` output when the global `Logger.level/0` is higher — Logger drops the message before `CaptureLog` sees it.
- Looking for a `:capture_device` option on `capture_io`; stderr is selected by passing `:stderr` (or `:standard_error`) as the device argument.
- Changing `Logger` level from within async tests; the docs explicitly warn the behaviour is undetermined.
- Expecting the `capture_log` tag to silence logs emitted from `setup_all`/`on_exit`; it wraps only the test body.

Sources: https://hexdocs.pm/ex_unit/ExUnit.CaptureIO.html, https://hexdocs.pm/ex_unit/ExUnit.CaptureLog.html, https://hexdocs.pm/ex_unit/ExUnit.html#configure/1, https://hexdocs.pm/ex_unit/ExUnit.Case.html, https://hexdocs.pm/logger/Logger.html, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_io.ex, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_log.ex, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/capture_server.ex, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/runner.ex

## Doctests

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Extract test cases from the documentation."
>
> "Doctests allow us to generate tests from code examples found in `@moduledoc` and `@doc` attributes."

A doctest is a test generated from the `iex>` examples you write inside a module's `@moduledoc` or a function's `@doc`. The `doctest/2` macro (auto-imported by `use ExUnit.Case`) reads those examples out of the compiled module's documentation and turns each one into an ExUnit test of type `:doctest`. This keeps your documentation executable: any example that stops producing the documented output fails the build.

The `doctest/2` macro is the entry point for module-based doctests; `doctest_file/2` (since v1.15.0) does the same for examples in standalone Markdown/text files such as a `README.md`.

### The `doctest/2` macro

It is a macro with the default-argument form, so both `doctest/1` (no options) and `doctest/2` (with options) exist:

```elixir
defmacro doctest(module, opts \\ [])
```

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "This macro is auto-imported with every `ExUnit.Case`. Calling `doctest(Module)` will generate tests for all doctests found in the module."

The minimal usage is a one-line test module:

```elixir
defmodule MyModuleTest do
  use ExUnit.Case, async: true
  doctest MyModule
end
```

A single `doctest MyModule` call generates one test per `iex>` example found in `MyModule`'s `@moduledoc` **and** every public function/macro's `@doc`. You do not write `doctest` per function; one call covers the whole module. To test a different module's docs, pass that module instead (`doctest AnotherModule`). `doctest/1` is simply the no-options call of `doctest/2` — to pass any option you must use `doctest/2`.

#### Options

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html), `doctest/2` accepts:

| Option | Default | Description |
|---|---|---|
| `:except` | `[]` | Generate tests for all examples **except** those listed. List of `{function, arity}` tuples and/or the `:moduledoc` atom. |
| `:only` | `[]` | Generate tests **only** for those listed. Same `{function, arity}` / `:moduledoc` element shape as `:except`. |
| `:import` | `false` | When `true`, the module under test is `import`ed into the generated test so its functions can be called without the module prefix. |
| `:tags` | `[]` | A list of tags applied to every generated doctest (e.g. `[:slow]` or `[external: true]`). |
| `:inspect_opts` | `[]` | A keyword list forwarded to `inspect/2` when comparing values, useful for opaque/pretty-printed output. |

There is **no `:imported` option**; the only prefix-related option is the boolean `:import`.

### Doctest syntax in `@moduledoc` and `@doc`

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Every new test starts on a new line, with an `iex>` prefix."

A doctest example is an `iex>` line (the expression) optionally followed by a result line (the expected return value):

```elixir
@doc """
Returns the sum of two numbers.

## Examples

    iex> MyApp.Math.add(1, 2)
    3
"""
def add(a, b), do: a + b
```

#### Multi-line expressions

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Multiline expressions can be used by prefixing subsequent lines with either `...>` (recommended) or `iex>`."
>
> "The expected result should start the line after the `iex>` and `...>` line(s) and be terminated by a newline."

```elixir
iex> Enum.map([1, 2, 3], fn x ->
...>   x * 2
...> end)
[2, 4, 6]
```

For examples copied straight out of an interactive `iex` session, numbered prompts are also accepted:

```elixir
iex(1)> [1 + 2,
...(1)>  3]
[3, 3]
```

#### `iex>` examples vs plain code examples

**Only** lines prefixed with `iex>` (or its `...>` / `iex(N)>` continuations) are turned into tests. Plain code in `@doc` / `@moduledoc` — a `defmodule` snippet, an unprefixed function call, prose — is documentation only and is never executed by doctest. If you want an example to run, it must be written as an `iex>` example.

#### Multiple results vs separate tests (variable binding)

Consecutive `iex>` lines **without a blank line between them** form a single test and share bindings, exactly like an `iex` session. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Multiple results can be checked within the same test:"

```elixir
iex> a = 1
1
iex> a + 1
2
```

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "If you want to keep any two tests separate, add an empty line between them."

A blank line starts a fresh test whose bindings do **not** carry over, so the second block below would fail with `undefined variable "a"`:

```elixir
iex> a = 1
1

iex> a + 1
2
```

#### Omitting the expected result

You can leave off the result line for any expression to skip asserting on that line's return value. This is the standard way to avoid non-deterministic output such as a PID. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "If you don't want to assert for every result in a doctest, you can omit the result. You can do so between expressions:"

```elixir
iex> pid = spawn(fn -> :ok end)
iex> is_pid(pid)
true
```

> "As well as at the end:"

```elixir
iex> Mod.do_a_call_that_should_not_raise!(...)
```

#### Output matching and opaque types

For a normal value, the result line must equal what `Kernel.inspect/2` would print; doctest compares structurally (`===` on the parsed term). When the inspected value starts with `#Name<...>` (structs, `DateTime`, pids, references, regexes, etc.) doctest falls back to a **string** comparison of the `inspect` output rather than structural equality. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Whenever a doctest starts with `'#Name<'`, doctest will perform a string comparison."

```elixir
iex> map = %{datetime: DateTime.from_naive!(~N[2023-06-26T09:30:00], "Asia/Tokyo")}
iex> map.datetime
#DateTime<2023-06-26 09:30:00+09:00 JST Asia/Tokyo>
```

Use the `:inspect_opts` option when you need to control that `inspect` output.

### Exceptions in doctests

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Doctest will look for a line starting with `** (` and it will parse it accordingly to extract the exception name and message. The exception parser will consider all following lines part of the exception message until there is an empty line or there is a new expression prefixed with `iex>`."

The syntax is:

```
** (ExceptionModule) message
```

Single-line example:

```elixir
iex> raise "some error"
** (RuntimeError) some error
```

The same form works for any exception — `ArgumentError`, `ArithmeticError`, `FunctionClauseError`, `MatchError`, etc. There is no special per-exception syntax; doctest matches the actual struct type and its message. You can match just the type with an empty/short message, or the type plus the full message.

Multi-line messages are allowed as long as they contain **no blank lines** (a blank line terminates the message):

```elixir
iex> raise "error: line1\nline2"
** (RuntimeError) error: line1
line2
```

Since Elixir **v1.19.0**, doctests allow a trailing ellipsis (`...`) to match only a message prefix, which is how you handle non-deterministic content such as a PID in the message. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Since Elixir 1.19.0, doctests allow the use of an ellipsis (`...`) at the end of messages:"

```elixir
iex> raise "some error in pid: #{inspect(self())}"
** (RuntimeError) some error in pid: ...
```

Internally, a message ending in `...` is matched with `String.starts_with?/2` after stripping the ellipsis. Doctests can only match **raised exceptions** (`** (...)`). A compile warning is not matchable, and a function that *returns* `{:error, ...}` is just compared as a value on the result line.

### Options: `:except`, `:only`, `:import`

#### `:except` and `:only`

Both take a list whose elements are `{function, arity}` tuples and/or the `:moduledoc` atom. `:moduledoc` refers collectively to the examples in `@moduledoc`; each `{name, arity}` refers to the examples in that function's `@doc`.

```elixir
defmodule MyModuleTest do
  use ExUnit.Case
  doctest MyModule, except: [:moduledoc, trick_fun: 1]
end
```

```elixir
doctest MyModule, only: [some_fun: 1, :moduledoc]
```

If `:only` references a function that does not exist or is private, doctest raises at compile time.

#### `:import`

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "`:import` - when `true`, one can test a function defined in the module without referring to the module name. However, this is not feasible when there is a clash with a module like `Kernel`. In these cases, `:import` should be set to `false` and `Module.function(...)` should be used instead."

With `import: true`, an example may call `some_fun(x)` instead of `MyModule.some_fun(x)`. Leave it `false` (the default) whenever the module defines functions whose names collide with `Kernel` (for example `length/1`, `round/1`, `send/2`) — otherwise the import shadows the intended behavior.

#### `:tags` and `:inspect_opts`

- `:tags` — a list of tags attached to every generated doctest, useful for filtering with `--only` / `--exclude` (e.g. `doctest MyMod, tags: [:slow]`).
- `:inspect_opts` — forwarded to `inspect/2` for value comparison, so you can pin pretty-printing / structured output for opaque types.

### `doctest_file/2` (since v1.15.0)

`doctest_file/2` extracts `iex>` examples from a standalone file — typically a `README.md` or guide — rather than from a module's docs.

```elixir
defmodule ReadmeTest do
  use ExUnit.Case
  doctest_file "README.md"
end
```

It uses the same syntax rules (`iex>`, `...>`, `** (...)`, the v1.19.0 ellipsis) as module doctests.

### Doctests and `setup` context

Each generated doctest is a real ExUnit test, registered through `ExUnit.Case.register_test/6` with `test_type: :doctest` and tagged with `:doctest` (the module being tested), `:doctest_line`, and `:doctest_data`. Because of that, the surrounding test module's `setup` / `setup_all` callbacks run normally before each doctest, and `on_exit` runs after. If the test module uses a `DataCase` / `ConnCase` template or starts supervised processes in `setup`, every doctest in that module benefits from them.

The catch is that the **example body itself** is the literal `iex>` code from `@doc` — it is not the test file's code. Setup gives the doctest the same surrounding environment (a started process, a checked-out DB connection, injected aliases) that any other test in the module gets, but it cannot rewrite the example text. If an example needs a resource the test module sets up (say a running GenServer registered under a known name), make sure the example refers to that resource by its stable, global handle rather than a value the test would bind.

### Running doctests: tags and `mix test`

Doctests carry the `:doctest` tag and have `test_type: :doctest`, so they can be selected and filtered independently of regular tests:

```bash
# Run only doctests across the whole suite
mix test --only doctest

# Run only the doctests for a specific module
mix test --only doctest:"Elixir.MyApp.Foo"

# Run a single doctest by its source line
mix test path/to/module_test.exs:LINE
```

The line-based form targets a doctest via its `:doctest_line` tag. Excluding doctests works the same way (`--exclude doctest`), and you can add your own tags via the `:tags` option to filter subsets.

### When to use doctests (and when not to)

Doctests shine for **pure, deterministic** functions where the `iex>` example is also the natural documentation snippet. They are the canonical way to guarantee that the examples in a library's docs actually run.

From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "In general, doctests are not recommended when your code examples contain side effects. For example, if a doctest prints to standard output, doctest will not try to capture the output."
>
> "Similarly, doctests do not run in any kind of sandbox. So any module defined in a code example is going to linger throughout the whole test suite run."

Concretely:

- **Good fit:** pure functions, parsers, formatters, data-structure operations — anything where input → `inspect`-able output is the whole story.
- **Poor fit:** functions that print to stdout/stderr (doctest will not capture it), that depend on `DateTime.utc_now/0`, random IDs, PIDs, map key ordering, or that mutate global state (ETS, the application env, registered names), or that define modules inside the example (the module persists for the whole suite).
- **Cannot test:** private functions — doctest only walks public `@doc`.

Doctests **complement** regular ExUnit tests; they verify the documentation, they do not replace focused unit tests for edge cases, side effects, or state.

### Module doctests vs function doctests

`doctest MyModule` covers both `@moduledoc` examples and every public function's `@doc` examples in one call — there is no per-function `doctest` form. Use `:only` / `:except` to scope:

- `doctest MyMod, only: [:moduledoc]` — just the module-level examples.
- `doctest MyMod, only: [foo: 1]` — just the examples in `foo/1`'s `@doc`.
- `doctest MyMod, except: [:moduledoc]` — every function's examples, skipping the module intro.

Each individual `iex>` example becomes its own test, so a single `doctest` call can expand to many tests.

### Common pitfalls

- **Non-deterministic output.** Timestamps, UUIDs, PIDs, references, and map key ordering all vary between runs. Omit the result line, match an exception message with a trailing `...` (v1.19.0+), or compare an opaque `#Name<...>` value whose printed form is stable.
- **Trailing whitespace / indentation.** The expected result line is matched exactly; trailing spaces fail. Continuation and result lines must share the same indent as the `iex>` prompt, or doctest raises an indentation-mismatch compile error.
- **Variable binding across examples.** Without a blank line between `iex>` lines, bindings are shared (like `iex`); an example that reuses a variable from an earlier block only works if the two are in the same block. Add a blank line to isolate.
- **Kernel name clashes.** With `import: true`, a module function named like a `Kernel` function (`length/1`, `hd/1`, `round/1`, …) is shadowed. Keep `import: false` and call `Module.fun(...)` in those cases.
- **Printing pids / references.** Avoid result lines that would print a pid or reference; either omit the result or assert on a stable derived value (`is_pid(pid)` → `true`).
- **Side effects and no sandbox.** `IO.puts` / `IO.write` in an example are not captured; modules defined in an example persist for the whole suite. Keep examples pure.
- **Color / inspect options.** If `:syntax_colors` is enabled, inspected output contains ANSI escapes that will not match the plain result string. Pin behavior with `:inspect_opts` and keep color off for compared values.
- **Plain (non-`iex>`) examples never run.** A fenced code block without `iex>` prefixes is documentation only.

### Common mistakes

- Calling `doctest/1` and expecting to pass options — `doctest/1` takes no options; options go to `doctest/2` (`doctest MyMod, except: [...]`).
- Looking for an `:imported` option — it does not exist; the option is the boolean `:import`.
- Writing doctests for private functions — only public `@doc` examples are discovered.
- Expecting `doctest` to capture `IO.puts` output the way `ExUnit.CaptureIO` does — doctests do not capture IO at all.
- Expecting `setup` / `setup_all` to rewrite the example body — setup provides the environment, but the example text is taken literally from `@doc`.
- Using `import: true` for a module whose functions clash with `Kernel`, silently getting `Kernel`'s behavior.
- Treating a multi-key map result line as order-independent — map output ordering is not guaranteed across runs.
- Forgetting that consecutive `iex>` lines without a blank line share variables, so a missing blank line can make a later example unexpectedly depend on an earlier one.
- Expecting a compile warning to be matchable with `** (...)` — only raised exceptions can be matched; warnings and `{:error, _}` return values are not.

Sources: https://hexdocs.pm/ex_unit/ExUnit.DocTest.html, https://hexdocs.pm/ex_unit/ExUnit.Case.html, https://hexdocs.pm/mix/Mix.Tasks.Test.html, https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/ex_unit/lib/ex_unit/doc_test.ex

## Test Organization and Tags

This section consolidates the ExUnit.Case guidance on tags, filters, registered attributes, `describe/2`, and test module organization. The Overview section above gives a brief summary; this section is the canonical reference.

From [ExUnit.Case.html "Tags"](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "The context is used to pass information from the callbacks to the test. In order to pass information from the test to the callback, ExUnit provides tags.
>
> By tagging a test, the tag value can be accessed in the context, allowing the developer to customize the test."

Tags flow in the opposite direction from context: callbacks push data into the test via the context map, while tags let the test (or module/describe) push metadata back into the context that callbacks can read. A typical use is a `:cd` tag read in `setup` to change the working directory:

```elixir
defmodule FileTest do
  # Changing directory cannot be async
  use ExUnit.Case, async: false

  setup context do
    if cd = context[:cd] do
      prev_cd = File.cwd!()
      File.cd!(cd)
      on_exit(fn -> File.cd!(prev_cd) end)
    end

    :ok
  end

  @tag cd: "fixtures"
  test "reads UTF-8 fixtures" do
    File.read("README.md")
  end
end
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Tags are also very effective when used with case templates (`ExUnit.CaseTemplate`) allowing callbacks in the case template to customize the test behaviour."

### `@tag`, `@moduletag`, `@describetag`

Tags are set at three scopes:

| Attribute | Scope | Cleared after |
|---|---|---|
| `@tag` | a single `test/3` | each test |
| `@describetag` | all tests in a `describe/2` block | each describe block |
| `@moduletag` | all tests in the module | never (module-wide) |

From [ExUnit.Case.html "Module and describe tags"](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "A tag can be set for all tests in a module or describe block by setting `@moduletag` or `@describetag` inside each context respectively:"

```elixir
defmodule ApiTest do
  use ExUnit.Case
  @moduletag :external

  describe "makes calls to the right endpoint" do
    @describetag :endpoint

    # ...
  end
end
```

#### Placement and precedence rules

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "If you are setting a `@moduletag` or `@describetag` attribute, you must set them after your call to `use ExUnit.Case` otherwise you will see compilation errors.
>
> If the same key is set via `@tag`, the `@tag` value has higher precedence.
>
> The `setup_all` blocks only receive tags that are set using `@moduletag`."

Three invariants to keep in mind:

1. **Placement**: set `@tag`, `@describetag`, and `@moduletag` **after** `use ExUnit.Case`. Setting them before raises a compile error (ExUnit installs the attribute registers in `__using__/1`).
2. **Precedence**: for the same key, `@moduletag` < `@describetag` < `@tag`. The most specific scope wins.
3. **`setup_all` visibility**: `setup_all/1,2` callbacks only see `@moduletag` values, because `setup_all` runs once per module before any individual test (and therefore before per-test/per-describe tags exist). Use `@moduletag` for any tag a `setup_all` callback needs to read.

#### Tag forms

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Note a tag can be set in two different ways:
>
> ```elixir
> @tag key: value
> @tag :key       # equivalent to setting @tag key: true
> ```
>
> If a tag is given more than once, the last value wins."

So `@tag :integration` is shorthand for `@tag integration: true`, and repeated `@tag` calls for the same key keep only the last value (within the same scope; precedence still applies across scopes).

### Known tags

From [ExUnit.Case.html "Known tags"](https://hexdocs.pm/ex_unit/ExUnit.Case.html), the following tags are **set automatically by ExUnit and are reserved** — do not set these yourself:

| Tag | Meaning |
|---|---|
| `:async` | Whether the test case is in async mode. |
| `:file` | The file on which the test was defined. |
| `:line` | The line on which the test was defined. |
| `:module` | The module on which the test was defined. |
| `:registered` | Values from `register_attribute/3` and friends. |
| `:test` | The test name. |
| `:test_group` | The group the test belongs to. |
| `:test_pid` | The PID of the testing process. |
| `:test_type` | Test type used when printing results and for filtering. Set by ExUnit to `:test`, `:doctest`, or the type given to `register_test/6`. |
| `:describe` | The describe block the test belongs to (if in a describe). |
| `:describe_line` | The line the describe block begins on (if in a describe); usable for line-based runs. |
| `:doctest` | The module or file being doctested (if a doctest). |
| `:doctest_data` | Additional doctest metadata in a map (e.g. `:end_line`) for reflection (if a doctest). |
| `:doctest_line` | The line the doctest was defined on (if a doctest); usable for line-based runs. |

The following tags **customize how tests behave** and may be set by the developer:

| Tag | Since | Effect |
|---|---|---|
| `:capture_log` | — | Capture log messages during the test and print them only on failure. Overrides the global `:capture_log` default; set via `@tag`/`@moduletag` (e.g. `@tag capture_log: false`). |
| `:skip` | — | Skip the test with the given reason. |
| `:timeout` | — | Custom test timeout in milliseconds (default `60_000`). Accepts `:infinity`. |
| `:tmp_dir` | v1.11.0 | Create a unique temp dir for the test and expose its path via the context. |

#### `:tmp_dir` detail

From [ExUnit.Case.html "Tmp Dir"](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "ExUnit automatically creates a temporary directory for tests tagged with `:tmp_dir` and puts the path to that directory into the test context. The directory is removed before being created to ensure we start with a blank slate.
>
> The temporary directory path is unique (includes the test module and test name) and thus appropriate for running tests concurrently. You can customize the path further by setting the tag to a string, e.g.: `tmp_dir: "my_path"`, which would make the final path to be: `tmp/<module>/<test>/my_path`.
>
> As with other tags, `:tmp_dir` can also be set as `@moduletag` and `@describetag`."

### Filters: `--only`, `--exclude`, `--include`, `ExUnit.configure/1`

From [ExUnit.Case.html "Filters"](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Tags can also be used to identify specific tests, which can then be included or excluded using filters. The most common functionality is to exclude some particular tests from running, which can be done via `ExUnit.configure/1`:

```elixir
# Exclude all external tests from running
ExUnit.configure(exclude: [external: true])
```

> From now on, ExUnit will not run any test that has the `:external` option set to `true`. This behaviour can be reversed with the `:include` option which is usually passed through the command line:
>
> ```
> $ mix test --include external:true
> ```
>
> Another use case for tags and filters is to exclude all tests that have a particular tag by default, regardless of its value, and include only a certain subset:
>
> ```elixir
> ExUnit.configure(exclude: :os, include: [os: :unix])
> ```
>
> A given include/exclude filter can be given more than once:
>
> ```elixir
> ExUnit.configure(exclude: [os: :unix, os: :windows])
> ```
>
> Keep in mind that all tests are included by default, so unless they are excluded first, the `include` option has no effect."

#### Filter semantics

From [ExUnit.html `configure/1`](https://hexdocs.pm/ex_unit/ExUnit.html):

> "`:exclude` - specifies which tests are run by skipping tests that match the filter.
>
> `:include` - specifies which tests are run by skipping tests that do not match the filter. Keep in mind that all tests are included by default, so unless they are excluded first, the `:include` option has no effect. To only run the tests that match the `:include` filter, exclude the `:test` tag first."

The `configure_opts/0` type for both options is `atom() | [atom() | {atom(), any()}]`, so a filter is either:

- a bare atom (`:os`) — matches any test that has the tag, regardless of value;
- a `{tag, value}` tuple (`{:os, :unix}`) — matches only tests where the tag equals that value;
- or a list mixing both.

Filter application order, from [Mix.Tasks.Test.html "Tags and filters"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "The order in which filters are applied is also consistent: first all exclusions are computed, then the inclusions."

#### Command-line filters

From [Mix.Tasks.Test.html "Tags and filters"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "For convenience, Mix also provides an `--only` option that excludes all tests and includes only the given ones:
>
> ```
> $ mix test --only external
> ```
>
> Which is similar to:
>
> ```
> $ mix test --include external --exclude test
> ```
>
> However, when the `--only` option is used and no tests run, the test run fails."

So `--only <filter>` is shorthand for `--include <filter> --exclude test` (the reserved `:test` tag is present on every real test, so excluding it removes everything, then the include re-adds the matching subset). Because `--only` excludes everything first, it does **not** suffer the "include has no effect unless excluded first" caveat. Note the failure mode: `--only` with zero matching tests fails the run, which is usually what you want for CI gates.

Value-based filters on the command line use `tag:value` syntax:

```bash
mix test --only async:true      # run only async tests
mix test --exclude async:true   # run only sync tests
mix test --include ci --include slow   # multiple includes
mix test --exclude ci --exclude slow   # multiple excludes
```

#### Line-specific runs

From [Mix.Tasks.Test.html "Tags and filters"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "In case a single file is being tested, it is possible to pass one or more specific line numbers to run only those given tests:
>
> ```
> $ mix test test/some/particular/file_test.exs:12
> ```
>
> Which is equivalent to:
>
> ```
> $ mix test --exclude test --include line:12 test/some/particular/file_test.exs
> ```
>
> Or:
>
> ```
> $ mix test test/some/particular/file_test.exs:12:24
> ```
>
> Which is equivalent to:
>
> ```
> $ mix test --exclude test --include line:12 --include line:24 test/some/particular/file_test.exs
> ```
>
> If a given line starts a `describe` block, that line filter includes all tests within the describe. Otherwise, it runs the closest test on or before the given line number."

So `:line` is a reserved-ish filter (set automatically from `:line`) and `path:line` is sugar for `--include line:N`. Passing the line where a `describe` block starts runs the whole block.

### Registered attributes: `register_attribute/3` and friends

ExUnit provides three attribute registers with different scopes, plus `register_test/6` for building custom test macros. Registered values surface in the test context under `context.registered.<name>`. They are **not** imported by `use ExUnit.Case`; call them fully qualified as `ExUnit.Case.register_...`.

The shared options type is:

```elixir
@type register_attribute_opts() :: [accumulate: boolean(), persist: boolean()]
```

These are passed through to `Module.register_attribute/3`.

#### `register_attribute/3` — per test (since v1.3.0)

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a new attribute to be used during `ExUnit.Case` tests.
>
> The attribute values will be available through `context.registered`. Registered values are cleared after each `test/3` similar to `@tag`.
>
> This function takes the same options as `Module.register_attribute/3`."

Spec: `@spec register_attribute(env(), atom(), register_attribute_opts()) :: :ok`, where `env()` is `module() | Macro.Env.t()`.

```elixir
defmodule MyTest do
  use ExUnit.Case

  ExUnit.Case.register_attribute(__MODULE__, :fixtures, accumulate: true)

  @fixtures :user
  @fixtures {:post, insert: false}
  test "using custom attribute", context do
    assert context.registered.fixtures == [{:post, insert: false}, :user]
  end

  test "custom attributes are cleared per test", context do
    assert context.registered.fixtures == []
  end
end
```

#### `register_describe_attribute/3` — per describe block (since v1.10.0)

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a new describe attribute to be used during `ExUnit.Case` tests.
>
> The attribute values will be available through `context.registered`. Registered values are cleared after each `describe/2` similar to `@describetag`.
>
> This function takes the same options as `Module.register_attribute/3`."

```elixir
defmodule MyTest do
  use ExUnit.Case

  ExUnit.Case.register_describe_attribute(__MODULE__, :describe_fixtures, accumulate: true)

  describe "using custom attribute" do
    @describe_fixtures :user
    @describe_fixtures {:post, insert: false}

    test "has attribute", context do
      assert context.registered.describe_fixtures == [{:post, insert: false}, :user]
    end
  end

  describe "custom attributes are cleared per describe" do
    test "doesn't have attributes", context do
      assert context.registered.describe_fixtures == []
    end
  end
end
```

#### `register_module_attribute/3` — whole module (since v1.10.0)

`register_module_attribute/3` registers an attribute that is **not** cleared between tests; it persists across all tests in the module (like `@moduletag`). Use it for module-wide fixtures shared by every test.

```elixir
defmodule MyTest do
  use ExUnit.Case

  ExUnit.Case.register_module_attribute(__MODULE__, :module_fixtures, accumulate: true)

  @module_fixtures :user
  @module_fixtures {:post, insert: false}

  test "using custom attribute", context do
    assert context.registered.module_fixtures == [{:post, insert: false}, :user]
  end

  test "still using custom attribute", context do
    assert context.registered.module_fixtures == [{:post, insert: false}, :user]
  end
end
```

#### `register_test/6` — custom test macros (since v1.11.0)

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Registers a function to run as a test for this module.
>
> This is used by third-party projects to implement macros like `property/3` that works like `test` but instead defines a property. See `test/3` implementation for an example of invoking this function.
>
> The test type will be converted to a string and pluralized for display. You can use `ExUnit.plural_rule/2` to set a custom pluralization."

Spec: `register_test(env, Path.t(), non_neg_integer(), test_type :: atom(), name :: atom(), tags :: Keyword.t()) :: :ok`. The older `register_test/4` is deprecated in favor of `/6`. Internally, `test/3` calls `ExUnit.Case.register_test(mod, file, line, :test, message, [])`. The `test_type` becomes the `:test_type` tag value and is pluralized for the summary line (e.g. `:doctest` → "doctests"); register custom pluralizations with `ExUnit.plural_rule/2`.

### `describe/2` blocks

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "Describes tests together.
>
> Every describe block receives a name which is used as prefix for upcoming tests. Inside a block, `ExUnit.Callbacks.setup/1` may be invoked and it will define a setup callback to run only for the current block. The describe name is also added as a tag, allowing developers to run tests for specific blocks."

Signature: `describe(message, list)` — macro, available since v1.3.0.

```elixir
defmodule StringTest do
  use ExUnit.Case, async: true

  describe "String.downcase/1" do
    test "with ascii characters" do
      assert String.downcase("HELLO") == "hello"
    end

    test "with Unicode" do
      assert String.downcase("HÉLLÒ") == "héllò"
    end
  end
end
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "When using Mix, you can run all tests in a describe block by name:
>
> ```
> $ mix test --only describe:"String.downcase/1"
> ```
>
> or by passing the exact line the describe block starts on:
>
> ```
> $ mix test path/to/file:123
> ```
>
> Note describe blocks cannot be nested. Instead of relying on hierarchy for composition, developers should build on top of named setups."

The supported pattern for what would otherwise be nested describes is shared named setups:

```elixir
defmodule UserManagementTest do
  use ExUnit.Case, async: true

  describe "when user is logged in and is an admin" do
    setup [:log_user_in, :set_type_to_admin]
    # ...
  end

  describe "when user is logged in and is a manager" do
    setup [:log_user_in, :set_type_to_manager]
    # ...
  end

  defp log_user_in(_context), do: :ok
  defp set_type_to_admin(_context), do: :ok
  defp set_type_to_manager(_context), do: :ok
end
```

From [ExUnit.Case.html](https://hexdocs.pm/ex_unit/ExUnit.Case.html):

> "By forbidding hierarchies in favor of named setups, it is straightforward for the developer to glance at each describe block and know exactly the setup steps involved."

Key `describe` facts:

- The describe name is added as the `:describe` tag (and `:describe_line` for the starting line), so `--only describe:"<name>"` or `path:line` runs a whole block.
- A `setup/1` written inside a `describe` runs only for tests in that block.
- Describe blocks **cannot be nested**; use named setups for composition.

### Test module organization and naming conventions

From [ExUnit.html "Integration with Mix"](https://hexdocs.pm/ex_unit/ExUnit.html):

> "Invoking `mix test` from the command line will run the tests in each file matching the pattern `*_test.exs` found in the `test` directory of your project."

Conventions reinforced by Mix and the ExUnit examples:

- **One test file per source module**, suffixed `_test.exs`, under `test/`. Mix discovers tests by the `*_test.exs` pattern (via `:test_pattern` plus `:test_load_filters` — see the Mix.Tasks.Test section); a file not matching it is not run as a test.
- **One test module per file**, named `<Subject>Test` (e.g. `MyApp.UserTest` for `MyApp.User`), beginning with `use ExUnit.Case`.
- **Group related tests with `describe/2`**, one describe per public function or per behavior; do not nest describes.
- **Write test names as descriptive sentences** (`test "returns an error when the input is empty"`), not as identifiers. The rendered test name is `"test <message>"` (or `"test <describe> <message>"` inside a describe).
- **Share setup and helpers across files via `ExUnit.CaseTemplate`** (e.g. `DataCase`, `ConnCase`) rather than copy-paste.
- **Tag cross-cutting concerns** (`:external`, `:integration`, `:slow`, `:flaky`) and exclude them by default in `test_helper.exs` so the default `mix test` stays fast and hermetic.

### Examples

#### Moduletag + describetag precedence

```elixir
defmodule MyApp.ApiTest do
  use ExUnit.Case
  @moduletag external: true          # every test in this module is :external

  describe "GET /users" do
    @describetag external: false     # ...except these, which override @moduletag
    @describetag :read

    test "lists users", context do
      assert context.external == false
      assert context.read == true
    end
  end
end
```

#### Excluding by default, including on demand

```elixir
# test/test_helper.exs
ExUnit.configure(exclude: [:external, :integration, slow: true])
ExUnit.start()
```

```bash
# Run only integration tests (fails if none match)
mix test --only integration

# Run everything except slow tests, plus re-enable external
mix test --exclude slow --include external

# Value-based: run only the :unix-tagged os tests
mix test --only os:unix
```

#### Custom registered attribute (accumulate)

```elixir
defmodule MyApp.FixtureTest do
  use ExUnit.Case

  ExUnit.Case.register_attribute(__MODULE__, :roles, accumulate: true)

  @roles :admin
  @roles :superuser

  test "sees both roles in reverse order", context do
    assert context.registered.roles == [:superuser, :admin]
  end
end
```

#### `:tmp_dir` and `:timeout` behavior tags

```elixir
defmodule MyApp.ExportTest do
  use ExUnit.Case, async: true

  @tag tmp_dir: "exports"
  @tag timeout: 30_000
  test "writes a file into a unique temp dir", %{tmp_dir: dir} do
    path = Path.join(dir, "out.csv")
    assert :ok = MyApp.Export.write(path, "a,b\n1,2\n")
    assert File.exists?(path)
  end
end
```

### Common mistakes

- Setting `@tag`/`@describetag`/`@moduletag` **before** `use ExUnit.Case`; it raises a compile error.
- Expecting `setup_all` to read a `@tag` or `@describetag` value; `setup_all` only sees `@moduletag`.
- Assuming `--include external` re-enables excluded tests on its own; includes only take effect after an exclude (use `--only` or `--exclude test` to force inclusion).
- Using `--only` and expecting an empty match to pass; `--only` with zero matching tests fails the run.
- Calling `register_attribute/3` etc. unqualified after `use ExUnit.Case` and expecting them imported; they are not imported — call them as `ExUnit.Case.register_...`.
- Trying to nest `describe/2` blocks; describe cannot be nested — use named setups instead.
- Setting a reserved tag (`:file`, `:line`, `:test`, `:module`, ...) yourself; ExUnit owns these.
- Forgetting that `@tag :key` is shorthand for `@tag key: true`, then being surprised that a value-based filter `--only key:custom` does not match it.

Sources: https://hexdocs.pm/ex_unit/ExUnit.Case.html, https://hexdocs.pm/ex_unit/ExUnit.html, https://hexdocs.pm/mix/Mix.Tasks.Test.html, https://hexdocs.pm/elixir/Module.html#register_attribute/3

## Mix.Tasks.Test

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "This task starts the current application, loads up `test/test_helper.exs` and then, requires all files matching the `test/**/*_test.exs` pattern in parallel."

`mix test` is the Mix task that runs an ExUnit suite. It compiles the project (and deps) for the `:test` environment, starts the current application, loads `test/test_helper.exs` (which must call `ExUnit.start/0` or `ExUnit.start/1` — Mix does not start ExUnit for you), then requires every file matching `test/**/*_test.exs` in parallel and runs the registered tests.

### Test file discovery and naming

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "A list of files and/or directories can be given after the task name in order to select the files to run:
>
> ```
> $ mix test test/some/particular/file_test.exs
> $ mix test test/some/particular/dir
> ```
>
> Tests in umbrella projects can be run from the root by specifying the full suite path, including `apps/my_app/test`, in which case recursive tests for other child apps will be skipped completely."

Discovery is governed by project config (see "Mix project configuration" below). The effective default behavior is: files matching `*.{ex,exs}` under `:test_paths` are candidates, then `:test_load_filters` keeps those ending in `_test.exs`, and `:test_ignore_filters` drops `*_helper.exs` and anything in `:elixirc_paths`. So the conventional test file name is `*_test.exs`, and the conventional helper file name is `*_helper.exs` (e.g. `test/support/data_case.ex` lives under an `:elixirc_paths` entry so it is compiled, not run as a test).

### Doctests

Doctests are not run by a separate Mix task. When a test module calls `doctest Module` (or `doctest_file/2`), ExUnit generates one test per doctest example, tagged with `:test_type` set to `:doctest` and the reserved `:doctest`/`:doctest_line`/`:doctest_data` tags. They run as part of the normal `mix test` suite and appear in the summary as "N doctests". You can filter them like any other test:

```bash
mix test --only doctest                # run only doctests
mix test --exclude doctest             # skip all doctests
mix test lib/my_app/math.ex:42         # run the doctest defined at that line
```

See the Doctests section above for `doctest/2` options and the matching rules.

### Parallel execution

ExUnit runs tests from **different modules marked `async: true`** concurrently, up to `:max_cases` (default `System.schedulers_online() * 2`). Tests within the same module always run serially (except for different `:parameterize` maps when `async: true`). Mix additionally compiles/requires test files in parallel, bounded by `--max-requires`.

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "`--max-cases` - sets the maximum number of tests running asynchronously. Only tests from different modules run in parallel. Defaults to twice the number of cores"

> "`--max-requires` - sets the maximum number of test files to compile in parallel. Setting this to `1` will compile test files sequentially."

### Command-line options

From [Mix.Tasks.Test.html "Command line options"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

| Flag | Since | Effect |
|---|---|---|
| `--all-warnings` / `--no-all-warnings` | — | Print all warnings, including from previous compilations (default true except on errors). |
| `-b`, `--breakpoints` | v1.17.0 | Set a breakpoint at the start of every test; step line-by-line with `n` (next) and `c` (next test). Automatically sets `--trace`. |
| `--color` | — | Enable ANSI color in ExUnit output. |
| `--cover` | — | Run the coverage tool. See "Coverage" below. |
| `--dry-run` | v1.19.0 | Print which tests would run based on current options without running them. Combines with `--stale`, `--only`, `--exclude`, etc. |
| `--exclude` | — | Exclude tests matching the filter. May be given multiple times (`--exclude ci --exclude slow`); supports `tag:value` (`--exclude async:true`). |
| `--exit-status` | — | Alternate exit status on suite failure (default `2`). |
| `--export-coverage` | — | Filename to export coverage results to; only with `--cover`. |
| `--failed` | — | Run only tests that failed the last time they ran. |
| `--force` | — | Force compilation regardless of modification times. |
| `--formatter` | — | Formatter module to print results (in addition to defaults). |
| `--include` | — | Include tests matching the filter. May be given multiple times (`--include ci --include slow`). |
| `--listen-on-stdin` | — | Run tests, then re-run on each newline received on stdin (for file-system watchers). |
| `--max-cases` | — | Max number of test modules running concurrently. Default twice the number of cores. |
| `--max-failures` | — | Stop evaluating tests after this many failures. Runs all if omitted. |
| `--max-requires` | — | Max number of test files to compile in parallel. `1` compiles sequentially. |
| `-n`, `--name-pattern` | v1.19.0 | Run only tests whose names match the given regular expression. |
| `--no-archives-check` | — | Do not check archives. |
| `--no-color` | — | Disable ANSI color in output. |
| `--no-compile` | — | Do not compile, even if files require compilation. |
| `--no-deps-check` | — | Do not check dependencies. |
| `--no-elixir-version-check` | — | Do not check the Elixir version from `mix.exs`. |
| `--no-start` | — | Do not start applications after compilation. |
| `--only` | — | Run only tests matching the filter (e.g. `--only ci`, `--only async:true`). Fails if no tests run. |
| `--partitions` | — | Split tests across N partitions; requires `MIX_TEST_PARTITION`. See "Partitions" below. |
| `--preload-modules` | — | Preload all modules defined in applications. |
| `--profile-require time` | — | Profile time spent requiring test files (debugging only; suite does not run). |
| `--raise` | — | Raise immediately if the suite fails instead of continuing other Mix tasks. |
| `--repeat-until-failure N` | v1.17.0 | Repeat the suite up to N times until the first failure; useful for flaky-test debugging within one VM instance. |
| `--seed N` | — | Seed the RNG used to randomize test order. `--seed 0` disables randomization (definition order within each file). |
| `--slowest N` | — | Print timing for the N slowest tests (includes `setup/1` time). Sets `--trace` and `--preload-modules`. |
| `--slowest-modules N` | v1.17.0 | Print timing for the N slowest modules. Sets `--trace` and `--preload-modules`. |
| `--stale` | — | Run only tests referencing modules changed since the last `--stale` run. See "Stale" below. |
| `--timeout N` | — | Set the test timeout. |
| `--trace` | — | Detailed reporting; sets `--max-cases 1` and ignores timeouts (timeout becomes `:infinity`). |
| `--warnings-as-errors` | v1.12.0 | Treat compilation warnings from loading the test suite as errors. See "Exit status" below. |

#### Exit status

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "`--exit-status` - use an alternate exit status to use when the test suite fails (default is 2)."

> "`--warnings-as-errors` (since v1.12.0) - treats compilation warnings (from loading the test suite) as errors and returns an exit status of 1 if the test suite would otherwise pass. If the test suite fails and also include warnings as errors, the exit status returned will be the value of the `--exit-status` option, which defaults to `2`, plus one. Therefore in the default case, this will be exit status `3`."

So the exit status matrix is:

- Suite passes, no warnings → `0`.
- Suite passes, but `--warnings-as-errors` and there were warnings → `1`.
- Suite fails (default) → `2` (or the `--exit-status` value).
- Suite fails **and** `--warnings-as-errors` → `--exit-status + 1` (default `3`).

Note: failures reported by `--warnings-as-errors` cannot be retried with `--failed`. To treat warnings as errors during both compilation and tests, run `MIX_ENV=test mix do compile --warnings-as-errors + test --warnings-as-errors`.

### Tags and filters on the command line

See the "Test Organization and Tags" section above for the full filter semantics. In summary:

```bash
mix test --only external              # only :external tests (fails if none)
mix test --only async:true           # only async tests
mix test --include external          # re-include :external (only if excluded)
mix test --exclude slow               # skip :slow tests
mix test --exclude async:true        # run only sync tests
mix test path/to/file_test.exs:12    # run the test at line 12
mix test path/to/file_test.exs:12:24 # run tests at lines 12 and 24
mix test --only describe:"String.downcase/1"  # run a whole describe block
```

`--only <filter>` is shorthand for `--include <filter> --exclude test`. Filter application order is: exclusions first, then inclusions.

### The `--stale` option

From [Mix.Tasks.Test.html "The --stale option"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "The `--stale` command line option attempts to run only the test files which reference modules that have changed since the last time you ran this task with `--stale`.
>
> The first time this task is run with `--stale`, all tests are run and a manifest is generated. On subsequent runs, a test file is marked 'stale' if any modules it references (and any modules those modules reference, recursively) were modified since the last run with `--stale`. A test file is also marked 'stale' if it has been changed since the last run with `--stale`.
>
> The `--stale` option is extremely useful for software iteration, allowing you to run only the relevant tests as you perform changes to the codebase."

`--stale` tracks a manifest of module → test-file references. Changing a module re-runs every test file that transitively references it. Combine with `--listen-on-stdin` and a file watcher (e.g. `fswatch lib test | mix test --stale --listen-on-stdin`) for a tight feedback loop.

### Coverage

From [Mix.Tasks.Test.html "Coverage"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "Elixir provides built-in line-based test coverage via the `--cover` flag. The test coverages shows which lines of code and in which files were executed during the test run."

> "Elixir uses Erlang's `:cover` for its default test coverage. Erlang coverage is done by tracking *executable lines of code*. See `mix test.coverage` for details."

The `:test_coverage` project key configures the coverage tool. The default tool (`Mix.Tasks.Test.Coverage`, a wrapper around OTP's `:cover`) accepts:

| Option | Default | Purpose |
|---|---|---|
| `:tool` | `Mix.Tasks.Test.Coverage` | Module specifying the coverage tool. Any module exporting `start/2` (receives the compile path and the `test_coverage` opts; returns `nil` or a zero-arity function run after the suite). |
| `:output` | `"cover"` | Output directory for cover results. |
| `:summary` | `[threshold: 90]` | Summary generation; set to `false` to disable. The task exits `1` if total coverage is below the threshold. |
| `:export` | unset | Filename to export results to (`.coverdata` appended automatically). Set by `--export-coverage` and by partitioning. |
| `:ignore_modules` | `[]` | Modules (atoms or regexps matched against module names) to exclude from reports and summaries. |
| `:local_only` | `true` | When `false`, track coverage across nodes. |

```elixir
# mix.exs
def project do
  [
    # ...
    test_coverage: [tool: ExCoveralls, threshold: 80, ignore_modules: [MyApp.Debug]]
  ]
end
```

A custom tool is any module exporting `start/2`. The built-in tool produces line-based coverage; for HTML/LCOV/Jenkins output use a third-party tool such as `excoveralls` or `covertool` via the `:tool` key. Compile a unified report from multiple partition exports with `mix test.coverage`.

### Operating-system process partitioning

From [Mix.Tasks.Test.html "Operating system process partitioning"](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "While ExUnit supports the ability to run tests concurrently within the same Elixir instance, it is not always possible to run all tests concurrently. For example, some tests may rely on global resources.
>
> For this reason, `mix test` supports partitioning the test files across different Elixir instances. This is done by setting the `--partitions` option to an integer, with the number of partitions, and setting the `MIX_TEST_PARTITION` environment variable to control which test partition that particular instance is running. This can also be useful if you want to distribute testing across multiple machines."

```bash
MIX_TEST_PARTITION=1 mix test --partitions 4
MIX_TEST_PARTITION=2 mix test --partitions 4
MIX_TEST_PARTITION=3 mix test --partitions 4
MIX_TEST_PARTITION=4 mix test --partitions 4
```

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "The test files are sorted upfront in a round-robin fashion. Note the partition itself is given as an environment variable so it can be accessed in config files and test scripts. For example, it can be used to setup a different database instance per partition in `config/test.exs`."

`--partitions 1` is a no-op. When partitioning is combined with `--cover`, no on-the-fly report is generated; instead each partition exports `cover/MIX_TEST_PARTITION.coverdata`, and `mix test.coverage` compiles the unified report.

### Aborting and watching

From [Mix.Tasks.Test.html](https://hexdocs.pm/mix/Mix.Tasks.Test.html):

> "It is possible to abort the test suite with `Ctrl+\`, which sends a SIGQUIT signal to the Erlang VM. ExUnit will intercept this signal to show all tests that have been aborted and print the results collected so far."

`--listen-on-stdin` re-runs the suite on each newline, for integration with file-system watchers:

```bash
fswatch lib test | mix test --listen-on-stdin
fswatch lib test | mix test --stale --listen-on-stdin
```

### Mix project configuration

From [Mix.Tasks.Test.html "Configuration"](https://hexdocs.pm/mix/Mix.Tasks.Test.html), the relevant `def project` keys are:

| Key | Default | Purpose |
|---|---|---|
| `:test_paths` | `["test"]` if the directory exists, else `[]` | Directories scanned for tests. Each must contain a `test_helper.exs`. |
| `:test_pattern` | `"*.{ex,exs}"` (since v1.19.0; was `*_test.exs` before) | Glob for candidate test files. |
| `:test_load_filters` | `[&String.ends_with?(&1, "_test.exs")]` | Files/regexps/one-arity fns that select which `:test_pattern` matches are loaded. Paths are relative to the project root, `/`-separated even on Windows. |
| `:test_ignore_filters` | ignores `*_helper.exs` and `:elixirc_paths` entries | Files/regexps/one-arity fns that drop matches without warning. Any non-loaded, non-ignored file emits a warning; use `[fn _ -> true end]` to silence all warnings. |
| `:test_coverage` | unset | Coverage tool configuration. See "Coverage" above. |
| `:test_elixirc_options` | disables debug chunk, docs chunk, and module type inference | Compiler options used when loading/compiling test files. |

Discovery pipeline: `:test_paths` → `:test_pattern` glob → `:test_load_filters` keep → `:test_ignore_filters` drop → remaining unmatched files warn. To put support modules under `test/support/` without warnings, either add `test/support` to `:elixirc_paths` (so they compile as part of the app) or add an ignore filter:

```elixir
def project do
  [
    # ...
    elixirc_paths: ["lib", "test/support"],
    test_ignore_filters: [&String.starts_with?(&1, "test/support/")]
  ]
end
```

### Examples

#### Run a focused subset

```bash
mix test                                  # full suite
mix test test/some/file_test.exs          # one file
mix test test/some/dir                    # one directory
mix test test/foo_test.exs:42            # one test by line
mix test --only integration              # only :integration (fails if none)
mix test --exclude slow --exclude external
mix test --seed 0                         # definition order, reproducible
mix test --seed 12345                     # fixed random order
mix test --max-failures 1                 # stop at first failure
mix test --trace                          # one case at a time, print each, ignore timeouts
mix test --stale                          # only tests affected by recent changes
mix test --failed                         # rerun last run's failures
mix test --cover                          # coverage report under cover/
mix test --slowest 10                     # 10 slowest tests (forces --trace)
mix test --dry-run                        # list what would run (v1.19.0+)
mix test --partitions 4                   # needs MIX_TEST_PARTITION set
mix test --warnings-as-errors            # warnings fail the suite
mix test --raise                         # raise instead of continuing Mix tasks
```

#### CI partition matrix (4 shards)

```bash
# shard 1
MIX_TEST_PARTITION=1 mix test --partitions 4 --cover
# ... shards 2-4 ...
mix test.coverage    # unify cover/*.coverdata into one report
```

#### Coverage config in `mix.exs`

```elixir
def project do
  [
    app: :my_app,
    # ...
    test_coverage: [tool: ExCoveralls, output: "cover", threshold: 85]
  ]
end
```

### Common mistakes

- Forgetting that `test/test_helper.exs` must call `ExUnit.start()`; Mix loads the helper but does not start ExUnit.
- Expecting `--include external` to re-enable tests that are excluded by default in the helper; includes only apply after an exclude — use `--only external` or `--exclude test`.
- Using `--only` and expecting an empty match to pass; it fails the run.
- Expecting `--stale` to be precise about individual tests; it tracks whole test files against transitively-referenced modules.
- Running `--cover` with `--partitions` and expecting an inline report; partitioning exports `.coverdata` files that must be unified with `mix test.coverage`.
- Setting `--seed 0` expecting global determinism across files; it only fixes order **within** each file (files themselves are still required in parallel).
- Confusing `--max-cases` (concurrent test modules at runtime) with `--max-requires` (concurrent test file compilation).
- Treating `--warnings-as-errors` failures as retriable with `--failed`; they are not.
- Putting support modules under `test/` without an ignore filter or `:elixirc_paths` entry, then being surprised by "not a test file" warnings.

Sources: https://hexdocs.pm/mix/Mix.Tasks.Test.html, https://hexdocs.pm/mix/Mix.Tasks.Test.Coverage.html, https://hexdocs.pm/ex_unit/ExUnit.html, https://hexdocs.pm/ex_unit/ExUnit.Case.html

## Review checklist

- [ ] `test/test_helper.exs` exists and calls `ExUnit.start()` or `ExUnit.start/1`.
- [ ] There is no public `ExUnit.start/2` usage; configuration is passed via `ExUnit.start/1` or `ExUnit.configure/1`.
- [ ] Async modules (`async: true`) do not mutate global state.
- [ ] `:max_cases` / `:seed` / `:trace` / `:timeout` values are chosen deliberately, not copied without reason.
- [ ] `:capture_log` default is appropriate for the project; per-test `@tag capture_log: ...` overrides are used where needed.
- [ ] Custom formatters are plain `GenServer`s with `init/1` and `handle_cast/2`; no fabricated `__format__` callback or behaviour.
- [ ] Color and formatter options match the project's CI output needs.
- [ ] Seed is fixed when reproducibility matters, random otherwise.
- [ ] `config/test.exs` `config :ex_unit` usage, if any, is treated as an idiom rather than an official API guarantee.

## Implementation checklist

- [ ] Create `test/test_helper.exs` with `ExUnit.start()` and the project's default options.
- [ ] Decide on `async: true` eligibility per test module; default to `async: false` for modules that touch global state.
- [ ] Set `:exclude` / `:include` defaults for tags like `:external`, `:integration`, or `:flaky`.
- [ ] Configure `:capture_log` globally if the project logs heavily during tests.
- [ ] Add a custom formatter only if the default `ExUnit.CLIFormatter` is insufficient; implement it as a `GenServer`.
- [ ] Set `:seed` to a fixed value in CI when diagnosing ordering bugs.
- [ ] Verify `mix test` flags are documented in the project README if the team relies on non-default flags.
- [ ] Keep `ExUnit.start/1` options in one place (helper or config) rather than scattered.

## Validation hooks

- `mix test` — run the full suite.
- `mix test --trace` — run with one case at a time and print each test.
- `mix test --seed 0` — run tests in definition order to check for ordering dependencies.
- `mix test --max-failures 1` — stop on the first failure.
- `mix test --only external` — run a tagged subset.
- `mix test --exclude external` — skip a tagged subset.
- `mix test --stale` — run only tests affected by recent changes.
- `mix test --failed` — rerun tests that failed last time.
- `mix test --dry-run` — list tests that would run (v1.19.0+).
- `mix test --warnings-as-errors` — fail on compilation warnings from the test suite.
- `mix test --raise` — fail fast when composed with other Mix tasks.

## Examples

### Minimal `test_helper.exs`

```elixir
# test/test_helper.exs
ExUnit.start()
```

### `test_helper.exs` with common options

```elixir
# test/test_helper.exs
ExUnit.start(
  capture_log: true,
  exclude: [external: true],
  formatters: [ExUnit.CLIFormatter, MyApp.JsonFormatter],
  seed: 12345,
  timeout: 120_000
)
```

### `config/test.exs` idiom

```elixir
# config/test.exs
import Config

config :ex_unit,
  assert_receive_timeout: 200,
  capture_log: true,
  exclude: [external: true]
```

### Async vs synchronous modules

```elixir
# Runs concurrently with other async modules
defmodule MyApp.PureTest do
  use ExUnit.Case, async: true

  test "addition" do
    assert 1 + 1 == 2
  end
end

# Runs serially
defmodule MyApp.StatefulTest do
  use ExUnit.Case

  test "modifies application env" do
    Application.put_env(:my_app, :flag, true)
    assert Application.get_env(:my_app, :flag) == true
  end
end
```

### Custom formatter GenServer

```elixir
defmodule MyApp.JsonFormatter do
  use GenServer

  def init(opts), do: {:ok, opts}

  def handle_cast({:suite_started, opts}, state) do
    {:noreply, state}
  end

  def handle_cast({:test_finished, test}, state) do
    {:noreply, state}
  end

  def handle_cast({:suite_finished, _times_us}, state) do
    {:noreply, state}
  end

  def handle_cast(_event, state), do: {:noreply, state}
end

# test/test_helper.exs
ExUnit.start(formatters: [ExUnit.CLIFormatter, MyApp.JsonFormatter])
```

### Per-test log capture

```elixir
defmodule MyApp.WarningTest do
  use ExUnit.Case

  @tag capture_log: true
  test "logs are captured and only shown on failure" do
    Logger.warning("this is captured")
    assert true
  end

  @tag capture_log: false
  test "logs are printed immediately" do
    Logger.warning("this is not captured")
    assert true
  end
end
```

### Fixed seed for reproducibility

```elixir
# test/test_helper.exs
ExUnit.start(seed: 42_000)
```

```bash
mix test --seed 42000
```

### Manual run with `autorun: false`

```elixir
ExUnit.start(autorun: false)
# optionally load extra modules
result = ExUnit.run()
IO.inspect(result)
```

## Common mistakes

- Calling the non-public `ExUnit.start/2` (the OTP application callback) instead of `ExUnit.start/0` or `ExUnit.start/1`.
- Forgetting to call `ExUnit.start()` in `test/test_helper.exs`.
- Marking tests `async: true` when they read or write global state (application env, ETS tables, files, registered processes).
- Expecting tests inside the same module to run concurrently; only different modules run concurrently.
- Confusing `capture_log` with `capture_io`: `capture_io` captures `IO.puts`/`IO.write`, while `capture_log` captures `Logger` messages.
- Assuming there is a built-in JSON formatter; only `ExUnit.CLIFormatter` ships with ExUnit.
- Treating `:fail_fast` as an option; use `:max_failures` instead.
- Treating `:default_formatter` as an option; use `:formatters` instead.
- Setting `--seed 0` without realizing it disables per-file randomization.
- Running trace mode in CI without accounting for the `:infinity` timeout change.
- Mentioning `ExUnit.Properties`; property-based testing uses the separate `StreamData` library.

## Strict vs contextual guidance

### Strict

- `test/test_helper.exs` MUST call `ExUnit.start/0` or `ExUnit.start/1`.
- `ExUnit.start/2` is NOT a public API; do not call it.
- `ExUnit.run/0` MUST be used only when `autorun: false`.
- `:max_cases` limits concurrency across modules; tests within a module never run concurrently.
- Async tests MUST NOT mutate global state.
- A formatter MUST be a `GenServer` exporting `init/1` and `handle_cast/2`.
- There is no built-in JSON formatter; `ExUnit.CLIFormatter` is the only shipped formatter.

### Conventions (not enforced by the compiler)

- Use `ExUnit.configure/1` before `ExUnit.start()` or pass options directly to `ExUnit.start/1`.
- Default to `async: false` and opt into `async: true` only when the module is stateless.
- Use `@tag capture_log: true/false` to override the global `:capture_log` default.
- Use `:seed` 0 to debug ordering dependencies within a file.
- Use `:trace` for local debugging; it forces serial execution and disables timeouts.
- Use `config/test.exs` `config :ex_unit` as an idiom, but recognize it is not an official ExUnit API guarantee.

### Contextual tradeoffs

- Global `capture_log: true` is convenient but can hide useful output on failures; choose per-project.
- Fixed seeds improve reproducibility but may hide order-dependent bugs; random seeds are the default for a reason.
- Custom formatters add value for CI dashboards but increase maintenance.
- `config/test.exs` centralizes config, while `test/test_helper.exs` keeps test config colocated with test code.

## Policy decisions for individual repos

- Decide whether `async: true` is the default for new test modules or requires explicit justification.
- Choose a global `:capture_log` default and a policy for per-test overrides.
- Decide whether to fix `:seed` in CI or always run randomly.
- Choose whether custom formatters are allowed/required for CI reporting.
- Decide whether `:max_cases` should be tuned for the CI runner size.
- Set a `:timeout` policy for slow tests (global default vs `@tag timeout:` per test).
- Choose which tags to exclude by default (`:external`, `:integration`, `:flaky`, etc.).
- Decide whether `config/test.exs` may configure `:ex_unit` or whether all ExUnit config belongs in `test/test_helper.exs`.
- Determine whether `--warnings-as-errors` is enabled for test compilation in CI.

## Related docs

- `docs/elixir/naming-conventions.md` — Elixir identifier conventions for test modules and functions.
- `docs/elixir/mix-project-structure.md` — Mix project layout, including `test/` and `config/test.exs`.
- `docs/elixir/static-analysis-credo.md` — linting and style checks for Elixir code.
- `docs/testing.md` — repo-level testing entry point.
- Related Elixir corpus docs: `docs/elixir/language-fundamentals.md` (modules and functions), `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.

## Related BEAM guidance

- `../beam/validation.md` — shared runtime validation hooks and test-gate parallels; ExUnit itself is Elixir-native but the validation-gate concepts cross-reference.

## Related skills

- None defined yet.
