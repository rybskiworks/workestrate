# StreamData: Getting Started

## Purpose

Install StreamData, write a first property, and understand the defining feature that separates StreamData from every other Elixir PBT library: **generators are lazy streams** (they implement `Enumerable`), so the same generator that feeds `check all` inside a test can be piped through `Stream`/`Enum` outside a test.

## Sources used

- https://hexdocs.pm/stream_data/ExUnitProperties.html — `use ExUnitProperties`, `property/3`, `check all`, `gen all`, `pick/1`
- https://hexdocs.pm/stream_data/StreamData.html — generator reference
- https://github.com/whatyouhide/stream_data — README, profile, license

This page reflects `stream_data` 1.3.0 (Apache-2.0, co-owned by José Valim and Andrea Leopardi).

## Library profile

- **Hex package:** `stream_data`
- **Version pinned here:** 1.3.0
- **License:** Apache-2.0
- **Maintainers:** José Valim, Andrea Leopardi
- **Defining feature:** generators implement `Enumerable` — they are lazy streams usable outside tests.

## Setup

### `mix.exs`

```elixir
defp deps do
  [
    {:stream_data, "~> 1.0", only: :test}
  ]
end
```

### `.formatter.exs`

So `mix format` knows about StreamData's macros, add it to `import_deps`:

```elixir
[
  import_deps: [:stream_data]
]
```

### `test/test_helper.exs`

StreamData integrates with ExUnit; no special start call is needed beyond the usual:

```elixir
# test/test_helper.exs
ExUnit.start()
```

## Core property syntax

Bring in the macros with `use ExUnitProperties` inside an `ExUnit.Case` module:

```elixir
defmodule MyTest do
  use ExUnit.Case, async: true
  use ExUnitProperties

  property "reversing a list doesn't change its length" do
    check all list <- list_of(integer()) do
      assert length(list) == length(:lists.reverse(list))
    end
  end
end
```

`property/3` is just like `ExUnit.Case.test/3` — it registers a test with ExUnit. `mix test` runs it like any other test. See [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) for the ExUnit integration details.

## Multi-clause `check all` with filters and bindings

`check all` supports multiple generator bindings, a `when`-style filter clause, and `=` bindings computed from the generated values:

```elixir
property "the sum of two positives is greater than either" do
  check all int1 <- integer(),
            int2 <- integer(),
            int1 > 0 and int2 > 0,
            sum = int1 + int2 do
    assert sum > int1
    assert sum > int2
  end
end
```

The clause order is: bindings (`<-`), filters (bare boolean expressions), then `=` assignments. Filters that fail too often cause `StreamData.FilterTooNarrowError` (see [`streamdata-combinators.md`](streamdata-combinators.md) → `filter/3`).

## A property inside a `test` block

`property/3` is sugar over `test/3`. You can also write a property inside an ordinary `test` block — `check all` is a macro available wherever `use ExUnitProperties` has been called:

```elixir
defmodule MyTest do
  use ExUnit.Case, async: true
  use ExUnitProperties

  test "list reversal is its own inverse" do
    check all list <- list_of(integer()) do
      assert :lists.reverse(:lists.reverse(list)) == list
    end
  end
end
```

## `check all` options

`check all` accepts options as the first argument (a keyword list) or via the `check all(..., [opts])` form. The documented options:

```elixir
check all list <- list_of(integer()),
          initial_size: 1,
          max_runs: 100,
          max_shrinking_steps: 100 do
  assert is_list(list)
end
```

| Option | Meaning |
|---|---|
| `:initial_size` | Starting size for the size parameter (default `1`). |
| `:max_runs` | Maximum number of generated values to test (default `100`). |
| `:max_shrinking_steps` | Cap on shrinking iterations when a counterexample is found (default `100`). |
| `:initial_seed` | Pin the random seed (an integer) for reproducible counterexamples. |

The seed of a failing run is printed by ExUnit; re-running with `:initial_seed` set to that integer reproduces the exact failing case.

## Generators as streams (the defining feature)

Because every StreamData generator implements `Enumerable`, you can pipe it through `Stream`/`Enum` directly — no test context required:

```elixir
StreamData.integer()
|> Stream.filter(&(&1 > 0))
|> Stream.map(&(&1 * 2))
|> Enum.take(10)
```

> **Warning:** `Stream`/`Enum` manipulation **disables shrinking**. A generator that has been through `Stream.filter`/`Stream.map`/`Enum.take` is no longer a StreamData generator with shrink directions — it is an ordinary lazy stream. Use this for ad-hoc data generation or REPL exploration; for properties that need shrinking, build the generator with StreamData combinators (`filter/3`, `map/2`, `bind/2`) instead. See [`streamdata-combinators.md`](streamdata-combinators.md) and [`streamdata-shrinking.md`](streamdata-shrinking.md).

## `pick/1`

`pick/1` materializes a single value from a generator (with a random seed). It is the canonical way to "just give me one sample":

```elixir
StreamData.pick(StreamData.integer())
#=> 42
```

`pick/1` is useful in ordinary `test` blocks or in `iex` when you want a concrete value without running a full property.

## Project-wide configuration

StreamData reads the `:max_runs` application env at compile/run time, so you can scale runs up in CI and down locally:

```elixir
# config/config.exs
import Config

config :stream_data, max_runs: if System.get_env("CI"), do: 1000, else: 50
```

See [`streamdata-shrinking.md`](streamdata-shrinking.md) for the full configuration reference.

## Related docs

- [`streamdata-combinators.md`](streamdata-combinators.md) — the full generator vocabulary.
- [`streamdata-shrinking.md`](streamdata-shrinking.md) — how counterexamples are minimized.
- [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) — `property/3` and `async: true`.
- [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) — when StreamData is the right choice.
- [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md) — ExUnit itself.
