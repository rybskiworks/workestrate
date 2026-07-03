# proptest vs quickcheck

Both `proptest` and `quickcheck` are Rust property-based testing frameworks, but they differ fundamentally in generation model and shrinking. proptest is **per-value** with integrated shrinking; quickcheck is **per-type** (the `Arbitrary` typeclass) with manual shrinking.

## Side-by-side

| dimension | proptest | quickcheck |
| --- | --- | --- |
| generation model | per-value `Strategy`/`ValueTree` | per-type `Arbitrary` trait |
| shrinking | integrated (ValueTree drives it) | manual `Arbitrary::shrink`; default is empty |
| combinators | rich (`prop_map`, `prop_flat_map`, `prop_compose!`, `prop_oneof!`, `prop_recursive`) | `Gen`-based, fewer combinators |
| state machine | yes (`proptest-state-machine`, sequential) | no |
| async | gap (issue #179); use `test-strategy` | no |
| failure persistence | yes (`proptest-regressions/`) | no |
| min/max test count config | `ProptestConfig` | `QuickCheck::new().tests(N)` |
| deterministic replay | `PROPTEST_RNG_SEED` | `Gen::from_size_and_seed` |
| param count limit | none practical | ≤8 (macro limit) |
| maintenance | passive but releasing | maintained |
| heritage | original Rust design (MacIver) | direct Haskell QuickCheck port (BurntSushi) |

## proptest's own comparison

The proptest README positions itself against quickcheck on four axes:

- **Integrated shrinking** — shrinks are always correct w.r.t. the generator because the same `ValueTree` that generated the value drives its shrinking. With quickcheck's manual `shrink`, the user must hand-write a shrinker that stays consistent with the generator.
- **Richer combinators** — `prop_map`, `prop_flat_map`, `prop_compose!`, `prop_oneof!`, `prop_recursive` make dependent and recursive generation ergonomic.
- **Failure persistence** — failing seeds are remembered in `proptest-regressions/` and replayed on the next run.
- **State machine support** — `proptest-state-machine` provides sequential stateful property testing; quickcheck has none.

quickcheck is simpler and familiar to Haskell users porting QuickCheck properties.

## Integrated shrinking (de Vries / Well-Typed)

Well-Typed's blog post on integrated shrinking states the core idea:

> "Integrated shrinking... the shrinking is driven by the same structure that generated the value."

The consequence for quickcheck is sharp: with manual `Arbitrary::shrink`, the user must hand-write a shrinker that stays consistent with the generator, and the **default `shrink` returns `empty_shrinker()` — i.e. NO shrinking**. Failing cases stay large, and the burden of writing correct shrinkers falls on every user. See [quickcheck](./quickcheck.md) for the `empty_shrinker` default.

## When to choose each

**Choose proptest when:**

- you need shrinking (and you almost always do — unshrunk counterexamples are useless for debugging);
- you need state machine testing ([state machine](./proptest-state-machine.md));
- you need failure persistence across runs;
- you need complex dependent generation (`prop_flat_map`, `prop_compose!`);
- you are doing production-grade PBT.

**Choose quickcheck when:**

- you want the minimal/classic API;
- you are porting Haskell QuickCheck properties verbatim;
- the dependency footprint must stay tiny;
- you are comfortable writing `shrink` by hand for every `Arbitrary` impl.

## Recommendation

proptest for serious work. This corpus's primary recommendation is proptest; quickcheck is documented for completeness and for users porting Haskell properties.

## See also

- [overview](./proptest-overview.md) — proptest profile, setup, macro, config.
- [quickcheck](./quickcheck.md) — the quickcheck crate.
- [generators](./proptest-generators.md) — `Strategy`/`ValueTree` model and combinators.
- [shrinking](./proptest-shrinking.md) — integrated shrinking via `ValueTree`.

## Sources used

- [github.com/proptest-rs/proptest](https://github.com/proptest-rs/proptest) — README comparison of proptest vs quickcheck.
- [well-typed.com/blog/2019/05/integrated-shrinking/](https://well-typed.com/blog/2019/05/integrated-shrinking/) — integrated shrinking theory (de Vries / Well-Typed).
- [docs.rs/proptest](https://docs.rs/proptest) — `Strategy`, `ValueTree`, combinators.
- [docs.rs/quickcheck](https://docs.rs/quickcheck) — `Arbitrary`, `Gen`, `empty_shrinker`.
