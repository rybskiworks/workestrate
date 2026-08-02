# StreamData vs PropCheck

## Purpose

Document the complementary relationship between StreamData and PropCheck, with a side-by-side capability table, guidance on when to choose each, and notes on the unsupported `check_spec` and PropCheck's `.ctex` counterexample persistence.

## Sources used

- https://hexdocs.pm/stream_data/ — StreamData library home
- https://github.com/whatyouhide/stream_data — StreamData README (the deferral quote)
- https://hexdocs.pm/propcheck/ — PropCheck library home
- https://github.com/alfert/propcheck — PropCheck README, `mix propcheck.clean`
- https://propertesting.org/ — background on QuickCheck/PropEr-style testing

This page reflects `stream_data` 1.3.0 and `propcheck` 1.5.0.

## The complementary relationship

StreamData and PropCheck are **complementary, not competing**. They share no code and no generator model. StreamData is the primary, ExUnit-native PBT library for stateless properties; PropCheck is the library for the things StreamData deliberately does not do.

The StreamData README makes the deferral explicit:

> "StreamData doesn't support stateful testing at the moment. For stateful testing, take a look at PropCheck."

This corpus treats that deferral as authoritative: use StreamData by default, reach for PropCheck when the problem is stateful, concurrent, or optimization-shaped.

## Side-by-side capability table

| Capability | StreamData 1.3.0 | PropCheck 1.5.0 |
|---|---|---|
| **License** | Apache-2.0 | GPL-3.0 |
| **Hex package name** | `stream_data` | `propcheck` (one word) |
| **Wraps** | (native Elixir) | Erlang PropEr 1.5.0 |
| **Maintainer(s)** | José Valim, Andrea Leopardi | Klaus Alfert (sole) |
| **ExUnit-native property macro** | ✔ `ExUnitProperties.property/3` | ✔ `PropCheck.property/3` |
| **Integrated shrinking** | ✔ (Hedgehog-style, per-generator) | ✔ (PropEr-style, per-generator) |
| **Generators usable outside tests** | ✔ (implement `Enumerable`) | ✗ (PropEr generators) |
| **State-machine testing** | ✗ | ✔ (`StateM`, `ModelDSL`, `FSM`) |
| **Parallel testing** | ✗ | ✔ (limited; 2-process linearizability) |
| **Targeted PBT (simulated annealing)** | ✗ | ✔ (`TargetedPBT`) |
| **Counterexample persistence** | seed only | `.ctex` file + seed |
| **Counterexample shrinking** | ✔ (integrated) | ✔ (integrated; not in targeted mode) |
| **`collect`/`aggregate` distribution** | ✗ | ✔ |
| **Adoption (downloads)** | ~36M | ~1.6M |
| **Adoption (Hex dependents)** | many | few (~3) |

## When to choose StreamData

Choose StreamData when:

- The property is **stateless** — a pure function from input to output.
- You want generators that double as **ordinary streams** (REPL exploration, sample data generation, fuzzing outside tests).
- You want **Apache-2.0** licensing (no GPL constraints on the test dependency).
- You are already in an ExUnit suite and want the lightest possible integration (`use ExUnitProperties`).
- You need broad ecosystem adoption and long-term maintenance confidence (Valim + Leopardi, 36M downloads).

See [`streamdata-getting-started.md`](streamdata-getting-started.md) for setup.

## When to choose PropCheck

Choose PropCheck when:

- The system under test is **stateful** — a server, cache, registry, or any process with mutable state. Use `StateM` or `ModelDSL` (see [`propcheck-state-machine.md`](propcheck-state-machine.md)).
- You need to test **concurrency** — race conditions, linearizability. Use `parallel_commands/1` + `Instrument` (see [`propcheck-state-machine.md`](propcheck-state-machine.md) → Parallel testing).
- You are doing **optimization** — find the input that maximizes/minimizes a metric. Use `TargetedPBT` (see [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md)).
- You want **distribution inspection** via `collect`/`aggregate`.
- GPL-3.0 on a test-only dependency is acceptable for your project.

## When to use both

A non-trivial Elixir codebase often has both stateless and stateful properties. The two libraries coexist without conflict — they have different module namespaces (`StreamData.*`, `ExUnitProperties.*` vs `PropCheck.*`) and different generator models. A single test module uses one or the other (not both), but a single test suite can mix modules freely.

## `check_spec` is unsupported

PropCheck ships a `mix check_spec` task that attempts to test functions against their `@spec` typespecs. **It is officially unsupported.** From the PropCheck docs:

> `check_spec` is not supported because PropEr cannot parse Elixir source.

The underlying issue is that PropEr's spec-checking machinery expects Erlang `-spec` syntax and parses Erlang AST; Elixir's `@spec` attributes are compiled to Erlang specs but the PropEr integration does not reliably consume them. Do not rely on `check_spec` as a CI gate. For typespec checking, use Dialyzer (see [`docs/elixir/typespecs-and-dialyzer.md`](../../../elixir/typespecs-and-dialyzer.md)).

## Counterexample persistence

### StreamData: seed only

When a StreamData property fails, ExUnit prints the seed:

```
Randomized seed: 123456789
```

To reproduce, set `:initial_seed` in `check all` (or the `:seed` ExUnit config) to that integer. There is no separate counterexample file — the failing value is regenerated from the seed.

### PropCheck: `.ctex` file + seed

PropCheck persists counterexamples to a `.ctex` file alongside the test. On the next run, PropCheck replays the stored counterexample first (a regression check) before generating new cases. The file is named after the test module/property.

To clear stale counterexamples (e.g. after a fix is confirmed), use:

```bash
mix propcheck.clean
```

`mix propcheck.clean` removes all `.ctex` files in the project. It is the PropCheck analogue of "clear the snapshot" — run it after a counterexample-driven fix is verified to stick.

## Adoption and maintenance notes

- **StreamData** is co-owned by José Valim (Elixir's creator) and Andrea Leopardi. It has ~36M downloads and many Hex dependents. It is the de facto Elixir PBT library.
- **PropCheck** is maintained solely by Klaus Alfert. It has ~1.6M downloads and few dependents (~3). It is the only Elixir library with state-machine, parallel, and targeted PBT, but its bus factor is lower.

This is not a quality judgment — PropCheck's narrow scope (stateful/parallel/targeted) is exactly why StreamData defers to it. The two are designed to be used together.

## Decision flowchart

```
Is the property stateless (pure function)?
├─ Yes → StreamData
└─ No → Is the system under test stateful?
        ├─ Yes, sequential → PropCheck StateM / ModelDSL
        ├─ Yes, concurrent → PropCheck parallel_commands + Instrument
        └─ No (optimization-shaped) → PropCheck TargetedPBT
```

## Related docs

- [`streamdata-getting-started.md`](streamdata-getting-started.md) — StreamData setup.
- [`streamdata-combinators.md`](streamdata-combinators.md) — StreamData generators.
- [`streamdata-shrinking.md`](streamdata-shrinking.md) — StreamData shrinking.
- [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) — StreamData + ExUnit.
- [`propcheck-state-machine.md`](propcheck-state-machine.md) — PropCheck state-machine and parallel testing.
- [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md) — PropCheck targeted PBT.
- [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md) — ExUnit itself.
- [`docs/elixir/typespecs-and-dialyzer.md`](../../../elixir/typespecs-and-dialyzer.md) — typespec checking (the supported alternative to `check_spec`).
