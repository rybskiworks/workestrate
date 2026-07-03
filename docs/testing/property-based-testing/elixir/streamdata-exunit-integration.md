# StreamData ExUnit Integration

## Purpose

Document how StreamData integrates with ExUnit via the `ExUnitProperties` module: `use ExUnitProperties`, the `property/3` macro, and how `async: true` interacts with property tests.

## Sources used

- https://hexdocs.pm/stream_data/ExUnitProperties.html — `use ExUnitProperties`, `property/3`, `check all`, `gen all`, `pick/1`

This page reflects `stream_data` 1.3.0.

## `ExUnitProperties`

`ExUnitProperties` is the module that bridges StreamData generators to ExUnit. Bring it into an `ExUnit.Case` module with `use ExUnitProperties`:

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

`use ExUnitProperties` imports the `property/3` macro and the `check all` / `gen all` special forms into the module.

## `property/3` is `ExUnit.Case.test/3`

From [ExUnitProperties.html](https://hexdocs.pm/stream_data/ExUnitProperties.html):

> `property/3` is just like `ExUnit.Case.test/3`.

That means:

- `property "name" do ... end` registers an ExUnit test named `"name"`.
- `mix test` runs it like any other test.
- `mix test path/to/file.exs:LINE` jumps to a specific property by line.
- `@tag :property` and `describe` blocks work as expected.
- The `:async` setting of the enclosing `ExUnit.Case` module applies.

A property with a `describe` block and a tag:

```elixir
defmodule ListPropsTest do
  use ExUnit.Case, async: true
  use ExUnitProperties

  describe "reversal" do
    @tag :property
    property "is its own inverse" do
      check all list <- list_of(integer()) do
        assert :lists.reverse(:lists.reverse(list)) == list
      end
    end
  end
end
```

## `async: true`

`async: true` is set on the `ExUnit.Case`, not on `ExUnitProperties`. Because `property/3` is sugar over `test/3`, a property inherits the module's `:async` setting:

```elixir
defmodule AsyncPropsTest do
  use ExUnit.Case, async: true
  use ExUnitProperties

  property "runs concurrently with other async modules" do
    check all n <- integer() do
      assert n * 2 == n + n
    end
  end
end
```

> **Note (inferred, not StreamData-documented):** StreamData's own docs do not explicitly call out `async: true` behavior. Because `property/3` compiles down to an ordinary ExUnit test, the standard ExUnit async semantics apply: tests in the same module run **sequentially** (ExUnit never runs two tests from one module in parallel), but tests across **different** async modules run concurrently. See [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md) for the ExUnit async model. Treat the `async: true` + property combination as ExUnit behavior, not StreamData behavior.

## `pick/1` outside tests

`pick/1` is available without `use ExUnitProperties` — it is a function on the `StreamData` module. It materializes a single value from a generator and is useful in ordinary `test` blocks or in `iex`:

```elixir
defmodule PickTest do
  use ExUnit.Case, async: true

  test "pick gives me a concrete value" do
    value = StreamData.pick(StreamData.integer())
    assert is_integer(value)
  end
end
```

## Related docs

- [`streamdata-getting-started.md`](streamdata-getting-started.md) — setup, `check all` options.
- [`streamdata-combinators.md`](streamdata-combinators.md) — the generator vocabulary.
- [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md) — ExUnit itself, including the async model.
