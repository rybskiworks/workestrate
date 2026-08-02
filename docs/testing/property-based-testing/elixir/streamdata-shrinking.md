# StreamData Shrinking

## Purpose

Explain StreamData's integrated, per-generator shrinking model: how counterexamples are minimized, what each generator's shrink directions are, how to opt out, and how to drive shrinking directly via `check_all/3`.

## Sources used

- https://hexdocs.pm/stream_data/StreamData.html — shrinking semantics, `unshrinkable/1`, `check_all/3`
- https://well-typed.com/blog/2019/05/integrated-shrinking/ — "Integrated Shrinking" (Hedgehog lineage)
- https://hypothesis.works/articles/integrated-shrinking/ — Hypothesis on integrated shrinking

This page reflects `stream_data` 1.3.0.

## Integrated shrinking

StreamData uses **integrated shrinking** (the Hedgehog-style model, as opposed to QuickCheck's type-class-based shrinking). The defining property:

> Each generator carries its own shrink logic, and shrinking is **invariant-preserving** — a shrunk value is always a value that the same generator could have produced. There is no separate "shrinker" typeclass to implement and no risk of shrinking into an invalid value.

From [well-typed.com](https://well-typed.com/blog/2019/05/integrated-shrinking/):

> "Integrated shrinking means that the shrinking strategy is part of the generator itself, so shrinking works correctly even when generators are composed."

This is why `Stream`/`Enum` manipulation disables shrinking (see [`streamdata-getting-started.md`](streamdata-getting-started.md)): once a generator has been through `Stream.filter`/`Stream.map`, the result is an ordinary lazy stream with no shrink information attached.

## Per-generator shrink directions

Each generator shrinks toward a documented "simplest" value:

| Generator | Shrinks toward | Notes |
|---|---|---|
| `integer()` | `0` | both positive and negative integers shrink toward 0 |
| `positive_integer()` | `1` | the smallest positive integer |
| `non_negative_integer()` | `0` | |
| `boolean()` | `false` | |
| `byte()` | `0` | |
| `list_of(g)` | remove elements, then shrink each element | empty list is the simplest |
| `string(:ascii)` | empty string | shrinks the underlying binary |
| `one_of([g1, g2, ...])` | earlier generator in the list | shrinks toward the first generator |
| `member_of(xs)` | earlier element of `xs` | |
| `constant(v)` | no shrinking | always returns `v` |
| `repeatedly/1` | no shrinking | opaque function output |
| `map(g, f)` | shrinks `g`, then re-applies `f` | invariant-preserving |
| `bind(g, f)` | shrinks `g`, re-runs `f` | invariant-preserving |
| `filter(g, p)` | shrinks `g`, re-checks `p` | may fail to shrink if filter is narrow |

## Opting out: `unshrinkable/1`

`unshrinkable/1` wraps a generator and disables shrinking entirely. Use it when shrinking is expensive (e.g. the generated value feeds a slow operation) or when the shrunk values would be misleading:

```elixir
StreamData.unshrinkable(StreamData.integer()) |> Enum.take(1)
#=> [42]
```

If a property using `unshrinkable/1` fails, the counterexample is the original failing value with no minimization.

## Driving shrinking directly: `check_all/3`

`check_all/3` is the function form of `check all`. It returns `{:ok, _result}` on success or `{:error, metadata}` on failure, where `metadata` carries the original and shrunk counterexamples:

```elixir
{:error, metadata} =
  StreamData.check_all(
    StreamData.integer(),
    [initial_seed: 42],
    fn int ->
      if int == 0 or rem(int, 11) != 0 do
        {:ok, nil}
      else
        {:error, Integer.to_string(int)}
      end
    end
  )

metadata.original_failure
#=> "22"

metadata.shrunk_failure
#=> "11"
```

The property above fails for any non-zero multiple of 11. The original failure (e.g. `22`) is shrunk to the smallest failing value (`11`), because `integer()` shrinks toward `0` and `11` is the smallest non-zero multiple of 11.

`metadata` also includes `:shrinking_steps` (how many steps shrinking took) and `:result` (the final shrunk result tuple).

## Project configuration

StreamData reads configuration from the `:stream_data` application environment. The most common knob is `:max_runs`, which scales the number of generated values per property:

```elixir
# config/config.exs
import Config

config :stream_data, max_runs: if System.get_env("CI"), do: 1000, else: 50
```

This lets CI run more cases (higher confidence) while keeping local iteration fast. The per-property `:max_runs` option in `check all` overrides this.

Other relevant options (settable per-`check all` or via `:initial_seed`):

| Option | Default | Meaning |
|---|---|---|
| `:initial_size` | `1` | starting size parameter |
| `:max_runs` | `100` | number of generated values |
| `:max_shrinking_steps` | `100` | cap on shrinking iterations |
| `:initial_seed` | random | pin the seed for reproducibility |

## When shrinking is disabled

Shrinking is lost when a generator is transformed by `Stream`/`Enum`:

```elixir
# Shrinking DISABLED — this is an ordinary stream, not a StreamData generator
StreamData.integer()
|> Stream.filter(&(&1 > 0))
|> Stream.map(&(&1 * 2))
|> Enum.take(10)
```

For properties that need shrinking, use the combinator equivalents:

```elixir
# Shrinking PRESERVED
StreamData.map(
  StreamData.filter(StreamData.integer(), &(&1 > 0)),
  &(&1 * 2)
)
```

## Related docs

- [`streamdata-getting-started.md`](streamdata-getting-started.md) — `check all` options and the generators-as-streams warning.
- [`streamdata-combinators.md`](streamdata-combinators.md) — `unshrinkable/1`, `filter/3`, `map/2`, `bind/2`.
- [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) — PropCheck also has integrated shrinking (PropEr-style).
