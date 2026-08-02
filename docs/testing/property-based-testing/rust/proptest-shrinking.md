# proptest: Integrated Shrinking

## Intro

proptest uses **integrated** shrinking: the shrinking logic lives in the per-value `ValueTree`, not in a per-type `shrink` function. This is the central architectural contrast with `quickcheck`, whose `Arbitrary::shrink` is a separate, manually-written per-type method (see [quickcheck](./quickcheck.md) and [proptest vs quickcheck](./proptest-vs-quickcheck.md)).

> "Integrated shrinking means that the shrinking is driven by the same structure that generated the value in the first place."

Because the `ValueTree` is produced by the same `Strategy` that generated the value, shrinking composes automatically: a `Vec<T>` strategy shrinks both the length and each element's `ValueTree` without the user writing any shrink logic.

## The `ValueTree` trait

From docs.rs/proptest, the trait is:

```rust
pub trait ValueTree {
    type Value;
    fn current(&self) -> Self::Value;
    fn simplify(&mut self) -> bool;
    fn complicate(&mut self) -> bool;
}
```

- `current` returns the value at the current position in the shrink tree.
- `simplify` moves toward a "smaller" value; returns `true` if it made progress.
- `complicate` walks back toward the original after a subtree was pruned; returns `true` if it moved toward the original.

The proptest runner drives `simplify`/`complicate` to find the minimal value that still fails the property.

## The binary-search model

The canonical shrinking model for an integer `ValueTree` holds three bounds: `low`, `current`, `high`. `simplify` moves `current` toward `low` (midpoint); `complicate` moves back toward `high`. This gives O(log n) shrinking toward the minimal failing value.

```text
initial: low=0, current=1000, high=1000
simplify -> current=500  (low=0, high=1000)
simplify -> current=250  (low=0, high=500)
... converges toward 0
```

## Shrinking trace for the date-parser

Consider a property that fails for some `(year, month, day)` triple. proptest interleaves `simplify`/`complicate` across the three component `ValueTree`s to find a minimal counterexample:

```text
failing case: (y=2024, m=12, d=31)
shrink year: 2024 -> 1012 -> 506 -> ... -> 0   (year 0 still fails)
shrink month: 12 -> 6 -> ... -> 10             (month 10 fails; <10 does not)
shrink day: 31 -> 15 -> ... -> 1               (day 1 still fails)
minimal: (y=0, m=10, d=1)
```

The runner shrinks one dimension at a time, re-running the property after each step. When a `simplify` step makes the property pass, `complicate` walks back to the last failing value, so the final reported case is the minimal failing combination.

## The `prop_filter` sharp edge

Filtering interacts poorly with shrinking. When a `ValueTree` is wrapped in `prop_filter`, the filter rejects shrunk values that no longer satisfy the predicate, which can stall the shrink walk:

> "Filtering interacts poorly with shrinking... proptest may not be able to shrink past the filter."

Prefer `prop_flat_map` / `prop_compose` dependent generation over filtering wherever possible — dependent generation produces only valid values, so shrinking never has to fight a filter.

## See also

- [generators](./proptest-generators.md) — `Strategy`/`ValueTree` model and combinators.
- [overview](./proptest-overview.md) — setup, macro, config, failure persistence.

## Sources used

- [proptest Book — Shrinking basics](https://proptest-rs.github.io/proptest/proptest/shrinking-basics.html) — `ValueTree`, `simplify`/`complicate`, the `prop_filter` warning.
- [docs.rs/proptest](https://docs.rs/proptest) — `ValueTree` trait definition.
- [well-typed.com — Integrated Shrinking](https://well-typed.com/blog/2019/05/integrated-shrinking/) (Edsko de Vries) — the integrated-shrinking rationale quoted above.
- [doc.ic.ac.uk/~maciver/](https://doc.ic.ac.uk/~maciver/) (Neil Mitchell / MacIver, ECOOP 2020 — "What is the essence of a shrinking strategy?") — foundational shrinking-strategy theory.
