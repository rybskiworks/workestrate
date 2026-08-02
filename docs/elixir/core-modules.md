# Core Modules and API Design

## Purpose

This document is a multi-section reference for Elixir's core modules and the API-design patterns they follow. It is intended for future AI agents who write, review, refactor, debug, or validate Elixir code. Each module section explains the evaluation model, key functions, common idioms, naming patterns, complexity characteristics, and version history that agents need to reason about locally without re-reading HexDocs from scratch.

This file covers `Enum`, `Stream`, `Map`, `Keyword`, `List`, `String`, `URI`, `Path`, `File`, `Code`, `Kernel`, and `Kernel.SpecialForms` in depth.

## Sources used

- https://hexdocs.pm/elixir/Enum.html (PRIMARY)
- https://hexdocs.pm/elixir/Stream.html
- https://hexdocs.pm/elixir/File.Stream.html
- https://hexdocs.pm/elixir/IO.Stream.html
- https://hexdocs.pm/elixir/IO.html
- https://hexdocs.pm/elixir/Enumerable.html
- https://hexdocs.pm/elixir/Collectable.html
- https://hexdocs.pm/elixir/enumerable-and-streams.html
- https://hexdocs.pm/elixir/Range.html
- https://hexdocs.pm/elixir/comprehensions.html
- https://hexdocs.pm/elixir/Kernel.SpecialForms.html#for/1
- https://hexdocs.pm/elixir/enum-cheat.html
- https://hexdocs.pm/elixir/naming-conventions.html

Map and related (used by the `Map` section):

- https://hexdocs.pm/elixir/Map.html (PRIMARY for Map section)
- https://hexdocs.pm/elixir/MapSet.html
- https://hexdocs.pm/elixir/Access.html
- https://hexdocs.pm/elixir/Keyword.html
- https://hexdocs.pm/elixir/keywords-and-maps.html
- https://hexdocs.pm/elixir/structs.html
- https://hexdocs.pm/elixir/Module.html
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/compatibility-and-deprecations.html

Keyword and related (used by the `Keyword` section):

- https://hexdocs.pm/elixir/Keyword.html (PRIMARY for Keyword section)
- https://hexdocs.pm/elixir/keywords-and-maps.html
- https://hexdocs.pm/elixir/Access.html
- https://hexdocs.pm/elixir/Module.html
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/compatibility-and-deprecations.html

List and related (used by the `List` section):

- https://hexdocs.pm/elixir/List.html (PRIMARY for List section)
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/Tuple.html
- https://hexdocs.pm/elixir/IO.html
- https://hexdocs.pm/elixir/Enum.html
- https://hexdocs.pm/elixir/compatibility-and-deprecations.html

String and related (used by the `String` section):

- https://hexdocs.pm/elixir/String.html (PRIMARY for String section)
- https://hexdocs.pm/elixir/binaries-strings-and-charlists.html
- https://hexdocs.pm/elixir/Regex.html
- https://www.erlang.org/doc/man/binary.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/string.ex

Kernel.SpecialForms and related (used by the `Kernel.SpecialForms` section):

- https://hexdocs.pm/elixir/Kernel.SpecialForms.html (PRIMARY for Kernel.SpecialForms section)
- https://hexdocs.pm/elixir/comprehensions.html
- https://hexdocs.pm/elixir/pattern-matching.html
- https://hexdocs.pm/elixir/structs.html
- https://hexdocs.pm/elixir/Kernel.html
- https://hexdocs.pm/elixir/Range.html
- https://hexdocs.pm/elixir/Function.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/kernel/special_forms.ex

This page reflects Elixir v1.20.2 docs.

## Enum

### Overview & evaluation model

From [Enum.html](https://hexdocs.pm/elixir/Enum.html):

> "The `Enum` module provides a set of algorithms to work with enumerables. It treats maps as two-element tuples `{key, value}` and therefore most functions are not suitable for working directly with maps. To manipulate maps, use the `Map` module instead."

> "The functions in this module are eager: they enumerate the enumerable immediately. Most of the functions also return a list. In particular, functions that map over an enumerable (such as `Enum.map/2`) return a list. Because the functions are eager, passing an infinite enumerable might run forever. If that is not the desired behaviour, `Stream` module offers lazy alternatives for many of these functions."

From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "The `Enum` module provides a huge range of functions to transform, sort, group, filter and retrieve enumerable items, and they are eager. Many functions in `Enum` are pipe-friendly, making it easier to compose operations."

> "The `Enum` module is extremely useful for a wide range of use cases. However, there are scenarios where using it can be expensive. For example, computing the square of each element in a range may generate intermediate lists. Such intermediate lists can be avoided by using `Stream` instead."

`Enum` is eager: every call traverses its input and produces a result, usually a list. Composing several `Enum` functions builds intermediate lists at each step. This is fine for small or finite data, but it is particularly dangerous when working with infinite enumerables because the operation will not terminate.

Common enumerable types include:

| Type | Notes |
|---|---|
| `list` | The canonical enumerable; `Enum` returns lists by default. |
| `Range.t` | Provides `Enumerable.slice/1` for O(1) count, member, at, etc. |
| `Map.t` | Iterated as two-element tuples; `Enum` returns lists of tuples, not maps. |
| `MapSet.t` | Set-backed enumerable with efficient membership. |
| `File.Stream!` | File-backed lazy stream; still eager if passed to `Enum`. |
| `IO.Stream` / `IO.binstream` | Stream over standard IO. |
| `Function` | An arity-1 function can be an enumerable when wrapped as a `Stream`. |
| `tuple` | Not enumerable directly; use `Tuple.to_list/1`. |

Source: [Enumerable.html](https://hexdocs.pm/elixir/Enumerable.html).

### Key functions

The summary tables below group `Enum` functions by purpose. Complexities are given for a list of `n` elements unless otherwise noted. Range and other structures that implement `Enumerable.slice/1` may be O(1) for random access and count.

#### Building / into

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.into(enumerable, collectable)` | Inserts each element into a `Collectable`. | O(n) |
| `Enum.into(enumerable, collectable, transform)` | Transforms each element before insertion. | O(n) |

```elixir
Enum.into([{:a, 1}, {:b, 2}], %{})      #=> %{a: 1, b: 2}
Enum.into(%{a: 1, b: 2}, [])            #=> [a: 1, b: 2]
Enum.into([1, 2, 3], [], &(&1 * 2))     #=> [2, 4, 6]
```

`into/2` is the bridge between `Enumerable` and `Collectable`. Source: [Collectable.html](https://hexdocs.pm/elixir/Collectable.html).

#### Traversal

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.map(enumerable, fun)` | Applies `fun` to each element; returns list of results. | O(n) |
| `Enum.each(enumerable, fun)` | Applies `fun` for side effects; returns `:ok`. | O(n) |
| `Enum.reduce(enumerable, acc, fun)` | Folds over the enumerable. | O(n) |
| `Enum.reduce(enumerable, fun)` | Folds with the first element as the initial accumulator. | O(n) |
| `Enum.reduce_while(enumerable, acc, fun)` | Folds with early termination via `{:cont, acc}` / `{:halt, acc}`. | O(n) |
| `Enum.scan(enumerable, acc, fun)` | Like `reduce` but returns each intermediate accumulator. | O(n) |
| `Enum.with_index(enumerable, offset \\ 0)` | Returns `{element, index}` tuples. | O(n) |
| `Enum.zip(enumerables)` | Zips one or more enumerables into a list of tuples. | O(n) |
| `Enum.zip_with(enumerables, fun)` | Zips and maps in one pass. | O(n) |

```elixir
Enum.map([1, 2, 3], &(&1 * 2))         #=> [2, 4, 6]
Enum.each([1, 2, 3], &IO.puts/1)       #=> :ok
Enum.reduce([1, 2, 3], 0, &+/2)        #=> 6
Enum.scan([1, 2, 3], 0, &+/2)          #=> [1, 3, 6]
Enum.with_index([:a, :b, :c])          #=> [{:a, 0}, {:b, 1}, {:c, 2}]
```

#### Filtering

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.filter(enumerable, fun)` | Returns elements where `fun` returns truthy. | O(n) |
| `Enum.reject(enumerable, fun)` | Returns elements where `fun` returns falsy. | O(n) |
| `Enum.uniq(enumerable)` | Returns unique elements (using `===`). | O(n) |
| `Enum.uniq_by(enumerable, fun)` | Returns unique elements by a derived key. | O(n) |
| `Enum.dedup(enumerable)` | Removes consecutive duplicates. | O(n) |
| `Enum.dedup_by(enumerable, fun)` | Removes consecutive duplicates by key. | O(n) |
| `Enum.frequencies(enumerable)` | Returns a map of element -> count. | O(n) |
| `Enum.frequencies_by(enumerable, fun)` | Returns a map of key -> count. | O(n) |

```elixir
Enum.filter([1, 2, 3, 4], &(&1 > 2))   #=> [3, 4]
Enum.uniq([1, 2, 2, 3, 3, 3])          #=> [1, 2, 3]
Enum.dedup([1, 1, 2, 2, 3, 3, 3])      #=> [1, 2, 3]
Enum.frequencies([:a, :a, :b, :c, :c, :c])
#=> %{a: 2, b: 1, c: 3}
```

#### Selecting

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.find(enumerable, default \\ nil, fun)` | First element matching `fun`, else `default`. | O(n) |
| `Enum.find_value(enumerable, default \\ nil, fun)` | Returns the first non-nil result of `fun`, else `default`. | O(n) |
| `Enum.find_index(enumerable, fun)` | Index of the first match, else `nil`. | O(n) |
| `Enum.at(enumerable, index, default \\ nil)` | Element at `index`, else `default`. | O(n) on list; O(1) on Range and slice-capable structures. |
| `Enum.fetch(enumerable, index)` | `{:ok, elem}` or `:error`. | O(n) on list; O(1) on slice-capable structures. |
| `Enum.fetch!(enumerable, index)` | Element or `OutOfBoundsError`. | O(n) on list; O(1) on slice-capable structures. |
| `Enum.min(enumerable, empty_fallback \\ fn -> raise Enum.EmptyError end)` | Smallest by term order. | O(n) |
| `Enum.max(enumerable, empty_fallback)` | Largest by term order. | O(n) |
| `Enum.min_by(enumerable, fun, sorter \\ &<=/2)` | Element minimizing `fun`. | O(n) |
| `Enum.max_by(enumerable, fun, sorter \\ &<=/2)` | Element maximizing `fun`. | O(n) |
| `Enum.min_max(enumerable, empty_fallback)` | `{min, max}` tuple. | O(n) |

```elixir
Enum.find([1, 2, 3], &(&1 > 1))        #=> 2
Enum.find_value([1, 2, 3], fn x -> if x > 1, do: x * 10 end)
#=> 20
Enum.at([:a, :b, :c], 1)               #=> :b
Enum.fetch([:a, :b, :c], 5)            #=> :error
Enum.fetch!([:a, :b, :c], 5)           #=> ** (Enum.OutOfBoundsError)
Enum.at(10..20, 5)                     #=> 15  # O(1) via slice
Enum.min([3, 1, 2])                    #=> 1
```

#### Counting and membership

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.count(enumerable)` | Number of elements. | O(n) generally; O(1) if `Enumerable.count/1` is implemented. |
| `Enum.count(enumerable, fun)` | Number of elements satisfying `fun`. | O(n) |
| `Enum.count_until(enumerable, limit)` | Count up to `limit`, then stop. | O(limit) |
| `Enum.count_until(enumerable, fun, limit)` | Count matches up to `limit`. | O(limit) |
| `Enum.member?(enumerable, element)` | True if `element` is present. | O(n) generally; O(1) on Range and set-like structures. |
| `Enum.empty?(enumerable)` | True if the enumerable has no elements. | O(1) if `count` is O(1); otherwise O(n). |
| `Enum.any?(enumerable, fun \\ fn x -> x end)` | True if any element is truthy / matches. | O(n) |
| `Enum.all?(enumerable, fun \\ fn x -> x end)` | True if all elements are truthy / match. | O(n) |

```elixir
Enum.count(1..10)                      #=> 10  # O(1) on Range
Enum.count([1, 2, 3, 4], &(&1 > 2))    #=> 2
Enum.count_until([1, 2, 3, 4, 5], 3)   #=> 3
Enum.member?(1..10, 5)                 #=> true
Enum.empty?([])                        #=> true
Enum.any?([false, false, true])        #=> true
Enum.all?([true, true, false])         #=> false
```

#### Sorting

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.sort(enumerable)` | Sorts by Erlang term ordering. | O(n log n) stable. |
| `Enum.sort(enumerable, sorter)` | Sorts with a custom sorter. | O(n log n) stable. |
| `Enum.sort_by(enumerable, mapper)` | Sorts by a derived key. | O(n log n) stable. |
| `Enum.sort_by(enumerable, mapper, sorter)` | Sorts by derived key with custom sorter. | O(n log n) stable. |

Elixir's `Enum.sort/1` is stable: equal elements retain their original order. It is built on Erlang's stable merge sort.

Term ordering for `Enum.sort/1` is:

```text
number < atom < reference < function < port < pid < tuple < map < list < bit string
```

This ordering produces a common gotcha with structs such as `Date`:

```elixir
Enum.max([~D[2017-03-31], ~D[2017-04-01]])
#=> ~D[2017-03-31]  # wrong by calendar semantics: 31 > 1 in term order

Enum.max([~D[2017-03-31], ~D[2017-04-01]], Date)
#=> ~D[2017-04-01]

Enum.sort_by([~D[2017-04-01], ~D[2017-03-31]], & &1, Date)
#=> [~D[2017-03-31], ~D[2017-04-01]]
```

#### Grouping and partitioning

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.chunk_every(enumerable, count, step \\ count, leftover \\ [])` | Splits into sublists of up to `count` elements. | O(n) |
| `Enum.chunk_by(enumerable, fun)` | Splits when `fun` result changes. | O(n) |
| `Enum.chunk_while(enumerable, acc, chunk_fun, after_fun)` | General chunked reduction. | O(n) |
| `Enum.group_by(enumerable, key_fun, value_fun \\ fn x -> x end)` | Builds a map keyed by `key_fun`. | O(n) |
| `Enum.split(enumerable, count)` | `{first_n, rest}` tuple. | O(count) |
| `Enum.split_while(enumerable, fun)` | Splits before the first falsy result. | O(n) |
| `Enum.split_with(enumerable, fun)` | `{matches, non_matches}` partition. | O(n) |

```elixir
Enum.chunk_every(1..10, 3)
#=> [[1, 2, 3], [4, 5, 6], [7, 8, 9], [10]]

Enum.chunk_every(1..5, 2, 2, :discard)
#=> [[1, 2], [3, 4]]

Enum.chunk_by([1, 2, 2, 3, 3, 3, 4], &(&1))
#=> [[1], [2, 2], [3, 3, 3], [4]]

Enum.group_by(~w[apple banana apricot cherry], &String.first/1)
#=> %{"a" => ["apple", "apricot"], "b" => ["banana"], "c" => ["cherry"]}

Enum.split([1, 2, 3, 4, 5], 3)
#=> {[1, 2, 3], [4, 5]}

Enum.split_with([1, 2, 3, 4, 5], fn x -> rem(x, 2) == 0 end)
#=> {[2, 4], [1, 3, 5]}
```

#### Combining

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.concat(enumerables)` | Concatenates enumerables into one list. | O(total) |
| `Enum.flat_map(enumerable, fun)` | Maps then flattens one level. | O(total) |
| `Enum.zip(enum1, enum2)` | Zips two enumerables into `{a, b}` tuples. | O(min(n, m)) |
| `Enum.zip(enumerables)` | Zips a list of enumerables. | O(min lengths) |
| `Enum.zip_with(enumerables, fun)` | Zips and applies `fun`. | O(min lengths) |
| `Enum.unzip(list_of_tuples)` | Inverse of `zip`; returns tuple of lists. | O(n) |
| `Enum.zip_reduce(enumerables, acc, fun)` | Zips and reduces in one pass. | O(min lengths) |

```elixir
Enum.concat([[1, 2], [3, 4], [5]])     #=> [1, 2, 3, 4, 5]
Enum.flat_map([1, 2, 3], fn x -> [x, x * 10] end)
#=> [1, 10, 2, 20, 3, 30]
Enum.zip([1, 2, 3], [:a, :b, :c])      #=> [{1, :a}, {2, :b}, {3, :c}]
Enum.zip_with([1, 2, 3], [10, 20, 30], &+/2)
#=> [11, 22, 33]
Enum.unzip([{1, :a}, {2, :b}])         #=> {[1, 2], [:a, :b]}
```

#### Slicing and rearranging

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.take(enumerable, count)` | First `count` elements. | O(count) |
| `Enum.drop(enumerable, count)` | All but the first `count` elements. | O(n) |
| `Enum.take_while(enumerable, fun)` | Elements while `fun` is truthy. | O(n) |
| `Enum.drop_while(enumerable, fun)` | Elements after `fun` first becomes falsy. | O(n) |
| `Enum.take_every(enumerable, nth)` | Every `nth` element, 1-based. | O(n) |
| `Enum.drop_every(enumerable, nth)` | Drops every `nth` element, 1-based. | O(n) |
| `Enum.slice(enumerable, start, size)` | `size` elements starting at `start`. | O(n) generally; O(1) on slice-capable structures. |
| `Enum.slice(enumerable, index_range)` | Slice by `Range`. | O(n) generally; O(1) on slice-capable structures. |
| `Enum.intersperse(enumerable, separator)` | Inserts separator between elements. | O(n) |
| `Enum.reverse(enumerable)` | Reverses the enumerable. | O(n) |
| `Enum.reverse(enumerable, tail)` | Reverses and prepends to `tail`. | O(n) |
| `Enum.shuffle(enumerable)` | Random permutation. | O(n) |

```elixir
Enum.take(1..100, 5)                   #=> [1, 2, 3, 4, 5]
Enum.drop(1..100, 95)                  #=> [96, 97, 98, 99, 100]
Enum.take_while([1, 2, 3, 4, 5], &(&1 < 4))
#=> [1, 2, 3]
Enum.take_every(1..10, 2)              #=> [1, 3, 5, 7, 9]
Enum.slice(1..10, 2, 4)                #=> [3, 4, 5, 6]
Enum.intersperse([1, 2, 3], 0)         #=> [1, 0, 2, 0, 3]
Enum.reverse([1, 2, 3])                #=> [3, 2, 1]
```

#### Random

| Function | Behaviour | Complexity |
|---|---|---|
| `Enum.random(enumerable)` | Random element. | O(n) for lists; O(1) for Range. |
| `Enum.take_random(enumerable, count)` | `count` random elements without replacement. | O(n) |
| `Enum.shuffle(enumerable)` | Random permutation. | O(n) |

```elixir
Enum.random([1, 2, 3])                 #=> 2 (non-deterministic)
Enum.shuffle([1, 2, 3])                #=> [3, 1, 2] (non-deterministic)
Enum.take_random(1..100, 3)            #=> [42, 7, 91] (non-deterministic)
```

### Lazy vs eager

From [Enum.html](https://hexdocs.pm/elixir/Enum.html):

> "The functions in this module are eager: they enumerate the enumerable immediately."

From [Stream.html](https://hexdocs.pm/elixir/Stream.html):

> "Streams are composable, lazy enumerables. Any enumerable that generates items one by one during enumeration is called a stream. Streams are lazy because intermediate results are not computed until you ask for them."

> "The `Stream` module provides a set of functions for creating and manipulating streams. It is built on top of the `Enumerable` protocol, so it works with any enumerable."

Every `Enum` function is eager and materializes intermediate results. `Stream` functions return a `%Stream{}` struct that composes computations but does not run them until a terminal `Enum` function (or another consumer) forces enumeration.

```elixir
# Eager: two intermediate lists are materialized.
1..100
|> Enum.map(&(&1 * 2))        #=> full list of 100 elements
|> Enum.filter(&(&1 > 100))   #=> another full list
|> Enum.take(5)               #=> [102, 104, 106, 108, 110]

# Lazy: no intermediate lists; only the first 5 matching items are computed.
1..100
|> Stream.map(&(&1 * 2))
|> Stream.filter(&(&1 > 100))
|> Enum.take(5)               #=> [102, 104, 106, 108, 110]
```

### Enum vs Stream decision guide

From [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "If you are new to Elixir, you should learn `Enum` first. If you find yourself needing laziness, move to `Stream`."

| Scenario | Preferred module | Why |
|---|---|---|
| Small or finite data, single pass | `Enum` | Simpler, lower per-step overhead. |
| Multi-stage pipeline with large intermediate lists | `Stream` | Avoids materializing intermediates. |
| Infinite enumerable | `Stream` only | Any greedy `Enum` function will not terminate. |
| Multi-pass over the same generated sequence | `Stream` | Computation is composed once and replayed. |
| Simple single-pass transformation | `Enum` | `Stream` has per-step overhead and is often slower for trivial pipelines. |
| File/IO line-by-line processing | `Stream` | Memory usage stays constant regardless of file size. |
| Need to count, sort, group, or reverse | `Enum` | These operations must materialize the full collection anyway. |

`Stream` is not universally faster. Its per-element overhead means a single-pass `Enum` pipeline on modest data is usually faster than the equivalent `Stream`. `Stream` wins when the data is large, infinite, or the pipeline is multi-pass.

### The Enumerable protocol

From [Enumerable.html](https://hexdocs.pm/elixir/Enumerable.html):

> "The `Enumerable` protocol is responsible for making data structures enumerable. The `Enum` module provides a set of functions for working with any data structure that implements `Enumerable`."

> "The `Enumerable` protocol requires four functions to be implemented: `count/1`, `member?/2`, `reduce/3`, and `slice/1`. The `reduce/3` function is the core of the protocol. The other three functions are optional optimization paths that allow specific data structures to provide faster implementations."

The four callbacks are:

| Callback | Purpose |
|---|---|
| `reduce/3` | Core traversal; every enumerable must implement it. |
| `count/1` | Optional O(1) count if the structure can provide one. |
| `member?/2` | Optional O(1) membership if the structure can provide one. |
| `slice/1` | Optional random-access metadata for O(1) `at`/`slice`/`fetch`. |

`Range` implements all four. As a result, `Enum.count/1`, `Enum.member?/2`, and `Enum.at/3` are O(1) on ranges, even though they are O(n) on plain lists:

```elixir
Enum.count(1..1_000_000)               #=> 1000000  # O(1)
Enum.member?(1..1_000_000, 500_000)    #=> true      # O(1)
Enum.at(1..1_000_000, 500_000)         #=> 500001    # O(1)

# Same operations on a list are O(n):
Enum.count(Enum.to_list(1..1_000_000)) #=> 1000000   # O(n)
```

Source: [Enumerable.html](https://hexdocs.pm/elixir/Enumerable.html), [Range.html](https://hexdocs.pm/elixir/Range.html).

### Enumeration patterns and idioms

#### Pipeline style

`Enum` functions are pipe-friendly. Prefer a linear pipeline when each step is clear:

```elixir
users
|> Enum.filter(& &1.active)
|> Enum.map(& &1.email)
|> Enum.uniq()
```

#### Pattern matching in `reduce`

```elixir
Enum.reduce([{:inc, 1}, {:inc, 2}, {:dec, 5}], 0, fn
  {:inc, n}, acc -> acc + n
  {:dec, n}, acc -> acc - n
end)
#=> -2
```

#### Early termination with `reduce_while`

```elixir
Enum.reduce_while([1, 2, 3, 4, 5], 0, fn x, acc ->
  if x > 3 do
    {:halt, acc}
  else
    {:cont, acc + x}
  end
end)
#=> 6
```

#### Batching with `chunk_every`

```elixir
1..10
|> Enum.chunk_every(3)
#=> [[1, 2, 3], [4, 5, 6], [7, 8, 9], [10]]
```

#### Grouping with `group_by`

```elixir
Enum.group_by(~w[apple banana apricot cherry], &String.length/1)
#=> %{5 => ["apple"], 6 => ["banana", "cherry"], 7 => ["apricot"]}
```

#### `for` comprehensions

Comprehensions are often clearer than chains of `Enum` functions. They support filters, `:into`, `:uniq`, and `:reduce`.

```elixir
for x <- 1..3, do: x * 2
#=> [2, 4, 6]

for x <- 1..5, rem(x, 2) == 0, do: x
#=> [2, 4]

for {k, v} <- %{a: 1, b: 2}, into: %{}, do: {k, v * 2}
#=> %{a: 2, b: 4}

for x <- [1, 1, 2, 2, 3], uniq: true, do: x
#=> [1, 2, 3]

for x <- 1..3, reduce: 0 do
  acc -> acc + x
end
#=> 6
```

Sources: [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html), [Kernel.SpecialForms.html#for/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#for/1).

#### Building maps with `into`

```elixir
Enum.into([{:a, 1}, {:b, 2}], %{})
#=> %{a: 1, b: 2}

# Equivalent comprehension:
for {k, v} <- [{:a, 1}, {:b, 2}], into: %{}, do: {k, v}
#=> %{a: 1, b: 2}
```

### API design / naming patterns

`Enum` is a good specimen of Elixir's naming conventions.

#### Boolean predicates (`?`)

Functions returning booleans end in `?`: `any?/1`, `all?/1`, `empty?/1`, `member?/2`.

```elixir
Enum.empty?([])                        #=> true
Enum.member?([1, 2, 3], 2)             #=> true
```

#### Raising variants (`!`)

Functions that can fail provide a `!` variant: `fetch!/2` raises `Enum.OutOfBoundsError` where `fetch/2` returns `:error`.

```elixir
Enum.fetch([:a], 5)                    #=> :error
Enum.fetch!([:a], 5)                   #=> ** (Enum.OutOfBoundsError)
```

#### The `_by` suffix

A `_by` function transforms each element with a function before performing the core operation:

| Function | Meaning of `_by` |
|---|---|
| `sort_by/2` | Sort by derived key. |
| `min_by/2` | Minimize by derived key. |
| `max_by/2` | Maximize by derived key. |
| `uniq_by/2` | Uniqueness by derived key. |
| `dedup_by/2` | Consecutive-dedup by derived key. |
| `group_by/3` | Group by derived key. |
| `frequencies_by/2` | Count by derived key. |
| `chunk_by/2` | Chunk while derived key is equal. |

```elixir
Enum.uniq_by([%{id: 1, n: "a"}, %{id: 2, n: "b"}, %{id: 1, n: "c"}], & &1.id)
#=> [%{id: 1, n: "a"}, %{id: 2, n: "b"}]
```

#### The `_while` suffix

A `_while` function continues while a predicate or continuation is truthy/active:

| Function | Meaning of `_while` |
|---|---|
| `take_while/2` | Take while predicate is truthy. |
| `drop_while/2` | Drop while predicate is truthy. |
| `split_while/2` | Split while predicate is truthy. |
| `reduce_while/3` | Reduce with explicit `{:cont, acc}` / `{:halt, acc}` control. |
| `chunk_while/4` | General chunking with continuation state. |

#### The `_every` suffix

An `_every` function operates at regular intervals:

| Function | Meaning of `_every` |
|---|---|
| `take_every/2` | Take every `nth` element. |
| `drop_every/2` | Drop every `nth` element. |
| `chunk_every/4` | Chunk into fixed-size groups. |
| `map_every/3` | Map every `nth` element, leaving others unchanged. |

#### `take` / `drop` symmetry

`take` and `drop` are paired opposites:

```elixir
Enum.take(1..10, 3)                    #=> [1, 2, 3]
Enum.drop(1..10, 3)                    #=> [4, 5, 6, 7, 8, 9, 10]

Enum.take_while([1, 2, 3, 4], &(&1 < 3))
#=> [1, 2]
Enum.drop_while([1, 2, 3, 4], &(&1 < 3))
#=> [3, 4]
```

#### `into` for Collectable

`Enum.into/2` converts from any `Enumerable` to any `Collectable`. It is the standard way to turn a list of tuples into a map, a map into a keyword list, etc.

```elixir
Enum.into([a: 1, b: 2], %{})           #=> %{a: 1, b: 2}
```

#### `count` vs `size` vs `length`

From [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html):

> "When you see `size` in a function name, it means the operation runs in constant time (also written as 'O(1) time') because the size is stored alongside the data structure. Examples: `map_size/1`, `tuple_size/1`"
> "When you see `length`, the operation runs in linear time ('O(n) time') because the entire data structure has to be traversed. Examples: `length/1`, `String.length/1`"

`Enum.count/1` is the enumerable-aware counterpart: it is O(n) on a plain list but O(1) when the underlying `Enumerable.count/1` callback is optimized (e.g. on `Range`). Prefer `Enum.count/1` over `length/1` when the input is any enumerable rather than specifically a list.

### Complexity reference table

| Function | List | Range | Map | MapSet |
|---|---|---|---|---|
| `map/2` | O(n) | O(n) | O(n) | O(n) |
| `filter/2` | O(n) | O(n) | O(n) | O(n) |
| `reduce/3` | O(n) | O(n) | O(n) | O(n) |
| `at/3` | O(n) | O(1) | O(n) | O(n) |
| `fetch/2` | O(n) | O(1) | O(n) | O(n) |
| `count/1` | O(n) | O(1) | O(n) | O(n) |
| `member?/2` | O(n) | O(1) | O(n) | O(1) |
| `empty?/1` | O(n) | O(1) | O(n) | O(1) |
| `slice/3` | O(n) | O(1) | O(n) | O(n) |
| `sort/1` | O(n log n) | O(n log n) | O(n log n) | O(n log n) |
| `uniq/1` | O(n) | O(n) | O(n) | O(n) |
| `group_by/3` | O(n) | O(n) | O(n) | O(n) |
| `chunk_every/2` | O(n) | O(n) | O(n) | O(n) |
| `reverse/1` | O(n) | O(n) | O(n) | O(n) |

The O(1) entries for `Range` come from `Enumerable.slice/1` and related callbacks. For `MapSet`, membership and emptiness are O(1) because the structure tracks size and uses a hash map internally. Note that `Enum` always traverses a `Map` as two-element tuples, so map-oriented operations should usually be done with the `Map` module.

### Version notes

Functions added in specific Elixir versions:

| Function | Added in | Notes |
|---|---|---|
| `Enum.chunk_every/2` (and `/4`) | v1.5 | Replaced `Enum.chunk/2`. |
| `Enum.count_until/2` / `count_until/3` | v1.12 | Count with an early-stopping limit. |
| `Enum.frequencies/1` / `frequencies_by/2` | v1.10 | Count occurrences. |
| `Enum.map_every/3` | v1.4 | Map every `nth` element. |
| `Enum.map_intersperse/3` | v1.10 | Map and intersperse in one pass. |
| `Enum.zip_with/2` / `zip_with/3` | v1.12 | Zip with a mapper function. |
| `Enum.zip/1` (list of enumerables) | v1.4 | Zip multiple enumerables. |

Deprecated:

- `Enum.chunk/2` -> use `Enum.chunk_every/2`.
- Passing a non-empty list as the collectable to `Enum.into/2` is deprecated; the collectable must start empty or be a structure that supports accumulation (e.g. `Map`, `MapSet`, bitstring, empty list).

Removed:

- `Enum.filter_map/3` was removed; use `Enum.filter/2` followed by `Enum.map/2`, or `Enum.flat_map/2`.

Does not exist:

- `Enum.partition/2` does not exist. Use `Enum.split_with/2` to separate elements into `{matches, non_matches}`.

## Stream

### Overview & evaluation model

From [Stream.html](https://hexdocs.pm/elixir/Stream.html):

> "Streams are composable, lazy enumerables. ... Any enumerable that generates elements one by one during enumeration is called a stream. For example, Elixir's `Range` is a stream."

> "We say the functions in `Stream` are *lazy* and the functions in `Enum` are *eager*. Due to their laziness, streams are useful when working with large (or even infinite) collections. When chaining many operations with `Enum`, intermediate lists are created, while `Stream` creates a recipe of computations that are executed at a later moment. Then when the stream is consumed later on, most commonly by using a function in the `Enum` module, the stream will emit its elements one by one."

A `Stream` operation returns an enumerable rather than a result. No work happens until a consumer — typically an `Enum` function such as `Enum.to_list/1` or `Enum.take/2`, or a `for` comprehension — drives the enumeration. This makes three things possible that `Enum` cannot do: representing **infinite** collections, building **multi-stage pipelines without intermediate lists**, and **terminating early** before the whole input is traversed.

#### Don't pattern-match on the Stream struct

From [Stream.html](https://hexdocs.pm/elixir/Stream.html):

> "While some functions in this module may return the `Stream` struct, you must never explicitly check for the `Stream` struct, as streams may come in several shapes, such as `IO.Stream`, `File.Stream`, or even `Range`s. The functions in this module only guarantee to return enumerables and their implementation (structs, anonymous functions, etc) may change at any time. ... Instead of checking for a particular type, you must instead write assertive code that assumes you have an enumerable."

Constructors such as `Stream.unfold/2`, `Stream.cycle/1`, `Stream.iterate/2`, `Stream.repeatedly/1`, and `Stream.resource/3` typically return anonymous functions, while combinators such as `Stream.map/2` wrap their source in a `Stream` struct. Both are valid streams. Treat any `Stream.*` result as an opaque `Enumerable.t()`.

### Laziness and stream composition

Because streams are lazy, the stages of a pipeline are **fused** into a single pass driven by the final consumer. The module page makes laziness visible with an `IO.inspect/1` side effect:

```elixir
# Eager (Enum): the range is traversed twice.
# Prints: 1, 2, 3, then 1, 2, 3 again.
1..3 |> Enum.map(&IO.inspect(&1)) |> Enum.map(&IO.inspect(&1))

# Lazy (Stream): the range is traversed once.
# Prints: 1, 1, 2, 2, 3, 3 (each element visits both stages before the next).
1..3
|> Stream.map(&IO.inspect(&1))
|> Stream.map(&IO.inspect(&1))
|> Enum.to_list()
```

In the eager version each `Enum.map/2` traverses the whole collection, so the range is enumerated twice and two intermediate lists are built. In the lazy version the range is enumerated **once**: each element flows through both `Stream.map/2` stages in turn, and only because `Enum.to_list/1` finally demands the values.

```elixir
# Eager: two intermediate lists are materialized before take runs.
1..1_000_000
|> Enum.map(&(&1 * 2))           #=> full list of 1_000_000 elements
|> Enum.filter(&(&1 > 100))      #=> another full list
|> Enum.take(5)                  #=> [102, 104, 106, 108, 110]

# Lazy: only the first 5 matching items are ever computed.
1..1_000_000
|> Stream.map(&(&1 * 2))
|> Stream.filter(&(&1 > 100))
|> Enum.take(5)                  #=> [102, 104, 106, 108, 110]
```

#### Reusability

A stream's reusability depends on its source:

| Source | Reusable? | Why |
|---|---|---|
| Plain `Enumerable` (list, `Range`, `Map`, `MapSet`) | Yes | Each `Enum` call drives a fresh `reduce/3` over an immutable source. |
| `Stream.resource/3`, `File.stream!/1`, `IO.stream/2`, `StringIO` | No (single use) | The underlying resource (file handle, socket, device) advances and is closed on enumeration. |

From [IO.html](https://hexdocs.pm/elixir/IO.html):

> "Note that an IO stream has side effects and every time you go over the stream you may get different results."

Treat any stream wrapping an external resource as single-use; recreate it (e.g. call `File.stream!/1` again) to read it a second time.

### Key functions

For streams, cost is best expressed relative to the number of elements actually **consumed** (`k`), not the size of the underlying source — a lazy pipeline only ever pulls as many elements as its terminal consumer asks for.

#### Creating infinite streams

| Function | Behaviour | Notes |
|---|---|---|
| `Stream.iterate(start, next_fun)` | Emits `start`, `next_fun.(start)`, ... forever. | No built-in halt; bound with `Stream.take/2`. |
| `Stream.cycle(enumerable)` | Repeats `enumerable` forever. | Bound with `Stream.take/2` before consuming. |
| `Stream.repeatedly(generator_fun)` | Calls `generator_fun` for each element, forever. | e.g. `Stream.repeatedly(&:rand.uniform/0)`. |
| `Stream.unfold(acc, next_fun)` | Emits elements until `next_fun` returns `nil`. | Can be infinite or finite; see below. |

```elixir
Stream.iterate(1, &(&1 * 2)) |> Enum.take(5)        #=> [1, 2, 4, 8, 16]
Stream.cycle([1, 2, 3]) |> Enum.take(5)             #=> [1, 2, 3, 1, 2]
Stream.repeatedly(&:rand.uniform/0) |> Enum.take(3) #=> [0.40, 0.73, 0.12] (non-deterministic)
```

#### Transforming

| Function | Behaviour |
|---|---|
| `Stream.map(enum, fun)` | Maps `fun` over each element lazily. |
| `Stream.flat_map(enum, fun)` | Maps and flattens one level lazily. |
| `Stream.scan(enum, acc, fun)` | Reducing scan; emits each intermediate accumulator. |
| `Stream.scan(enum, fun)` | As above, seeded with the first element. |
| `Stream.intersperse(enum, sep)` | Inserts `sep` between each element lazily. |
| `Stream.transform(enum, acc, reducer)` | Stateful pipeline; reducer emits any number of elements and may `{:halt, acc}`. |
| `Stream.each(enum, fun)` | Runs `fun` for side effects; returns the (still-lazy) stream. |

```elixir
Stream.map([1, 2, 3], &(&1 * 2)) |> Enum.to_list()         #=> [2, 4, 6]
Stream.flat_map([1, 2, 3], fn x -> [x, x * 2] end) |> Enum.to_list()
#=> [1, 2, 2, 4, 3, 6]
Stream.scan(1..5, 0, &(&1 + &2)) |> Enum.to_list()         #=> [1, 3, 6, 10, 15]
Stream.intersperse([1, 2, 3], 0) |> Enum.to_list()         #=> [1, 0, 2, 0, 3]
```

`Stream.transform/3` is the most general combinator. The reducer returns `{list_of_emitted, new_acc}` or `{:halt, acc}`; it can implement `take`, stateful filtering, windowing, and more. This is how `Stream.take/2` is expressed:

```elixir
stream = Stream.transform(1..100, 0, fn i, acc ->
  if acc < 3, do: {[i], acc + 1}, else: {:halt, acc}
end)
Enum.to_list(stream)   #=> [1, 2, 3]
```

#### Filtering and selecting

| Function | Behaviour |
|---|---|
| `Stream.filter(enum, fun)` | Keeps elements where `fun` is truthy. |
| `Stream.reject(enum, fun)` | Inverse of `filter/2`. |
| `Stream.uniq(enum)` | Drops duplicates, keeping the first occurrence. |
| `Stream.uniq_by(enum, fun)` | Uniqueness by a derived key. |
| `Stream.dedup(enum)` / `Stream.dedup_by(enum, fun)` | Drops consecutive duplicates only. |

```elixir
Stream.filter([1, 2, 3, 4], &(rem(&1, 2) == 0)) |> Enum.to_list()  #=> [2, 4]
Stream.uniq([1, 1, 2, 2, 3]) |> Enum.to_list()                      #=> [1, 2, 3]
```

> Caveat from [Stream.html](https://hexdocs.pm/elixir/Stream.html): `uniq/1` and `uniq_by/2` "need to store all unique values emitted by the stream. Therefore, if the stream is infinite, the number of elements stored will grow infinitely, never being garbage-collected." `dedup/1` only stores the last value and is safe on infinite streams.

#### Slicing, taking, and dropping

| Function | Behaviour |
|---|---|
| `Stream.take(enum, count)` | Lazily takes the next `count` elements then halts. A negative `count` takes from the end. |
| `Stream.take_every(enum, nth)` | Every `nth` element, 1-based. |
| `Stream.take_while(enum, fun)` | Takes while `fun` is truthy. |
| `Stream.drop(enum, count)` | Lazily drops the first `count` elements. A negative `count` drops the last `count`. |
| `Stream.drop_every(enum, nth)` | Drops every `nth` element. |
| `Stream.drop_while(enum, fun)` | Drops while `fun` is truthy. |
| `Stream.with_index(enum, offset_or_fun)` | Tags elements with their index, or maps with `{element, index}`. |

```elixir
Stream.drop(1..10, 5) |> Enum.to_list()       #=> [6, 7, 8, 9, 10]
Stream.take_every(1..10, 2) |> Enum.to_list() #=> [1, 3, 5, 7, 9]
Stream.with_index([:a, :b, :c]) |> Enum.to_list()
#=> [{:a, 0}, {:b, 1}, {:c, 2}]
```

#### Chunking

| Function | Behaviour |
|---|---|
| `Stream.chunk_every(enum, count)` | Shortcut for `chunk_every(enum, count, count)`. |
| `Stream.chunk_every(enum, count, step, leftover)` | Fixed-size chunks; `:discard` drops the short tail, or supply a `leftover` enumerable. |
| `Stream.chunk_by(enum, fun)` | Emits a chunk whenever `fun`'s return value changes. |
| `Stream.chunk_while(enum, acc, chunk_fun, after_fun)` | Low-level chunked reduction with continuation state. |

```elixir
Stream.chunk_every([1, 2, 3, 4, 5, 6], 2) |> Enum.to_list()
#=> [[1, 2], [3, 4], [5, 6]]

Stream.chunk_every([1, 2, 3, 4, 5, 6], 3, 2, :discard) |> Enum.to_list()
#=> [[1, 2, 3], [3, 4, 5]]

Stream.chunk_by([1, 2, 2, 3, 4, 4, 6, 7, 7], &(rem(&1, 2) == 1)) |> Enum.to_list()
#=> [[1], [2, 2], [3], [4, 4, 6], [7, 7]]
```

#### Combining

| Function | Behaviour |
|---|---|
| `Stream.concat(enumerables)` | Concatenates an enumerable of enumerables lazily. |
| `Stream.concat(first, second)` | Concatenates two enumerables lazily. |
| `Stream.zip(enumerables)` | Zips a collection of enumerables; stops when any ends. |
| `Stream.zip(enum1, enum2)` | Zips two enumerables; stops when either ends. |
| `Stream.zip_with(enumerables, fun)` | Zips and maps in one pass. |
| `Stream.into(enum, collectable)` | Writes stream elements into a `Collectable` as a side effect. |
| `Stream.run(stream)` | Consumes the stream for side effects only; returns `:ok`. |

```elixir
Stream.concat([1..3, 4..6, 7..9]) |> Enum.to_list()
#=> [1, 2, 3, 4, 5, 6, 7, 8, 9]

Stream.zip(Stream.concat(1..3, 4..6), Stream.cycle([:a, :b, :c])) |> Enum.to_list()
#=> [{1, :a}, {2, :b}, {3, :c}, {4, :a}, {5, :b}, {6, :c}]
```

`Stream.into/2` combined with `Stream.run/1` is the idiomatic file-to-file pipeline:

```elixir
File.stream!("/path/to/file")
|> Stream.map(&String.replace(&1, "#", "%"))
|> Stream.into(File.stream!("/path/to/other_file"))
|> Stream.run()   #=> :ok
```

### Infinite streams

`Stream.iterate/2`, `Stream.cycle/1`, `Stream.repeatedly/1`, and `Stream.unfold/2` (when its function never returns `nil`) produce **infinite** streams. They must always be bounded — with `Stream.take/2`, `Stream.take_while/2`, `Stream.zip/2`, or any consumer that halts — before being passed to an `Enum` function. An unbounded infinite stream fed to `Enum.to_list/1`, `Enum.count/1`, or `Enum.map/2` will never return.

```elixir
# First ten natural numbers, generated lazily:
Stream.iterate(0, &(&1 + 1)) |> Enum.take(10)
#=> [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

# Fibonacci, bounded with take:
Stream.unfold({0, 1}, fn {a, b} -> {a, {b, a + b}} end) |> Enum.take(8)
#=> [0, 1, 1, 2, 3, 5, 8, 13]
```

### `unfold/2` vs `iterate/2`

| | `iterate/2` | `unfold/2` |
|---|---|---|
| Seed | a value `start` | an accumulator `acc` |
| Step | `next_fun.(prev)` -> next value | `next_fun.(acc)` -> `{element, next_acc}` or `nil` |
| Termination | cannot terminate — always infinite | terminates when `next_fun` returns `nil` |
| Expressive power | emits only the value | can carry separate state, can stop |

`unfold/2` is strictly more expressive: it can build `iterate/2`-like streams (`Stream.unfold(n, fn x -> {x, x + 1} end)`) and also finite ones with a stop condition. Reach for `iterate/2` when you have a pure recurrence with no end, and for `unfold/2` when you either need a separate accumulator or a termination condition.

```elixir
# unfold with a termination condition: count down to 1.
Stream.unfold(5, fn
  0 -> nil
  n -> {n, n - 1}
end) |> Enum.to_list()
#=> [5, 4, 3, 2, 1]
```

### `Stream.resource/3` — wrapping external resources

From [Stream.html](https://hexdocs.pm/elixir/Stream.html):

> "Emits a sequence of values for the given resource. Similar to `transform/3` but the initial accumulated value is computed lazily via `start_fun` and executes an `after_fun` at the end of enumeration (both in cases of success and failure)."

```elixir
Stream.resource(
  fn -> File.open!("sample") end,                       # start_fun: open lazily
  fn file ->                                            # next_fun
    case IO.read(file, :line) do
      data when is_binary(data) -> {[data], file}
      _ -> {:halt, file}
    end
  end,
  fn file -> File.close(file) end                       # after_fun: always close
)
```

Semantics:

- `start_fun/0` runs **once**, only when enumeration actually begins — not when the stream is constructed. A `Stream.resource/3` that is never consumed performs no `start_fun` and no `after_fun`.
- `next_fun/1` returns `{[elements], new_acc}` to emit and continue, or `{:halt, acc}` to stop.
- `after_fun/1` runs **once at the end of enumeration, in both success and failure** — covering normal completion, `next_fun` returning `{:halt, acc}`, early halting by the consumer (e.g. `Enum.take/2` got enough), and consumer exceptions. This guarantee is what makes `Stream.resource/3` safe for file handles, sockets, and database cursors. (`File.stream!/1` is itself built on `Stream.resource/3`.)

### Stream vs Enum

From the [enumerables and streams guide](https://hexdocs.pm/elixir/enumerable-and-streams.html):

> "You may also focus on the `Enum` module first and only move to `Stream` for the particular scenarios where laziness is required, to either deal with slow resources or large, possibly infinite, collections."

Use **`Stream`** when:

- The source is large or **infinite** and you only need a subset.
- A pipeline has **many stages**, so avoiding intermediate lists matters.
- You can **terminate early** (first match, bounded `take`, `take_while`).
- You are streaming an **external resource** (file, socket, DB cursor) and need cleanup — use `Stream.resource/3` or `File.stream!/1`.

Use **`Enum`** when:

- The data is small and already in memory, and you want the materialized result immediately.
- The pipeline is short — one or two steps — where `Stream`'s per-element overhead does not pay off.
- The operation inherently needs the whole collection anyway (`sort`, `count` on a non-optimized source, `group_by`, `frequencies`, `reverse`, `shuffle`, `random`).

See also the **Enum vs Stream decision guide** under the `Enum` section above.

### Performance characteristics

From [Stream.html](https://hexdocs.pm/elixir/Stream.html):

> "Like with `Enum`, the functions in this module work in linear time."

Streams are **not** "free" laziness:

- **Where streams win:** they avoid materializing intermediate lists, support early termination, and let a `take(n)` over an infinite source run in O(n). A three-stage `Enum` pipeline over a million elements builds two throwaway lists of a million elements; the `Stream` equivalent pulls only what the terminal consumer asks for.
- **Where streams cost:** every pipeline stage is a function call (closure invocation) per element. For small, fully in-memory inputs and a single pass, the eager `Enum` version is usually **faster** than the equivalent `Stream` because it skips that per-step dispatch. This is why the official guidance recommends `Enum` by default and reserving `Stream` for the laziness-driven cases above.

Do not assume "lazy means faster." Profile when a pipeline is performance-sensitive.

### `File.stream!` and IO

Several standard-library functions return streams backed by IO:

| Source | Returns | Notes |
|---|---|---|
| `File.stream!(path, line_or_bytes, modes)` | `File.Stream` | Built on `Stream.resource/3`; reads line-by-line (or `line_or_bytes` bytes) without loading the whole file. See [File.Stream.html](https://hexdocs.pm/elixir/File.Stream.html). |
| `IO.stream(device, line_or_bytes)` | `IO.Stream` | Implements both `Enumerable` and `Collectable`; side-effecting. See [IO.Stream.html](https://hexdocs.pm/elixir/IO.Stream.html). |
| `IO.binstream(device, line_or_bytes)` | `IO.Stream` | Byte-level (Unicode-unsafe) variant. |
| `URI.query_decoder/1` | stream | Lazily decodes a URL query string. |

Line-by-line file processing is the canonical `Stream` use case — memory stays constant regardless of file size:

```elixir
# First 10 non-empty lines of a large file, without loading it into memory.
"/path/to/large.log"
|> File.stream!()
|> Stream.filter(&(&1 != ""))
|> Enum.take(10)
```

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `Stream.chunk_every/2`, `chunk_every/4`, `chunk_while/4` | v1.5 | Replaced the old `Stream.chunk/4`. |
| `Stream.zip/1`, `zip_with/2`, `zip_with/3` | v1.4 / v1.12 | Multi-enumerable zip. |
| `Stream.intersperse/2` | v1.6 | |
| `Stream.transform/5` | v1.14 | Adds `last_fun`/`after_fun`. |
| `Stream.duplicate/2` | v1.14 | |
| `Stream.from_index/1` | v1.17 | Build a stream from an index offset or function. |
| `Stream.iodata_empty?/1` | v1.20 | |

Sources: [Stream.html](https://hexdocs.pm/elixir/Stream.html), [enumerable-and-streams.html](https://hexdocs.pm/elixir/enumerable-and-streams.html), [File.Stream.html](https://hexdocs.pm/elixir/File.Stream.html), [IO.Stream.html](https://hexdocs.pm/elixir/IO.Stream.html), [IO.html](https://hexdocs.pm/elixir/IO.html).

## Map

### Overview & evaluation model

From [Map.html](https://hexdocs.pm/elixir/Map.html):

> "Maps are the 'go to' key-value data structure in Elixir."

> "Maps do not impose any restriction on the key type: anything can be a key in a map. As a key-value structure, maps do not allow duplicate keys. Keys are compared using the exact-equality operator (`===/2`). If colliding keys are defined in a map literal, the last one prevails."

> "Key-value pairs in a map do not follow any order."

A map is an immutable, persistent, key-value collection. Any term (atom, string, number, tuple, ...) can be a key; duplicate keys are impossible because keys are matched by `===/2`. Maps are created with the `%{}` literal using two pair syntaxes: `key => value` (general) and the `key: value` shorthand (atoms only). When both appear in one literal, the `key: value` shorthand entries must come last. The printed key order is the map's internal order, not insertion order — never rely on it.

```elixir
%{}                                     #=> %{}
%{"one" => :two, 3 => "four"}           #=> %{3 => "four", "one" => :two}
%{a: 1, b: 2}                           #=> %{a: 1, b: 2}   (atom shorthand)
%{"hello" => "world", a: 1, b: 2}       # shorthand MUST come after `=>`
```

Maps are pattern-matchable. A map on the left of `=` matches any map that contains those keys with equal values — an empty map matches every map:

```elixir
%{} = %{foo: "bar"}                     #=> %{foo: "bar"}
%{a: a} = %{:a => 1, "b" => 2, [:c, :e, :e] => 3}
a                                       #=> 1
```

Maps implement `Enumerable`, so most bulk operations live in `Enum` (which iterates a map as `{key, value}` tuples — see the `Enum` section above). The only map-specific kernel primitive is `map_size/1`, which runs in constant time and is allowed in guards:

```elixir
map_size(%{a: "foo", b: "bar"})         #=> 2   # O(1)
```

Source: [Kernel.html#map_size/1](https://hexdocs.pm/elixir/Kernel.html#map_size/1).

### Immutability and the update syntax

Every `Map` operation returns a **new** map; the original is never mutated. For updating *existing* keys there is a dedicated literal syntax `%{map | key => value}`. It raises `KeyError` if the key is absent, which makes the "update, not insert" intent explicit at compile time:

```elixir
map = %{one: 1, two: 2}
%{map | one: "one"}                     #=> %{one: "one", two: 2}

other = %{"three" => 3, "four" => 4, "five" => 5}
%{other | "three" => "three", "four" => "four"}
#=> %{"five" => 5, "four" => "four", "three" => "three"}

%{map | three: 3}                       #=> ** (KeyError) key :three not found
```

Because the update syntax guarantees no new keys are introduced, the runtime can share structure between the old and new maps. From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "When using the update syntax (`|`), Elixir is aware that no new keys will be added to the struct, allowing the maps underneath to share their structure in memory."

To *insert* new keys use `Map.put/3`; the `%{map | ...}` syntax is strictly an update of existing keys.

### Reading keys: `map.key` vs `map[key]`

From [Map.html](https://hexdocs.pm/elixir/Map.html):

> "The two syntaxes for accessing keys reveal the dual nature of maps. The `map[key]` syntax is used for dynamically created maps that may have any key, of any type. `map.key` is used with maps that hold a predetermined set of atom keys, which are expected to always be present. Structs ... are one example of such 'static maps', where the keys can also be checked during compile time."

| Syntax | Works for | Missing key | When the map is `nil` | Mechanism |
|---|---|---|---|---|
| `map.key` | atom keys only | raises `KeyError` | raises | direct field access (compiler-checked for atom keys / structs) |
| `map[key]` | any key type | returns `nil` | returns `nil` | the `Access` behaviour (`Access.fetch/2`) |

```elixir
map = %{foo: "bar", baz: "bong"}
map.foo                                 #=> "bar"
map.non_existing                        #=> ** (KeyError) key :non_existing not found
map[:baz]                               #=> "bong"
map["non_existing_key"]                 #=> nil
```

> Do not write `data.key()` (with parentheses) for field access — Elixir will read `data` as a module name and attempt to call function `key/0` in it. Source: [Map.html](https://hexdocs.pm/elixir/Map.html).

The bracket form goes through the `Access` behaviour and is **nil-safe** when nested, because accessing anything on `nil` returns `nil`. From [Access.html](https://hexdocs.pm/elixir/Access.html):

> "This works because accessing anything on a `nil` value, returns `nil` itself: `nil[:a]` returns `nil`."

Bracket access is a *runtime* call to the `Access` callback, so it **cannot appear in a pattern match** — only literal `%{key: value}` patterns match on keys. For explicit reads prefer `Map.get/3` or `Map.fetch/2`.

### Key functions

Complexity is given for a map of `n` entries. Per [Map.html](https://hexdocs.pm/elixir/Map.html):

> "The functions in this module that need to find a specific key work in logarithmic time... Some functions, such as `keys/1` and `values/1`, run in linear time because they need to get to every element in the map."

#### Creating

| Function | Behaviour | Complexity |
|---|---|---|
| `%{...}` literal | Create a map directly. | O(k) |
| `Map.new/0` | Returns `%{}`. | O(1) |
| `Map.new/1` | Build a map from an enumerable of `{key, value}` pairs; duplicate keys removed, **last wins**. | O(n) |
| `Map.new/2` | As `new/1` but applies a transform to each element first. | O(n) |
| `Map.from_keys/2` *(v1.14)* | Build a map from a list of keys, all mapped to the same fixed `value`. | O(k) |
| `Map.from_struct/1` | Strip the `__struct__` field from a struct, returning a plain map. | O(k) |

```elixir
Map.new()                               #=> %{}
Map.new([{:b, 1}, {:a, 2}])             #=> %{a: 2, b: 1}
Map.new(a: 1, a: 2, a: 3)               #=> %{a: 3}   # last wins
Map.new([:a, :b], fn x -> {x, x} end)   #=> %{a: :a, b: :b}
Map.from_keys([1, 2, 3], :number)       #=> %{1 => :number, 2 => :number, 3 => :number}
Map.from_struct(%User{name: "john"})    #=> %{name: "john"}
```

#### Reading

| Function | Behaviour | Complexity |
|---|---|---|
| `Map.get/3` | Value for `key`, or `default` (defaults to `nil`). | O(log n) |
| `Map.get_lazy/3` | As `get/3` but the default is a 0-arity fun, evaluated only when the key is missing. | O(log n) |
| `Map.fetch/2` | `{:ok, value}` if present, else `:error`. | O(log n) |
| `Map.fetch!/2` | Value, or raises `KeyError`. | O(log n) |
| `Map.has_key?/2` | Boolean membership. | O(log n) |

```elixir
Map.get(%{"a" => 1}, "a")               #=> 1
Map.get(%{"a" => 1}, "b")               #=> nil
Map.get(%{"a" => 1}, "b", 3)            #=> 3
Map.get(%{"a" => nil}, "a", 1)          #=> nil     # value is literally nil
Map.fetch(%{a: 1}, :a)                  #=> {:ok, 1}
Map.fetch(%{a: 1}, :b)                  #=> :error
Map.fetch!(%{a: 1}, :b)                 #=> ** (KeyError) key :b not found
Map.has_key?(%{a: 1}, :a)               #=> true
```

Prefer `fetch/2` (or `fetch!/2`) when you must distinguish "absent" from "the stored value is `nil`" — `get/3` cannot tell them apart.

#### Writing and updating

| Function | Behaviour | Complexity |
|---|---|---|
| `Map.put/3` | Insert or overwrite `key` with `value`. | O(log n) |
| `Map.put_new/3` | Set `key` only if it is **absent** (no-op if present). | O(log n) |
| `Map.put_new_lazy/3` | As `put_new/3` but the value is computed lazily, only when the key is absent. | O(log n) |
| `Map.update/4` | If present, replace via `fun`; if absent, insert `default` (the default is *not* passed through `fun`). | O(log n) |
| `Map.update!/3` | Replace an **existing** key via `fun`; raises `KeyError` if absent. | O(log n) |
| `Map.replace/3` | Overwrite only if the key already exists; otherwise return the map unchanged. | O(log n) |
| `Map.replace!/3` | As `replace/3` but raises `KeyError` if the key is absent. | O(log n) |
| `Map.replace_lazy/3` *(v1.14)* | Replace an existing key via `fun`, computed only when present. | O(log n) |
| `Map.get_and_update/3` | Read the current value and set a new one in one pass; returns `{current, new_map}`. `fun` may return `:pop` to delete. | O(log n) |
| `Map.get_and_update!/3` | As above but raises `KeyError` if the key is absent. | O(log n) |

```elixir
Map.put(%{a: 1}, :b, 2)                 #=> %{a: 1, b: 2}
Map.put(%{a: 1, b: 2}, :a, 3)           #=> %{a: 3, b: 2}
Map.put_new(%{a: 1, b: 2}, :a, 3)       #=> %{a: 1, b: 2}   # present -> untouched
Map.update(%{a: 1}, :a, 13, &(&1 * 2))  #=> %{a: 2}          # existing -> fun
Map.update(%{a: 1}, :b, 11, &(&1 * 2))  #=> %{a: 1, b: 11}   # absent  -> default
Map.update!(%{"a" => 1}, "b", &(&1 * 2))#=> ** (KeyError) key "b" not found
Map.replace(%{a: 1, b: 2}, :a, 3)       #=> %{a: 3, b: 2}
Map.replace(%{"a" => 1}, "b", 2)        #=> %{"a" => 1}      # absent -> unchanged
Map.get_and_update(%{a: 1}, :a, fn v -> {v, "new"} end)
#=> {1, %{a: "new"}}
Map.get_and_update(%{a: 1}, :a, fn _ -> :pop end)
#=> {1, %{}}
```

The failure modes differ and should drive the choice: `put/3` always inserts; `replace/3` only overwrites existing keys; `update/4` overwrites-or-uses-a-default; `update!/3` overwrites-or-raises. `%{map | key: v}` is the literal equivalent of `update!/3` (existing keys only).

#### Merging

| Function | Behaviour | Complexity |
|---|---|---|
| `Map.merge/2` | Merge `map2` into `map1`; on key collision `map2`'s value wins. | O(n) |
| `Map.merge/3` | As `merge/2` but call `fun(key, v1, v2)` to resolve collisions. | O(n) |
| `Map.intersect/2` *(v1.15)* | Keep only keys present in both maps; values come from `map2`. | O(n) |
| `Map.intersect/3` *(v1.15)* | As `intersect/2` but resolve collisions via `fun(key, v1, v2)`. | O(n) |

```elixir
Map.merge(%{a: 1, b: 2}, %{a: 3, d: 4}) #=> %{a: 3, b: 2, d: 4}
Map.merge(%{a: 1, b: 2}, %{a: 3, d: 4}, fn _k, v1, v2 -> v1 + v2 end)
#=> %{a: 4, b: 2, d: 4}
Map.intersect(%{a: 1, b: 2}, %{b: "b", c: "c"})      #=> %{b: "b"}
Map.intersect(%{a: 1, b: 2}, %{b: 2, c: 3}, fn _k, v1, v2 -> v1 + v2 end)
#=> %{b: 4}
```

> Do **not** use `Map.merge/2` to merge keys into a struct — it will inject non-struct keys. Use `Kernel.struct/2` / `struct!/2` instead. Source: [Map.html](https://hexdocs.pm/elixir/Map.html).

#### Deleting and extracting

| Function | Behaviour | Complexity |
|---|---|---|
| `Map.delete/2` | Remove `key`; return the map unchanged if absent. | O(log n) |
| `Map.drop/2` | Remove every key in a **list** of keys (absent keys ignored). | O(k log n) |
| `Map.take/2` | Keep only the keys in a **list** of keys (absent keys ignored). | O(k log n) |
| `Map.split/2` | Return `{taken_map, rest_map}` for the given keys. | O(k log n) |
| `Map.split_with/2` *(v1.15)* | Partition into `{matching_map, non_matching_map}` by `fun({k, v})`. | O(n) |
| `Map.pop/3` | Remove `key` and return `{value, new_map}`; `{default, map}` if absent. | O(log n) |
| `Map.pop!/2` *(v1.10)* | As `pop/3` but raises `KeyError` if absent. | O(log n) |
| `Map.pop_lazy/3` | As `pop/3` but the default is a lazily-evaluated fun. | O(log n) |

```elixir
Map.delete(%{a: 1, b: 2}, :a)           #=> %{b: 2}
Map.delete(%{b: 2}, :a)                 #=> %{b: 2}
Map.drop(%{a: 1, b: 2, c: 3}, [:b, :d]) #=> %{a: 1, c: 3}
Map.take(%{a: 1, b: 2, c: 3}, [:a, :c, :e]) #=> %{a: 1, c: 3}
Map.split(%{a: 1, b: 2, c: 3}, [:a, :c, :e]) #=> {%{a: 1, c: 3}, %{b: 2}}
Map.split_with(%{a: 1, b: 2, c: 3, d: 4}, fn {_k, v} -> rem(v, 2) == 0 end)
#=> {%{b: 2, d: 4}, %{a: 1, c: 3}}
Map.pop(%{a: 1}, :a)                    #=> {1, %{}}
Map.pop(%{"a" => 1}, "b")               #=> {nil, %{"a" => 1}}
Map.pop(%{"a" => 1}, "b", 3)            #=> {3, %{"a" => 1}}
```

> `drop/2`, `split/2`, and `take/2` require the second argument to be a **list**. Passing a plain enumerable (e.g. a `MapSet`) is deprecated — call `Enum.to_list/1` on it first. Source: [compatibility-and-deprecations.html](https://hexdocs.pm/elixir/compatibility-and-deprecations.html).

#### Keys, values, filtering, comparison

| Function | Behaviour | Complexity |
|---|---|---|
| `Map.keys/1` | All keys (unspecified order). | O(n) |
| `Map.values/1` | All values (unspecified order). | O(n) |
| `Map.to_list/1` | List of `{key, value}` tuples. | O(n) |
| `Map.filter/2` *(v1.13)* | Keep pairs where `fun({k, v})` is truthy; returns a **map**. | O(n) |
| `Map.reject/2` *(v1.13)* | Drop pairs where `fun({k, v})` is truthy; returns a **map**. | O(n) |
| `Map.equal?/2` | Strict equality of keys **and** values (via `===/2`). | O(n) |

```elixir
Map.keys(%{a: 1, b: 2})                 #=> [:a, :b]
Map.values(%{a: 1, b: 2})               #=> [1, 2]
Map.to_list(%{1 => 2})                  #=> [{1, 2}]
Map.filter(%{one: 1, two: 2, three: 3}, fn {_k, v} -> rem(v, 2) == 1 end)
#=> %{one: 1, three: 3}
Map.equal?(%{a: 1.0}, %{a: 1})          #=> false   # === : 1.0 !== 1
```

`Map.equal?/2` exists mainly for API parity with `Keyword`; in practice maps are compared directly with `==/2` / `===/2`, which already compare keys and values. Note the `===/2` subtlety: `1` and `1.0` are considered different.

From [Map.html](https://hexdocs.pm/elixir/Map.html), on `Map.filter/2` / `reject/2`:

> "If you find yourself doing multiple calls to `Map.filter/2` and/or `Map.reject/2` in a pipeline, it is likely more efficient to use `Enum.filter/2` and `Enum.reject/2` instead and convert to a map at the end using `Map.new/1` or `Map.new/2`."

### Nested updates with Kernel macros

For nested maps/structs/keyword lists, `Kernel` provides read-and-update macros in two shapes:

| Shape | Functions | How the path is given |
|---|---|---|
| **Path form** | `put_in/2`, `update_in/2`, `get_in/1`, `pop_in/1`, `get_and_update_in/2` | A path expression like `users[:john].age`, expanded at compile time using the `Access` behaviour. |
| **Data form** | `put_in/3`, `update_in/3`, `get_in/2`, `pop_in/2`, `get_and_update_in/3` | A value plus an explicit list of keys / accessors, traversed at runtime. |

```elixir
users = %{"john" => %{age: 27}, "meg" => %{age: 23}}

# Path form (compile-time expanded, Access-based):
put_in(users["john"].age, 28)
#=> %{"john" => %{age: 28}, "meg" => %{age: 23}}

update_in(users["meg"].age, &(&1 + 1))
#=> %{"john" => %{age: 27}, "meg" => %{age: 24}}

# Data form (runtime list of keys/accessors, nil-safe traversal):
get_in(users, ["john", :age])           #=> 27
get_in(users, ["unknown", :age])        #=> nil

# Mixing in Accessors:
langs = [%{name: "elixir"}, %{name: "c"}]
user = %{name: "john", languages: langs}
get_in(user, [:languages, Access.all(), :name])
#=> ["elixir", "c"]
```

From [Access.html](https://hexdocs.pm/elixir/Access.html):

> "The access syntax can also be used with the `Kernel.put_in/2`, `Kernel.update_in/2`, `Kernel.get_and_update_in/2`, and `Kernel.pop_in/1` macros to further manipulate values in nested data structures."

`get_in` is **nil-safe**: a missing intermediate key yields `nil` instead of raising. Outside `get_in`, `nil.field` raises — so if any path segment may be absent, traverse it via `get_in`.

### Structs: typed maps

From [structs.html](https://hexdocs.pm/elixir/structs.html):

> "Structs are extensions built on top of maps that provide compile-time checks and default values."

> "Structs are simply maps with a 'special' field named `__struct__` that holds the name of the struct... `is_map(john)` returns `true`."

A struct is declared with `defstruct/1` inside a module and created with the `%ModuleName{}` syntax. Only declared fields may exist — unknown fields raise at **compile time**:

```elixir
defmodule User do
  defstruct name: "John", age: 27
end

%User{}                                 #=> %User{age: 27, name: "John"}
%User{name: "Jane"}                     #=> %User{age: 27, name: "Jane"}
%User{oops: :field}                     #=> ** (KeyError) key :oops not found expanding struct: User.__struct__/1

john = %User{name: "John"}
jane = %{john | name: "Jane"}           # update syntax works and shares structure
%{jane | oops: :field}                  #=> ** (KeyError) key :oops not found
```

`@enforce_keys` marks fields that must be supplied at construction (it is not re-checked on update and performs no value validation):

```elixir
defmodule Car do
  @enforce_keys [:make]
  defstruct [:model, :make]
end

%Car{}                                  #=> ** (ArgumentError) the following keys must also be given when building struct Car: [:make]
```

#### Structs do not implement `Access` or `Enumerable`

Structs intentionally opt out of map protocols, so bracket access and `Enum` do **not** work on them — use field access (`struct.key`):

```elixir
john = %User{}
john.name                               #=> "John"
john[:name]                             #=> ** (UndefinedFunctionError) User.fetch/2 is undefined (User does not implement the Access behaviour)
Enum.to_list(john)                      #=> ** (Protocol.UndefinedError) protocol Enumerable not implemented for %User{} (a struct)
```

From [Access.html](https://hexdocs.pm/elixir/Access.html):

> "Since structs are maps and structs have predefined keys, they only allow the `struct.key` syntax and they do not allow the `struct[key]` access syntax."

#### Updating structs from dynamic data

Use `Kernel.struct!/2` to apply a keyword list or map of updates to a struct; it validates that every key is a declared field. Use the `%{struct | field: value}` syntax only when fields are known at compile time.

```elixir
struct!(%User{name: "John", age: 27}, name: "Jane", age: 30)
#=> %User{age: 30, name: "Jane"}
struct!(%User{}, invalid: "field")       #=> ** (KeyError) key :invalid not found
```

> Always use `struct!/2` instead of `Map` functions to preserve struct integrity. Source: [structs.html](https://hexdocs.pm/elixir/structs.html).

### MapSet

From [MapSet.html](https://hexdocs.pm/elixir/MapSet.html):

> "A set is a data structure that can contain unique elements of any kind, without any particular order. `MapSet` is the 'go to' set data structure in Elixir."

> "By definition, sets can't contain duplicate elements: when inserting an element in a set where it's already present, the insertion is simply a no-op."

> "`MapSet` is built on top of Erlang's `:sets` (version 2). This means that they share many properties, including logarithmic time complexity. Erlang `:sets` (version 2) are implemented on top of maps, so see the documentation for `Map` for more information on its execution time complexity."

`MapSet` is for unordered, **unique** collections of any element type. Like maps, iteration order is internal and must not be relied upon. The struct fields are private — pattern-match with `%MapSet{}` if you must, but operate on sets via the module functions.

| Function | Behaviour | Complexity |
|---|---|---|
| `MapSet.new/0` | Empty set. | O(1) |
| `MapSet.new/1` | From enumerable (duplicates collapsed). | O(n) |
| `MapSet.new/2` | From enumerable with a transform. | O(n) |
| `MapSet.put/2` | Insert (no-op if already present). | O(log n) |
| `MapSet.delete/2` | Remove an element (no-op if absent). | O(log n) |
| `MapSet.member?/2` | Membership test. | O(log n) |
| `MapSet.size/1` | Element count. | O(1) |
| `MapSet.to_list/1` | Convert to a list (order not guaranteed). | O(n) |
| `MapSet.union/2` | Set union. | O(n) |
| `MapSet.difference/2` | Elements in set 1 not in set 2. | O(n) |
| `MapSet.intersection/2` | Elements in both sets. | O(n) |
| `MapSet.symmetric_difference/2` *(v1.14)* | Elements in exactly one of the two sets. | O(n) |
| `MapSet.disjoint?/2` | No common elements? | O(n) |
| `MapSet.subset?/2` | Is set 1 a subset of set 2? | O(n) |
| `MapSet.equal?/2` | Same elements (compared with `===/2`)? | O(n) |
| `MapSet.filter/2` *(v1.14)* / `reject/2` | Keep / remove elements by predicate. | O(n) |
| `MapSet.split_with/2` *(v1.15)* | Partition into two sets by predicate. | O(n) |

```elixir
MapSet.new([3, 3, 3, 2, 2, 1])          #=> MapSet.new([1, 2, 3])
MapSet.put(MapSet.new([1, 2, 3]), 3)    #=> MapSet.new([1, 2, 3])   # no-op
MapSet.member?(MapSet.new([1, 2, 3]), 2)#=> true
MapSet.union(MapSet.new([1, 2]), MapSet.new([2, 3, 4]))        #=> MapSet.new([1, 2, 3, 4])
MapSet.intersection(MapSet.new([1, 2]), MapSet.new([2, 3, 4])) #=> MapSet.new([2])
MapSet.disjoint?(MapSet.new([1, 2]), MapSet.new([3, 4]))       #=> true
MapSet.subset?(MapSet.new([1, 2]), MapSet.new([1, 2, 3]))      #=> true
MapSet.equal?(MapSet.new([1]), MapSet.new([1.0]))              #=> false   # ===
```

### Map vs Keyword

From the [keywords and maps guide](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Remember that you should: use keyword lists for passing optional values to functions; use maps for general key-value data structures; use maps when working with data that has a predefined set of keys."

> "Keyword lists ... have three special characteristics: Keys must be atoms; Keys are ordered, as specified by the developer; Keys can be given more than once."

> "Keyword lists are simply lists, and as such they provide the same linear performance characteristics... If you need to store a large amount of keys in a key-value data structure, Elixir offers maps."

| Property | `Map` | `Keyword` |
|---|---|---|
| Key types | any term | atoms only |
| Duplicate keys | impossible | allowed |
| Ordering | internal, not guaranteed | developer-specified (but module functions make no ordering guarantee either) |
| Lookup / insert | O(log n) | O(n) |
| Pattern matchable on keys | yes (`%{key: v}`) | order-dependent, discouraged |
| Size | O(1) via `map_size/1` | O(n) via `length/1` |
| Typical use | records, caches, arbitrary keyed data | function options / DSLs |

Use `Map` for keyed data of any size or key type, or when you need unique keys, fast lookup, or pattern matching. Use `Keyword` (a list of two-tuples) for small, ordered, atom-keyed option lists passed to functions.

### Performance characteristics

| Operation | Complexity |
|---|---|
| `map_size/1` | O(1) |
| `get` / `fetch` / `fetch!` / `has_key?` | O(log n) |
| `put` / `put_new` / `update` / `replace` / `delete` / `pop` / `get_and_update` | O(log n) |
| `merge` / `intersect` / `filter` / `reject` / `split_with` / `equal?` | O(n) |
| `keys` / `values` / `to_list` | O(n) |
| `drop` / `take` / `split` (k keys) | O(k log n) |
| `MapSet` `member?` / `put` / `delete` | O(log n) |
| `MapSet` `size` | O(1) |
| `MapSet` union / intersection / difference / symmetric_difference | O(n) |

Maps are **immutable and persistent**: each update produces a new map that shares unchanged structure with the old one; there is no in-place mutation. `MapSet` is layered on maps (via Erlang `:sets` v2), so it inherits the same logarithmic lookup cost and immutability.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `Map.intersect/2`, `intersect/3`, `split_with/2` | v1.15 | Set-style operations on maps. |
| `Map.from_keys/2`, `replace_lazy/3` | v1.14 | Fixed-value map / lazy replace. |
| `Map.filter/2`, `reject/2` | v1.13 | Map-returning filters. |
| `Map.replace/3` | v1.11 | Overwrite-only-if-present. |
| `Map.pop!/2` | v1.10 | Raising pop. |
| `Map.replace!/3` | v1.5 | Raising overwrite-if-present. |
| `MapSet.filter/2`, `reject/2`, `symmetric_difference/2` | v1.14 | Set filter / algebra. |
| `MapSet.split_with/2` | v1.15 | Predicate partition. |
| `Kernel.is_map_key/2`, `is_struct/1` | v1.10 | Guard-friendly map / struct tests. |
| `Kernel.is_struct/2` | v1.11 | Typed struct guard. |
| `Kernel.is_non_struct_map/1` | v1.17 | Map-but-not-struct guard. |

Deprecated:

- Passing a non-list enumerable as the second argument to `Map.drop/2`, `Map.split/2`, and `Map.take/2` (deprecated in v1.9) — call `Enum.to_list/1` on it first.
- `Map.size/1` (deprecated in v1.3) — use `Kernel.map_size/1`.
- `Access.key/1` (deprecated in v1.4) — use `Access.key/2`.

Source: [compatibility-and-deprecations.html](https://hexdocs.pm/elixir/compatibility-and-deprecations.html).

## Keyword

### Overview & evaluation model

From [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

> "A keyword list is a list that consists exclusively of two-element tuples.
>
> The first element of these tuples is known as the *key*, and it must be an atom. The second element, known as the value, can be any term.
>
> Keywords are mostly used to work with optional values. For a general introduction to keywords and how they compare with maps, see our [Keyword and Maps](https://hexdocs.pm/elixir/keywords-and-maps.html) guide."

A keyword list (type `Keyword.t()`, a.k.a. `keyword()`) is literally `[{atom, term}, ...]` — an ordinary linked list whose every element is a two-element tuple with an atom first. Because it is just a list, it is `Enumerable`, pattern-matchable as a list, and shares list performance characteristics. The empty keyword list is `[]`, and `Keyword.new/0` returns `[]`.

From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Keyword lists are important because they have three special characteristics:
> - Keys must be atoms.
> - Keys are ordered, as specified by the developer.
> - Keys can be given more than once."

These three properties — **atom keys only**, **developer-specified order**, and **duplicate keys allowed** — are what distinguish a keyword list from a `Map` (any key type, no defined order, no duplicate keys). They are also what make keyword lists the canonical shape for **function options**: order can matter, and a few options legitimately repeat.

```elixir
[{:exit_on_close, true}, {:active, :once}, {:packet_size, 1024}]   # explicit
[exit_on_close: true, active: :once, packet_size: 1024]            # shorthand; same value
[]                                                                 # empty keyword list
```

### Keyword literal syntax and the call-site shortcut

There are two equivalent literal forms. The shorthand `[key: value]` is sugar for `[{:key, value}]`; the two "return the exact same value." Keys are always atoms — to use an atom with spaces or special characters, quote it (e.g. `["exit on close": true]`).

Because tuples, lists, and maps are treated like function-argument groups in Elixir syntax, a keyword list passed as the **last argument to a function call may drop its surrounding brackets**:

From [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

> "When keyword lists are passed as the last argument to a function, the square brackets around the keyword list can be omitted."

```elixir
String.split("1-0", "-", [trim: true, parts: 2])   # with brackets
String.split("1-0", "-", trim: true, parts: 2)      # identical; brackets dropped (last arg)
```

This shorthand is what makes `def foo(arg, opts \\ [])` calls read as `foo(x, active: true, timeout: 5)` rather than `foo(x, [active: true, timeout: 5])`. The same rule underlies `do`/`else` blocks: `if(true, do: "yes", else: "no")` is the keyword-list form of the block syntax.

### Duplicate keys and ordering

Duplicate keys are legal, and how each `Keyword` function treats them is the most important thing to know about this module. From [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

> "A keyword may have duplicate keys so it is not strictly a key-value data type. However, most of the functions in this module work on a key-value structure and behave similar to the functions you would find in the `Map` module. For example, `Keyword.get/3` will get the first entry matching the given key, regardless if duplicate entries exist. Similarly, `Keyword.put/3` and `Keyword.delete/2` ensure all duplicate entries for a given key are removed when invoked. ... A handful of functions exist to handle duplicate keys, for example, `get_values/2` returns all values for a given key and `delete_first/2` deletes just the first entry of the existing ones.
>
> Even though lists preserve the existing order, the functions in `Keyword` do not guarantee any ordering. For example, if you invoke `Keyword.put(opts, new_key, new_value)`, there is no guarantee for where `new_key` will be added to (the front, the end or anywhere else)."

The duplicate-key behaviour therefore splits the API into several groups:

| Behaviour class | Examples | What happens to duplicates |
|---|---|---|
| First match only | `get/3`, `get_lazy/3`, `fetch/2`, `fetch!/2` | Read the **first** value for the key. |
| All matches | `get_values/2`, `keys/1`, `values/1`, `pop_values/2`, `take/2` | Return/keep **every** entry. |
| Remove duplicates | `put/3`, `delete/2`, `update/4`, `update!/3`, `replace/3`, `get_and_update/3`, `pop/3`, `drop/2` | Collapse to a single entry (acting on the first). |
| Keep duplicates | `delete_first/2`, `pop_first/3`, `split/2` | Modify only the first occurrence; the rest stay. |
| Adds duplicates | `merge/2`, `merge/3` | Append `keywords2` verbatim, **including** its duplicates. |

```elixir
Keyword.get([a: 1, a: 2], :a, 3)             #=> 1            # first match
Keyword.get_values([a: 1, b: 2, a: 3], :a)   #=> [1, 3]       # all matches
Keyword.put([a: 1, b: 2, a: 4], :a, 3)       #=> [a: 3, b: 2] # duplicates removed
Keyword.delete([a: 1, b: 2, a: 3], :a)       #=> [b: 2]       # all :a removed
Keyword.delete_first([a: 1, b: 2, a: 3], :a) #=> [b: 2, a: 3] # only first :a removed
Keyword.merge([a: 1, b: 2], [a: 3, d: 4, a: 5]) #=> [b: 2, a: 3, d: 4, a: 5]  # duplicates kept
```

Note the asymmetry: `put/3`, `delete/2`, `update/4`, and `update!/3` **remove** duplicates, while `merge/2` **keeps** duplicates coming from the right-hand list. Reach for `get_values/2` and `delete_first/2` when you specifically want the duplicate-preserving semantics.

### Reading keys: `kw[key]`, Access, and pattern matching

Keyword lists implement the `Access` behaviour, so the `kw[key]` bracket form works and is **nil-safe** on missing keys:

From [Access.html](https://hexdocs.pm/elixir/Access.html):

> "`Access` supports keyword lists (`Keyword`) and maps (`Map`) out of the box. ... Both return `nil` if the key does not exist."

```elixir
keywords = [a: 1, b: 2]
keywords[:a]                              #=> 1
keywords[:c]                              #=> nil
Access.pop([name: "Elixir", creator: "Valim"], :name)
#=> {"Elixir", [creator: "Valim"]}
```

Because `Access` is a runtime call, bracket access **cannot appear in a pattern match**. And because the `Keyword` functions do not guarantee order, **pattern matching on keyword lists is discouraged** — it is order-dependent and brittle:

From [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

> "Given ordering is not guaranteed, it is not recommended to pattern match on keyword lists either. ... a function such as `def my_function([some_key: value, another_key: another_value])` will match `my_function([some_key: :foo, another_key: :bar])` but it won't match `my_function([another_key: :bar, some_key: :foo])`."

To read options, prefer `Keyword.get/3` / `Keyword.fetch/2` over pattern matching. Use `keyword?/1` (below) to validate that a term is a keyword list.

### Key functions

Per [Keyword.html](https://hexdocs.pm/elixir/Keyword.html):

> "Most of the functions in this module work in linear time. This means that the time it takes to perform an operation grows at the same rate as the length of the list."

Complexity below is for a keyword list of `n` entries. Almost every operation is **O(n)** because the whole list must be traversed to find a key; the only constant-time operations are `Keyword.new/0` and `Keyword.to_list/1` (identity). This is the key performance difference from `Map`, whose lookups are O(log n).

#### Creating / converting

| Function | Behaviour | Duplicate-key handling | Complexity |
|---|---|---|---|
| `Keyword.new/0` | Returns `[]`. | — | O(1) |
| `Keyword.new/1` | Build from an enumerable of `{key, value}`; **last wins** and duplicates are removed. | removes duplicates | O(n) |
| `Keyword.new/2` | As `new/1` but transform each element first. | removes duplicates | O(n) |
| `Keyword.from_keys/2` *(v1.14)* | Build from a list of atom keys, all mapped to one fixed value. | — | O(k) |
| `Keyword.to_list/1` | Returns the keyword list unchanged (identity). | preserves | O(1) |

```elixir
Keyword.new()                            #=> []
Keyword.new([{:b, 1}, {:a, 2}])          #=> [b: 1, a: 2]
Keyword.new(a: 1, a: 2, a: 3)            #=> [a: 3]            # last wins, deduped
Keyword.from_keys([:a, :b], 0)           #=> [a: 0, b: 0]
```

#### Reading

| Function | Behaviour | Duplicate-key handling | Complexity |
|---|---|---|---|
| `Keyword.get/3` | Value for `key`, or `default` (default `nil`). | first match | O(n) |
| `Keyword.get_lazy/3` | As `get/3` but `default` is a 0-arity fun, evaluated only when missing. | first match | O(n) |
| `Keyword.get_values/2` | All values for `key`. | all matches | O(n) |
| `Keyword.fetch/2` | `{:ok, value}` if present, else `:error`. | first match | O(n) |
| `Keyword.fetch!/2` | Value, or raises `KeyError`. | first match | O(n) |
| `Keyword.has_key?/2` | Boolean membership. | any match | O(n) |
| `Keyword.keyword?/1` | True if the term is a keyword list (traverses to the end). | — | O(n) |

```elixir
Keyword.get([a: 1, a: 2], :a, 3)         #=> 1
Keyword.get([a: 1], :b, 3)               #=> 3
Keyword.get_values([a: 1, b: 2, a: 3], :a) #=> [1, 3]
Keyword.fetch([a: 1], :a)                #=> {:ok, 1}
Keyword.fetch!([a: 1], :b)               #=> ** (KeyError) key :b not found
Keyword.has_key?([a: 1], :a)             #=> true
Keyword.keyword?(a: 1)                   #=> true
Keyword.keyword?([:key])                 #=> false   # not a 2-tuple
Keyword.keyword?([{}])                   #=> false   # not a 2-tuple
```

> `keyword?/1` lives in the **`Keyword`** module and is a regular function, **not** a `Kernel` guard (it must traverse the list, which guards cannot do). There is no `Keyword.get!/2`; use `fetch!/2` for the raising read.

#### Writing and updating

| Function | Behaviour | Duplicate-key handling | Complexity |
|---|---|---|---|
| `Keyword.put/3` | Set `key` to `value`. | removes duplicates | O(n) |
| `Keyword.put_new/3` | Set `key` only if **absent** (no-op if present). | no-op if present | O(n) |
| `Keyword.put_new_lazy/3` | As `put_new/3` but value computed only when absent. | no-op if present | O(n) |
| `Keyword.replace/3` *(v1.11)* | Overwrite `key` only if it **already exists**; unchanged otherwise. | removes duplicates | O(n) |
| `Keyword.replace!/3` *(v1.5)* | As `replace/3` but raises `KeyError` if absent. | removes duplicates | O(n) |
| `Keyword.replace_lazy/3` *(v1.14)* | Replace existing `key` via fun, computed only when present. | removes duplicates | O(n) |
| `Keyword.update/4` | If present, replace via `fun`; if absent, insert `default` (default not passed through `fun`). | removes duplicates | O(n) |
| `Keyword.update!/3` | Replace an **existing** key via `fun`; raises `KeyError` if absent. | removes duplicates | O(n) |
| `Keyword.get_and_update/3` | Read current value and set new in one pass; returns `{current, new_kw}`. `fun` may return `:pop`. | removes duplicates | O(n) |
| `Keyword.get_and_update!/3` | As above but raises `KeyError` if absent. | removes duplicates | O(n) |

```elixir
Keyword.put([a: 1, b: 2, a: 4], :a, 3)   #=> [a: 3, b: 2]
Keyword.put_new([a: 1, b: 2], :a, 3)     #=> [a: 1, b: 2]   # present -> untouched
Keyword.replace([a: 1, b: 2], :a, 3)     #=> [a: 3, b: 2]
Keyword.replace([a: 1], :b, 2)           #=> [a: 1]          # absent -> unchanged
Keyword.update([a: 1], :a, 13, &(&1 * 2))#=> [a: 2]          # existing -> fun
Keyword.update([a: 1], :b, 11, &(&1 * 2))#=> [a: 1, b: 11]   # absent  -> default
Keyword.get_and_update([a: 1], :a, fn v -> {v, "new"} end)
#=> {1, [a: "new"]}
```

The failure modes mirror `Map`: `put/3` always sets and dedupes; `replace/3` only overwrites existing keys; `update/4` overwrites-or-uses-a-default; `update!/3` overwrites-or-raises. Note that `update/4`, `update!/3`, and `get_and_update/3` all **delete duplicate keys** and act on the first match.

#### Deleting and extracting

| Function | Behaviour | Duplicate-key handling | Complexity |
|---|---|---|---|
| `Keyword.delete/2` | Remove **all** entries for `key`. | removes duplicates | O(n) |
| `Keyword.delete_first/2` | Remove only the **first** entry for `key`. | keeps duplicates | O(n) |
| `Keyword.drop/2` | Remove every key in a list of keys. | removes duplicates | O(k·n) |
| `Keyword.take/2` | Keep only the keys in a list of keys. | preserves | O(n) |
| `Keyword.split/2` | Return `{taken, rest}` for the given keys. | keeps duplicates | O(n) |
| `Keyword.split_with/2` *(v1.15)* | Partition into `{matches, rest}` by `fun({k, v})`. | — | O(n) |
| `Keyword.pop/3` | Remove `key`, return `{value, new_kw}`; `{default, kw}` if absent. | removes duplicates | O(n) |
| `Keyword.pop!/2` *(v1.10)* | As `pop/3` but raises `KeyError` if absent. | removes duplicates | O(n) |
| `Keyword.pop_lazy/3` | As `pop/3` but `default` is a lazily-evaluated fun. | removes duplicates | O(n) |
| `Keyword.pop_first/3` | Remove only the first entry for `key`. | keeps duplicates | O(n) |
| `Keyword.pop_values/2` *(v1.10)* | Return **all** values for `key` and remove all entries. | all matches | O(n) |

```elixir
Keyword.delete([a: 1, b: 2, a: 3], :a)        #=> [b: 2]
Keyword.delete_first([a: 1, b: 2, a: 3], :a)  #=> [b: 2, a: 3]
Keyword.take([a: 1, b: 2, c: 3], [:a, :c])    #=> [a: 1, c: 3]
Keyword.split([a: 1, b: 2, c: 3], [:a, :c])   #=> {[a: 1, c: 3], [b: 2]}
Keyword.pop([a: 1], :a)                       #=> {1, []}
Keyword.pop_values([a: 1, b: 2, a: 3], :a)    #=> {[1, 3], [b: 2]}
```

#### Merging, filtering, and validating

| Function | Behaviour | Duplicate-key handling | Complexity |
|---|---|---|---|
| `Keyword.merge/2` | Add all entries of `keywords2` to `keywords1`; on collision `keywords2` wins. | **adds** duplicates | O(n) |
| `Keyword.merge/3` | As `merge/2` but resolve collisions via `fun(key, v1, v2)`. | adds duplicates | O(n) |
| `Keyword.intersect/3` *(v1.17)* | Keep only keys in both; default keeps `keyword2` value; order from `keyword1`. | — | O(n) |
| `Keyword.filter/2` *(v1.13)* | Keep entries where `fun({k, v})` is truthy. | preserves | O(n) |
| `Keyword.reject/2` *(v1.13)* | Drop entries where `fun({k, v})` is truthy. | preserves | O(n) |
| `Keyword.validate/2` *(v1.13)* | Validate against allowed keys; `{:ok, kw}` (defaults applied) or `{:error, invalid_keys}`. | errors on duplicates | O(n) |
| `Keyword.validate!/2` *(v1.13)* | As `validate/2` but raises `ArgumentError` on invalid/duplicate keys. | errors on duplicates | O(n) |
| `Keyword.keys/1` | All keys (keeps duplicates). | preserves | O(n) |
| `Keyword.values/1` | All values (keeps duplicates). | preserves | O(n) |
| `Keyword.equal?/2` | Same keys and values via `===/2`. | — | O(n) |

```elixir
Keyword.merge([a: 1, b: 2], [a: 3, d: 4, a: 5])  #=> [b: 2, a: 3, d: 4, a: 5]   # dups kept
Keyword.merge([a: 1, b: 2], [a: 3, d: 4], fn _k, v1, v2 -> v1 + v2 end)
#=> [b: 2, a: 4, d: 4]
Keyword.filter([one: 1, two: 2, three: 3], fn {_k, v} -> rem(v, 2) == 1 end)
#=> [one: 1, three: 3]
Keyword.keys([a: 1, a: 2])               #=> [:a, :a]
Keyword.equal?([a: 1.0], [a: 1])         #=> false   # === : 1.0 !== 1
Keyword.validate([foo: "bar"], [:foo, bar: "default"])          #=> {:ok, [foo: "bar", bar: "default"]}
Keyword.validate([foo: "bar", unknown: "key"], [:foo, :bar])    #=> {:error, [:unknown]}
Keyword.validate([one: 1, one: 1], [:one])                      #=> {:error, [:one]}   # duplicates invalid
```

`validate/2` / `validate!/2` (added in v1.13) treat the keyword list as a **schema**: they reject unknown keys, apply defaults for omitted `{atom, default}` entries, and treat internal duplicates as an error. They are the idiomatic way to sanitize caller-supplied options at a function boundary.

### Keyword lists vs maps

The canonical guidance, from [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Remember that you should:
> - Use keyword lists for passing optional values to functions
> - Use maps for general key-value data structures
> - Use maps when working with data that has a predefined set of keys"

> "Keyword lists are simply lists, and as such they provide the same linear performance characteristics: the longer the list, the longer it will take to find a key, to count the number of items, and so on. If you need to store a large amount of keys in a key-value data structure, Elixir offers maps."

The detailed `Map` vs `Keyword` comparison table is in the [Map vs Keyword](#map-vs-keyword) subsection above. In short: reach for `Keyword` for **small, ordered, atom-keyed option lists** (and the rare case where duplicate keys are meaningful); reach for `Map` for keyed data of any size or key type, fast O(log n) lookup, unique keys, or key-based pattern matching. Keyword lookups are O(n); never use a keyword list as a large lookup table.

### When keyword lists are idiomatic

From [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html):

> "Keyword lists are mostly used to work with optional values."

- **Function options (the dominant use).** The trailing-options convention is `def foo(arg, opts \\ [])`, read with `Keyword.get/3`. Callers may drop the brackets because the keyword list is the last argument:

```elixir
# Definition uses an empty-list default; reads options with Keyword.get/3.
def split(text, sep, opts \\ []) do
  trim = Keyword.get(opts, :trim, false)
  parts = Keyword.get(opts, :parts, :infinity)
  # ...
end

split("1-2-3", "-", trim: true, parts: 2)   # brackets dropped (last arg)
```

  `String.split/3` and most of the standard library follow this shape: options are atom-keyed, optional, and unordered from the caller's perspective.

- **`do`/`else` block syntax.** A call like `if(c, do: x, else: y)` is precisely a keyword list — `do:` and `else:` are the keys. The block form is sugar over this keyword-list call.

- **DSLs and module attributes.** `import`, `use`, `defmodule`, `defimpl`, `defdelegate`, and `@moduledoc`/`@compile` metadata all consume keyword options:

```elixir
import String, only: [split: 1, split: 2]
use GenServer, restart: :transient
@compile {:parse_transform, :my_transform}
```

- **When duplicate keys are meaningful.** Because keyword lists allow repeats, they model multi-valued options that maps cannot — e.g. multiple `:where` clauses, repeated headers, or repeated query params. Use `Keyword.get_values/2` to read them.

### Performance characteristics

| Operation | Complexity |
|---|---|
| `new/0`, `to_list/1` | O(1) |
| `get` / `get_lazy` / `fetch` / `fetch!` / `has_key?` / `keyword?` (first match) | O(n) |
| `get_values` / `pop_values` / `keys` / `values` (all matches) | O(n) |
| `put` / `put_new` / `replace` / `update` / `update!` / `delete` / `pop` / `get_and_update` (dedupe) | O(n) |
| `delete_first` / `pop_first` / `take` / `split` (keep duplicates) | O(n) |
| `merge` / `intersect` / `filter` / `reject` / `split_with` / `equal?` / `validate` | O(n) |
| `drop` / `take` / `split` (k keys) | O(k·n) |

Keyword lists are immutable and persistent (they are lists). The dominant cost is traversal: every lookup scans the list, so operations are linear. For large keyed data, switch to `Map`.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `Keyword.intersect/3` | v1.17 | Set-style intersection on keyword lists. |
| `Keyword.split_with/2` | v1.15 | Predicate partition. |
| `Keyword.from_keys/2`, `replace_lazy/3` | v1.14 | Fixed-value keyword / lazy replace. |
| `Keyword.filter/2`, `reject/2`, `validate/2`, `validate!/2` | v1.13 | Filtering + option validation. |
| `Keyword.replace/3` | v1.11 | Overwrite-only-if-present. |
| `Keyword.pop!/2`, `pop_values/2` | v1.10 | Raising pop / all-values pop. |
| `Keyword.replace!/3` | v1.5 | Raising overwrite-if-present. |

Deprecated:

- `Keyword.size/1` — deprecated in **v1.3**; use `Kernel.length/1` to count entries (a keyword list is a list, so `length/1` is the O(n) size operation).

Does not exist (common mistakes to avoid):

- `Keyword.get!/2` does not exist; use `Keyword.fetch!/2` for the raising read.
- `keyword?/1` is **not** a `Kernel` guard — it is `Keyword.keyword?/1`, a regular function that traverses the list.

Sources: [Keyword.html](https://hexdocs.pm/elixir/Keyword.html), [keywords-and-maps.html](https://hexdocs.pm/elixir/keywords-and-maps.html), [Access.html](https://hexdocs.pm/elixir/Access.html), [Module.html](https://hexdocs.pm/elixir/Module.html), [Kernel.html](https://hexdocs.pm/elixir/Kernel.html), [compatibility-and-deprecations.html](https://hexdocs.pm/elixir/compatibility-and-deprecations.html).

## List

### Overview & evaluation model

From [List.html](https://hexdocs.pm/elixir/List.html):

> "Linked lists hold zero, one, or more elements in the chosen order."

> "Lists in Elixir are effectively linked lists, which means they are internally represented in pairs containing the head and the tail of a list."

> "Lists also implement the `Enumerable` protocol, so many functions to work with lists are found in the `Enum` module. Additionally, the following functions and operators for lists are found in `Kernel`: `++/2`, `--/2`, `hd/1`, `tl/1`, `in/2`, `length/1`."

A list is a singly-linked, immutable sequence of cons cells. Each cell holds a head and a pointer to the tail; the empty list `[]` terminates a *proper* list. Because lists implement `Enumerable` directly (not via a `List` protocol), most general-purpose collection work — `map`, `filter`, `reduce`, `take`, `sort`, `group_by` — lives in `Enum` (see the `Enum` section above). The `List` module holds only the operations that exploit list *structure*: O(1) head access, O(1) prepend, cons-cell recursion, the keyed-tuple helpers inherited from Erlang's `:lists`, charlist conversions, flattening, and diffing.

> "Most of the functions in this module work in linear time. This means that the time it takes to perform an operation grows at the same rate as the length of the list. For example `length/1` and `last/1` will run in linear time because they need to iterate through every element of the list, but `first/1` will run in constant time because it only needs the first element."

Source: [List.html](https://hexdocs.pm/elixir/List.html).

### Lists as linked lists: head, tail, cons

The cons cell `[head | tail]` is both the constructor and the deconstructor:

```elixir
[head | tail] = [1, 2, 3]
head                                   #=> 1
tail                                   #=> [2, 3]

# Any list is just nested cons cells terminated by []:
[1 | [2 | [3 | []]]]                   #=> [1, 2, 3]
```

`hd/1` and `tl/1` (in `Kernel`; both allowed in guards and inlined by the compiler) return the head and tail. Each raises `ArgumentError` on the empty list, and both *do* work on improper lists.

```elixir
hd([1, 2, 3])                          #=> 1
tl([1, 2, 3])                          #=> [2, 3]
hd([])                                 #=> ** (ArgumentError) argument error
tl([:a, :b | :improper_end])           #=> [:b | :improper_end]   # works on improper lists
```

`length/1` is **O(n)** — it walks the whole list — and is allowed in guards. (Recall from [naming-conventions.html](https://hexdocs.pm/elixir/naming-conventions.html): `length` means O(n); `size` means O(1). There is no `list_size/1`.)

```elixir
length([1, 2, 3, 4, 5])                #=> 5
```

### Prepend O(1) vs append O(n)

This is the single most important performance fact about Elixir lists. From [List.html](https://hexdocs.pm/elixir/List.html):

> "Due to their cons cell based representation, prepending an element to a list is always fast (constant time), while appending becomes slower as the list grows in size (linear time): `[0 | list]` is fast; `list ++ [4]` is slow."

The `++/2` operator (in `Kernel`) appends by walking the *left* operand, so its cost is `length(a)`:

> "The complexity of `a ++ b` is proportional to `length(a)`, so avoid repeatedly appending to lists of arbitrary length, for example, `list ++ [element]`. Instead, consider prepending via `[element | rest]` and then reversing."

```elixir
list = [1, 2, 3]
[0 | list]                             #=> [0, 1, 2, 3]   # O(1) prepend
list ++ [4]                            #=> [1, 2, 3, 4]   # O(n) append
```

The idiomatic way to build a list of unknown length is to **prepend** while accumulating, then **reverse once** at the end:

```elixir
# Bad: O(n^2) — each step re-walks the accumulator.
Enum.reduce(1..1000, [], fn x, acc -> acc ++ [x] end)

# Good: O(n) — prepend, then a single reverse.
1..1000
|> Enum.reduce([], fn x, acc -> [x | acc] end)
|> Enum.reverse()
```

`Enum.map/2`, `Enum.filter/2`, and friends already build their result lists this way internally, so prefer them over hand-rolled `++` loops.

### Key functions

Complexities are for a list of `n` elements. Most `List` functions are O(n) because a singly-linked list must be traversed from the head to reach any later element.

#### Accessing

| Function | Behaviour | Complexity |
|---|---|---|
| `List.first(list, default \\ nil)` | The first element, or `default` if empty. | O(1) |
| `List.first!(list)` *(v1.20)* | The first element; raises `ArgumentError` if empty. | O(1) |
| `List.last(list, default \\ nil)` | The last element, or `default` if empty. | O(n) |
| `List.last!(list)` *(v1.20)* | The last element; raises `ArgumentError` if empty. | O(n) |

```elixir
List.first([1, 2, 3])                  #=> 1
List.first([])                         #=> nil
List.first([], 0)                      #=> 0
List.last([1, 2, 3])                   #=> 3
List.first!([])                        #=> ** (ArgumentError) attempted to get the first element of an empty list
```

`List.first/1` is O(1) because it only reads the head; `List.last/1` is O(n) because it must walk to the end. (`List.first/2` and `List.last/2` — the default-value arities — were added in v1.12; the `/1` forms have existed since v1.0.)

#### Inserting, updating, and deleting at an index

| Function | Behaviour | Complexity |
|---|---|---|
| `List.insert_at(list, index, value)` | Inserts `value` at `index`; `index` is capped at the list length. Negative = offset from end. | O(n) |
| `List.replace_at(list, index, value)` | Replaces the value at `index`. Out of bounds -> original list unchanged. | O(n) |
| `List.update_at(list, index, fun)` | Replaces the value at `index` with `fun.(old)`. Out of bounds -> original list unchanged. | O(n) |
| `List.delete_at(list, index)` | Removes the value at `index`. Out of bounds -> original list unchanged. | O(n) |
| `List.pop_at(list, index, default \\ nil)` *(v1.4)* | `{value, new_list}`; removes the value at `index`. Out of bounds -> `{default, original}`. | O(n) |
| `List.delete(list, element)` | Removes the *first* occurrence of `element` (by `===`). No match -> original list. | O(n) |

```elixir
List.insert_at([1, 2, 3], 2, 0)        #=> [1, 2, 0, 3]
List.insert_at([1, 2, 3], -1, 0)       #=> [1, 2, 3, 0]
List.replace_at([1, 2, 3], -1, 0)      #=> [1, 2, 0]
List.update_at([1, 2, 3], 0, &(&1 + 10)) #=> [11, 2, 3]
List.update_at([1, 2, 3], 50, &(&1 + 1)) #=> [1, 2, 3]   # out of bounds: unchanged
List.delete_at([1, 2, 3], -1)          #=> [1, 2]
List.pop_at([1, 2, 3], 0)              #=> {1, [2, 3]}
List.pop_at([1, 2, 3], 5, :none)       #=> {:none, [1, 2, 3]}
List.delete([:a, :b, :b, :c], :b)      #=> [:a, :b, :c]   # only first match removed
```

> Note: `update_at/3`, `replace_at/3`, `delete_at/2`, and `pop_at/3` are *silent no-ops* on an out-of-bounds index — they return the original list rather than raising. For O(1) insertion at the head, prefer `[value | list]` over `List.insert_at(list, 0, value)`.

#### Building

| Function | Behaviour | Complexity |
|---|---|---|
| `List.duplicate(elem, n)` | A list of `elem` repeated `n` times; `n = 0` -> `[]`. | O(n) |
| `List.wrap(term)` | `nil` -> `[]`; a list -> itself; anything else -> `[term]`. | O(1) |
| `List.flatten(list)` | Recursively flattens nested lists; discards empty sublists. | O(total) |
| `List.flatten(list, tail)` | Flattens `list` then appends `tail` (whose own empties are kept). | O(total) |

```elixir
List.duplicate("hi", 3)                #=> ["hi", "hi", "hi"]
List.duplicate(:x, 0)                  #=> []
List.wrap("hello")                     #=> ["hello"]
List.wrap([1, 2, 3])                   #=> [1, 2, 3]
List.wrap(nil)                         #=> []
List.flatten([1, [[2], 3]])            #=> [1, 2, 3]
List.flatten([1, [[2], 3]], [4, 5])    #=> [1, 2, 3, 4, 5]
```

`List.wrap/1` is the standard way to normalise "maybe a list, maybe nil, maybe a single value" into a list — common when accepting flexible function arguments.

#### Folding

| Function | Behaviour | Complexity |
|---|---|---|
| `List.foldl(list, acc, fun)` | Left fold; `fun.(elem, acc)`. Tail-recursive. | O(n) |
| `List.foldr(list, acc, fun)` | Right fold; `fun.(elem, acc)`. Not tail-recursive. | O(n) |

```elixir
List.foldl([1, 2, 3, 4], 0, fn x, acc -> x - acc end)   #=> 2
List.foldr([1, 2, 3, 4], 0, fn x, acc -> x - acc end)   #=> -2
```

`List.foldl/3` and `List.foldr/3` are thin wrappers over Erlang's `:lists.foldl/3` and `:lists.foldr/3`. For new code prefer `Enum.reduce/3` (the same left-fold semantics, but working on any enumerable); reach for `List.foldr/3` only when you specifically need right-to-left traversal. Neither is deprecated.

#### Keyed tuple operations (`key*`)

These operate on a *list of tuples*, matching an element by the value at a given `position`. They are direct counterparts to Erlang's `:lists.key*` functions and work on any list of tuples — a keyword list `[a: 1, b: 2]` is just the common special case where `position` is `0`.

| Function | Behaviour | Complexity |
|---|---|---|
| `List.keyfind(list, key, position, default \\ nil)` | First tuple matching `key` at `position`, else `default`. | O(n) |
| `List.keyfind!(list, key, position)` *(v1.13)* | As `keyfind/3` but raises `KeyError` if not found. | O(n) |
| `List.keymember?(list, key, position)` | `true` if a tuple matches `key` at `position`. | O(n) |
| `List.keyreplace(list, key, position, new_tuple)` | Replaces the matching tuple with `new_tuple`; no match -> unchanged. | O(n) |
| `List.keystore(list, key, position, new_tuple)` | Replaces the matching tuple, or *appends* `new_tuple` if absent. | O(n) |
| `List.keydelete(list, key, position)` | Removes the first tuple matching `key` at `position`. | O(n) |
| `List.keytake(list, key, position)` | `{matching_tuple, rest}` or `nil`. | O(n) |
| `List.keysort(list, position, sorter \\ :asc)` | Stable sort by the element at `position`. | O(n log n) |

```elixir
users = [{:admin, 1}, {:user, 2}, {:user, 3}]
List.keyfind(users, :admin, 0)                  #=> {:admin, 1}
List.keymember?(users, :mod, 0)                 #=> false
List.keystore(users, :guest, 0, {:guest, 4})    #=> [{:admin, 1}, {:user, 2}, {:user, 3}, {:guest, 4}]
List.keytake(users, :user, 0)                   #=> {{:user, 2}, [{:admin, 1}, {:user, 3}]}
List.keysort([a: 5, b: 1, c: 3], 1)             #=> [b: 1, c: 3, a: 5]
List.keysort([a: 5, b: 1, c: 3], 1, :desc)      #=> [a: 5, c: 3, b: 1]
```

As with `Enum.sort/2`, avoid the default term-order sorter for structs (e.g. `Date`) — pass a comparator module or `{:desc, module}`:

```elixir
users = [{"Ellis", ~D[1943-05-11]}, {"Lovelace", ~D[1815-12-10]}]
List.keysort(users, 1, Date)            # sorts by date, semantically
```

The 3-arity `List.keysort/3` (with `sorter`) was added in v1.14.

#### Conversions

| Function | Behaviour | Complexity |
|---|---|---|
| `List.to_tuple(list)` | List -> tuple. Inlined. | O(n) |
| `List.to_string(list)` | Charlist/codepoints/binary fragments -> string. | O(n) |
| `List.to_charlist(list)` *(v1.8)* | As `to_string/1` but returns a charlist. | O(n) |
| `List.to_integer(charlist)` | Charlist of digits -> integer. Inlined. | O(n) |
| `List.to_integer(charlist, base)` | As above, in `base` `2..36`. Inlined. | O(n) |
| `List.to_float(charlist)` | Charlist of a float -> float. Inlined. | O(n) |
| `List.to_atom(charlist)` | Charlist -> atom. Inlined. | O(n) |
| `List.to_existing_atom(charlist)` | Charlist -> existing atom; raises if absent. Inlined. | O(n) |

```elixir
List.to_tuple([:share, [:elixir, 163]])  #=> {:share, [:elixir, 163]}
List.to_string([0x00E6, 0x00DF])         #=> "æß"
List.to_charlist([0x0061, "bc"])         #=> ~c"abc"
List.to_integer(~c"123")                 #=> 123
List.to_integer(~c"3FF", 16)             #=> 1023
List.to_atom(~c"Elixir")                 #=> :Elixir
```

> `List.to_string/1` and `List.to_charlist/1` expect integers to be **Unicode code points**, not bytes. For a list of raw bytes use the Erlang `:binary` module instead.

#### Prefix / suffix and diffing

| Function | Behaviour | Complexity |
|---|---|---|
| `List.starts_with?(list, prefix)` *(v1.5)* | `true` if `list` begins with `prefix`. Empty prefix -> `true`. | O(length prefix) |
| `List.ends_with?(list, suffix)` *(v1.18)* | `true` if `list` ends with `suffix`. Empty suffix -> `true`. | O(n) |
| `List.myers_difference(list1, list2)` *(v1.4)* | Edit script `[{:eq \| :ins \| :del, sublist}]`. | O((N+D)·D) |
| `List.myers_difference(list1, list2, diff)` *(v1.8)* | As above with nested diffs under `:diff`. | O((N+D)·D) |

```elixir
List.starts_with?([1, 2, 3], [1, 2])   #=> true
List.ends_with?([1, 2, 3], [2, 3])     #=> true
List.myers_difference([1, 4, 2, 3], [1, 2, 3, 4])
#=> [eq: [1], del: [4], eq: [2, 3], ins: [4]]
```

### Charlists

A charlist is a list of integers where each integer is a Unicode code point. The `~c` sigil is the canonical literal syntax; `?c` yields the code point of `c`.

```elixir
~c"abc"                                #=> ~c"abc"   (i.e. [97, 98, 99])
?d                                     #=> 100
~c"héllo"                              #=> [104, 233, 108, 108, 111]
```

From [List.html](https://hexdocs.pm/elixir/List.html):

> "If a list is made of non-negative integers, where each integer represents a Unicode code point, the list can also be called a charlist. These integers must be within the range `0..0x10FFFF` ... and be out of the range `0xD800..0xDFFF` (reserved for UTF-16 surrogate pairs)."

> "The rationale behind this behavior is to better support Erlang libraries which may return text as charlists instead of Elixir strings. In Erlang, charlists are the default way of handling strings, while in Elixir it's binaries."

Charlists exist primarily for **Erlang interop** — many Erlang APIs (e.g. `Application.loaded_applications/0`) return charlists. In Elixir itself, prefer binaries (`String.t`). Convert between the two with `List.to_string/1` and `to_charlist/1`:

```elixir
List.to_string(~c"hello")              #=> "hello"
to_charlist("hello")                   #=> ~c"hello"
```

`List.ascii_printable?/2` *(v1.6)* checks whether a charlist is made only of printable ASCII characters (optionally up to a `limit`). Improper lists are never printable.

```elixir
List.ascii_printable?(~c"abc")         #=> true
List.ascii_printable?(~c"abc" ++ [0])  #=> false
List.ascii_printable?(~c"abc" ++ ?d)   #=> false   # improper list
```

> Deprecation: single-quoted charlists `'foo'` are deprecated since **v1.17** — use the `~c"foo"` sigil instead. (See [compatibility-and-deprecations.html](https://hexdocs.pm/elixir/compatibility-and-deprecations.html).)

### Improper lists

A *proper* list ends in `[]`. An *improper* list ends in some other value. From [List.html](https://hexdocs.pm/elixir/List.html):

> "Some lists, called improper lists, do not have an empty list as the second element in the last cons cell: `[1 | [2 | [3 | 4]]]` is printed `[1, 2, 3 | 4]`. Although improper lists are generally avoided, they are used in some special circumstances like iodata and chardata entities (see the `IO` module)."

```elixir
[1, 2 | 3]                             #=> [1, 2 | 3]   # improper
List.improper?([1, 2 | 3])             #=> true         # (since v1.8)
List.improper?([1, 2, 3])              #=> false
```

Gotchas with improper lists:

- `length/1` raises `ArgumentError` — it requires a proper list.
- `List.to_string/1`, `Enum` traversal, and most `List` functions are undefined on improper tails.
- `hd/1` and `tl/1` *do* work (their spec is `maybe_improper_list`); e.g. `tl([:a | %{b: 1}])` returns `%{b: 1}`.
- Improper lists are never treated as charlists.

```elixir
length([1 | 2])                        #=> ** (ArgumentError) argument error
```

Use `List.improper?/1` to guard before traversing untrusted list-shaped data.

### iodata and chardata (brief)

Lists are the backbone of **iodata** (lists of bytes/binaries) and **chardata** (lists of Unicode code points/binaries), which let IO functions avoid copying binaries on every concatenation. From [IO.html](https://hexdocs.pm/elixir/IO.html):

> "A term of type IO data is a binary or a list containing bytes (integers within the 0..255 range) or nested IO data. The type is recursive."

```elixir
IO.iodata_to_binary([?h, "el", ["l", [?o]]])   #=> "hello"
```

Most IO/socket APIs accept iodata directly; flatten it with `IO.iodata_to_binary/1` (`IO.iodata_empty?/1` was added in v1.20). This is the main legitimate use of improper lists.

### List vs Enum

Lists implement `Enumerable` directly; `List` does not define its own protocol. Use `List` for structure-aware work and `Enum` for generic transformations.

| Scenario | Use | Why |
|---|---|---|
| O(1) head access | `List.first/1`, `hd/1`, or `[h \| _] = list` | Reads the head in O(1). |
| Random index `n` | `Enum.at(list, n)` | O(n) on a list either way; `Enum` is the general API. (`List` has no `get/2`.) |
| Prepend / build a list | `[x \| acc]` then `Enum.reverse/1` | O(n) total; avoids the O(n^2) of `acc ++ [x]`. |
| `map`, `filter`, `reduce`, `sort`, `group_by` | `Enum` | Generic, pipe-friendly; builds results efficiently. |
| Operations on a list of tuples by key | `List.keyfind/3`, `keysort/2`, etc. | Direct, and the Erlang `:lists` counterpart. |
| Charlist <-> string conversion | `List.to_string/1`, `to_charlist/1` | List-specific; `Enum` has no equivalent. |
| Append two lists | `++/2` or `Enum.concat/2` | There is no `List.append/2`. |
| Membership test | `x in list` / `Enum.member?/2` | There is no `List.member?/2`; `in/2` expands to `Enum.member?/2`. |

Membership via `in/2` in a *guard* with a list literal expands to `x === v1 or x === v2 or ...`, which is O(n) and inefficient for large sets — use a `MapSet` or `Range` instead.

### Complexity reference table

| Function | Complexity |
|---|---|
| `first/1`, `first!/1`, `hd/1`, cons `[x \| list]`, `wrap/1` | O(1) |
| `last/1`, `last!/1`, `length/1` | O(n) |
| `delete/2`, `delete_at/2`, `insert_at/3`, `replace_at/3`, `update_at/3`, `pop_at/3` | O(n) |
| `flatten/1,2`, `foldl/3`, `foldr/3`, `duplicate/2` | O(n) |
| `keyfind/3,4`, `keyfind!/3`, `keymember?/3`, `keyreplace/4`, `keystore/4`, `keydelete/3`, `keytake/3` | O(n) |
| `keysort/2,3` | O(n log n) |
| `to_tuple/1`, `to_string/1`, `to_charlist/1`, `to_integer/1,2`, `to_atom/1` | O(n) |
| `starts_with?/2` | O(length prefix) |
| `ends_with?/2` | O(n) |
| `myers_difference/2,3` | O((N + D)·D), D = edit distance |
| `++/2` (append), `--/2` | O(length of left list) |

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `List.first!/1`, `List.last!/1` | v1.20 | Raising variants of `first/1`, `last/1`. |
| `List.ends_with?/2` | v1.18 | Suffix check (counterpart to `starts_with?/2`). |
| `List.keysort/3` (with `sorter`) | v1.14 | Custom sorter / `:asc` / `:desc` / module. |
| `List.keyfind!/3` | v1.13 | Raising key lookup. |
| `List.first/2`, `last/2` (default value) | v1.12 | The `/1` forms exist since v1.0. |
| `List.ascii_printable?/2` | v1.6 | Printable-ASCII charlist check. |
| `List.starts_with?/2` | v1.5 | Prefix check. |
| `List.pop_at/3`, `List.myers_difference/2` | v1.4 | Index pop / Myers diff. |
| `List.improper?/1`, `List.to_charlist/1`, `List.myers_difference/3` | v1.8 | Improper predicate, charlist conversion, nested diff. |

Deprecated:

- `List.zip/1` — deprecated in **v1.18**; use `Enum.zip/1`.
- Single-quoted charlists `'foo'` — deprecated in **v1.17**; use the `~c"foo"` sigil.

Does not exist (common mistakes to avoid):

- `List.append/2` does not exist; use `++/2` (Kernel) or `Enum.concat/2`.
- `List.get/2` does not exist; use `List.first/1`, pattern matching, or `Enum.at/2`.
- `List.member?/2` does not exist; use `in/2` or `Enum.member?/2`.
- `List.head/1` / `List.tail/1` do not exist; they are `Kernel.hd/1` / `Kernel.tl/1`.
- `List.empty?/1` does not exist; use `list == []` or pattern matching.
- `List.printable?/1` and `List.ascii_zero?/1` do not exist; use `List.ascii_printable?/2`.
- `List` does not define its own protocol (lists implement `Enumerable` directly).

Sources: [List.html](https://hexdocs.pm/elixir/List.html), [Kernel.html](https://hexdocs.pm/elixir/Kernel.html), [Tuple.html](https://hexdocs.pm/elixir/Tuple.html), [IO.html](https://hexdocs.pm/elixir/IO.html), [Enum.html](https://hexdocs.pm/elixir/Enum.html), [compatibility-and-deprecations.html](https://hexdocs.pm/elixir/compatibility-and-deprecations.html).

## String

### Overview & evaluation model

From [String.html](https://hexdocs.pm/elixir/String.html):

> "Strings in Elixir are UTF-8 encoded binaries. Strings in Elixir are a sequence of Unicode characters, typically written between double quoted strings, such as `"hello"` and `"héllò"`."

> "The functions in this module act according to The Unicode Standard, Version 17.0.0."

A `String.t()` is a UTF-8 encoded `binary`. The two types are interchangeable at runtime:

> "`@type t() :: binary()` — A UTF-8 encoded binary. The types `String.t()` and `binary()` are equivalent to analysis tools. Although, for those reading the documentation, `String.t()` implies it is a UTF-8 encoded binary."

String literals are double-quoted; concatenation uses `<>` and interpolation uses `#{}` (which invokes the `String.Chars` protocol for non-string values):

```elixir
"hello" <> " " <> "world"              #=> "hello world"
name = "joe"
"hello #{name}"                        #=> "hello joe"
```

Every string is a binary, but not every binary is a valid string:

> "A string is a UTF-8 encoded binary, where the code point for each character is encoded using 1 to 4 bytes. Thus every string is a binary, but due to the UTF-8 standard encoding rules, not every binary is a valid string."
> — [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html)

```elixir
is_binary("hello")                     #=> true
String.valid?(<<239, 191, 19>>)         #=> false   # bytes are not valid UTF-8
```

Source: [String.html](https://hexdocs.pm/elixir/String.html).

### Bytes, code points, and graphemes

String functions reason about three distinct units, and confusing them is the single most common source of string bugs:

| Unit | What it is | O(1) read? | Functions |
|---|---|---|---|
| **byte** | one raw 8-bit octet of the underlying binary | yes | `Kernel.byte_size/1`, `Kernel.binary_part/3` |
| **code point** | a Unicode code point (an integer); UTF-8 encodes each in 1–4 bytes | no | `String.codepoints/1` (as strings), `String.to_charlist/1` (as ints) |
| **grapheme** | a "user-perceived character"; may be several code points | no | `String.graphemes/1`, `String.length/1`, `String.first/1`, `String.at/2` |

From [String.html](https://hexdocs.pm/elixir/String.html):

> "This module also works with the concept of grapheme cluster (from now on referenced as graphemes). Graphemes can consist of multiple code points that may be perceived as a single character by readers."

The canonical illustration is "é", which has two valid Unicode representations — a single code point (U+00E9), or `e` (U+0065) plus a combining acute accent (U+0301):

```elixir
string = "\u0065\u0301"                 #=> "é"        # looks like one character
byte_size(string)                       #=> 3          # 1 byte for 'e' + 2 for the accent
String.length(string)                   #=> 1          # one grapheme
String.codepoints(string)               #=> ["e", "́"] # two code points
String.graphemes(string)                #=> ["é"]      # one grapheme
```

Grapheme boundaries follow the Extended Grapheme Cluster algorithm from [Unicode Standard Annex #29](https://www.unicode.org/reports/tr29/). Emoji sequences are a common multi-code-point grapheme:

```elixir
String.codepoints("👩‍🚒")               #=> ["👩", "‍", "🚒"]   # woman + ZWJ + fire engine
String.graphemes("👩‍🚒")                #=> ["👩‍🚒"]            # one grapheme
String.length("👩‍🚒")                   #=> 1
```

> "In general, the functions in this module rely on the Unicode Standard, but do not contain any of the locale specific behavior."
> — [String.html](https://hexdocs.pm/elixir/String.html)

This distinction drives the performance model: `Kernel.byte_size/1` is O(1) because the byte count is stored with the binary, while `String.length/1` is O(n) because the whole string must be walked to count graphemes. (See the `count` vs `size` vs `length` rule quoted under the `Enum` section.)

#### UTF-8 validity and invalid bytes

> "The UTF-8 encoding is self-synchronizing. This means that if malformed data ... is encountered, only one code point needs to be rejected. This module relies on this behavior to ignore such invalid characters. ... In other words, this module expects invalid data to be detected elsewhere, usually when retrieving data from the external source."
> — [String.html](https://hexdocs.pm/elixir/String.html)

So most `String` functions do **not** raise on stray invalid bytes — they skip them. To validate or repair untrusted input use:

| Function | Behaviour |
|---|---|
| `String.valid?/1` | `true` iff the binary is valid UTF-8. |
| `String.valid?/2` | Accepts an `algorithm` (`:default` \| `:utf8`). |
| `String.chunk(string, :valid)` | Splits into a list of valid and invalid sub-binaries. |
| `String.replace_invalid/2` *(since 1.16)* | Replaces every invalid byte with `replacement` (`"�"` by default). |

```elixir
String.replace_invalid("asd" <> <<0xFF>>)    #=> "asd�"
```

> "Note it is generally not advised to use `\xNN` in Elixir strings, as introducing an invalid byte sequence would make the string invalid. If you have to introduce a character by its hexadecimal representation, it is best to work with Unicode code points, such as `\uNNNN`."
> — [String.html](https://hexdocs.pm/elixir/String.html)

### String vs binary vs charlist

- A **string** (`String.t()` / `binary`) is a UTF-8 byte sequence — the default text type.
- A **charlist** (`charlist`) is a *list of integer code points*, written `~c"hello"`. It is a linked list, not a binary; concatenate with `++`, never `<>`.
- A **binary** that is not valid UTF-8 is just bytes — operate on it with the `:binary` module or binary patterns.

> "A charlist is a list of integers where all the integers are valid code points. In practice, you will not come across them often, only in specific scenarios such as interfacing with older Erlang libraries that do not accept binaries as arguments."
> — [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html)

```elixir
~c"hello"                               #=> ~c"hello"
String.to_charlist("héllo")             #=> [104, 233, 108, 108, 111]
[?h, ?e, ?l, ?l, ?o] == ~c"hello"        #=> true   # ?c returns the code point as an integer
```

### Key functions

Complexities are given for a string of `n` graphemes unless noted. Grapheme-based functions are O(n) because they walk the binary; byte-level `Kernel`/`:binary` functions are O(1).

#### Access and size

| Function | Behaviour | Complexity |
|---|---|---|
| `String.length/1` | Number of graphemes. | O(n) |
| `String.at/2` | Grapheme at `position` (negative counts from end); `nil` if out of range. | O(n) |
| `String.first/1` | First grapheme; `nil` if empty. | O(1) |
| `String.last/1` | Last grapheme; traverses the whole string. | O(n) |
| `String.byte_slice/3` *(since 1.17)* | Substring bounded by `start_bytes`/`size_bytes`; trims truncated code points. | byte-offset based |
| `byte_size/1` *(Kernel)* | Byte length. | **O(1)** |

> "**Linear Access**: This function [`at/2`] has to linearly traverse the string. If you want to access a string or a binary in constant time based on the number of bytes, use `Kernel.binary_slice/3` or `:binary.at/2` instead."
> — [String.html](https://hexdocs.pm/elixir/String.html)

```elixir
String.length("elixir")                 #=> 6
String.at("elixir", 0)                  #=> "e"
String.at("elixir", -1)                 #=> "r"
String.at("elixir", 10)                 #=> nil
byte_size("é")                          #=> 2          # O(1); one grapheme, two bytes
```

#### Building and shaping

| Function | Behaviour | Complexity |
|---|---|---|
| `String.duplicate/2` | `subject` repeated `n` times; inlined by the compiler. | O(n) |
| `String.pad_leading/3` | Left-pad to `count` graphemes using `padding` (default `" "`). | O(n) |
| `String.pad_trailing/3` | Right-pad to `count`. | O(n) |
| `String.reverse/1` | Reverses **graphemes**, not bytes. | O(n) |

```elixir
String.duplicate("abc", 2)              #=> "abcabc"
String.pad_leading("abc", 6, "12")      #=> "121abc"
String.pad_trailing("abc", 5)           #=> "abc  "
String.reverse("Elxîr")                 #=> "rîxlE"
```

#### Case conversion

| Function | Behaviour | Complexity |
|---|---|---|
| `String.upcase/2` | Uppercase per `mode` (default `:default`). | O(n) |
| `String.downcase/2` | Lowercase per `mode`. | O(n) |
| `String.capitalize/2` | First grapheme uppercase, the rest lowercase, per `mode`. | O(n) |

```elixir
String.upcase("olá")                    #=> "OLÁ"
String.downcase("OLÁ")                  #=> "olá"
String.capitalize("abcd")               #=> "Abcd"
```

The `mode` argument is covered under **Case conversion and Unicode modes** below.

#### Splitting, slicing, and trimming

| Function | Behaviour | Complexity |
|---|---|---|
| `String.split/1` | Splits on Unicode whitespace, collapsing runs and trimming empties (does not split on non-breaking space). | O(n) |
| `String.split/3` | Splits on a `pattern` (string / list / regex / compiled pattern); options `:parts`, `:trim`. | O(n) |
| `String.split_at/2` | `{first, rest}` at a grapheme offset (negative counts from end; clamped). | O(n) |
| `String.slice/2` | Substring by `Range` (grapheme offsets). | O(n) |
| `String.slice/3` | `length` graphemes starting at `start`. | O(n) |
| `String.splitter/3` | Returns a **lazy** `Enumerable` that emits parts on demand (no regex). | O(consumed) |
| `String.trim/1` | Removes leading + trailing Unicode whitespace. | O(n) |
| `String.trim/2` | Removes leading + trailing occurrences of `to_trim` (exact-string, greedy). | O(n) |
| `String.trim_leading/1,2` | Leading only. | O(n) |
| `String.trim_trailing/1,2` | Trailing only. | O(n) |

> "**Linear Access**: This function [`split_at/2`] splits on graphemes and for such it has to linearly traverse the string. If you want to split a string or a binary based on the number of bytes, use `Kernel.binary_part/3` instead."
> — [String.html](https://hexdocs.pm/elixir/String.html)

```elixir
String.split(" foo   bar ")             #=> ["foo", "bar"]
String.split("a,b,c", ",", parts: 2)    #=> ["a", "b,c"]
String.split_at("sweetelixir", 5)       #=> {"sweet", "elixir"}
String.split_at("sweetelixir", -6)      #=> {"sweet", "elixir"}
String.slice("elixir", 1..3)            #=> "lix"
String.slice("elixir", 1, 3)            #=> "lix"
String.trim("\n  abc\n  ")              #=> "abc"
String.trim_leading("__ abc _", "_")    #=> " abc _"
```

The lazy `String.splitter/3` is preferred when you only need the first few parts of a long string:

```elixir
"1,2 3,4 5,6"
|> String.splitter([" ", ","])
|> Enum.take(4)                         #=> ["1", "2", "3", "4"]
```

Note the `trim/2` family matches whole `to_trim` strings greedily, so `String.trim_leading("1 abc", "11")` is a no-op (there is no leading `"11"`).

#### Searching and matching

| Function | Behaviour | Complexity |
|---|---|---|
| `String.contains?/2` | `true` if `string` contains any of `contents` (string, list of strings, or compiled pattern). | O(n) |
| `String.starts_with?/2` | `true` if `string` starts with any of the given prefixes; `""` always matches. | fast |
| `String.ends_with?/2` | `true` if `string` ends with any of the given suffixes; `""` always matches. | fast |
| `String.match?/2` | `true` if `string` matches a `Regex`. | O(n) |
| `String.count/2` *(since 1.19)* | Count of non-overlapping matches of a pattern/regex. | O(n) |
| `String.next_codepoint/1` | `{codepoint, rest}` or `nil`. | O(1) |
| `String.next_grapheme/1` | `{grapheme, rest}` or `nil`. | O(1) |
| `String.equivalent?/2` | Canonical (NFD) equivalence of two strings. | O(n) |

```elixir
String.contains?("elixir of life", ["life", "death"])   #=> true
String.starts_with?("elixir", "el")                     #=> true
String.ends_with?("language", ["youth", "age"])         #=> true
String.match?("foo", ~r/foo/)                           #=> true
String.count("hello world", "o")                        #=> 2
```

`String.match?/2`, `Kernel.=~/2`, and `Regex.match?/2` are interchangeable for testing a string against a regex.

#### Replacing

| Function | Behaviour | Complexity |
|---|---|---|
| `String.replace/4` | Replace occurrences of `pattern` (string/list/regex/compiled) with `replacement` (string or function). Option `:global` (default `true`). | O(n) |
| `String.replace_leading/3` | Replace all leading occurrences of `match`. Raises on `match == ""`. | O(n) |
| `String.replace_trailing/3` | Replace all trailing occurrences. Raises on `match == ""`. | O(n) |
| `String.replace_prefix/3` | Replace `match` once if it is a prefix; `""` match prepends. | fast |
| `String.replace_suffix/3` | Replace `match` once if it is a suffix; `""` match appends. | fast |

```elixir
String.replace("a,b,c", ",", "-")                    #=> "a-b-c"
String.replace("a,b,c", ",", "-", global: false)     #=> "a-b,c"
String.replace_leading("   abc", " ", "-")           #=> "---abc"
String.replace_prefix("hello world", "hello ", "")   #=> "world"
String.replace_suffix("hello world", " world", "")   #=> "hello"
```

When the pattern is a regular expression, `replacement` may reference captures with `\N` / `\g{N}` (`\0` is the whole match):

> "When the pattern is a regular expression, one can give `\N` or `\g{N}` in the `replacement` string to access a specific capture in the regular expression ... By giving `\0`, one can inject the whole match in the replacement string."
> — [String.html](https://hexdocs.pm/elixir/String.html)

```elixir
String.replace("a,b,c", ~r/,(.)/, ",\\1\\g{1}")     #=> "a,bb,cc"
```

An empty-string pattern is treated as an implicit empty position between each grapheme, so `String.replace("abc", "", "-")` intersperses: `"-a-b-c-"`.

#### Conversion

| Function | Behaviour | Failure mode |
|---|---|---|
| `String.to_integer/1` | Parse a base-10 integer literal. | `ArgumentError` |
| `String.to_integer/2` | Parse an integer in `base` (2–36). | `ArgumentError` |
| `String.to_float/1` | Parse a float literal (must contain `.` or scientific notation). | `ArgumentError` |
| `String.to_atom/1` | Convert to an atom, **creating it if absent** (atoms are never GC'd). | — |
| `String.to_existing_atom/1` | Convert to an already-existing atom. | `ArgumentError` |
| `String.to_charlist/1` | List of integer code points. | — |

```elixir
String.to_integer("123")                #=> 123
String.to_integer("ff", 16)             #=> 255
String.to_float("3.14")                 #=> 3.14
String.to_existing_atom("ok")           #=> :ok
```

Prefer `String.to_existing_atom/1` for untrusted input — `String.to_atom/1` can grow the atom table without bound.

#### Graphemes, code points, and inspection

| Function | Behaviour |
|---|---|
| `String.codepoints/1` | Code points as a list of 1-grapheme strings. |
| `String.graphemes/1` | Graphemes per Unicode Annex #29. |
| `String.chunk/2` | Split into `:valid`/`:invalid` or `:printable`/`:non-printable` runs. |
| `String.normalize/2` | Convert to `:nfc`/`:nfd`/`:nfkc`/`:nfkd`. |
| `String.printable?/2` | `true` if all chars are printable up to `character_limit`. |
| `String.valid?/1` | `true` if valid UTF-8. |

#### Similarity and diff

| Function | Added | Behaviour |
|---|---|---|
| `String.jaro_distance/2` | — | Jaro similarity in `0.0..1.0`; powers Elixir's "did you mean?". |
| `String.bag_distance/2` | v1.8 | Cheap lower-bound approximation in `0.0..1.0`. |
| `String.myers_difference/2` | v1.3 | Edit script as `[{:eq \| :ins \| :del, t()}]`. |

```elixir
String.jaro_distance("Dwayne", "Duane")  #=> 0.8222222222222223
String.myers_difference("fox hops", "fox jumps")
#=> [eq: "fox ", del: "ho", ins: "jum", eq: "ps"]
```

### Case conversion and Unicode modes

`upcase/2`, `downcase/2`, and `capitalize/2` each take an optional second argument `mode` (default `:default`):

> "`mode` may be `:default`, `:ascii`, `:greek` or `:turkic`. The `:default` mode considers all non-conditional transformations outlined in the Unicode standard. `:ascii` uppercases only the letters a to z. `:greek` includes the context sensitive mappings found in Greek. `:turkic` properly handles the letter i with the dotless variant."
> — [String.html](https://hexdocs.pm/elixir/String.html)

| Mode | Effect | When to use |
|---|---|---|
| `:default` | Full Unicode case folding (no locale). | Default; correct for general text. |
| `:ascii` | Only `a–z`/`A–Z`; everything else passes through. | ASCII-only data; faster, no Unicode tables. |
| `:greek` | Context-sensitive sigma (`Σ` → `ς` word-finally). | Greek text. |
| `:turkic` | Dotted/dotless I rules (`i`↔`İ`, `ı`↔`I`). | Turkish/Azerbaijani text. |

```elixir
String.downcase("OLÁ", :ascii)           #=> "olÁ"        # 'Á' untouched
String.downcase("ΣΣ", :greek)            #=> "σς"         # final sigma
String.upcase("ıi", :turkic)             #=> "Iİ"         # Turkish dotted/dotless I
String.downcase("Iİ", :turkic)           #=> "ıi"
```

There is **no** arbitrary-locale mode — only these four. `capitalize/2` additionally lowercases the remainder; for title-style casing without lowercasing see Erlang's `:string.titlecase/1`.

### Regex support

Regular expressions are built with the `~r` sigil (Kernel) and represented by the `%Regex{}` struct; see [Regex.html](https://hexdocs.pm/elixir/Regex.html) for the full API. Common modifiers: `i` (caseless), `u` (Unicode), `m` (multiline), `s` (dotall), `x` (extended).

```elixir
~r/foo/iu                                #=> ~r/foo/iu
```

Of the `String` functions, `String.match?/2`, `String.split/3`, and `String.replace/4` accept a `Regex`. When `split/3` is given a regex it delegates to `Regex.split/3` and accepts its options (`:parts`, `:trim`, `:on`, `:include_captures`).

```elixir
String.split("a1b2c", ~r/\d/)            #=> ["a", "b", "c"]
String.replace("a1b", ~r/\d/, "<\\0>")   #=> "a<1>b"
```

For plain (non-regex) search patterns, `String` also accepts a **compiled pattern** from `:binary.compile_pattern/1`, which avoids re-parsing the pattern on every call:

```elixir
pattern = :binary.compile_pattern([" ", "!"])
String.contains?("foo bar!", pattern)    #=> true
```

> "The compiled pattern is useful when the same match will be done over and over again. Note though that the compiled pattern cannot be stored in a module attribute as the pattern is generated at runtime and does not survive compile time."
> — [String.html](https://hexdocs.pm/elixir/String.html)

### Unicode normalization

`String.normalize/2` converts between the four Unicode normalization forms:

| Form | Name |
|---|---|
| `:nfc` | Canonical Composition (recommended default for storage) |
| `:nfd` | Canonical Decomposition |
| `:nfkc` | Compatibility Composition |
| `:nfkd` | Compatibility Decomposition |

```elixir
String.normalize("ﬁ", :nfkd)             #=> "fi"        # ligature -> two letters
String.equivalent?("man\u0303ana", "mañana")  #=> true   # "n"+combining tilde == "ñ"
```

`String.equivalent?/2` is exactly `normalize(s1, :nfd) == normalize(s2, :nfd)`.

> "Normalization forms `:nfkc` and `:nfkd` should not be blindly applied to arbitrary text. Because they erase many formatting distinctions, they will prevent round-trip conversion to and from many legacy character sets."
> — [String.html](https://hexdocs.pm/elixir/String.html)

> "If you plan to compare multiple strings, multiple times in a row, you may normalize them upfront and compare them directly to avoid multiple normalization passes."

Invalid code points are skipped silently; for strict behavior use `:unicode.characters_to_nfc_binary/1` and friends.

### String vs the `:binary` module

> "To act according to the Unicode Standard, many functions in this module run in linear time, as they need to traverse the whole string considering the proper Unicode code points. For example, `String.length/1` will take longer as the input grows. On the other hand, `Kernel.byte_size/1` always runs in constant time (i.e. regardless of the input size)."
> — [String.html](https://hexdocs.pm/elixir/String.html)

When you need raw-byte speed or are handling non-UTF-8 bytes, drop down to byte-oriented primitives:

| Layer | When to use | Examples |
|---|---|---|
| `String` | Correct Unicode text handling (graphemes, case, normalization). | `String.length/1`, `String.upcase/1` |
| `Kernel` binary ops | O(1) byte access/slicing on known-valid binaries. | `byte_size/1`, `binary_part/3`, `binary_slice/2,3` |
| `:binary` (Erlang) | Highly optimized byte search/split/replace; non-UTF-8 data. | `:binary.match/2`, `:binary.split/2`, `:binary.replace/4` |
| Binary pattern matching | Destructuring bytes/code points. | `<<x::utf8, rest::binary>>` |

> "Library for handling binary data. This module provides functions for manipulating byte-oriented binaries. While most of these functions could be implemented using the bit syntax, the functions in this module are highly optimized ..."
> — [`:binary`](https://www.erlang.org/doc/man/binary.html)

Most useful `:binary` functions: `at/2`, `part/2,3`, `first/1`, `last/1`, `match/2,3`, `matches/2,3`, `split/2,3`, `replace/3,4`, `compile_pattern/1`, `copy/2`, `encode_hex/1,2`, `decode_hex/1`, `longest_common_prefix/1`. Note `:binary.replace/4` exposes options (`{scope, ...}`, `{insert_replaced, ...}`) that `String.replace/4` does not.

### Performance characteristics

- **O(1)** (byte-level, stored with the binary): `byte_size/1`, `bit_size/1`, `is_binary/1`, `binary_part/3`, `Kernel.binary_slice/2,3`, `:binary.at/2`, `:binary.part/2,3`.
- **O(n)** (grapheme walk — the bulk of `String`): `length/1`, `graphemes/1`, `codepoints/1`, `last/1`, `at/2`, `split_at/2`, `slice/2,3`, `reverse/1`, `upcase/downcase/capitalize`, `replace/4`, `split/3`, `contains?/2`, `normalize/2`. (`first/1` is O(1).)
- `starts_with?/2` / `ends_with?/2` short-circuit on mismatch.
- `String.duplicate/2` is inlined by the compiler.

Rules of thumb: reach for `byte_size/1` instead of `String.length/1` when you only need a byte bound; use the `:ascii` mode for known-ASCII case folding; compile search patterns when reused; and avoid chaining several full-string grapheme walks.

### Complexity reference table

| Function | Complexity |
|---|---|
| `byte_size/1` (Kernel) | **O(1)** |
| `binary_part/3` (Kernel) | **O(1)** |
| `binary_slice/2,3` (Kernel) | **O(1)** |
| `String.length/1` | O(n) graphemes |
| `String.codepoints/1` | O(n) |
| `String.graphemes/1` | O(n) |
| `String.first/1` | O(1) |
| `String.last/1` | O(n) |
| `String.at/2` | O(n) |
| `String.slice/2,3` | O(n) |
| `String.split_at/2` | O(n) |
| `String.split/1,3` | O(n) |
| `String.replace/4` | O(n) |
| `String.contains?/2` | O(n) |
| `String.starts_with?/2` / `ends_with?/2` | fast (short-circuit) |
| `String.upcase/2` / `downcase/2` / `capitalize/2` | O(n) |
| `String.reverse/1` | O(n) |
| `String.normalize/2` | O(n) |
| `String.duplicate/2` | O(n), inlined |

### Common mistakes

- **Pattern matching matches bytes, not characters.** `<<head, rest::binary>> = "über"` binds `head` to the first *byte* (`0xC3`), not the letter. Use `<<x::utf8, rest::binary>>` to bind a code point, or `String.next_grapheme/1` for a grapheme.
- **Confusing the three units.** `String.length("é") == 1` (grapheme), `byte_size("é") == 2` (bytes). And `"é"` written as `e` + combining accent has *two* code points but still one grapheme.
- **Using `to_atom/1` on untrusted input.** Atoms are never garbage-collected; prefer `to_existing_atom/1`.
- **Expecting `String.replace_leading/3` to work with `""`.** It (and `replace_trailing/3`) raise `ArgumentError`.
- **Expecting `String.split(s, "")` to return graphemes.** It returns `["", "a", "b", "c", ""]` — interspersed with empties.
- **Treating every `binary` as valid UTF-8.** A `binary` can hold invalid bytes; validate untrusted input with `String.valid?/1` before grapheme work.
- **`String.at(s, -1)` vs `binary_slice(s, -1, 1)`.** The former returns the last *grapheme*; the latter the last *byte*.

### Does not exist

- `String.join/2` — use `Enum.join/2`.
- `String.compare/2` — use `Kernel.compare/2` or the relational operators.
- `String.distance?/2` / Levenshtein — not provided; `String.jaro_distance/2` is the closest similarity metric.
- `String.to_string/1` — this is `Kernel.to_string/1` (the `String.Chars` protocol), not a `String` module function.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `String.myers_difference/2` | v1.3 | Edit script between two strings. |
| `String.bag_distance/2` | v1.8 | Cheap similarity approximation. |
| `String.replace_invalid/2` | v1.16 | Replace invalid UTF-8 bytes with `"�"`. |
| `String.byte_slice/3` | v1.17 | Byte-bounded substring. |
| `String.count/2` | v1.19 | Count non-overlapping pattern matches. |
| `Regex` `:export` (E) modifier | v1.19.3 | Exported pattern. |

The Unicode tables track **The Unicode Standard, Version 17.0.0** as of Elixir v1.20.x.

Sources: [String.html](https://hexdocs.pm/elixir/String.html), [binaries-strings-and-charlists.html](https://hexdocs.pm/elixir/binaries-strings-and-charlists.html), [Regex.html](https://hexdocs.pm/elixir/Regex.html), [Kernel.html](https://hexdocs.pm/elixir/Kernel.html), [`:binary`](https://www.erlang.org/doc/man/binary.html), [elixir/lib/string.ex (v1.20.2)](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/string.ex).

## URI

### Overview & evaluation model

From [URI.html](https://hexdocs.pm/elixir/URI.html):

> "Utilities for working with URIs. This module provides functions for working with URIs (for example, parsing URIs or encoding query strings). The functions in this module are implemented according to RFC 3986 and it also provides additional functionality for handling 'application/x-www-form-urlencoded' segments. Additionally, the Erlang `:uri_string` module provides additional functionality such as RFC 3986 compliant URI normalization."

`URI` parses URIs into a `%URI{}` struct whose fields map to the RFC 3986 grammar `[scheme]://[userinfo]@[host]:[port][path]?[query]#[fragment]`. The struct is defined as:

```elixir
defstruct [:scheme, :authority, :userinfo, :host, :port, :path, :query, :fragment]
```

Two important properties shape how the module behaves. First, the struct fields store the **encoded** URI components — when you set a field directly you are responsible for percent-encoding it. Second, `URI` implements **RFC 3986**, not the WHATWG URL Standard that browsers follow; this means `URI` rejects URLs that browsers happily accept (e.g. unencoded `[]` in a query, or backslashes used as path separators). When you need browser-compatible parsing, reach for a third-party library rather than fighting `URI`.

The `authority` field is **deprecated**. `URI.parse/1` still populates it for backwards compatibility, but new code should read `host` and `port` instead.

### Key functions

#### Parsing

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `URI.parse/1` | Parses a URI into a `%URI{}` struct without validation. | Accepts absolute and relative URIs; normalizes scheme to lowercase; fills `port` from `default_port/1` when absent; returns the struct unmodified if given a struct. |
| `URI.new/1` | Parses and validates a URI. | Returns `{:ok, uri} \| {:error, part}`; delegates to Erlang `:uri_string.parse/1`; rejects URLs browsers accept. Since v1.13.0. |
| `URI.new!/1` | Parses and validates, raising on failure. | Raises `URI.Error`. Since v1.13.0. |

```elixir
URI.parse("https://elixir-lang.org/")
#=> %URI{authority: "elixir-lang.org", host: "elixir-lang.org", path: "/",
#=>      port: 443, scheme: "https", ...}

URI.new("/invalid_greater_than_in_path/>")
#=> {:error, ">"}
```

`parse/1` is permissive (it never raises on malformed input); `new/1` is the validating counterpart. Prefer `new/1` whenever the input is untrusted or must be a well-formed RFC 3986 URI.

#### Encoding and decoding

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `URI.encode/1, /2` | Percent-encodes characters requiring escaping. | Predicate arg defaults to `URI.char_unescaped?/1`; alternatives `URI.char_unreserved?/1` (for query/fragment components) and `URI.char_reserved?/1`. |
| `URI.decode/1` | Percent-unescapes a string. | Does NOT treat `+` as space. |
| `URI.encode_www_form/1` | Encodes as `application/x-www-form-urlencoded`. | `%20` becomes `+`. |
| `URI.decode_www_form/1` | Decodes `application/x-www-form-urlencoded`. | Decodes `+` as space. |

```elixir
URI.encode("hello world")          #=> "hello%20world"
URI.decode_www_form("a+b")         #=> "a b"
```

The split between `encode/1` (RFC 3986, `%20`) and `encode_www_form/1` (form data, `+`) is the source of most encoding bugs. Use `encode_www_form/1` only for HTML form field values.

#### Query handling

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `URI.encode_query/1, /2` | Encodes an enumerable of 2-tuples into `"k1=v1&k2=v2"`. | Encoding `:www_form` (default since v1.12) or `:rfc3986` (since v1.12); values cannot be lists. |
| `URI.decode_query/1, /2, /3` | Decodes a query string into a map. | `:www_form` (default) decodes `+` as space; `:rfc3986` leaves `+` literal. |
| `URI.query_decoder/1, /2` | Returns a stream of `{key, value}` tuples. | Use to preserve duplicate keys (a map would collapse them). |

```elixir
URI.encode_query(%{"key" => "value with spaces"})
#=> "key=value+with+spaces"

URI.encode_query(%{"key" => "value with spaces"}, :rfc3986)
#=> "key=value%20with+spaces"

URI.decode_query("foo=1&bar=2")
#=> %{"bar" => "2", "foo" => "1"}
```

#### Default ports

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `URI.default_port/1` | Returns the default port for a scheme, or `nil`. | Known: ftp→21, http→80, https→443. |
| `URI.default_port/2` | Registers a default port globally. | Call from an application start callback; affects `parse/1` and `to_string/1`. |

#### Manipulation

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `URI.append_path/2` | Appends a path to a URI's path. | Path must start with `/` and not `//`; raises `ArgumentError` otherwise. Since v1.15.0. |
| `URI.append_query/2` | Appends a query (not auto-encoded). | Joins with `&` if an existing query is present. Since v1.14.0. |
| `URI.merge/2` | Merges a URI with a reference per RFC 3986. | Resolves relative references against a base URI. |
| `URI.to_string/1` | Assembles a `%URI{}` back into a string. | Implements `String.Chars`; omits port if it equals the scheme default; wraps IPv6 hosts in `[...]`. |

```elixir
URI.append_path(URI.parse("http://example.com/foo/?x=1"), "/my-path") |> URI.to_string()
#=> "http://example.com/foo/my-path?x=1"
```

### Common mistakes

- **`URI.parse/1` does not validate.** It accepts invalid characters that `new/1` rejects. Use `new/1` when you need validation of untrusted input.
- **`URI` is RFC 3986, not WHATWG.** Browsers accept URLs this module rejects (unencoded `[]` in a query, backslashes as path separators). Do not expect browser-equivalent parsing.
- **`URI.decode/1` does not decode `+` as space.** Use `decode_www_form/1` for form-encoded data.
- **The `authority` struct field is deprecated.** Use `host` and `port` instead.
- **Struct fields store encoded components.** When setting fields directly you must percent-encode them yourself; otherwise `to_string/1` will assemble an invalid URI.

### Does not exist

- `URI.template/1` (RFC 6570 URI templates) — not in the standard library; use a third-party Hex package.
- `URI.append_segment/2` — does not exist; use `append_path/2` or `merge/2`.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `URI.new/1`, `URI.new!/1` | v1.13.0 | Validating parser (RFC 3986). |
| `URI.encode_query/2` `:rfc3986` mode | v1.12.0 | Encodes space as `%20` instead of `+`. |
| `URI.decode_query/3` encoding opt | v1.12.0 | `:rfc3986` leaves `+` literal. |
| `URI.append_query/2` | v1.14.0 | Append to an existing query with `&`. |
| `URI.append_path/2` | v1.15.0 | Append a path segment. |
| `authority` field deprecation | — | Avoid; use `host`/`port`. |

Sources: [URI.html](https://hexdocs.pm/elixir/URI.html), [:uri_string](https://www.erlang.org/doc/apps/stdlib/uri_string.html), [RFC 3986](https://tools.ietf.org/html/rfc3986).

## Path

### Overview & evaluation model

From [Path.html](https://hexdocs.pm/elixir/Path.html):

> "This module provides conveniences for manipulating or retrieving file system paths. The functions in this module may receive chardata as arguments and will always return a string encoded in UTF-8. ... The majority of the functions in this module do not interact with the file system, except for a few functions that require it (like `wildcard/2` and `expand/1`)."

`Path` is a pure string-manipulation module for the most part: `join/2`, `split/1`, `dirname/1`, `basename/1` and friends operate on chardata and never touch the disk. The exceptions are `expand/1` (which resolves the current working directory via `File.cwd!/0`) and `wildcard/2` (which traverses the filesystem). Because path separators are OS-dependent, `Path` functions use the current operating system's conventions — code that hard-codes `/` is not portable to Windows, and conversely `wildcard/2` requires `/` even on Windows.

There is exactly one predicate, `Path.type/1`, which classifies a path as `:absolute`, `:relative`, or `:volumerelative` (the last is Windows-only, e.g. `D:bar.ex`). There is no `Path.absolute?/1` or `Path.relative?/1`.

### Key functions

#### Joining and splitting

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Path.join/1` | Joins a list of path segments. | Trailing slash removed; raises on empty list; an absolute path in the list contributes its root and subsequent elements are appended (made relative). |
| `Path.join/2` | Joins two paths. | Right path is expanded to relative form (leading `/` stripped) and appended; a chardata list arg is treated as a single value, NOT a list of paths. |
| `Path.split/1` | Splits a path into a list at the separator. | On Windows splits on both `\` and `/` and lowercases the drive letter. |

```elixir
Path.join(["~", "foo"])           #=> "~/foo"
Path.join(["/", "foo", "bar/"])   #=> "/foo/bar"
Path.join("foo", "/bar/")         #=> "foo/bar"
Path.split("/foo/bar")             #=> ["/", "foo", "bar"]
```

#### Components

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Path.dirname/1` | Returns the directory component. | Returns `.` for a bare filename. |
| `Path.basename/1` | Returns the last path component. | — |
| `Path.basename/2` | Returns the last component with the given extension stripped (only if present). | — |
| `Path.extname/1` | Returns the extension of the last component. | Returns `""` for dotfiles like `.gitignore` (since Erlang/OTP 24). |
| `Path.rootname/1` | Returns the path with the extension stripped. | — |
| `Path.rootname/2` | Strips a specific extension if present. | — |

```elixir
Path.dirname("/foo/bar.ex")               #=> "/foo"
Path.basename("~/foo/bar.ex", ".ex")      #=> "bar"
Path.extname(".gitignore")                #=> ""
```

#### Expanding and relativizing

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Path.expand/1` | Converts to an absolute path, expanding `.`/`..` and a leading `~`. | `~` expands to the home dir via `System.user_home!/0`; relative paths expand against `File.cwd!/0`. |
| `Path.expand/2` | Expands relative to a second argument. | Treats `~`-leading paths as absolute (ignores `relative_to`); only `~` and `~/...` are expanded, `~user` is left literal. |
| `Path.relative/1` | Forces a path to be relative (strips the root). | Does NOT collapse `.`/`..`. |
| `Path.relative_to/3` | Returns the path of `path` relative to `cwd`. | Pure string manipulation; assumes no symlinks. Arity bumped from `/2` in v1.16 with a `:force` option. |
| `Path.relative_to_cwd/2` | Relative path from the current working directory. | Arity bumped from `/1` in v1.16. |

```elixir
Path.expand("/foo/bar/../baz")   #=> "/foo/baz"
Path.relative("/usr/local/bin")  #=> "usr/local/bin"
Path.relative_to("/usr/local/foo", "/usr/local")  #=> "foo"
```

#### Predicates and wildcard

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Path.type/1` | Returns `:absolute`, `:relative`, or `:volumerelative`. | The ONLY path predicate; `:volumerelative` is Windows-only. |
| `Path.wildcard/1, /2` | Traverses the filesystem per a glob. | `?`=one char, `*`=chars up to end/dot/slash, `**`=recursive dirs, `[a,b]`=char class, `{a,b}`=alternatives; case-sensitive; directory separators MUST be `/` even on Windows; `:match_dot` option (default false) controls dotfile matching. |

```elixir
Path.type("/")            #=> :absolute
Path.type("D:bar.ex")     #=> :volumerelative  # Windows only
Path.wildcard("projects/*/ebin/**/*.beam")
```

### Common mistakes

- **Path functions use the current OS separators.** Code is not portable across OSes; `Path.wildcard` requires `/` even on Windows.
- **`Path.expand/1` touches the filesystem** (via `File.cwd!/0`) — it can fail if the cwd is unavailable.
- **`Path.expand/2` treats `~user/...` as absolute but does not expand it.** Only `~` and `~/...` are expanded.
- **`Path.relative/1` does not collapse `.`/`..`.** The behaviour would be ambiguous without filesystem access.
- **`Path.relative_to/3` does pure string manipulation** — it assumes no symlinks. Use `Path.safe_relative/2` (v1.14) for traversal-safe paths.
- **Trailing slashes are stripped by `Path.join`.**
- **`Path.join/1` raises on an empty list; `Path.join/2` treats a chardata list as a single value** (not a list of paths).
- **`:volumerelative` is Windows-only.**
- **`Path.extname(".gitignore")` returns `""`** (since OTP 24) — dotfiles are not treated as having an extension.

### Does not exist

- `Path.absolute?/1` and `Path.relative?/1` — do NOT exist in any version. Use `Path.type(path) == :absolute` or `Path.type(path) in [:absolute, :volumerelative]`.
- `:cwd` option on `Path.expand` — does not exist; the cwd is the positional second argument to `Path.expand/2`.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `Path.safe_relative/2` | v1.14.0 | Traversal-safe relative paths. |
| `Path.relative_to/3` `:force` option | v1.16.0 | Arity bumped from `/2`. |
| `Path.relative_to_cwd/2` | v1.16.0 | Arity bumped from `/1`. |
| `Path.extname/1` dotfile behaviour | Erlang/OTP 24 | `.gitignore` now returns `""`. |

Sources: [Path.html](https://hexdocs.pm/elixir/Path.html), [elixir/lib/path.ex (v1.20.2)](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/path.ex).

## File

### Overview & evaluation model

From [File.html](https://hexdocs.pm/elixir/File.html):

> "This module contains functions to manipulate files. Some of those functions are low-level, allowing the user to interact with files or IO devices, like `open/2`, `copy/3` and others. This module also provides higher level functions that work with filenames and have their naming based on Unix variants. For example, one can copy a file via `cp/3` and remove files and directories recursively via `rm_rf/1`. Paths given to functions in this module can be either relative to the current working directory (as returned by `File.cwd/0`), or absolute paths. Shell conventions like `~` are not expanded automatically. To use paths like `~/Downloads`, you can use `Path.expand/1` or `Path.expand/2` to expand your path to an absolute path."

> "Most of the functions in this module return `:ok` or `{:ok, result}` in case of success, `{:error, reason}` otherwise. Those functions also have a variant that ends with `!` which returns the result (instead of the `{:ok, result}` tuple) in case of success or raises an exception in case it fails."

`File` splits into two layers. The low-level layer (`open/2`, `read/1`, `write/2`, `copy/3`) wraps Erlang's `:file` module and deals with IO devices and raw file descriptors. The high-level layer (`cp/3`, `rm_rf/1`, `mkdir_p/1`, `ls/1`) provides Unix-flavoured filename operations. The bang convention is consistent: the tuple form lets you react to missing files, while the bang form crashes on failure — choose based on whether absence is expected.

Paths are relative to the BEAM-global current working directory; `~` is never expanded automatically. Charlist filenames are treated as UTF-8; binary filenames are passed raw to the OS.

### Key functions

#### Reading and writing

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `File.read/1` | Reads an entire file into a binary. | Returns `{:ok, binary} \| {:error, reason}`; `:raw` opt since v1.20. |
| `File.read!/1` | Reads, raising on failure. | Raises `File.Error`. |
| `File.write/2, /3` | Writes iodata to a file. | Creates the file if absent, OVERWRITES if present; `:encoding` has NO effect; spawns a process per call (use `open/3` + `IO` for loops). |
| `File.write!/3` | Writes, raising on failure. | Raises `File.Error`. |

```elixir
File.read("hello.txt")            #=> {:ok, "world"}
File.read!("missing.txt")         #=> ** (File.Error) ... no such file or directory
File.write("hello.txt", "world!")  #=> :ok
```

#### Existence and directories

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `File.exists?/1, /2` | Whether a path exists. | Returns `false` for dangling symlinks; RACY; `:raw` opt since v1.20. |
| `File.mkdir/1` | Creates a directory. | Does not create parents; `:ok \| {:error, ...}`. |
| `File.mkdir_p/1` | Creates a directory and all parents. | — |
| `File.rmdir/1` | Removes an empty directory. | Returns `{:error, :eexist}` if not empty. |
| `File.ls/1` | Lists directory entries. | Returns `{:ok, [binary]}`; hidden files included, NOT sorted. |

```elixir
File.exists?("test/")                  #=> true
File.mkdir_p("non/existing/parents")    #=> :ok
File.rmdir("non_empty_dir")            #=> {:error, :eexist}
File.ls("bin")                          #=> {:ok, ["iex", "elixir"]}
```

#### Copy, move, and delete

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `File.cp/3` | Copies contents and modes. | Source must be a file/symlink; dest must be a non-existent file path (returns `:eisdir` if dest is a dir, unlike Unix `cp`); `:on_conflict` callback since v1.14. |
| `File.cp_r/3` | Recursive copy preserving file modes. | Dir modes only with `:preserve_directory_permissions` (v1.20); `:on_conflict`/`:dereference_symlinks` since v1.14; returns `{:ok, [files]}`; on failure leaves destination DIRTY; returns `{:error, :einval, path}` if dest is inside source. |
| `File.rm/1` | Deletes a file. | Does not delete dirs (`:eperm` if dir). |
| `File.rm_rf/1` | Recursive delete. | Symlinks not followed; non-existing ignored; EXTREMELY DANGEROUS. |
| `File.rename/2` | Moves/renames files or directories. | Since v1.1; rejects Unix `mv` "into existing directory" behaviour. |

```elixir
File.cp("hello.txt", "copy.txt")   #=> :ok
File.cp_r("samples", "tmp")       #=> {:ok, ["z.txt", "y.txt"]}
File.rm_rf("unknown")             #=> {:ok, []}
File.rename("a.txt", "b.txt")     #=> :ok
```

#### Streams and IO devices

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `File.stream!/2, /3` | Returns a `File.Stream` over a file. | Arg order `(path, line_or_bytes, modes)` since v1.16; `:line` (default, CRLF→LF) or `pos_integer` bytes; auto-opens with `:raw` + `:read_ahead` if no encoding; `:trim_bom` (v1.16), `:read_offset` (v1.16). |
| `File.open/2, /3` | Opens a path with modes or a function (auto-closes). | — |
| `File.open!/2, /3` | Opens, raising on failure. | — |
| `File.close/1` | Closes an IO device. | — |

```elixir
File.stream!("./test.txt", [:trim_bom, encoding: :utf8])

File.open("file.txt", [:read, :write], fn f -> IO.read(f, :line) end)
#=> {:ok, "file content"}
```

`File.Stream` implements both `Enumerable` and `Collectable`, so the same stream can be read from and written to.

#### Touch and stat

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `File.touch/2` | Updates mtime/atime, creating the file if absent. | May require root/owner to update an existing file's mtime. |
| `File.stat/2` | Returns `{:ok, %File.Stat{}}`. | `{:time, :universal\|:local\|:posix}` (default `:universal`). |
| `File.stat!/2` | Stat, raising on failure. | — |
| `File.lstat/2` | Like `stat/2` but reports symlinks as `:symlink` (does not follow). | — |
| `File.lstat!/2` | Lstat, raising on failure. | — |
| `File.write_stat/3` | Writes a `File.Stat` back to the file (round-trip). | — |

```elixir
File.stat("hello.txt")   #=> {:ok, %File.Stat{...}}
File.lstat("link")       #=> {:ok, %File.Stat{type: :symlink, ...}}
```

#### File modes (for `File.open`)

| Mode | Behaviour |
|---|---|
| `:read` | Open for reading. |
| `:write` | Truncates unless `:read` is also given. |
| `:append` | Append writes. |
| `:exclusive` | Errors `:eexist` if the file exists. |
| `:charlist` | Return charlists instead of binaries. |
| `:compressed` | Gzip; combine with `:read` OR `:write`, not both. |
| `:utf8` | Auto UTF-8 conversion; use `IO.read`/`IO.write`. |
| `:sync` | Sync every write. |
| `:delayed_write` | Buffer writes; errors surface on close. |
| `:raw` | Bypass the IO process; returns `:file.fd`. |
| `:read_ahead` | Read-ahead buffering. |
| `:ram` | In-memory file. |

The default is `:binary`, which requires `IO.binread/2`/`IO.binwrite/2`. Pass `:utf8` to use `IO.read/2`/`IO.write/2` (slower, Unicode-aware).

#### `File.Stat` struct

Returned by `stat/2` and `lstat/2`. Notable fields:

| Field | Meaning |
|---|---|
| `size` | Size in bytes. |
| `type` | `:device \| :directory \| :regular \| :other \| :symlink`. |
| `access` | `:read \| :write \| :read_write \| :none`. |
| `atime` / `mtime` / `ctime` | Access / modification / inode-change (Unix) or creation (Windows) time. |
| `mode` | Permissions. |
| `links` | Link count. |
| `major_device` / `minor_device` | Device identifiers. |
| `inode` | Inode number. |
| `uid` / `gid` | Owner / group IDs. |

### Common mistakes

- **`File.rm_rf/1` is destructive with no trash/undo.** Validate paths first.
- **`File.cp_r/3` failures leave the destination in a dirty half-copied state.**
- **`File.cp/3` and `File.cp_r/3` reject file→directory copy (`:eisdir`)**, unlike Unix `cp`.
- **`File.exists?/1` is racy** — prefer matching on the operation's return value.
- **`File.exists?/1` returns `false` for dangling symlinks** — use `File.lstat/2` to distinguish.
- **`File.write/3` spawns a process per call** — use `File.open/3` for loops.
- **`File.write/3` `:encoding` has no effect** — content must be iodata.
- **Default `:binary` mode requires `IO.binread`/`IO.binwrite`; `:utf8` requires `IO.read`/`IO.write`.**
- **`:delayed_write` defers errors to `close/1`** — not recommended with `File.open/3` (which assumes close success).
- **`File.rm/1` doesn't delete dirs** — use `rmdir/1` (empty) or `rm_rf/1`.
- **`File.mkdir/1` doesn't create parents** — use `mkdir_p/1`.
- **`File.touch/2` may require root/owner** to update an existing file's mtime.
- **`File.cd/1` changes the BEAM-global cwd** — racy across processes.

### Does not exist

- `File.append/2` — use `File.write(path, content, [:append])` or open with `:append`.
- `File.read_stat/1` — use `File.stat/2` / `File.lstat/2`.
- `File.exists?` bang variant — existence is boolean by design.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `File.rename/2` | v1.1.0 | Move/rename. |
| `File.ln/2`, `File.ln_s/2`, `File.read_link/1` (+ bang) | v1.5.0 | Symlinks. |
| `File.rename!/2` | v1.9.0 | Bang variant. |
| `File.cp/3` `:on_conflict`, `File.cp_r/3` `:on_conflict`/`:dereference_symlinks` | v1.14.0 | Conflict callbacks. |
| `File.stream!/3` arg order `(path, line_or_bytes, modes)` | v1.16.0 | Old `(path, modes, line_or_bytes)` deprecated v1.20. `:read_offset`, `:trim_bom` added. |
| `File.read/2` `:raw` opt, `File.exists?/2` `:raw`, `File.cp_r/3` `:preserve_directory_permissions` | v1.20.0 | Raw bypass; dir perms. |

Sources: [File.html](https://hexdocs.pm/elixir/File.html), [File.Stat.html](https://hexdocs.pm/elixir/File.Stat.html), [File.Stream.html](https://hexdocs.pm/elixir/File.Stream.html), [:file (Erlang)](https://www.erlang.org/doc/apps/kernel/file.html).

## Code

### Overview & evaluation model

From [Code.html](https://hexdocs.pm/elixir/Code.html):

> "Utilities for managing code compilation, code evaluation, and code loading. This module complements Erlang's `:code` module to add behavior which is specific to Elixir. For functions to manipulate Elixir's AST (rather than evaluating it), see the `Macro` module."

`Code` is the bridge between source text and the running VM. It parses strings into quoted AST, evaluates or compiles that AST into loaded modules, formats source code, and manages compiler options and load paths. The central security caveat is that every evaluation/compilation function (`eval_string/1`, `compile_string/1`, `eval_file/2`) runs code with full VM privileges — never feed untrusted input to these functions.

Three functions work with files and differ in tracking: `require_file/2` compiles and tracks the filename (idempotent — it will not recompile an already-required file), `compile_file/2` compiles without tracking (recompiles every call, useful when you want fresh modules), and `eval_file/2` evaluates the file's contents and returns the last expression's result without triggering compilation tracers.

### Key functions

#### Evaluation

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.eval_string/1, /2, /3` | Evaluates a string, returning `{value, binding}`. | Raises on error; binding is a keyword list of vars. |
| `Code.eval_quoted/3` | Evaluates a quoted AST. | WARNING: calling inside a macro is bad practice (evaluates runtime values at compile time). |

```elixir
{result, binding} = Code.eval_string("a + b", [a: 1, b: 2], __ENV__)
#=> {3, [a: 1, b: 2]}
```

#### Compilation

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.compile_string/1, /2` | Compiles a string into `[{module, bytecode}]`. | SECURITY: code runs with VM privileges — don't use with untrusted input. |
| `Code.compile_quoted/1, /2` | Compiles a quoted AST. | — |
| `Code.compile_file/2` | Compiles a file without tracking. | Recompiles each call. Since v1.7. |
| `Code.require_file/2` | Compiles and tracks a file. | Idempotent. Since v1.7. |
| `Code.required_files/0` | Lists tracked files. | Since v1.7. |
| `Code.unrequire_files/1` | Removes files from tracking. | Since v1.7. |

```elixir
Code.compile_string("defmodule Hello do\n  def world, do: :ok\nend")
#=> [{Hello, <<bytecode>>}]

Code.require_file("../eex/test/eex_test.exs")
```

#### Parsing

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.string_to_quoted/1, /2` | Parses a string into AST. | Returns `{:ok, ast} \| {:error, {meta, msg, token}}`. |
| `Code.string_to_quoted!/1, /2` | Parses, raising on failure. | Raises `TokenMissingError`/`MismatchedDelimiterError`/`SyntaxError`. |
| `Code.string_to_quoted_with_comments/1, /2` | Parses preserving comments. | Used by the formatter. |

```elixir
Code.string_to_quoted("1 + 3")
#=> {:ok, {:+, [line: 1], [1, 3]}}

Code.string_to_quoted!("1 + 3")
#=> {:+, [line: 1], [1, 3]}
```

Parser options for `string_to_quoted` include `:file`, `:line`, `:column` (v1.11), `:columns` (attach `:column` metadata), `:unescape` (v1.10, default true), `:existing_atoms_only`, `:token_metadata` (v1.10), `:literal_encoder` (v1.10), `:static_atoms_encoder`, `:emit_warnings` (v1.16), and `:indentation` (v1.19).

#### Formatting

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.format_string!/1, /2` | Formats Elixir source into iodata. | RAISES on invalid syntax (no non-bang variant); since v1.6. |
| `Code.format_file!/1, /2` | Formats a file. | Since v1.6. |

Options for `format_string!/2`: `:line_length` (default 98), `:file`, `:locals_without_parens`, `:force_do_end_blocks` (v1.9), `:migrate` (v1.18) plus `:migrate_bitstring_modifiers`/`:migrate_charlists_as_sigils`/`:migrate_unless` (v1.18), and `:migrate_call_parens_on_pipe` (v1.19).

```elixir
Code.format_string!("foo   bar") |> IO.iodata_to_binary()
#=> "foo bar\n"
```

#### Compiler options

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.put_compiler_option/2` | Sets a compiler option. | Options are VM-WIDE (affect all processes). Since v1.10. |
| `Code.get_compiler_option/1` | Reads a compiler option. | Since v1.10. |
| `Code.compiler_options/0, /1` | Reads or sets multiple options. | — |

Key compiler options: `:docs`, `:debug_info`, `:ignore_module_conflict`, `:relative_paths`, `:tracers`, `:parser_options` (default `[columns: true]`, v1.10), `:no_warn_undefined` (v1.10), `:on_undefined_variable` (`:raise`|`:warn`, v1.15), `:ignore_already_consolidated` (v1.10), `:infer_signatures` (v1.18), `:module_definition` (`:compiled`|`:interpreted`, v1.20).

```elixir
Code.get_compiler_option(:debug_info)   #=> true
Code.put_compiler_option(:docs, false)  #=> :ok
```

#### Paths

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.prepend_path/2` | Prepends a path to the code path. | Requires the path to exist (returns `false` otherwise). |
| `Code.append_path/2` | Appends a path to the code path. | Requires the path to exist. |
| `Code.delete_path/1` | Removes a path. | — |
| `Code.prepend_paths/2` / `append_paths/2` / `delete_paths/1` | Bulk path operations. | Since v1.15. |

#### Module loading

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.ensure_loaded/1` | Loads a module from `.beam` at runtime. | Returns `{:module, _} \| {:error, reason}`. |
| `Code.ensure_loaded!/1` | Loads, raising on failure. | Since v1.12. |
| `Code.ensure_loaded?/1` | Loads, returning a boolean. | — |
| `Code.ensure_all_loaded/1` | Loads a list of modules. | Since v1.15. |
| `Code.ensure_compiled/1` | Ensures a module is compiled. | May return `{:error, :unavailable}` (halts compilation); for same-project modules. |
| `Code.ensure_compiled!/1` | Ensures compilation, raising on failure. | Since v1.12; prefer this over `ensure_compiled/1`. |
| `Code.loaded?/1` | Whether a module is loaded. | Since v1.15. |
| `Code.can_await_module_compilation?/0` | Whether module compilation can be awaited. | Since v1.11. |

```elixir
Code.ensure_loaded(Atom)  #=> {:module, Atom}
```

#### Docs and typespecs

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `Code.fetch_docs/1` | Returns the EEP 48 docs chunk or `{:error, reason}`. | Since v1.7. |
| `Code.Typespec.fetch_types/1` | Fetches a module's types. | Internal API (`@moduledoc false`). |
| `Code.Typespec.fetch_specs/1` | Fetches a module's specs. | Internal API. |
| `Code.Typespec.fetch_callbacks/1` | Fetches a module's callbacks. | Internal API. |

```elixir
Code.fetch_docs(Atom)
#=> {:docs_v1, _, :elixir, _, %{"en" => _}, _, _}
```

### Common mistakes

- **`eval_string`/`compile_string` run code with VM privileges** — never use with untrusted input.
- **`require_file` is idempotent** — it won't recompile on repeat calls; use `unrequire_files/1` first or use `compile_file/2`.
- **`format_string!` RAISES on invalid syntax** (there is no non-bang variant) and returns iodata (wrap with `IO.iodata_to_binary/1`).
- **`eval_quoted` inside a macro is bad practice** — transform via unquoting instead.
- **Don't use `ensure_compiled/1` as `case ... {:error, _} -> raise`** — use `ensure_compiled!/1`.
- **`ensure_compiled!` overuse can deadlock** (two modules waiting on each other).
- **`ensure_compiled!` only applies to same-project modules** — deps are always compiled upfront.
- **Compiler options are VM-wide**, not process-local.
- **`:force_do_end_blocks` is convergent** — once on, all keywords become `do`/`end`.
- **Migration options (`:migrate*`) CHANGE the AST** — a risk for metaprogramming-heavy code.
- **`prepend_path`/`append_path` require the path to exist** (return `false` otherwise).

### Does not exist

- `Code.load_file/1` — use Erlang's `:code.load_file/1` for `.beam` loading.
- `Code.paths/0` — use Erlang's `:code.get_path/0`.
- `Code.fetch_types/1` (public) — use `Code.Typespec.fetch_types/1` (internal, `@moduledoc false`).
- `:rename_deprecated_at` format option — not in v1.20.2.
- `:on_undefined_module` compiler option — not visible in v1.20.2 (closest is `:on_undefined_variable`, v1.15).

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `Code.format_string!/2`, `Code.format_file!/2` | v1.6.0 | Code formatter. |
| `Code.compile_file/2`, `Code.require_file/2`, `Code.required_files/0`, `Code.fetch_docs/1` | v1.7.0 | File compilation API. |
| `Code.put_compiler_option/2`, `Code.get_compiler_option/1`, `:no_warn_undefined`, `:parser_options` | v1.10.0 | Compiler options API. |
| `:token_metadata`, `:literal_encoder`, `:unescape` parser opts | v1.10.0 | AST metadata. |
| `Code.ensure_loaded!/1`, `Code.ensure_compiled!/1` | v1.12.0 | Bang variants. |
| `Code.env_for_eval/1`, `Code.eval_quoted_with_env/4` | v1.14.0 | Eval env API. |
| `Code.prepend_paths/2`, `Code.ensure_all_loaded/1`, `Code.loaded?/1`, `:on_undefined_variable` | v1.15.0 | Bulk paths; undefined var handling. |
| `:emit_warnings` parser opt, `File.stream!` arg order | v1.16.0 | Parser warnings. |
| `:migrate`, `:migrate_bitstring_modifiers`, `:migrate_charlists_as_sigils`, `:migrate_unless`, `:infer_signatures` | v1.18.0 | Formatter migrations. |
| `:indentation` parser opt, `:migrate_call_parens_on_pipe` | v1.19.0 | Embedded parsing; pipe parens. |
| `:module_definition` (`:compiled`|`:interpreted`), `:dbg_callback` eval opt | v1.20.0 | Interpreted defmodule. |

Sources: [Code.html](https://hexdocs.pm/elixir/Code.html), [Code.Typespec (source)](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/code/typespec.ex), [:code (Erlang)](https://www.erlang.org/doc/apps/kernel/code.html), [EEP 48](https://www.erlang.org/eeps/eep-0048.html).

## Kernel

### Overview & evaluation model

From [Kernel.html](https://hexdocs.pm/elixir/Kernel.html):

> "`Kernel` is Elixir's default environment. It mainly consists of: basic language primitives, such as arithmetic operators, spawning of processes, data type handling, and others; macros for control-flow and defining new functionality (modules, functions, and the like); guard checks for augmenting pattern matching. You can invoke `Kernel` functions and macros anywhere in Elixir code without the use of the `Kernel.` prefix since they all have been automatically imported. ... Elixir also has special forms that are always imported and cannot be skipped. These are described in `Kernel.SpecialForms`."

`Kernel` is the default environment: its functions and macros are imported into every module automatically. It mixes three kinds of constructs. **Functions** are runtime (often inlined to `:erlang` BIFs, e.g. `&Kernel.is_atom/1 #=> &:erlang.is_atom/1`) and many are marked "Allowed in guard tests". **Macros** expand at compile time and are marked "(macro)". **Special forms** live in `Kernel.SpecialForms`, are always imported, and cannot be skipped via `import Kernel, except:`.

Two foundational rules govern the module. A value is **truthy** when it is neither `false` nor `nil`, and **falsy** when it is `false` or `nil`. And **structural comparison** follows a fixed term ordering: `number < atom < reference < function < port < pid < tuple < map < list < bitstring`.

### Key functions

#### Pipe and composition

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `\|>/2` (macro) | Passes the LHS as the first argument of the RHS call. | Special form. |
| `tap/2` (macro) | Passes a value through a side-effecting fun, returns the value. | Since v1.12. |
| `then/2` (macro) | Passes a value into a fun, returns the fun's result. | Closure-based pipe for any expression position. Since v1.12. |

```elixir
"Elixir rocks" |> String.split() |> Enum.map(&String.upcase/1) |> Enum.join(" ")
#=> "ELIXIR ROCKS"

then({:ok, 5}, fn {:ok, v} -> v * 2 end)  #=> 10
tap([1,2,3], &IO.inspect/1)               #=> [1, 2, 3]
```

#### Matching and binding

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `=/2` | Match operator. | Special form. |
| `match?/2` (macro) | Boolean pattern test. | Guard-compatible. |
| `==/2` | Structural equality. | `1 == 1.0 #=> true`. |
| `===/2` | Strict equality. | `1 === 1.0 #=> false`. |
| `!=/2`, `!==/2` | Inequality variants. | — |
| `</2`, `<=/2`, `>/2`, `>=/2` | Structural comparison. | Follows term ordering. |

```elixir
1 == 1.0    #=> true
1 === 1.0   #=> false
```

#### Control flow

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `if/2` (macro) | Conditional. | — |
| `unless/2` (macro) | Negated conditional. | DEPRECATED — prefer negated `if`. |
| `case/2` | Pattern match with guards. | Special form; clause vars don't leak; pin with `^`. |
| `cond/1` | First truthy clause. | Special form; raises if all clauses are false. |
| `with/1` | `<-` clauses with optional `else`. | Special form. |
| `raise/1` | Raises `RuntimeError` from a string. | — |
| `raise/2` | Raises an exception struct/module with attrs. | — |
| `reraise/2, /3` | Re-raises preserving the stacktrace. | — |
| `try/1` | `rescue`/`catch`/`after`/`else`. | Special form. |
| `exit/1` (macro) | Terminates the process. | — |
| `throw/1` (macro) | Non-local return, caught by `catch :throw`. | — |

```elixir
case Date.from_iso8601("2015-01-23") do
  {:ok, date} -> date
  {:error, _} -> Date.utc_today()
end

with {:ok, user} <- get_user(1),
     {:ok, token} <- generate_token(user) do
  {:ok, token}
else
  {:error, _} = e -> e
end

raise "oops"
raise ArgumentError, message: "invalid"
reraise e, __STACKTRACE__
```

#### Type-check guards

All inlined BIFs, guard-eligible:

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `is_atom/1`, `is_binary/1`, `is_bitstring/1`, `is_boolean/1` | Type checks. | Inlined. |
| `is_float/1`, `is_integer/1`, `is_number/1` | Numeric type checks. | Inlined. |
| `is_function/1`, `is_function/2` | Function check (arity optional). | Inlined. |
| `is_list/1`, `is_map/1`, `is_pid/1`, `is_port/1`, `is_reference/1`, `is_tuple/1`, `is_nil/1` | Type checks. | Inlined. |

Macros (guard-eligible):

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `is_exception/1, /2` | Exception check. | Since v1.11. |
| `is_struct/1, /2` | Struct check. | Since v1.10. |
| `is_map_key/2` | Map key check. | Since v1.10. |
| `is_non_struct_map/1` | Non-struct map check. | — |

#### Other guards

Inlined, guard-eligible:

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `abs/1`, `div/2`, `rem/2` | Arithmetic. | Inlined. |
| `elem/2`, `put_elem/3`, `tuple_size/1` | Tuple access. | Inlined. |
| `hd/1`, `tl/1`, `length/1` | List access. | Inlined. |
| `map_size/1`, `byte_size/1`, `bit_size/1` | Size functions. | Inlined. |
| `node/0, /1`, `self/0` | Node/pid. | Inlined. |
| `round/1`, `trunc/1`, `ceil/1`, `floor/1` | Rounding. | `ceil`/`floor` since v1.8. |
| `binary_part/3` | Binary substring. | Inlined. |
| `min/2`, `max/2` | Structural min/max. | Inlined. |

#### Access and inspection

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `inspect/1, /2` (macro) | Inspects via the `Inspect` protocol. | Opts: `:label`, `:limit`, `:pretty`, `:charlists`, `:binaries`, `:syntax_colors`, `:width`. |
| `map.key` | Dot access. | Special form. |
| `map[key]` | Bracket access. | Via `Access`. |
| `get_in/2`, `put_in/3`, `update_in/3`, `get_and_update_in/3`, `pop_in/2` | Nested access. | Macros via `Access`. |
| `struct/2` (macro) | Builds a struct from a map/keyword. | Silently drops unknown keys. |
| `struct!/2` (macro) | Builds a struct, raising on unknown keys. | Raises `KeyError`. |

```elixir
inspect([1,2,3,4,5], limit: 3, label: "list")
#=> list: [1, 2, 3, ...]

struct(%User{}, unknown: 1)   #=> %User{...}        (silently dropped)
struct!(%User{}, unknown: 1)  #=> ** (KeyError)
```

#### Attributes and module definition

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `@` (macro) | Module attribute. | Accumulator pattern requires pre-declaration. |
| `defmodule/2`, `def/2`, `defp/2` | Module/function definition. | Macros. |
| `defmacro/2`, `defmacrop/2` | Macro definition. | Macros. |
| `defguard/1`, `defguardp/1` | Guard definition. | Macros. |
| `defoverridable/1` | Marks overridable defs. | Macro. |
| `use/2` | Invokes `__using__/1`. | Macro. |
| `import/2`, `alias/2`, `require/2` | Lexical helpers. | Special forms. |

Built-in attributes: `@doc`, `@moduledoc`, `@spec`, `@type`, `@callback`, `@behaviour`, `@impl`, `@compile`, `@derive`, `@external_resource`.

#### Concurrency

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `spawn/1`, `spawn/3` | Spawns a process. | Functions. |
| `spawn_link/1`, `spawn_link/3` | Spawns a linked process. | Functions. |
| `spawn_monitor/1`, `spawn_monitor/3` | Spawns a monitored process. | Functions. |
| `send/2` | Sends a message (non-blocking, returns the message). | Function. |
| `self/0` | Returns the current pid. | Function. |
| `receive/1` | Receives a message. | Special form (in `Kernel.SpecialForms`); supports `after` timeout. |

```elixir
spawn(fn -> IO.puts("hi") end)
send(self(), :hello)
receive do
  :hello -> :got_it
after
  1000 -> :timeout
end
```

#### Binary, list, and tuple operators

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `<>/2` | Binary concatenation. | Macro. |
| `++/2` | List concatenation. | Macro. |
| `--/2` | List difference. | Removes first occurrence per RHS element. Macro. |
| `in/2` | Membership. | Macro, since v1.5; guard-eligible only with range or list RHS; `not in` supported. |

```elixir
x = 1
x in [1, 2, 3]       #=> true
x not in [1, 2, 3]   #=> false
```

#### Boolean operators

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `and/2`, `or/2`, `not/1` | Strictly boolean. | Macros; raise `BadBooleanError` on non-boolean; guard-eligible; short-circuit. |
| `&&/2`, `\|\|/2`, `!/1` | Truthy/falsy. | Macros; return operands; NOT guard-eligible. |

```elixir
true and "yay!"   #=> "yay!"
1 and 2           #=> ** (BadBooleanError)
nil || "default"  #=> "default"
```

#### Misc

| Function | Behaviour | Complexity/Notes |
|---|---|---|
| `to_string/1` (macro) | Converts via `String.Chars`. | — |
| `to_charlist/1` (macro) | Converts via `List.Chars`. | — |
| `binding/0, /1` (macro) | Returns the current binding. | — |
| `apply/2, /3` | Dynamic dispatch. | Function; loses compile-time checks. |
| `make_ref/0` | Creates a unique reference. | Function. |
| `dbg/2` (macro) | Debugger. | Since v1.14. |
| `__ENV__/0`, `__MODULE__/0`, `__DIR__/0`, `__CALLER__/0`, `__STACKTRACE__/0` | Environment introspection. | Special forms (in `Kernel.SpecialForms`). |

### Common mistakes

- **`and`/`or`/`not` require strict booleans** (raise `BadBooleanError`); `&&`/`||`/`!` work on truthy/falsy. Use `and`/`or` in guards, `&&`/`||` for control-flow defaults.
- **`=` is match/bind** (raises `MatchError` on failure); `==` is structural equality (`1 == 1.0`); `===` is strict (`1 !== 1.0`).
- **`raise` for recoverable errors** (caught by `rescue`); `throw` for non-local return (caught by `catch :throw`); `exit` terminates the process (and linked processes).
- **`struct/2` silently drops unknown keys; `struct!/2` raises `KeyError`.**
- **`in/2` in guards only works with a range or list RHS** — lists expand to `===` comparisons (inefficient for large lists; use `MapSet`).
- **`case` clause variables don't leak** — pin existing vars with `^` to match against them.
- **`apply/2,3` is dynamic dispatch** — loses compile-time checks; prefer static calls.
- **`unless/2` is deprecated** — prefer `if !condition`.
- **Macros must be `require`d before use; functions only need `import`.**
- **The `@attr` accumulator pattern requires pre-declaration** (`@all []` before the first append).
- **`receive` is a special form** (in `Kernel.SpecialForms`), not a `Kernel` function — it cannot be skipped via `import Kernel, except:`.

### Does not exist

The following are special forms in `Kernel.SpecialForms`, not `Kernel` functions — they are always imported and cannot be skipped via `import Kernel, except:`:

- `receive/1`, `case/2`, `cond/1`, `with/1`, `try/1`, `=/2`, `./2`, `&/1`, `<</1`, `alias/2`, `import/2`, `require/2`, `__ENV__/0`, `__MODULE__/0`, `__DIR__/0`, `__CALLER__/0`, `__STACKTRACE__/0`.

### Version notes

| Function / change | Added in | Notes |
|---|---|---|
| `in/2` membership operator | v1.5.0 | Guard-eligible with range/list RHS. |
| `ceil/1`, `floor/1` | v1.8.0 | Kernel-level (was `Float.ceil`/`Float.floor`). |
| `is_struct/1`, `is_struct/2`, `is_map_key/2` | v1.10.0 | Guard macros. |
| `is_exception/1`, `is_exception/2` | v1.11.0 | Guard macros. |
| `tap/2`, `then/2` | v1.12.0 | Closure-based pipe helpers. |
| `dbg/2` | v1.14.0 | Debugger macro. |
| `unless/2` | deprecated | Prefer negated `if`. |

Sources: [Kernel.html](https://hexdocs.pm/elixir/Kernel.html), [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html), [operators.html](https://hexdocs.pm/elixir/operators.html), [patterns-and-guards.html](https://hexdocs.pm/elixir/patterns-and-guards.html), [elixir/lib/kernel.ex (v1.20.2)](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/kernel.ex).

## Kernel.SpecialForms

### Overview & evaluation model

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Special forms are the basic building blocks of Elixir, and therefore cannot be overridden by the developer."
>
> "The `Kernel.SpecialForms` module consists solely of macros that can be invoked anywhere in Elixir code without the use of the `Kernel.SpecialForms.` prefix. This is possible because they all have been automatically imported, in the same fashion as the functions and macros from the `Kernel` module."
>
> "Some of these special forms are lexical (such as `alias/2` and `case/2`). The macros `{}/1` and `<<>>/1` are also special forms used to define tuple and binary data structures respectively."
>
> "Additionally, it documents two special forms, `__block__/1` and `__aliases__/1`, which are not intended to be called directly by the developer but they appear in quoted contents since they are essential in Elixir's constructs."

Special forms are the irreducible constructs the compiler expands; they cannot be overridden or reimported. Unlike `Kernel` macros such as `if/2`, `unless/2`, `def/2`, and `defmodule/2` (which are regular macros a developer could in principle shadow), the forms in `Kernel.SpecialForms` are reserved. Every entry is implemented as a macro that raises if invoked directly with the `Kernel.SpecialForms.` prefix — they are expanded by the compiler.

The module also documents the compile-time environment introspection macros `__ENV__/0`, `__MODULE__/0`, `__DIR__/0`, `__CALLER__/0`, `__STACKTRACE__/0`, and the internal `__cursor__/1`, even though these are not "forms" in the syntactic sense.

Source: [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html) (Elixir v1.20.2).

### Summary of special forms

| Form | Arity | Purpose |
|---|---|---|
| `=/2` | 2 | Match operator: matches the value on the right against the pattern on the left. |
| `^var` (`^/1`) | 1 | Pin operator: uses an already-bound variable's value in a match. |
| `&expr` (`&/1`) | 1 | Capture operator: captures or creates an anonymous function. |
| `left . right` (`./2`) | 2 | Dot operator: remote call, anonymous-function call, or alias. |
| `left :: right` (`::/2`) | 2 | Type operator: used by types and bitstring segments. |
| `{args}` (`{}/1`) | 1 | Creates a tuple. |
| `<<args>>` (`<<>>/1`) | 1 | Defines a new bitstring. |
| `%{}` (`%{}/1`) | 1 | Creates a map. |
| `%struct{}` (`%/2`) | 2 | Matches on or builds a struct. |
| `fn(clauses)` | 1 | Defines an anonymous function. |
| `for(args)` | 1 | Comprehension: build a data structure from an enumerable or bitstring. |
| `case(condition, clauses)` | 2 | Match an expression against clauses (pattern + guards). |
| `cond(clauses)` | 1 | First truthy clause wins. |
| `with(args)` | 1 | Combine matching clauses; abort on first non-match. |
| `receive(args)` | 1 | Consume the first matching message from the process mailbox. |
| `try(args)` | 1 | Evaluate expressions; handle error/exit/throw. |
| `quote(opts, block)` | 2 | Get the AST representation of an expression. |
| `unquote(expr)` | 1 | Unquote inside a quoted expression. |
| `unquote_splicing(expr)` | 1 | Unquote a list, splicing its elements. |
| `alias(module, opts)` | 2 | Set up a module alias. |
| `import(module, opts)` | 2 | Import functions/macros from a module. |
| `require(module, opts)` | 2 | Require a module to use its macros. |
| `super(args)` | 1 | Call the overridden function (with `defoverridable/1`). |
| `__aliases__(args)` | 1 | Internal: holds alias information in quoted AST. |
| `__block__(args)` | 1 | Internal: block expressions in quoted AST. |
| `__ENV__` | 0 | Current environment as a `Macro.Env` struct. |
| `__MODULE__` | 0 | Current module name as an atom (or `nil`). |
| `__DIR__` | 0 | Absolute directory of the current file. |
| `__CALLER__` | 0 | Caller's environment as a `Macro.Env` (macro context). |
| `__STACKTRACE__` | 0 | Stacktrace for the currently handled exception (since 1.7). |
| `__cursor__(args)` | 1 | Internal: cursor position (see `Code.Fragment`). |

> **Not in `Kernel.SpecialForms` on v1.20.2:** `destructure/2` is a regular `Kernel` macro, and range literals (`../2`, `../3`, `..//3`) are handled by `Kernel`/`Range`. They are covered under **Related forms** below for completeness, but cite `Kernel` or `Range` — not this module — when referencing them.

### Lexical directives: `alias`, `import`, `require`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "`import/2`, `require/2` and `alias/2` are called directives and all have lexical scope. This means you can set up aliases inside specific functions and it won't affect the overall scope."

| Directive | What it does | Typical use |
|---|---|---|
| `alias/2` | Creates a short name for a module. | `alias MyApp.Repo` so you can write `Repo.all(...)`. |
| `require/2` | Loads a module so its **macros** can be used. | `require Logger` before `Logger.info/1`. |
| `import/2` | Brings a module's functions/macros into the local scope. | `import Enum` to call `map/2` unqualified. |

```elixir
alias MyApp.Accounts.User        # User instead of MyApp.Accounts.User
require Logger                    # macros must be required before use
import Enum, only: [map: 2]      # bring map/2 into scope
```

#### `alias/2`

- `:as` overrides the default alias (the last name segment): `alias MyApp.Repo, as: Database`.
- Multi-alias: `alias MyApp.{Accounts, Billing, Repo}` expands to three `alias` calls.
- An unused alias emits a warning unless `:warn` is set explicitly (`warn: false` to silence generated aliases).
- To reach a module shadowed by an alias, prefix with `Elixir.`: `Elixir.Keyword.values/1`.

#### `import/2`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "By default, Elixir imports functions and macros from the given module, except the ones starting with an underscore (which are usually callbacks)."

- `:only` accepts `:functions`, `:macros`, `:sigils`, or `name/arity` pairs. `:except` is always exclusive on a previously declared `import/2` of the same module.
- Re-importing the same module erases the previous import unless `:except` is used.
- Functions starting with `_` are not imported by default; include them explicitly in `:only`.
- Lexical: you can `import` inside a single function to override `if/2` or another macro just there.
- Ambiguity is detected lazily — only when an ambiguous call is actually made.

#### `require/2`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Public functions in modules are globally available, but in order to use macros, you need to opt-in by requiring the module they are defined in."

- Calling a macro from a non-required module raises at compile time.
- `require` also accepts `:as`, combining require + alias in one directive.

### Control-flow forms

#### `case/2`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Matches the given expression against the given clauses."
> "`case/2` relies on pattern matching and guards to choose which clause to execute. If your logic cannot be expressed within patterns and guards, consider using `if/2` or `cond/1` instead."

- If no clause matches, an error is raised — provide a catch-all `_` clause when appropriate.
- Variables bound in a clause do **not** leak to the outer context, and outer variables cannot be overridden (they are separate bindings). To match against an existing binding, pin it: `^x -> ...`.
- Nested `case` expressions are a smell — reach for `with/1`.

```elixir
case File.read(path) do
  {:ok, contents} -> contents
  {:error, :enoent} -> "missing"
end
```

#### `cond/1`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Evaluates the expression corresponding to the first clause that evaluates to a truthy value."
> "If all clauses evaluate to `nil` or `false`, `cond` raises an error. For this reason, it may be necessary to add a final always-truthy condition ... which will always match."

- A two-clause `cond` whose last branch is `true` is usually better written as `if/2`.
- "Truthy" means neither `false` nor `nil`.

```elixir
cond do
  n < 0 -> :negative
  n == 0 -> :zero
  true -> :positive
end
```

#### `with/1`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Consider `<-` as a sibling to `=`, except that, while `=` raises in case of not matches, `<-` will simply abort the `with` chain and return the non-matched value."
> "As in `for/1`, variables bound inside `with/1` won't be accessible outside of `with/1`."

- Each `<-` clause is a pattern match that, on failure, short-circuits and returns the non-matching value as the whole `with`'s result.
- Plain `=` clauses inside `with` still raise `MatchError` on mismatch (they do not abort).
- An optional `else` block re-pattern-matches the failed value, like `case`. If the `else` has no match, `WithClauseError` is raised.
- Caveat: all failure paths are flattened into a single `else`, which can erase the distinction between different failure sources.

```elixir
with {:ok, width} <- Map.fetch(opts, :width),
     {:ok, height} <- Map.fetch(opts, :height) do
  width * height
else
  :error -> 0
end
```

#### `receive/1`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Consumes the first message matching any of the given clauses in the current process mailbox."
> "If there is no matching message, the current process waits until a matching message arrives or until after a given timeout value."
> "Any new and existing messages that do not match will remain in the mailbox."

- `after` timeout values: `:infinity` (wait forever, same as no `after`), `0` (non-blocking), or a positive integer ≤ `4_294_967_295` (must fit an unsigned 32-bit int).
- `after` may appear with no match clauses.
- Variable binding follows the same rules as `case/2`.

```elixir
receive do
  {:ok, data} -> data
  {:error, _} = err -> err
after
  5_000 -> {:error, :timeout}
end
```

#### `try/1`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "The `rescue` clause is used to handle exceptions while the `catch` clause can be used to catch thrown values and exits. The `else` clause can be used to control flow based on the result of the expression. `catch`, `rescue`, and `else` clauses work based on pattern matching (similar to the `case` special form)."
> "Calls inside `try/1` are not tail recursive since the VM needs to keep the stacktrace in case an exception happens. To retrieve the stacktrace, access `__STACKTRACE__/0` inside the `rescue` or `catch` clause."

- `rescue` patterns: bare name (`ArithmeticError -> ...`), list (`[ArithmeticError, ArgumentError] -> ...`), or `var in Exception -> ...`.
- `catch` handles `:throw`, `:exit`, and `:error` tuples.
- Avoid wrapping tail-recursive loops in `try` — the saved stacktrace defeats tail-call optimization.

```elixir
try do
  Enum.fetch!(list, index)
rescue
  e in Enum.OutOfBoundsError -> {:error, e}
end
```

#### `for/1` (comprehensions)

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Comprehensions allow you to quickly build a data structure from an enumerable or a bitstring."

- Generators use `<-`; non-matching patterns are discarded (so generators also filter).
- Filters are bare truthy expressions; `nil`/`false` discards the iteration.
- Bitstring generators: `for <<r::8, g::8, b::8 <- pixels>>, do: {r, g, b}`.
- Options: `:into` (any `Collectable`, e.g. `into: ""`), `:uniq` (deduplicate results), and `:reduce` (since v1.8 — fuses filtering and building into a single reduction using `acc -> new_acc` clauses).
- Variables bound inside a `for` do not leak outside it.

```elixir
for x <- 1..5, rem(x, 2) == 0, do: x * x     #=> [4, 16]
for {k, v} <- %{a: 1, b: 2}, into: %{}, do: {k, v * 2}
#=> %{a: 2, b: 4}

for x <- [1, 1, 2, 3], uniq: true, reduce: 0 do
  acc -> acc + x
end
#=> 6
```

Sources: [Kernel.SpecialForms.html#for/1](https://hexdocs.pm/elixir/Kernel.SpecialForms.html#for/1), [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html).

#### `fn/1`

Defines an anonymous function. Multiple clauses are allowed but every clause must have the same arity. See [Function.html](https://hexdocs.pm/elixir/Function.html).

```elixir
add = fn a, b -> a + b end
add.(1, 2)                              #=> 3

negate = fn true -> false; false -> true end
negate.(true)                          #=> false
```

### Pattern-matching and data-literal forms

#### `=/2` (match operator)

> "Match operator. Matches the value on the right against the pattern on the left."

`=` is not assignment — it is a pattern match that binds variables in the left-side pattern to parts of the right-side value. A match that fails raises `MatchError`. See [pattern-matching.html](https://hexdocs.pm/elixir/pattern-matching.html).

```elixir
{:ok, value} = {:ok, 42}               #=> {:ok, 42}; value == 42
^known = compute()                     # asserts compute() == known
```

#### `^/1` (pin operator)

> "Accesses an already bound variable in match clauses."

Without `^`, a variable in a pattern is a fresh binding (and always matches). Pinning uses the variable's current value as part of the pattern.

```elixir
x = 1
^x = 1                                 #=> 1   (matches because x is 1)
^x = 2                                 #=> ** (MatchError)
```

#### `&/1` (capture operator)

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "The capture operator is most commonly used to capture a function with given name and arity from a module: `&Kernel.is_atom/1`."
> "The capture operator can also be used to partially apply functions, where `&1`, `&2` and so on can be used as value placeholders. ... `&(&1 * 2)` is equivalent to `fn x -> x * 2 end`."

- `&Mod.fun/arity` captures a named function; `&Mod.fun/1` is a remote capture, `&local_fun/1` is a local capture.
- **Hot code reloading:** local captures dispatch to the module version at capture time; remote captures dispatch to the current version.
- Partial application needs at least one placeholder (`&1`); block expressions are not supported (`&(&1; &2)` and `&(:foo)` are invalid).
- Works with lists and tuples too: `&{&1, &2}`, `&[&1 | &2]`.

```elixir
Enum.map([1, 2, 3], &(&1 * 2))         #=> [2, 4, 6]
Enum.filter(list, &is_atom/1)
```

#### `%{}/1` and `%/2` (map and struct literals)

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "A struct is a tagged map that allows developers to provide default values for keys, tags to be used in polymorphic dispatches and compile time assertions."
> "Underneath a struct is a map with a `:__struct__` key pointing to the struct module, where the keys are validated at compile-time."

- `%{}` creates a map; `%Mod{}` creates a struct (which is a map plus a `:__struct__` key and compile-time key validation).
- The update syntax `%{map | key: value}` works for both maps and structs; for structs it raises `KeyError` on unknown keys (see the `Map` section above).
- Pattern matching: `%User{name: name} = user` matches the struct type and binds `name`; `%struct{} = user` binds the struct module; `%_{} = user` matches any struct.
- In the AST, map pairs are always a list of 2-tuples regardless of `=>` vs keyword syntax.

```elixir
%{a: 1, "b" => 2}
%User{name: "Alice", age: 30}
%{user | age: 31}
```

See [structs.html](https://hexdocs.pm/elixir/structs.html) and the `Map` section above.

#### `<<>>/1` (bitstring literal)

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Defines a new bitstring."

- Segment types: `integer` (default), `float`, `bits`/`bitstring`, `binary`/`bytes`, `utf8`, `utf16`, `utf32`.
- A literal string in a bitstring expands to its bytes: `<<"foo">>` ≡ `<<102, 111, 111>>`.
- Size = `unit * size`. Defaults: integers unit 1 bit / size 8; floats unit 1 / size 64 (must be 16, 32, or 64); binaries unit 8 bits.
- Modifiers: `signed`/`unsigned` (default), `big` (default)/`little`/`native`; order of modifiers is arbitrary (`<<x::integer-native>>` ≡ `<<x::native-integer>>`).
- **At runtime, a bound variable used as a binary segment must be tagged** — `<<102, rest>>` raises `ArgumentError` if `rest` is a string; use `<<102, rest::binary>>`.
- **Only the last binary segment may use the default size**; all others must specify size explicitly.
- A segment's size may reference earlier variables *within the same bitstring*, but not variables bound outside the match (raises `CompileError`).

```elixir
<<102, 111, 111>> == "foo"             #=> true
<<r::8, g::8, b::8>> = <<255, 0, 0>>
<<x::size(8)-unit(4)>>                 # 32-bit integer
```

#### `{}/1`, `./2`, `::/2`

- `{}/1` creates a tuple (also a special form so tuples have a first-class AST node).
- `./2` is the dot operator: remote call (`Mod.fun`), anonymous-function call (`fun.(arg)`), or alias reference.
- `::/2` is the type operator, used both in typespecs (`@type t :: integer()`) and in bitstring segments (`<<x::integer-8>>`).

### Metaprogramming forms

#### `quote/2`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Gets the representation of any expression."
> "The building block of Elixir macros is a tuple with three elements, for example: `{:sum, [], [1, 2, 3]}`."

The AST tuple is `{call_or_atom, metadata, args_or_atom}`. Options:

| Option | Effect |
|---|---|
| `:bind_quoted` | Passes a binding to the macro; when set, `unquote/1` is automatically disabled. |
| `:context` | Sets the resolution context. |
| `:generated` | Marks code as generated so it does not emit warnings. |
| `:file` / `:line` | Sets the file/line of the quoted expressions. |
| `:location` | `:keep` retains the current line/file from `quote`. |
| `:unquote` | When `false`, disables unquoting; `unquote` calls stay as-is in the AST. |

- Macros are **hygienic**: variables introduced in a macro do not clash with the caller's. Use `var!(name)` to break hygiene and set/get a caller variable.
- Caveat: do not combine `location: :keep` with `unquote`d arguments — mismatched line info produces erroneous stacktraces.

```elixir
quote do: sum(1, 2, 3)                 #=> {:sum, [], [1, 2, 3]}
quote do: Foo.Bar                      #=> {:__aliases__, [alias: false], [:Foo, :Bar]}
```

#### `unquote/1` and `unquote_splicing/1`

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "This function expects a valid Elixir AST, also known as quoted expression, as argument. If you would like to `unquote` any value, such as a map or a four-element tuple, you should call `Macro.escape/1` before unquoting."

- `unquote/1` injects a precomputed AST fragment into a `quote`.
- For non-AST values (maps, tuples), `Macro.escape/1` first.
- `unquote_splicing/1` injects a *list* of AST nodes, splicing them into the enclosing list; it also works in block context when wrapped in parentheses.

```elixir
x = 1
quote do: unquote(x) + 1               #=> {:+, [], [1, 1]}

requires = [quote(do: Integer), quote(do: Logger)]
quote do: (unquote_splicing(requires)) # splices both require calls
```

#### `super/1`

> "Calls the overridden function when overriding it with `Kernel.defoverridable/1`."

`super` only has meaning inside a function marked overridable via `defoverridable/1`. See [Kernel.defoverridable/1](https://hexdocs.pm/elixir/Kernel.html#defoverridable/1).

#### `__aliases__/1` and `__block__/1` (internal)

From [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html):

> "Elixir represents `Foo.Bar` as `__aliases__` so calls can be unambiguously identified by the operator `:.`."
> "This is the special form used whenever we have a block of expressions in Elixir. This special form is private and should not be invoked directly."

- `__aliases__/1` holds module references in the AST (`Foo.Bar` → `{:__aliases__, [alias: false], [:Foo, :Bar]}`). The head element may be any term expanding to an atom at compile time; tail elements are always atoms; when the head is `:Elixir`, no expansion happens.
- `__block__/1` wraps a sequence of expressions (`quote do 1; 2; 3 end` → `{:__block__, [], [1, 2, 3]}`).

Both appear in quoted output and are essential to Elixir's constructs, but you should not invoke them directly.

### Compile-time environment forms

| Form | Returns |
|---|---|
| `__ENV__/0` | Current environment as a `%Macro.Env{}` (file, line, module, function, aliases, etc.). |
| `__CALLER__/0` | The caller's `%Macro.Env{}` — only meaningful inside a macro. |
| `__MODULE__/0` | The current module name as an atom, or `nil` outside a module. |
| `__DIR__/0` | Absolute path of the current file's directory as a binary. |
| `__STACKTRACE__/0` | Stacktrace for the exception currently being handled (valid only inside a `rescue`/`catch` clause; since v1.7). |

```elixir
defmodule MyApp do
  def __info__, do: {__MODULE__, __ENV__.file, __ENV__.line}
end
```

### Related forms (not in `Kernel.SpecialForms` on v1.20.2)

These are commonly grouped with special forms in tutorials but live elsewhere in v1.20.2:

| Form | Where it actually lives | Notes |
|---|---|---|
| `destructure/2` | `Kernel.destructure/2` | Splits a list into named variables in one match; extra variables get `nil`, extras are ignored. |
| `../2`, `../3` | `Kernel` + `Range` | `first..last` (inclusive); `first..last//step` for a stepped range. See [Range.html](https://hexdocs.pm/elixir/Range.html). |
| `if/2`, `unless/2` | `Kernel` | Regular macros, not special forms. |
| `def/2`, `defmodule/2`, `defprotocol/2`, etc. | `Kernel` | Regular macros. |

```elixir
# destructure (Kernel, not SpecialForms):
destructure [a, b, c], [1, 2]            #=> a == 1, b == 2, c == nil

# ranges (Kernel/Range):
1..10                                   #=> 1..10
1..10//2                                #=> 1..10//2 (stepped)
```

### Common mistakes

- **Treating `=` as assignment.** It is a match; `{:ok, x} = result` raises `MatchError` if `result` is `{:error, _}`. Use `with` or an explicit `case` when mismatch is expected.
- **Forgetting to pin in `case`/`receive`.** `x -> ...` rebinds `x`; to compare against an existing `x`, write `^x -> ...`.
- **Expecting `cond` to fall through.** `cond` raises if no clause is truthy — always include a `true ->` fallback when failure is possible.
- **Wrapping tail-recursive loops in `try`.** `try` is not tail-recursive; the VM keeps the stack for a potential stacktrace.
- **Calling a macro without `require`.** Macros are not globally available; the defining module must be `require`d (or `import`ed, which implies require).
- **Confusing local vs remote captures under hot code reloading.** Local captures pin the module version; remote captures follow the current version.
- **Untagged binary segments at runtime.** `<<head, rest>>` raises if `rest` is a string; use `rest::binary`.
- **Relying on `with`'s `else` to distinguish failures.** All failures flatten into one `else`; structure your tagged tuples so the failure source is recoverable.
- **Invoking `__aliases__`/`__block__` directly.** They are internal AST nodes; only meaningful in quoted output.

### Version notes

| Form / change | Added in | Notes |
|---|---|---|
| `__STACKTRACE__/0` | v1.7 | Replaces the deprecated `System.stacktrace/0`. |
| `for` `:reduce` option | v1.8 | Single-pass filtering + accumulation. |
| `__cursor__/1` | recent | Internal cursor-position form; see `Code.Fragment`. |
| `with/1` `else` block | v1.5 | `with` itself predates this; the `else` clause landed in v1.5. |

No deprecations are called out on the v1.20.2 `Kernel.SpecialForms` page.

Sources: [Kernel.SpecialForms.html](https://hexdocs.pm/elixir/Kernel.SpecialForms.html) (v1.20.2), [comprehensions.html](https://hexdocs.pm/elixir/comprehensions.html), [pattern-matching.html](https://hexdocs.pm/elixir/pattern-matching.html), [structs.html](https://hexdocs.pm/elixir/structs.html), [Kernel.html](https://hexdocs.pm/elixir/Kernel.html), [Range.html](https://hexdocs.pm/elixir/Range.html), [Function.html](https://hexdocs.pm/elixir/Function.html), [elixir/lib/kernel/special_forms.ex (v1.20.2)](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/elixir/lib/kernel/special_forms.ex).

## Review checklist

- [ ] `Enum` is used only when eager evaluation and materialization are acceptable.
- [ ] Infinite or unbounded enumerables are consumed with `Stream`, not `Enum`.
- [ ] `?` functions (`any?`, `all?`, `empty?`, `member?`) are expected to return booleans.
- [ ] `!` variants (`fetch!`, etc.) raise on failure; a non-bang variant is preferred when errors are handled.
- [ ] `at/3`, `fetch/2`, and `slice/3` on lists are understood to be O(n), not O(1).
- [ ] Maps passed to `Enum` are treated as two-element tuples; the result is a list of tuples, not a map.
- [ ] Sorting structs uses an explicit sorter module or `sort_by/3` with the comparator module.
- [ ] `length/1` is used only for lists; `Enum.count/1` is used for arbitrary enumerables.
- [ ] `chunk_every/4` arity and `leftover` argument are used correctly.
- [ ] `Enum.partition/2` is not used; `Enum.split_with/2` is the correct function.

## Implementation checklist

- [ ] Choose `Enum` for finite, eager, single- or few-pass transformations.
- [ ] Choose `Stream` for large files, infinite sequences, or pipelines that would create huge intermediate lists.
- [ ] Use `Enum.into/2` (or a `for` comprehension with `:into`) to convert enumerables into maps, keyword lists, or other collectables.
- [ ] Prefer `for` comprehensions over long `Enum` chains when readability improves.
- [ ] Use `reduce_while/3` when early termination is required.
- [ ] Use `group_by/3` or `frequencies/1` instead of hand-rolled counting `reduce`s.
- [ ] Use `_by`, `_while`, and `_every` variants where they match the semantic need.
- [ ] Provide an explicit sorter when sorting structs such as `Date`, `DateTime`, `Time`, or `NaiveDateTime`.

## Validation hooks

- `mix format --check-formatted` — catches formatting issues.
- `mix credo` — flags non-idiomatic `Enum`/`Stream` choices, overly complex pipelines, and unused variables.
- `mix dialyzer` — type-checks function specs, including predicate return types and collection shapes.
- Compiler warnings and errors — unused bindings, deprecated functions (e.g. `Enum.chunk/2`), and missing imports surface at compile time.
- Keep the official [enum-cheat.html](https://hexdocs.pm/elixir/enum-cheat.html) cheatsheet handy as a quick reference.
- Note: there is no automated "Enum-specific" gate; the hooks above are general Elixir validation tools.

## Examples

### Map + filter pipeline

```elixir
[1, 2, 3, 4, 5]
|> Enum.map(&(&1 * 2))
|> Enum.filter(&(&1 > 5))
#=> [6, 8, 10]
```

### Reduce to build a map

```elixir
["apple", "banana", "apricot"]
|> Enum.reduce(%{}, fn word, acc ->
  Map.update(acc, String.first(word), [word], &[word | &1])
end)
#=> %{"a" => ["apricot", "apple"], "b" => ["banana"]}
```

### `reduce_while` early exit

```elixir
Enum.reduce_while([1, 2, 3, 4, 5], 0, fn x, acc ->
  if x > 3 do
    {:halt, acc}
  else
    {:cont, acc + x}
  end
end)
#=> 6
```

### `chunk_every` batching

```elixir
1..10
|> Enum.chunk_every(3)
#=> [[1, 2, 3], [4, 5, 6], [7, 8, 9], [10]]
```

### `group_by`

```elixir
Enum.group_by(~w[apple banana apricot cherry], &String.first/1)
#=> %{"a" => ["apple", "apricot"], "b" => ["banana"], "c" => ["cherry"]}
```

### `for` comprehension with `:into`

```elixir
for {k, v} <- %{a: 1, b: 2}, into: %{}, do: {k, v * 2}
#=> %{a: 2, b: 4}
```

## Common mistakes

- Using `Enum` with an infinite enumerable. Any greedy function such as `Enum.map/2`, `Enum.filter/2`, or `Enum.count/1` will not terminate.
- Assuming `Enum.at/3` is O(1). On a plain list it is O(n); O(1) only on structures that expose `Enumerable.slice/1` such as `Range`.
- Confusing `length/1` with `Enum.count/1`. `length/1` works only on lists; `Enum.count/1` works on any enumerable and may be O(1) when optimized.
- Forgetting that `Enum.filter/2` on a map returns a list of two-element tuples, not a map:

  ```elixir
  Enum.filter(%{a: 1, b: 2}, fn {_, v} -> v > 1 end)
  #=> [b: 2]  # i.e. [{:b, 2}], a list of tuples, not a map
  ```

- Mapping over a map and expecting a map back:

  ```elixir
  Enum.map(%{a: 1, b: 2}, fn {k, v} -> {k, v * 2} end)
  #=> [a: 2, b: 4]  # list of tuples; use Map.new/2 or a comprehension for a map
  ```

- Sorting structs by term order instead of semantic order. `~D[2017-03-31]` is greater than `~D[2017-04-01]` in raw term order because `31 > 1`.
- Using `Enum.chunk/2` (deprecated/removed) instead of `Enum.chunk_every/2`.
- Looking for `Enum.partition/2`; it does not exist. Use `Enum.split_with/2`.
- Confusing `find/3` (returns the element) with `find_value/3` (returns the function's result).
- Passing a non-empty list as a collectable to `Enum.into/2`; this is deprecated.

## Strict vs contextual guidance

### Strict

- `Enum` functions are always eager and return a list (or other concrete result), never a lazy stream.
- Passing an infinite enumerable to a greedy `Enum` function will hang or fail to terminate.
- Functions ending in `?` (`any?`, `all?`, `empty?`, `member?`) must return booleans.
- Functions ending in `!` (`fetch!`, etc.) raise on failure; do not silently return a default.
- `Enumerable` requires `reduce/3`; the other callbacks are optional optimizations.

### Conventions

- Use `Stream` for laziness, large files, infinite sequences, and pipelines that would otherwise allocate large intermediate lists.
- Prefer `for` comprehensions over multi-function `Enum` chains when the logic is clearer.
- Use `_by` variants when the operation should key on a derived value.
- Use `_while` variants for predicate-driven or continuation-driven stopping.
- Use `_every` variants for regular-interval operations.
- Use `into/2` or `:into` to build maps, keyword lists, and other collectables.
- Use `count/1` for enumerables; reserve `length/1` for lists and `size` names for O(1) operations.

### Contextual tradeoffs

- A simple single-pass `Enum` pipeline is often faster than the equivalent `Stream` because `Stream` has per-step overhead.
- Eager intermediate lists are acceptable when the data is small and the code is simpler to read.
- `for` comprehensions and `Enum` pipelines are often interchangeable; choose the one that is more readable for the specific transformation.
- `Map`-specific work should usually use the `Map` module rather than `Enum`, because `Enum` flattips maps to tuples.

## Policy decisions for individual repos

- Whether to require `Stream` for any multi-stage pipeline that could process large inputs.
- Whether to standardize on `for` comprehensions or `Enum` pipeline style for transformations.
- Lint stack choices: `mix format --check-formatted`, `mix credo --strict`, `mix dialyzer`.
- Whether to forbid deprecated `Enum` functions (e.g. `Enum.chunk/2`) in CI.
- Whether to require explicit sorter modules when sorting structs.
- Whether to enforce module-to-file path mirroring for Elixir code in CI/credo.
- Whether to maintain a repo-specific "use `Enum` unless Stream is justified" rule.

## Related docs

- [naming-conventions](./naming-conventions.md) — Elixir naming conventions that `Enum` follows (`?`, `!`, `_by`, `_while`, `_every`, `count` vs `size` vs `length`).
- Related Elixir corpus docs: `docs/elixir/language-fundamentals.md` (modules and functions), `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.

## Related skills

- None defined yet.
