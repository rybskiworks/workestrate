# PropCheck: State-Machine and Parallel Testing

## Purpose

Document the capability that is PropCheck's **key differentiator** versus StreamData: state-machine property-based testing, including **parallel** state-machine testing (the feature StreamData explicitly lacks). Covers the `StateM`, `StateM.ModelDSL` (recommended), `StateM.DSL` (deprecated), and `FSM` behaviours, plus the `BasicTypes` generator vocabulary and the `let`/`such_that`/`let_shrink` combinators.

## Sources used

- https://hexdocs.pm/propcheck/PropCheck.StateM.html — `StateM` behaviour and callbacks
- https://hexdocs.pm/propcheck/PropCheck.StateM.ModelDSL.html — recommended DSL
- https://hexdocs.pm/propcheck/PropCheck.BasicTypes.html — `nat`, `integer`, `list`, `oneof`, etc.
- https://github.com/alfert/propcheck — README, `test/cache_dsl_test.exs`, `test/parallel_test.exs`
- https://propertesting.org/ — PropEr/QuickCheck-style testing background
- Arts, Hughes, Johansson, Wiger (2006) — origin of `eqc_commands` state-machine testing
- Hughes, Claessen (2007, PADL) — QuickCheck testing of imperative programs
- Claessen, Palka, Smallbone, Hughes, Svensson, Arts (2009, ICFP) — PULSE linearizability checking

This page reflects `propcheck` 1.5.0 (hex package name: `propcheck`, one word), wrapping Erlang **PropEr 1.5.0**. License: **GPL-3.0**.

## Why PropCheck for stateful testing

StreamData does not implement state-machine, parallel, or targeted PBT (see [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md)). PropCheck is the only Elixir library that does. It wraps Erlang PropEr, which provides:

- **Sequential state-machine testing** — generate a sequence of commands, run them against a model, check postconditions.
- **Parallel state-machine testing** — run two concurrent command sequences against the system under test and check that the result is linearizable.
- **Targeted PBT** — simulated-annealing search (see [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md)).

## Setup

### `mix.exs`

```elixir
defp deps do
  [
    {:propcheck, "~> 1.5", only: [:test, :dev]}
  ]
end
```

> **Hex package name:** `propcheck` (one word). It is **not** `prop_check`.

### `.formatter.exs`

```elixir
[
  import_deps: [:propcheck]
]
```

## Basic property syntax

`use PropCheck` brings in the `property/3` macro and the generator vocabulary:

```elixir
defmodule NatTest do
  use ExUnit.Case, async: true
  use PropCheck

  property "nat is always non-negative" do
    forall n <- nat() do
      n >= 0
    end
  end
end
```

### Multiple bindings

```elixir
property "addition is commutative" do
  forall a <- integer(), b <- integer() do
    a + b == b + a
  end
end
```

### Options form

```elixir
property "runs 500 times verbosely", [:verbose, numtests: 500] do
  forall n <- nat() do
    n >= 0
  end
end
```

## `BasicTypes` generator vocabulary

PropCheck generators (wrapping PropEr's) differ from StreamData's. The core set:

```elixir
nat()                 # 0, 1, 2, ...
integer()             # any integer
integer(0, 10)       # bounded integer
float()               # any float
boolean()             # true | false
atom()                # any atom
binary()              # any binary
binary(10)            # binary of length 10
utf8()                # UTF-8 string
byte()                # 0..255
list(integer())       # list of integers
list(integer(), 3)    # list of exactly 3 integers
non_empty(list(integer()))  # non-empty list
vector(3, integer()) # fixed-length list
tuple({integer(), boolean()})
map(atom(), integer())
oneof([integer(), boolean()])
frequency([{1, integer()}, {9, boolean()}])
elements([:a, :b, :c])
exactly(:foo)
function(integer(), boolean())  # function from int to bool
```

## Combinators: `let`, `such_that`, `lazy`, `let_shrink`, `shrink`

```elixir
# let — bind a generated value into a new generator
let n <- nat() do
  elements([n, n * 2, n * 3])
end

# such_that — filter (raises if too narrow, like StreamData filter/3)
such_that(n <- nat(), when n > 0)

# such_that_maybe — filter that does not raise on narrow filters
such_that_maybe(n <- nat(), when n > 0)

# lazy — defer generator construction (useful for recursive generators)
lazy(elements([:leaf, {:node, lazy(elements([:leaf, :leaf]))}]))

# let_shrink — control the shrinking structure (recursive trees)
let_shrink(leaves <- list(nat()) do
  Enum.reduce(leaves, nil, fn leaf, acc -> {:node, leaf, acc} end)
end)

# shrink — provide explicit shrink values
shrink(integer(), fn n -> [0, n - 1, n + 1] end)
```

### Recursive tree via `let_shrink`

```elixir
defmodule TreeGen do
  use PropCheck

  def tree do
    let_shrink(subtrees <- list(tree())) do
      oneof([leaf(), {:node, subtrees}])
    end
  end

  def leaf, do: elements([:a, :b, :c])
end
```

## State-machine testing: the three behaviours

PropCheck provides four state-machine entry points:

| Behaviour | Status | Use for |
|---|---|---|
| `PropCheck.StateM` | base behaviour | direct callback implementation |
| `PropCheck.StateM.ModelDSL` | **recommended** | new state-machine code |
| `PropCheck.StateM.DSL` | **deprecated** | legacy code only |
| `PropCheck.FSM` | advanced | finite-state-machine models with explicit transitions |

> **Note on `StateM.DSL`:** it is deprecated because its commands return a struct rather than the tuple shape `StateM` expects. Use `ModelDSL` for all new code.

## Sequential state-machine testing: `StateM`

The `StateM` behaviour requires five callbacks: `initial_state/0`, `command/1`, `precondition/2`, `postcondition/3`, `next_state/3`. The classic example (from the PropCheck test suite) is the **process dictionary**:

```elixir
defmodule PropCheckPDictTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.StateM

  property "process dictionary behaves as a model" do
    forall cmds <- commands(__MODULE__) do
      {_history, _state, result} = run_commands(__MODULE__, cmds)
      result == :ok
    end
  end

  # --- StateM callbacks ---

  def initial_state, do: %{}

  def command(_state) do
    oneof([
      {:set, var(), nat()},
      {:get, var()}
    ])
  end

  def precondition(_state, {:get, var}) when is_reference(var), do: true
  def precondition(_state, {:set, _var, _val}), do: true
  def precondition(_state, _command), do: false

  def postcondition(state, {:get, var}, result) do
    Map.get(state, var) == result
  end
  def postcondition(_state, {:set, _var, _val}, _result), do: true

  def next_state(state, value, {:set, var, _val}) do
    Map.put(state, var, value)
  end
  def next_state(state, _value, _command), do: state
end
```

The flow:

1. `commands(__MODULE__)` generates a list of symbolic commands using `command/1`.
2. `run_commands(__MODULE__, cmds)` runs them sequentially, threading `next_state/3` and checking `precondition/2` + `postcondition/3`.
3. The property asserts `result == :ok` (no postcondition failed).

## `ModelDSL` (recommended)

`ModelDSL` provides a declarative `defcommand` macro that bundles the command implementation, generator, precondition, postcondition, and next-state logic. The canonical example is the **cache** (from `test/cache_dsl_test.exs`):

```elixir
defmodule CacheModelDSLTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.StateM.ModelDSL

  # The system under test
  defmodule Cache do
    use Agent

    def start_link, do: Agent.start_link(fn -> %{} end)
    def put(cache, k, v), do: Agent.update(cache, &Map.put(&1, k, v))
    def get(cache, k), do: Agent.get(cache, &Map.get(&1, k))
  end

  defstruct [:cache, :model]

  def initial_state, do: %__MODULE__{cache: nil, model: %{}}

  def command_gen(%__MODULE__{model: model}) do
    keys = Map.keys(model)
    frequency([
      {1, {:call, __MODULE__, :start_cache, []}},
      {3, {:call, __MODULE__, :put, [var(:cache), key(), value()]}},
      {3, {:call, __MODULE__, :get, [var(:cache), key()]}},
      {1, {:call, __MODULE__, :stop, [var(:cache)]}}
    ])
  end

  defcommand :start_cache do
    def impl, do: Cache.start_link()
    def post(_state, [], {:ok, _pid}), do: true
    def next(state, {:ok, pid}, []), do: %{state | cache: pid}
  end

  defcommand :put do
    def impl(cache, k, v), do: Cache.put(cache, k, v)
    def args(_state), do: [var(:cache), key(), value()]
    def next(state, _result, [_cache, k, v]), do: %{state | model: Map.put(state.model, k, v)}
    def post(_state, [_cache, _k, _v], :ok), do: true
  end

  defcommand :get do
    def impl(cache, k), do: Cache.get(cache, k)
    def args(%__MODULE__{model: model}), do: [var(:cache), key()]
    def post(%__MODULE__{model: model}, [_cache, k], result), do: Map.get(model, k) == result
  end

  defcommand :stop do
    def impl(cache), do: Agent.stop(cache)
    def next(state, _result, [_cache]), do: %{state | cache: nil}
  end

  def key, do: elements([:a, :b, :c])
  def value, do: integer()

  property "cache sequential model" do
    forall cmds <- commands(__MODULE__) do
      {_history, _state, result} = run_commands(__MODULE__, cmds)
      result == :ok
    end
  end
end
```

`defcommand` bundles:

- `impl/arity` — the actual implementation called against the SUT.
- `args/1` (optional) — generator for the command's arguments given the state.
- `pre/2` (optional) — precondition.
- `post/3` — postcondition.
- `next/3` — next-state transition.

`command_gen/1` returns the command generator given the current state (using `frequency/1` to weight commands).

## Parallel state-machine testing (the feature StreamData lacks)

This is PropCheck's unique capability. The model:

1. Generate **two** sequences of commands.
2. Run them **sequentially** first to establish a valid model trace.
3. Run them **in parallel** (two processes) against a fresh SUT.
4. Check that the parallel result is **linearizable** — i.e. there exists an interleaving of the two sequential traces that the model accepts.

The linearizability check is the PULSE lineage (Claessen et al., ICFP 2009). PropCheck exposes it via `parallel_commands/1` and `run_parallel_commands/2`:

```elixir
defmodule ParallelCacheTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.StateM.ModelDSL

  # ... same CacheModelDSLTest commands as above ...

  property "cache is linearizable under concurrency" do
    forall cmds <- parallel_commands(__MODULE__) do
      {_history, _state, result} = run_parallel_commands(__MODULE__, cmds)
      result == :ok
    end
  end
end
```

### Why `Instrument` is required

Parallel testing only finds races if the scheduler actually interleaves the two processes. By default the BEAM scheduler may run them too deterministically. PropCheck provides `PropCheck.Instrument.instrument_module/2` to inject yielding points so races surface:

```elixir
defmodule ParallelCacheInstrumentedTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.StateM.ModelDSL

  defmodule YieldInstrumenter do
    use PropCheck.Instrument
    # inject a yield after each operation to expose races
  end

  setup do
    Instrument.instrument_module(Cache, YieldInstrumenter)
    on_exit(fn -> Instrument.uninstrument_module(Cache) end)
    :ok
  end

  property "cache races surface under instrumentation" do
    forall cmds <- parallel_commands(__MODULE__) do
      {_history, _state, result} = run_parallel_commands(__MODULE__, cmds)
      result == :ok
    end
  end
end
```

### Limitations of parallel testing

From the PropCheck README:

> Parallel testing is very limited.

Specifically:

- **Exactly two processes.** There is no `numworkers` option; PropCheck always runs two command sequences in parallel. This is a PropEr/PULSE limitation, not a PropCheck choice.
- **Linearizability only.** The check proves there exists *some* interleaving the model accepts; it does not prove all interleavings are safe.
- **Instrumentation required.** Without `Instrument`, races rarely surface because the scheduler is too cooperative.
- **No targeted/collect composition.** Parallel properties do not compose with `TargetedPBT` or `collect`/`aggregate`.

### Academic lineage

- **Arts, Hughes, Johansson, Wiger (2006)** — origin of `eqc_commands`, the command-sequence state-machine model that PropEr's `commands/1` descends from.
- **Hughes (PADL 2007)** — "QuickCheck Testing for Fun and Profit": the formalization of state-machine PBT for imperative/stateful systems.
- **Claessen, Palka, Smallbone, Hughes, Svensson, Arts (ICFP 2009)** — PULSE: the linearizability checker that backs `run_parallel_commands/2`.

## `StateM.DSL` is deprecated

`StateM.DSL` returns a struct from its commands rather than the tuple shape `StateM` expects. It is retained for legacy code only. **Do not use it for new code** — use `ModelDSL`.

## Related docs

- [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md) — simulated-annealing PBT.
- [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) — capability comparison.
- [`streamdata-getting-started.md`](streamdata-getting-started.md) — StreamData's (non-stateful) model.
