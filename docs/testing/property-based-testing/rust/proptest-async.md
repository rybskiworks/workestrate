# proptest and Async Tests

## The async gap

Core `proptest!` has **no async arm**. The `proptest!` macro expands to a synchronous `#[test]` fn; you cannot write `async fn` inside it. This is tracked in the long-standing issue:

- [github.com/proptest-rs/proptest/issues/179](https://github.com/proptest-rs/proptest/issues/179) — *"Support async functions"*, open since 2020, with no resolution.

The issue's intent is to allow `#[proptest(async = "...")]`-style ergonomics inside proptest itself. Until that lands, you must use a bridge crate or drive the runtime manually.

## test-strategy 0.4.5 — the maintained bridge

`test-strategy` is the only maintained crate that bridges proptest and async. It provides a `#[proptest]` attribute macro that accepts an `async = "tokio"` (or `async = "async-std"`) argument and is a near-drop-in replacement for the `proptest!` macro style.

```toml
[dev-dependencies]
proptest = "1.11.0"
test-strategy = "0.4.5"
tokio = { version = "1", features = ["macros", "rt"] }
```

Runnable example:

```rust
use test_strategy::proptest;

#[proptest(async = "tokio")]
async fn my_test_async(#[strategy(0i32..1000)] n: i32) {
    let result = some_async_fn(n).await;
    assert!(result >= 0);
}

async fn some_async_fn(n: i32) -> i32 {
    n
}
```

| element | role |
| --- | --- |
| `#[proptest(async = "tokio")]` | wrap the async fn in a tokio runtime and run it as a proptest case |
| `#[strategy(0i32..1000)]` | annotate each parameter with the `Strategy` that generates it (replaces `proptest!`'s `name in strategy` syntax) |

`test-strategy` also supports `#[proptest]` *without* `async` for synchronous properties, so it can replace `proptest!` entirely if you prefer the attribute style.

## Manual workaround

For users who cannot adopt `test-strategy`, the workaround from issue #179 is to drive the async runtime manually inside a synchronous `proptest!` block:

```rust
use proptest::prelude::*;
use tokio::runtime::Runtime;

proptest! {
    #[test]
    fn async_inside_sync(n in 0i32..1000) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let result = some_async_fn(n).await;
            assert!(result >= 0);
        });
    }
}

async fn some_async_fn(n: i32) -> i32 {
    n
}
```

This works but loses `test-strategy`'s ergonomic attribute form and pays a runtime-creation cost per case. For high case counts, hoist the `Runtime` out of the `proptest!` body into a `OnceLock` or thread-local.

## `proptest-async` 0.2.1 — do NOT use

There is a crate `proptest-async` but it is **stale** (last published ~0.2.1, unmaintained). Do not use it. Prefer `test-strategy` or the manual `block_on` workaround above.

| crate | status | recommendation |
| --- | --- | --- |
| `test-strategy` | maintained, 0.4.5 | use this |
| `proptest-async` | stale, ~0.2.1 | avoid |

## See also

- [overview](./proptest-overview.md) — proptest profile, setup, macro, config.

## Sources used

- [github.com/proptest-rs/proptest/issues/179](https://github.com/proptest-rs/proptest/issues/179) — the async gap, open since 2020.
- [docs.rs/test-strategy](https://docs.rs/test-strategy) — `#[proptest(async = "tokio")]` and `#[strategy(...)]` attribute.
- [crates.io/crates/test-strategy](https://crates.io/crates/test-strategy) — version, maintenance status.
- [crates.io/crates/proptest-async](https://crates.io/crates/proptest-async) — stale, ~0.2.1.
