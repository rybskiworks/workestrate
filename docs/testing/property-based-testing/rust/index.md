# Property-Based Testing in Rust

This corpus is an operational reference for property-based testing (PBT) in Rust. It covers the two mainstream crates in the Rust ecosystem — `proptest` and `quickcheck` — from setup through generators, shrinking, state-machine testing, and async support. Each topic doc is self-contained, runnable, and pinned to a specific crate version so the examples remain reproducible.

## Crate map

| crate | version | model | shrinking | state machine | async | status |
| --- | --- | --- | --- | --- | --- | --- |
| `proptest` | 1.11.0 | per-value `Strategy`/`ValueTree` | integrated | yes, via `proptest-state-machine` | gap — see [async](./proptest-async.md) | "passive maintenance" but still releasing |
| `quickcheck` | 1.1.0 | per-type `Arbitrary` typeclass | manual (`Arbitrary::shrink`, defaults to empty) | no | no | maintained |

## Doc map

- [proptest: Overview, Setup, and Configuration](./proptest-overview.md) — profile, Cargo setup, the `proptest!` macro, `ProptestConfig`, and failure-persistence regressions.
- [proptest: Generators — Strategies and ValueTrees](./proptest-generators.md) — the per-value two-step model, primitive/collection/enum/struct strategies, combinators, recursive JSON AST.
- [proptest: Integrated Shrinking](./proptest-shrinking.md) — the `ValueTree` trait, `simplify`/`complicate`, the binary-search model, shrinking traces, the `prop_filter` sharp edge.
- [proptest: State Machine Testing](./proptest-state-machine.md) — `proptest-state-machine` 0.8.0, model vs system under test, transitions, shrinking of command sequences.
- [proptest: Async Gap and Bridges](./proptest-async.md) — why proptest has no first-class async, the `PropertyTest` bridges, `tokio` runtime patterns.
- [proptest vs quickcheck](./proptest-vs-quickcheck.md) — side-by-side comparison of model, shrinking, ergonomics, and when to pick each.
- [quickcheck](./quickcheck.md) — the `quickcheck` 1.1.0 crate, `Arbitrary` typeclass, `quickcheck_macros`, manual shrinking.

## Recommendation

For serious property-based testing in Rust, **proptest is the primary recommendation in this corpus**. It offers integrated (per-value) shrinking that composes across generators, a rich combinator library (`prop_map`, `prop_flat_map`, `prop_compose!`, `prop_recursive`), failure-seed persistence that catches regressions in CI, and a dedicated state-machine testing crate. Reach for **quickcheck** when you want a minimal, classic PBT loop, when you are already familiar with Haskell's QuickCheck, or when a per-type `Arbitrary` typeclass fits your domain better than per-value strategies.

## Sources used

- [proptest Book](https://proptest-rs.github.io/proptest/) — canonical reference for the proptest crate.
- [docs.rs/proptest](https://docs.rs/proptest) — API docs for `Strategy`, `ValueTree`, `ProptestConfig`.
- [docs.rs/quickcheck](https://docs.rs/quickcheck) — API docs for the `quickcheck` crate.
- [crates.io/crates/proptest](https://crates.io/crates/proptest) — version, license, download counts.
- [crates.io/crates/quickcheck](https://crates.io/crates/quickcheck) — version, license, download counts.
