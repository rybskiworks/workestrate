# quickcheck

## Profile

| field | value |
| --- | --- |
| crate | `quickcheck` |
| version | 1.1.0 |
| license | MIT OR Unlicense |
| downloads | ~59M |
| maintainer | BurntSushi |
| heritage | direct port of Haskell QuickCheck |
| generation | per-type `Arbitrary` typeclass |
| shrinking | MANUAL (`Arbitrary::shrink`; default is empty) |

`quickcheck` is the direct Rust port of Haskell QuickCheck. Generation is per-type: each type implements the `Arbitrary` typeclass, and properties receive randomly-generated values of that type. Shrinking is manual — the user must override `Arbitrary::shrink` to get any shrinking at all; the default returns `empty_shrinker()` (no shrinking). This is the key quickcheck weakness; see [proptest vs quickcheck](./proptest-vs-quickcheck.md).

## Setup

```toml
[dev-dependencies]
quickcheck = "1"
quickcheck_macros = "1"
```

## The `#[quickcheck]` attribute macro

The attribute form. Runnable:

```rust
use quickcheck_macros::quickcheck;

#[quickcheck]
fn double_reversal_is_identity(xs: Vec<isize>) -> bool {
    let rev: Vec<isize> = xs.iter().rev().cloned().collect();
    let revrev: Vec<isize> = rev.iter().rev().cloned().collect();
    xs == revrev
}
```

The `#[quickcheck]` macro inspects the function's parameters, requires each to be `Arbitrary`, generates random values, and calls the function. The function must return `bool` (or `TestResult`).

## The `quickcheck!` macro form

The macro form, useful inside a `#[cfg(test)] mod`:

```rust
#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    quickcheck! {
        fn prop(xs: Vec<u32>) -> bool {
            xs.len() == xs.iter().count()
        }
    }
}
```

## `Arbitrary` impl for a `Point` struct

Hand-composed `Arbitrary` with a hand-written `shrink`:

```rust
use quickcheck::{Arbitrary, Gen};

#[derive(Debug, Clone, PartialEq)]
struct Point { x: i32, y: i32 }

impl Arbitrary for Point {
    fn arbitrary(g: &mut Gen) -> Point {
        Point {
            x: i32::arbitrary(g),
            y: i32::arbitrary(g),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Point>> {
        Box::new(
            self.x.shrink().map(|x| Point { x, y: self.y })
                .chain(self.y.shrink().map(|y| Point { x: self.x, y })),
        )
    }
}
```

The `shrink` impl chains shrinks of `x` (holding `y` fixed) with shrinks of `y` (holding `x` fixed). This is the standard pattern for product types.

## Manual `shrink()` with `empty_shrinker` / `single_shrinker`

```rust
use quickcheck::{empty_shrinker, single_shrinker, Arbitrary, Gen};

#[derive(Debug, Clone)]
struct MyType;

impl Arbitrary for MyType {
    fn arbitrary(_: &mut Gen) -> MyType { MyType }
    fn shrink(&self) -> Box<dyn Iterator<Item = MyType>> {
        // No smaller values to try:
        empty_shrinker()
        // or a single candidate:
        // single_shrinker(MyType)
    }
}
```

The **default `shrink`** (if you do not override it) returns `empty_shrinker()` — i.e. **NO shrinking**. Failing cases stay at whatever size the generator produced. This is the central quickcheck weakness; see [proptest vs quickcheck](./proptest-vs-quickcheck.md).

## `==>`-equivalent: `TestResult::discard()`

In Haskell QuickCheck you use `==>` to discard cases that do not satisfy a precondition. In Rust quickcheck you discard by returning `TestResult::discard()`:

```rust
use quickcheck::{TestResult, quickcheck};

fn prop_even_only(n: u32) -> TestResult {
    if n % 2 == 0 {
        TestResult::from_bool(n * 2 == n + n)
    } else {
        TestResult::discard()
    }
}

quickcheck(prop_even_only as fn(u32) -> TestResult);
```

Discarded cases do not count toward the `tests` target; quickcheck keeps generating until it has enough accepted cases or hits `max_tests`.

## `QuickCheck` builder

For explicit configuration (case count, max attempts), use the `QuickCheck` builder:

```rust
use quickcheck::QuickCheck;

fn prop(xs: Vec<u32>) -> bool { xs.len() == xs.iter().count() }

#[test]
fn with_config() {
    QuickCheck::new()
        .tests(1000)
        .max_tests(10000)
        .quickcheck(prop as fn(Vec<u32>) -> bool);
}
```

| builder method | role |
| --- | --- |
| `.tests(N)` | number of successful cases required |
| `.max_tests(N)` | maximum number of cases to attempt (including discards) before giving up |
| `.quickcheck(prop)` | run the property |

## Deterministic replay with `Gen`

`Gen` can be constructed with a fixed size and seed for deterministic replay. The exact constructor signature has varied across quickcheck releases; consult [docs.rs/quickcheck::Gen](https://docs.rs/quickcheck/latest/quickcheck/struct.Gen.html) for the precise API on your version. The intent is:

```rust
use quickcheck::{Arbitrary, Gen};

// Build a Gen with a fixed size for deterministic generation.
let mut g = Gen::new(10);
// Gen::from_size_and_seed(size, seed) — see docs.rs/quickcheck for the exact
// constructor on your version; the key point is that Gen can be constructed
// with a fixed seed for replay.
let _v: i32 = i32::arbitrary(&mut g);
```

The key point: `Gen` carries the RNG state, so constructing one with a fixed seed makes generation reproducible. Unlike proptest's `PROPTEST_RNG_SEED` env var, there is no built-in failure-persistence mechanism that records and replays failing seeds across runs.

## `NoShrink<A>` wrapper

`NoShrink` wraps an `Arbitrary` to disable shrinking for that value. See [docs.rs/quickcheck](https://docs.rs/quickcheck) for the `NoShrink` type. Useful when shrinking is expensive or produces unhelpful candidates:

```rust
use quickcheck::Arbitrary;

// NoShrink wraps an Arbitrary to disable shrinking for that value.
// (See docs.rs/quickcheck for the NoShrink type.)
```

## Gaps

The quickcheck README documents several deliberate limitations relative to Haskell QuickCheck:

- **No `Coarbitrary`:**

  > "Coarbitrary does not exist in any form."

  So there is no function-generation. You cannot write properties over functions the way Haskell QuickCheck does.

- **No state machine testing.** (See [proptest-state-machine](./proptest-state-machine.md) for the proptest equivalent.)
- **No async support.** (See [proptest-async](./proptest-async.md) for the proptest gap and bridges.)
- **No failure persistence.** Failing seeds are not remembered across runs; you must record and replay them manually.
- **The `#[quickcheck]` macro supports at most 8 parameters.** Beyond that, use the `quickcheck!` macro form or factor inputs into a struct.
- **Stack overflow during generation is NOT caught.** Deeply recursive `Arbitrary` impls can overflow the stack without a useful error.

## See also

- [proptest vs quickcheck](./proptest-vs-quickcheck.md) — comparative axis.
- [overview](./proptest-overview.md) — proptest profile, setup, macro, config.
- [index](./index.md) — corpus overview.

## Sources used

- [github.com/BurntSushi/quickcheck](https://github.com/BurntSushi/quickcheck) — README, gaps, `Coarbitrary` quote.
- [docs.rs/quickcheck](https://docs.rs/quickcheck) — `Arbitrary`, `Gen`, `empty_shrinker`, `single_shrinker`, `NoShrink`, `TestResult`.
- [crates.io/crates/quickcheck](https://crates.io/crates/quickcheck) — version, license, download counts.
- [well-typed.com/blog/2019/05/integrated-shrinking/](https://well-typed.com/blog/2019/05/integrated-shrinking/) — integrated shrinking theory (de Vries / Well-Typed).
