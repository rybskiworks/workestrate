# proptest: Overview, Setup, and Configuration

## Profile

| field | value |
| --- | --- |
| crate | `proptest` |
| version | 1.11.0 |
| released | Mar 2026 |
| license | MIT OR Apache-2.0 |
| downloads | ~146M |
| maintenance | "passive maintenance" but still releasing |
| maintainer | proptest-rs org |

proptest is the canonical property-based testing crate for Rust. It is per-value (not per-type): each generator is a `Strategy` that produces a `ValueTree` holding both the current value and the ability to shrink it. See [generators](./proptest-generators.md) and [shrinking](./proptest-shrinking.md).

## Setup

```toml
[dev-dependencies]
proptest = "1.11.0"
```

## The `proptest!` macro

The `proptest!` macro expands a property into a `#[test]` function that runs the body against many generated inputs. The canonical first example from the proptest Book is the "doesn't crash" date-parser test:

```rust
use proptest::prelude::*;

/// Parses a "YYYY-MM-DD" string into (year, month, day).
/// Returns None for any input that is not a valid date in that shape.
fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut parts = s.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if (1..=12).contains(&m) && (1..=31).contains(&d) {
        Some((y, m, d))
    } else {
        None
    }
}

proptest! {
    #[test]
    fn doesnt_crash(s in "\\PC*") {
        parse_date(&s);
    }
}
```

`"\\PC*"` is a regex strategy matching any sequence of non-control characters. The test asserts only that `parse_date` does not panic — the classic "parse, don't crash" property.

## `ProptestConfig`

Configuration is supplied via the `#![proptest_config(...)]` inner attribute inside the `proptest!` block:

```rust
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

/// Parses a "YYYY-MM-DD" string into (year, month, day).
fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut parts = s.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if (1..=12).contains(&m) && (1..=31).contains(&d) {
        Some((y, m, d))
    } else {
        None
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn parses_valid_components(year in 0i32..10000, month in 1u32..13, day in 1u32..32) {
        let encoded = format!("{year:04}-{month:02}-{day:02}");
        prop_assert_eq!(parse_date(&encoded), Some((year, month, day)));
    }
}
```

`ProptestConfig::with_cases(1000)` raises the default 256 cases to 1000.

### `ProptestConfig` field table

| field | default | env var | meaning |
| --- | --- | --- | --- |
| `cases` | 256 | `PROPTEST_CASES` | number of successful test cases to run |
| `max_shrink_iters` | `u32::MAX` (effective limit: four times `cases`) | `PROPTEST_MAX_SHRINK_ITERS` | max shrinking iterations per failing case |
| `failure_persistence` | `FileFailurePersistence::SourceParallel("proptest-regressions")` | no persistence-path environment variable | where to persist failing seeds; `PROPTEST_DISABLE_FAILURE_PERSISTENCE` disables it |
| `fork` | `false` | `PROPTEST_FORK` | run each test in a subprocess |
| `timeout` | 0 (disabled) | `PROPTEST_TIMEOUT` | per-test timeout in ms (requires `fork`) |
| `rng_seed` | random | `PROPTEST_RNG_SEED` | RNG seed for deterministic replay |

## Failure persistence

proptest persists failing seeds in a `proptest-regressions/` directory. On the next run it replays those seeds before new random cases. A seed depends on the generator: preserve a concrete minimized fixture as well when it demonstrates a defect. Shrinking finds a smaller failing case, not necessarily a globally minimal one.

> "proptest remembers the seed... and replays it on the next run"

Recommend checking `proptest-regressions/` into version control so CI catches regressions. Do **not** add it to `.gitignore`:

```text
# .gitignore — do NOT ignore this:
# proptest-regressions/
```

## See also

- [generators](./proptest-generators.md) — `Strategy`/`ValueTree` model and combinators.
- [shrinking](./proptest-shrinking.md) — integrated shrinking via `ValueTree`.
- [state machine](./proptest-state-machine.md) — `proptest-state-machine` 0.8.0.
- [async](./proptest-async.md) — the async gap and bridges.

## Sources used

- [proptest Book](https://proptest-rs.github.io/proptest/) — macro, config, failure persistence.
- [docs.rs/proptest](https://docs.rs/proptest) — `ProptestConfig` fields and env vars.
- [crates.io/crates/proptest](https://crates.io/crates/proptest) — version, license, download counts.
