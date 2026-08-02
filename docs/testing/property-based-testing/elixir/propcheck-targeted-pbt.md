# PropCheck: Targeted Property-Based Testing

## Purpose

Document PropCheck's `TargetedPBT` module: simulated-annealing search over the generator space, which has **no StreamData analogue**. Covers `exists`, `forall_targeted`, `not_exists`, `minimize`/`maximize`, `user_nf`, the `:search_steps` option, and the caveats that make targeted PBT unsuitable for state-machine testing.

## Sources used

- https://hexdocs.pm/propcheck/PropCheck.TargetedPBT.html — `exists`, `forall_targeted`, `not_exists`, `minimize`, `maximize`, `user_nf`, `:search_steps`
- https://github.com/alfert/propcheck — `test/targeted_path_test.exs`, `test/targeted_tree_test.exs`, `test/level_tpbt_test.exs`
- https://propertesting.org/ — targeted PBT tutorials

This page reflects `propcheck` 1.5.0.

## What targeted PBT is

Standard PBT generates values **uniformly at random** and checks a boolean property. Targeted PBT instead performs a **guided search** (simulated annealing) over the generator space, optimizing toward a fitness function. It is useful when:

- You want to **find** an input that satisfies a condition (`exists`).
- You want to **maximize** or **minimize** some metric (`maximize`/`minimize`).
- The search space is too large for uniform random to find the interesting region.

StreamData has no equivalent. This is a PropCheck-only capability.

## Core forms

### `exists` — find an input satisfying a condition

```elixir
defmodule TargetedPathTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.TargetedPBT

  # A path is a list of moves: :north, :south, :east, :west
  def path, do: list(elements([:north, :south, :east, :west]))

  def move(:north, {x, y}), do: {x, y + 1}
  def move(:south, {x, y}), do: {x, y - 1}
  def move(:east, {x, y}), do: {x + 1, y}
  def move(:west, {x, y}), do: {x - 1, y}

  property "there exists a path reaching distance >= 10" do
    exists p <- path() do
      {x, y} = Enum.reduce(p, {0, 0}, &move/2)
      maximize(x * x + y * y)
      x * x + y * y >= 100
    end
  end
end
```

The flow:

1. `exists` generates an initial path.
2. `maximize(fitness)` tells the search engine to push the fitness (here, squared distance from origin) upward.
3. The boolean `x*x + y*y >= 100` is the target — the search tries to find a path that satisfies it.
4. With `:search_steps` high enough, the annealer climbs toward longer paths and finds one that reaches distance 10.

### `forall_targeted` — the dual of `exists`

`forall_targeted` is the targeted analogue of `forall`. It is equivalent to `forall_targeted(...) do ...; not fails() end` — i.e. it searches for a counterexample using the fitness function:

```elixir
property "forall_targeted finds a counterexample" do
  forall_targeted n <- nat() do
    minimize(n)
    n != 42
  end
end
```

### `not_exists` — negated `exists`

```elixir
property "no path reaches distance > 1000 in 10 steps" do
  not_exists p <- path() do
    {x, y} = Enum.reduce(p, {0, 0}, &move/2)
    x * x + y * y > 1_000_000
  end
end
```

### `:search_steps` option

The number of simulated-annealing steps. Higher = more thorough search:

```elixir
property "thorough search", search_steps: 200 do
  exists p <- path() do
    {x, y} = Enum.reduce(p, {0, 0}, &move/2)
    maximize(x * x + y * y)
    x * x + y * y >= 100
  end
end
```

## `user_nf` — custom neighborhood function

By default the search engine perturbs the generated value using the generator's own structure. For complex shapes (e.g. trees), you can supply a **neighborhood function** that produces "nearby" values:

```elixir
defmodule TargetedTreeTest do
  use ExUnit.Case, async: true
  use PropCheck
  use PropCheck.TargetedPBT

  def tree do
    let_shrink(subtrees <- list(tree()) do
      oneof([:leaf, {:node, subtrees}])
    end)
  end

  # A neighborhood function: given a tree, produce a list of "nearby" trees.
  def user_nf({:node, subtrees}) do
    # remove a subtree, or shrink a subtree
    [
      {:node, tl(subtrees)}
      | Enum.map(subtrees, fn s -> {:node, List.delete(subtrees, s) ++ [s]} end)
    ]
  end
  def user_nf(:leaf), do: [{:node, []}]

  property "maximize tree depth" do
    exists t <- tree() do
      depth = tree_depth(t)
      maximize(depth)
      depth >= 5
    end
  end

  def tree_depth(:leaf), do: 0
  def tree_depth({:node, subtrees}), do: 1 + (subtrees |> Enum.map(&tree_depth/1) |> Enum.max(fn -> 0 end))
end
```

`user_nf` is wired into the `exists`/`forall_targeted` via the `:search_strategy` or by passing the function to the generator — see the `test/targeted_tree_test.exs` for the exact wiring.

## Escaping local optima: `:proper_target.reset()`

Simulated annealing can get stuck in a local optimum. The PropEr target engine exposes `:proper_target.reset()` to restart the search from a fresh random point:

```elixir
# Inside a long-running targeted property, periodically reset
:proper_target.reset()
```

This is an escape hatch, not a default — the engine already incorporates reheating. Use it only when you observe the fitness plateauing.

## Caveats

Targeted PBT is powerful but constrained. From the PropCheck README and `PropCheck.TargetedPBT` docs:

1. **No shrinking / no counterexamples.** Targeted search does not produce a minimized counterexample the way standard PBT does. If `forall_targeted` finds a counterexample, it is the value the search happened to be at, not a shrunk one.

2. **No `collect`/`aggregate`.** The distribution-inspection macros do not work in targeted mode — the search is guided, not random, so distribution statistics would be misleading.

3. **Does NOT compose with state machines.** From the PropCheck README:

   > Targeted PBT renders the approach unusable for state-based PBT.

   The state-machine `commands/1` model assumes a random command sequence; targeted search over command sequences does not produce a valid linearizable trace. Do not combine `TargetedPBT` with `StateM`/`ModelDSL`/`parallel_commands`.

4. **Fitness function is critical.** A poor fitness function (e.g. one with no gradient) will cause the search to wander. Spend time designing `maximize`/`minimize` to reflect what you actually want to optimize.

## When to use targeted PBT

- **Optimization problems** — find the input that maximizes/minimizes a metric (path distance, tree depth, allocation cost).
- **Existence proofs** — find *an* input satisfying a complex condition where uniform random would rarely hit it.
- **Stress testing** — push a system toward its edge cases by maximizing load/size.

## When NOT to use targeted PBT

- **Stateful systems** — use `StateM`/`ModelDSL` instead (see [`propcheck-state-machine.md`](propcheck-state-machine.md)).
- **Distribution inspection** — use standard `forall` + `collect`/`aggregate`.
- **When you need a minimized counterexample** — use standard `forall`; targeted mode does not shrink.

## Related docs

- [`propcheck-state-machine.md`](propcheck-state-machine.md) — state-machine and parallel testing (the other PropCheck-only capability).
- [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) — capability comparison.
