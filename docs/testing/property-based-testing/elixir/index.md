# Elixir Property-Based Testing

## Purpose

This corpus is the Elixir home for **property-based testing (PBT)**. It elevates property-based testing from a "reference only" footnote in `docs/elixir/source-map.md` to a first-class, multi-document reference. Two complementary libraries are covered:

- **StreamData** (`stream_data` 1.3.0) — the primary, ExUnit-native PBT library. Generators are lazy streams (they implement `Enumerable`), so they are usable both inside `ExUnitProperties.property/3` tests and outside tests as ordinary streams. Apache-2.0 licensed, co-owned by José Valim and Andrea Leopardi.
- **PropCheck** (`propcheck` 1.5.0) — the complementary library for the things StreamData deliberately does not do: **state-machine testing**, **parallel testing**, and **targeted PBT**. PropCheck wraps Erlang PropEr 1.5.0. GPL-3.0 licensed, maintained by Klaus Alfert.

StreamData and PropCheck are **complementary, not competing**. The StreamData README explicitly defers stateful testing to PropCheck. The two libraries share no code and no generator model; they are chosen per problem shape, not per team allegiance.

## Sources used

- https://hexdocs.pm/stream_data/ (StreamData library home)
- https://hexdocs.pm/stream_data/ExUnitProperties.html — `ExUnitProperties` ExUnit integration
- https://hexdocs.pm/stream_data/StreamData.html — generator combinator reference
- https://github.com/whatyouhide/stream_data — StreamData source and README
- https://hexdocs.pm/propcheck/ (PropCheck library home)
- https://hexdocs.pm/propcheck/PropCheck.StateM.html — state-machine behaviour
- https://hexdocs.pm/propcheck/PropCheck.StateM.ModelDSL.html — recommended state-machine DSL
- https://hexdocs.pm/propcheck/PropCheck.TargetedPBT.html — targeted PBT
- https://github.com/alfert/propcheck — PropCheck source and README
- https://propertesting.org/ — PropEr/QuickCheck-style testing background

This page reflects `stream_data` 1.3.0 and `propcheck` 1.5.0.

## Relationship to ExUnit

ExUnit itself is documented in [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md). That document covers unit testing, assertions, callbacks, capture helpers, doctests, tags, and coverage. It mentions StreamData only as a "separate library" footnote. **Property-based testing is out of scope for the ExUnit doc and in scope here.** StreamData integrates with ExUnit (via `ExUnitProperties`) but is a separate Hex dependency; PropCheck likewise integrates with ExUnit but is a separate dependency with a different generator model.

## The complementary relationship

The defining design choice is that StreamData **does not** implement state-machine, parallel, or targeted PBT. From the StreamData README:

> "StreamData doesn't support stateful testing at the moment. For stateful testing, take a look at PropCheck."

PropCheck fills exactly those gaps. The table in [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) summarizes the side-by-side capabilities. The short version:

| Capability | StreamData 1.3.0 | PropCheck 1.5.0 |
|---|---|---|
| License | Apache-2.0 | GPL-3.0 |
| Integrated shrinking | ✔ (Hedgehog-style) | ✔ (PropEr-style) |
| ExUnit-native property macro | ✔ (`ExUnitProperties.property/3`) | ✔ (`PropCheck.property/3`) |
| Generators usable outside tests | ✔ (Enumerable streams) | ✗ (PropEr generators) |
| State-machine testing | ✗ | ✔ (`StateM`, `ModelDSL`, `FSM`) |
| Parallel testing | ✗ | ✔ (limited; 2-process linearizability) |
| Targeted PBT (simulated annealing) | ✗ | ✔ (`TargetedPBT`) |
| Counterexample persistence | seed only | `.ctex` file + seed |
| Adoption (downloads / dependents) | ~36M / many | ~1.6M / few |

## Document map

| Document | Covers | Library |
|---|---|---|
| [`streamdata-getting-started.md`](streamdata-getting-started.md) | Setup, `use ExUnitProperties`, `property/3`, `check all`, options, generators-as-streams, `pick/1` | StreamData |
| [`streamdata-combinators.md`](streamdata-combinators.md) | Full combinator library: numeric, binary/string, containers, choice, `bind/2`, `map/2`, `filter/3`, size control, `tree/2`, `atom(:alphanumeric)`, `term/0`, `date/0`, wrappers | StreamData |
| [`streamdata-shrinking.md`](streamdata-shrinking.md) | Integrated per-generator shrinking, shrink directions, `unshrinkable/1`, `check_all/3`, project config | StreamData |
| [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) | `ExUnitProperties`, `property/3` as `ExUnit.Case.test/3`, `async: true` | StreamData |
| [`propcheck-state-machine.md`](propcheck-state-machine.md) | `StateM`, `ModelDSL` (recommended), `FSM`, sequential + **parallel** state-machine testing, `BasicTypes`, `let`/`such_that`/`let_shrink` | PropCheck |
| [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md) | `TargetedPBT`: `exists`, `forall_targeted`, `minimize`/`maximize`, `user_nf`, `:search_steps`, caveats | PropCheck |
| [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) | Side-by-side comparison, when to choose each, `check_spec` (unsupported), `.ctex` persistence, `mix propcheck.clean` | both |

## Reading paths

### New to property-based testing in Elixir

1. [`streamdata-getting-started.md`](streamdata-getting-started.md) — install StreamData, write a first property.
2. [`streamdata-combinators.md`](streamdata-combinators.md) — learn the generator vocabulary.
3. [`streamdata-shrinking.md`](streamdata-shrinking.md) — understand how counterexamples are minimized.
4. [`streamdata-exunit-integration.md`](streamdata-exunit-integration.md) — wire properties into an ExUnit suite.

### Need stateful, parallel, or targeted testing

1. [`streamdata-vs-propcheck.md`](streamdata-vs-propcheck.md) — confirm PropCheck is the right tool.
2. [`propcheck-state-machine.md`](propcheck-state-machine.md) — state-machine and parallel testing.
3. [`propcheck-targeted-pbt.md`](propcheck-targeted-pbt.md) — targeted (optimization-style) PBT.

## Conventions used in every file

Each topic file follows the corpus pattern:

1. **Purpose** — what the file covers.
2. **Sources used** — the HexDocs / GitHub pages consulted, with version notes.
3. **Runnable examples** — every code block is runnable Elixir lifted from HexDocs or the library README/test suite. No invented APIs.
4. **Cross-references** — to sibling files and to `docs/elixir/testing-exunit.md`.

## Version pins

All examples in this corpus are pinned to:

- `stream_data` **1.3.0** (March 2026 release line)
- `propcheck` **1.5.0** (hex package name is `propcheck`, one word — **not** `prop_check`)
- PropCheck wraps Erlang **PropEr 1.5.0**

### StreamData API notes (1.3.0)

Several functions that are commonly assumed to exist **do not exist** in StreamData 1.3.0. This corpus uses the documented substitutes exclusively:

| Does NOT exist | Use instead |
|---|---|
| `nonempty_list_of/1` | `nonempty(list_of(...))` |
| `atom/0` | `atom(:alphanumeric)` |
| `atom_of/1` | `member_of([...])` of atoms, or `atom(:alphanumeric)` |
| `tree/1` | `tree/2` (takes a leaf generator and a node combinator) |
| `bind/3` | `bind/2` (the generator + a function returning a generator) |
| `sequence/1` | `gen all` (the `ExUnitProperties.gen all` special form) |
| `any/0` | `term/0` |
| `time/0`, `datetime/0` | `date/0` (1.3.0); for time/datetime compose from primitives |
| `StreamData.Let` | `gen all` |

## Related indexes

- [Elixir Guidance Index](../../../elixir/index.md) — the parent Elixir corpus; ExUnit itself lives at [`docs/elixir/testing-exunit.md`](../../../elixir/testing-exunit.md).
- Property-based testing corpus shared files (top-level `docs/testing/property-based-testing/`) are maintained by a separate lead.
