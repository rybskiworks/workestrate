# StreamData Combinators

## Purpose

Reference the full StreamData 1.3.0 generator combinator library. Every snippet below is a runnable `Enum.take/2` against a `StreamData.*` generator, lifted from the HexDocs examples.

## Sources used

- https://hexdocs.pm/stream_data/StreamData.html — the combinator reference (primary source for every snippet below)

This page reflects `stream_data` 1.3.0.

## Numeric generators

```elixir
StreamData.integer() |> Enum.take(3)
#=> [-1, 0, 1]

StreamData.integer(0..10) |> Enum.take(3)
#=> [4, 7, 2]

StreamData.positive_integer() |> Enum.take(3)
#=> [1, 3, 2]

StreamData.non_negative_integer() |> Enum.take(3)
#=> [0, 2, 1]

StreamData.boolean() |> Enum.take(3)
#=> [false, true, false]

StreamData.byte() |> Enum.take(3)
#=> [0, 255, 42]

StreamData.float(min: 0.0, max: 1.0) |> Enum.take(3)
#=> [0.5, 0.1, 0.9]
```

## Binary and string generators

```elixir
StreamData.binary() |> Enum.take(1)
#=> [<<0, 1, 2>>]

StreamData.bitstring() |> Enum.take(1)
#=> [<<0, 1::1>>]

StreamData.string(:ascii) |> Enum.take(3)
#=> ["a", "B", "0"]

StreamData.string(Enum.concat([?a..?c, ?l..?o])) |> Enum.take(3)
#=> ["a", "lo", "cm"]

StreamData.codepoint() |> Enum.take(3)
#=> [97, 98, 99]

StreamData.chardata() |> Enum.take(1)
#=> ["hello"]

StreamData.iodata() |> Enum.take(1)
#=> [["h", "i"]]
```

## Container generators

```elixir
StreamData.list_of(StreamData.integer()) |> Enum.take(1)
#=> [[1, 2, 3]]

StreamData.map_of(StreamData.atom(:alphanumeric), StreamData.integer()) |> Enum.take(1)
#=> [%{foo: 1, bar: 2}]

StreamData.tuple({StreamData.integer(), StreamData.boolean()}) |> Enum.take(1)
#=> [{1, true}]

StreamData.fixed_list([StreamData.integer(), StreamData.boolean()]) |> Enum.take(1)
#=> [1, true]

StreamData.keyword_of(StreamData.integer()) |> Enum.take(1)
#=> [foo: 1, bar: 2]

StreamData.mapset_of(StreamData.integer()) |> Enum.take(1)
#=> [MapSet.new([1, 2, 3])]

StreamData.nonempty(StreamData.list_of(StreamData.integer())) |> Enum.take(1)
#=> [[1, 2, 3]]

StreamData.fixed_map(%{name: StreamData.string(:alphanumeric), age: StreamData.integer(0..120)}) |> Enum.take(1)
#=> [%{name: "abc", age: 42}]

StreamData.optional_map(%{name: StreamData.string(:alphanumeric), age: StreamData.integer(0..120)}) |> Enum.take(1)
#=> [%{name: "abc"}]
```

> **Note:** `nonempty_list_of/1` does **not** exist in StreamData 1.3.0. Use `nonempty(list_of(...))` as shown above.

## Choice generators

```elixir
StreamData.one_of([StreamData.integer(), StreamData.boolean()]) |> Enum.take(3)
#=> [1, true, 0]

StreamData.member_of([:foo, :bar, :baz]) |> Enum.take(3)
#=> [:foo, :baz, :bar]

StreamData.frequency([
  {1, StreamData.constant(:rare)},
  {9, StreamData.constant(:common)}
]) |> Enum.take(10)
#=> [:common, :common, :rare, :common, ...]

StreamData.shuffle([1, 2, 3, 4, 5]) |> Enum.take(1)
#=> [[3, 1, 5, 2, 4]]
```

## Combinators

### `bind/2` — dependent generators

`bind/2` takes a generator and a function that returns a new generator based on the generated value. (Note: `bind/3` does **not** exist in 1.3.0 — use `bind/2`.)

```elixir
StreamData.bind(StreamData.list_of(StreamData.integer()), fn list ->
  StreamData.member_of(list)
end)
|> Enum.take(1)
#=> [3]
```

A nested `bind/2` with `constant/1`:

```elixir
StreamData.bind(StreamData.integer(), fn n ->
  StreamData.bind(StreamData.integer(), fn m ->
    StreamData.constant({n, m})
  end)
end)
|> Enum.take(1)
#=> [{3, 7}]
```

For multi-binding without nested `bind/2`, prefer the `gen all` special form from `ExUnitProperties` (see [`streamdata-getting-started.md`](streamdata-getting-started.md)).

### `map/2` — transform generated values

```elixir
StreamData.map(StreamData.integer(), &(&1 * 2)) |> Enum.take(3)
#=> [-2, 0, 2]
```

### `filter/3` — reject generated values

`filter/3` retries the underlying generator up to 25 times; if no value passes, it raises `StreamData.FilterTooNarrowError`:

```elixir
StreamData.filter(StreamData.integer(), &(&1 > 0)) |> Enum.take(3)
#=> [1, 3, 2]
```

## Size control

StreamData generators are size-driven: the size parameter grows from `:initial_size` up across `:max_runs`. Three combinators manipulate size directly:

```elixir
# Force a fixed size
StreamData.resize(StreamData.list_of(StreamData.integer()), 5) |> Enum.take(1)
#=> [[1, 2, 3, 4, 5]]

# Scale the size by a function
StreamData.scale(StreamData.list_of(StreamData.integer()), &(&1 * 2)) |> Enum.take(1)
#=> [a longer list]

# Inspect and choose the size
StreamData.sized(fn size ->
  StreamData.list_of(StreamData.integer(), length: size)
end)
|> Enum.take(1)
#=> [[1, 2, 3]]
```

## Trees: `tree/2`

`tree/2` builds a recursive generator from a leaf generator and a "node" combinator. The node combinator receives a generator of children and returns a generator of nodes. (Note: `tree/1` does **not** exist — `tree/2` is the only arity.)

```elixir
defmodule Branch do
  defstruct [:left, :right]
end

leaf = StreamData.integer()

node = fn children_gen ->
  StreamData.map(
    StreamData.tuple({children_gen, children_gen}),
    fn {left, right} -> %Branch{left: left, right: right} end
  )
end

StreamData.tree(leaf, node) |> Enum.take(1)
#=> [%Branch{left: 1, right: %Branch{left: 2, right: 3}}]
```

## Atoms and terms

```elixir
StreamData.atom(:alphanumeric) |> Enum.take(3)
#=> [:a, :b, :c]

StreamData.term() |> Enum.take(1)
#=> [:some_term]
```

> **Note:** `atom/0` and `atom_of/1` do **not** exist in 1.3.0. Use `atom(:alphanumeric)` (the only documented arity/argument shape) or `member_of([...])` for a fixed set of atoms. `any/0` does not exist — use `term/0`.

## `date/0` (1.3.0)

```elixir
StreamData.date() |> Enum.take(1)
#=> [~D[2021-01-15]]
```

> **Note:** `time/0` and `datetime/0` do **not** exist in 1.3.0. For `Time` or `DateTime`, compose from primitives (e.g. `map(StreamData.integer(0..86_399), &Time.from_seconds_after_midnight/1)`).

## Wrappers

```elixir
# nullable/2 — wraps a generator, sometimes returning nil
StreamData.nullable(StreamData.integer(), 0.5) |> Enum.take(3)
#=> [nil, 1, nil]

# constant/1 — always returns the same value
StreamData.constant(:always) |> Enum.take(3)
#=> [:always, :always, :always]

# repeatedly/1 — calls the given zero-arity function each time
StreamData.repeatedly(fn -> :erlang.unique_integer() end) |> Enum.take(3)
#=> [-1, -2, -3]

# seeded/2 — like repeatedly/1 but seeded for reproducibility
StreamData.seeded(0, fn -> :rand.uniform(100) end) |> Enum.take(3)
#=> [44, 37, 82]

# unshrinkable/1 — opt out of shrinking for this generator
StreamData.unshrinkable(StreamData.integer()) |> Enum.take(1)
#=> [42]
```

`unshrinkable/1` is covered in depth in [`streamdata-shrinking.md`](streamdata-shrinking.md).

## Related docs

- [`streamdata-getting-started.md`](streamdata-getting-started.md) — setup and `check all`.
- [`streamdata-shrinking.md`](streamdata-shrinking.md) — per-generator shrink directions.
- [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) — `property/3`.
