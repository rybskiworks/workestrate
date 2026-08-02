# Testing in Rust

## Purpose

Provide concrete, repo-independent guidance for writing, organizing, and running Rust tests. These rules help agents produce test suites that are reliable, maintainable, and idiomatic. The document covers the built-in test harness (`#[test]`, assertion macros, `cargo test`, doctests), Cargo target configuration, test organization, common ecosystem tooling, and the mistakes that most often waste agent time.

## Sources used

### Seed references

- [The Rust Book, chapter 11: Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [The Rust Book: Writing tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html)
- [The Rust Book: Running tests](https://doc.rust-lang.org/book/ch11-02-running-tests.html)
- [The Rust Book: Test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [Cargo `cargo test` command reference](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Rustdoc documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)

### Primary references

- [Rust Reference: Test attributes](https://doc.rust-lang.org/reference/attributes/testing.html)
- [Rust Reference: Conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [rustc Tests chapter](https://doc.rust-lang.org/rustc/tests/index.html)
- [Cargo reference: Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)
- [Rustdoc: The `doc` attribute](https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html)
- [Cargo reference: Environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html)

## Core guidance

Rust has first-class built-in testing. The compiler and Cargo provide [`#[test]`](https://doc.rust-lang.org/reference/attributes/testing.html), [`#[cfg(test)]`](https://doc.rust-lang.org/reference/conditional-compilation.html), assertion macros, and doctests with no extra tooling for the common case. Prefer the built-in harness until a concrete requirement forces a specialized crate. Tests should be fast, isolated, deterministic, and focused on behavior rather than implementation details.

A test function must be a free function, monomorphic, taking no arguments, and returning a type implementing [`std::process::Termination`](https://doc.rust-lang.org/std/process/trait.Termination.html). In practice this means `()` or `Result<T, E>` where `T: Termination` and `E: Debug`. Pass/fail is decided by `Termination::report()`. Tests are compiled only when the compiler is invoked in test mode (`rustc --test` or `cargo test`).

## Practical rules

### Test attributes

Use the attributes from the [Rust Reference testing attributes](https://doc.rust-lang.org/reference/attributes/testing.html) and the [writing tests chapter](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

| Attribute | Forms | Meaning |
| --- | --- | --- |
| `#[test]` | bare | Marks a free function as a test. Must be monomorphic and take no arguments. |
| `#[ignore]` | bare or `#[ignore = "reason"]` | Skips the test by default. The reason is shown by `--list`. The test is still compiled. |
| `#[should_panic]` | bare, `#[should_panic = "msg"]`, or `#[should_panic(expected = "msg")]` | Test passes only if the function panics. The `expected` form does substring matching against the panic message. |
| `#[cfg(test)]` | bare | Gates the annotated item so it is compiled only in test mode. |

Rules for `#[should_panic]`:

- The test function return type MUST be `()`. `#[should_panic]` is incompatible with `Result<T, E>`-returning tests.
- Use the `expected = "..."` form whenever possible. It performs substring matching: the given string must appear somewhere within the panic message, not exact equality.
- Without `expected`, any panic passes, which often masks the wrong failure.

```rust
#[test]
#[should_panic(expected = "division by zero")]
fn panics_on_zero() {
    divide(10, 0);
}

#[test]
#[ignore = "flaky network dependency"]
fn remote_service_roundtrip() {
    // ...
}
```

### Assertion macros

Use the macros from the [writing tests chapter](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

| Macro | Requirements | Notes |
| --- | --- | --- |
| `assert!(cond)` | `cond: bool` | Optional trailing format args evaluated only on failure. |
| `assert!(cond, "msg {args}")` | `cond: bool` | Use for boolean conditions, not for value comparison. |
| `assert_eq!(left, right)` | `T: PartialEq + Debug` | Uses `==`. On failure prints both operands via `Debug`. Labels are `left:` and `right:`. |
| `assert_ne!(left, right)` | `T: PartialEq + Debug` | Uses `!=`. |
| `debug_assert!` / `debug_assert_eq!` / `debug_assert_ne!` | Same as non-debug counterparts | Elided in optimized builds unless `-C debug-assertions` is enabled. Always type-checked. |

Rules:

- Custom types used with `assert_eq!` or `assert_ne!` must implement `PartialEq` and `Debug`; derive them with `#[derive(PartialEq, Debug)]` when possible.
- Use `assert_eq!`/`assert_ne!` for value comparison so the failure output shows both values. Bare `assert!` loses the diff.
- Failure output uses `left:` and `right:`, not `expected:`/`actual:`. Order does not matter in Rust; do not rely on it.
- `debug_assert!` macros are disabled in release builds by default. Do not use them to guard `unsafe` code; use real `assert!` there.
- `assert*!` macros (without the `debug_` prefix) are always enabled in debug and release.

```rust
#[test]
fn custom_message() {
    let value = 7;
    assert!(
        value % 2 == 0,
        "expected even value, got {value}"
    );
}
```

### Returning `Result<T, E>` from tests

A test function may return `Result<(), E: Debug>`. The test passes on `Ok(())` and fails on `Err`. This enables the `?` operator inside the test body.

Rules:

- Do NOT combine `#[should_panic]` with a `Result`-returning test. It will not compile.
- To assert that a `Result`-returning call produced an error, do NOT use `?`; use `assert!(value.is_err())` or pattern matching.

```rust
#[test]
fn read_config_succeeds() -> std::io::Result<()> {
    let config = std::fs::read_to_string("fixtures/config.toml")?;
    assert!(!config.is_empty());
    Ok(())
}
```

### Test organization

Use the three official test locations described in the [test organization chapter](https://doc.rust-lang.org/book/ch11-03-test-organization.html):

1. **Unit tests** live inside the crate in a `#[cfg(test)] mod tests` block, usually one per source file.
2. **Integration tests** live in files under the `tests/` directory at the crate root.
3. **Doctests** live as executable code blocks inside `///` doc comments.

Unit tests:

- Place them in `#[cfg(test)] mod tests { use super::*; ... }` inside `src/`.
- Name the module `tests` by convention.
- Non-`#[test]` helpers inside the module are compiled but not run.
- A child module can access private items of its ancestor module. This is the canonical way to test private functions.

Integration tests:

- Top-level `tests/` directory, sibling of `src/`.
- Each `.rs` file directly in `tests/` is compiled as its own separate crate that depends on the library.
- Do NOT put `#[cfg(test)]` on integration test files; Cargo handles them specially.
- Only the library's public API is accessible; import with `use my_crate::...`.
- Cargo prints a separate `Running tests/<file>.rs` section per integration test binary. Test names are not prefixed with `tests::`.

Shared helper pattern:

- Put shared code at `tests/common/mod.rs` (NOT `tests/common.rs`).
- Files in subdirectories of `tests/` are NOT compiled as separate crates.
- `tests/common.rs` would wrongly appear as a 0-test section in output.
- Reference the helper from each test file with `mod common;` then `common::setup();`.

Binary crate caveat:

- A project with only `src/main.rs` and no `src/lib.rs` cannot have integration tests import from it.
- Put logic in `src/lib.rs` and make `src/main.rs` a thin shim. This is the standard reason to split a binary crate into a bin+lib layout.

### Running tests with `cargo test`

The Cargo `test` command synopsis is:

```text
cargo test [options] [testname] [-- test-options]
```

The `--` separator is critical. Flags before `--` go to Cargo. The first positional `testname` and everything after `--` go to libtest (the test binary). `cargo test --test-threads=1` is wrong; it must be `cargo test -- --test-threads=1`.

- `cargo test --help` shows Cargo flags.
- `cargo test -- --help` shows libtest flags.

#### Cargo-side flags

These appear before `--`.

| Flag | Meaning |
| --- | --- |
| `--lib` | Test only the library target. |
| `--bins` / `--bin NAME` | Test binaries. |
| `--examples` / `--example NAME` | Test examples. |
| `--tests` / `--test NAME` | Test integration test targets (`--tests` = all targets with `test = true`). |
| `--benches` / `--bench NAME` | Test benchmarks. |
| `--all-targets` | Equivalent to `--lib --bins --tests --benches --examples`. |
| `--doc` | Run ONLY doctests. Cannot be mixed with other target flags. |
| `--no-run` | Compile tests but do not run them. |
| `--no-fail-fast` | Run all test binaries even after one fails. Cargo has no `--keep-going`. |
| `-r` / `--release` | Run tests in release mode. |
| `--profile NAME` | Use a named Cargo profile. |
| `--target TRIPLE` | Cross-compile test target. |
| `--target-dir DIR` | Build directory (also `CARGO_TARGET_DIR`). |
| `-p` / `--package SPEC` | Test a workspace package; repeatable, globs supported. |
| `--workspace` | Test the entire workspace (`--all` is a deprecated alias). |
| `--exclude SPEC` | Exclude a package; requires `--workspace`. |
| `-F` / `--features FEATS` | Space/comma-separated features; supports `pkg/feat` syntax for workspaces. |
| `--all-features` / `--no-default-features` | Feature toggles. |
| `--manifest-path PATH` | Path to `Cargo.toml`. |
| `--locked` / `--offline` / `--frozen` | `frozen` = `locked` + `offline`. |
| `--config KEY=VALUE` | Extra Cargo configuration. |
| `-j` / `--jobs N` | Build parallelism only; does not affect test run threads. |
| `--timings` | Emit build timing info. |
| `-v` / `--verbose`, `-q` / `--quiet` | Verbosity. |
| `--color auto\|always\|never` | Color control. |
| `--message-format fmt` | Output format. |

#### Libtest flags (after `--`)

| Flag | Meaning |
| --- | --- |
| `<testname>` | Positional substring filter against the full path of the test function. |
| `--exact` | Require the filter to match the full path exactly. |
| `--skip FILTER` | Repeatable; skip tests whose name contains `FILTER`. |
| `--test-threads N` | Parallel threads (default = available parallelism); also `RUST_TEST_THREADS` (deprecated). |
| `--show-output` | Show stdout/stderr of passing tests after all tests run. |
| `--no-capture` | Stream stdout/stderr live (may interleave). `--nocapture` is a deprecated alias. Also `RUST_TEST_NOCAPTURE` (deprecated). |
| `--ignored` | Run only `#[ignore]` tests. |
| `--include-ignored` | Run ignored and non-ignored tests. |
| `--list` | List all tests without running. |
| `--quiet` / `-q` | Same as `--format=terse`. |
| `--format pretty\|terse\|json` | Output format; `json` is unstable. |
| `--color auto\|always\|never` | Color control. |

Unstable libtest flags (require `-Z unstable-options` / nightly): `--fail-fast`, `--shuffle`, `--shuffle-seed`, `--exclude-should-panic`, `--report-time`, `--ensure-time`, `--force-run-in-process`.

Filter behavior:

- Only the first positional `testname` is used by default.
- The filter is a substring match on the full path, so the module path is part of the name. For example, `cargo test tests::add` works.

Common invocations:

```bash
# run all tests
cargo test

# run unit tests only
cargo test --lib

# run one integration test file
cargo test --test integration

# run serially
cargo test -- --test-threads=1

# show output of passing tests
cargo test -- --show-output

# run ignored tests only
cargo test -- --ignored

# run a specific test by substring
cargo test withdraw_refuses
```

### Test isolation

Each test runs in its own thread by default. Parallelism is free, but there is no ordering guarantee. Tests must not share mutable state such as files, environment variables, or the current working directory.

- Serialize tests with `cargo test -- --test-threads=1` when they unavoidably touch shared mutable state.
- Stdout of passing tests is captured and hidden. Stdout of failing tests is always shown.
- `--show-output` shows passing output after all tests finish, contiguously.
- `--no-capture` streams stdout/stderr live and may interleave.
- If a test binary fails, Cargo does not start later binaries unless `--no-fail-fast` is set. Within one binary, all tests run to completion by default.
- Tests must be compiled with `panic = "unwind"`. The `abort` strategy is not supported by stable libtest.

Working directories:

- Unit and integration test CWD is the package root, not the test file's directory.
- Doctest CWD is the package root (`rustdoc --test-run-directory`).
- Use `env!("CARGO_MANIFEST_DIR")` or the `CARGO_MANIFEST_DIR` environment variable for path resolution.

### Doctests

Code blocks in `///` and `//!` doc comments are compiled and run by `cargo test --doc` and by default as part of `cargo test`. They appear in a final `Doc-tests <crate>` section. Each code block compiles to its own hidden crate executable and runs in its own process; compilation is the slow part and the run model is not guaranteed.

- Doctests link only against public items. Private items are not visible; use unit tests for those.
- A fenced block with no language is assumed Rust; `rust` is equivalent to the default.
- Use `# Examples` as a convention header; rustdoc does not require it, but the standard library, `regex`, and `serde` use it consistently.

Code block annotations (info-string after the opening fence):

| Annotation | Meaning |
| --- | --- |
| `ignore` | Generic ignore. Rustdoc says this is almost never what you want; prefer `text` or `#`-hidden lines. |
| `should_panic` | Compiles; must panic at runtime. |
| `no_run` | Compiles but does not run (e.g. network or filesystem examples). |
| `compile_fail` | Must fail to compile; passes if it does not compile. |
| `edition2015` / `edition2018` / `edition2021` / `edition2024` | Per-block edition override. |
| `standalone_crate` | Opt out of 2024-edition doctest merging; use when line numbers matter, e.g. `std::panic::Location::caller()`. |
| `ignore-<target>` | Target-conditional ignore, e.g. `ignore-x86_64`; comma-separate multiple. Starting Rust 1.88.0, `ignore-<target>` overrides a bare `ignore`. |
| `text` / `json` | Non-Rust / language hints. |

Indented (4-space) code blocks are accepted but cannot carry annotations; prefer fenced blocks.

Hidden lines:

- Lines starting with `# ` are compiled but hidden from rendered docs. Use them to hide setup/boilerplate.
- Escape a leading `#` with `##` (for example, string literals beginning with `#`).

`?` operator in doctests:

- Either write an explicit `fn main() -> io::Result<()> { ... Ok(()) }`, or
- Since Rust 1.34, omit `fn main` and end with `Ok::<(), io::Error>(())` written as one token `(())` with no whitespace.

Rustdoc preprocessing for each doctest:

1. Inserts common allow attrs: `unused_variables`, `unused_assignments`, `unused_mut`, `unused_attributes`, `dead_code`.
2. Applies `#![doc(test(attr(...)))]`.
3. Keeps leading `#![...]` inner attributes.
4. Injects `extern crate <mycrate>;` unless `#![doc(test(no_crate_inject))]` is set.
5. Wraps the block in `fn main()` if absent.

`#[doc]` attribute forms:

- `///` is equivalent to `#[doc = "..."]`.
- `//!` is equivalent to `#![doc = "..."]`.
- `#![doc(test(no_crate_inject))]` disables automatic `extern crate` injection.
- `#![doc(test(attr(...)))]` applies attributes to all doctests; inner-module attrs are appended, not replaced.
- `#[doc(hidden)]` hides an item from docs unless `--document-hidden-items` is passed.

Running and disabling doctests:

```bash
# run only doctests
cargo test --doc

# pass libtest args to doctests
cargo test --doc -- --show-output
```

Disable doctests entirely for the library:

```toml
[lib]
doctest = false
```

Rustdoc sets `#[cfg(doctest)]` when collecting doctests. Gate doctest-only items (for example, README doctests via `#[doc = include_str!("../README.md")]`) with it.

### Cargo target configuration

The [`[lib]`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) and `[[test]]` tables control test targets.

```toml
[lib]
doctest = true

[[test]]
name = "integration"
path = "tests/integration.rs"
harness = true
```

Fields:

| Field | Applies to | Default | Meaning |
| --- | --- | --- | --- |
| `doctest` | `[lib]` | `true` | Whether rustdoc runs doctests for this library. |
| `test` | `[[test]]`, `[[bench]]`, `[[example]]`, `[[bin]]` | `true` for tests, varies for others | Whether Cargo builds and runs this target as a test. |
| `harness` | `[[test]]`, `[[bench]]`, `[[example]]` | `true` | Whether to use the built-in test harness. |

If `harness = false`, you must provide your own `fn main()`. This is used for custom runners such as `libtest_mimic`, criterion, or examples that keep their own `main`. `#[cfg(test)]` stays enabled whether or not `harness` is set.

Custom test frameworks via `#![feature(custom_test_frameworks)]` with `#[test_case]` and `#![test_runner(...)]` are nightly-only (tracking issue #50297). The stable path is `harness = false` plus your own `main`, often via `libtest_mimic`.

### Environment variables

Cargo and libtest set or honor several environment variables:

| Variable | Meaning |
| --- | --- |
| `CARGO_TARGET_DIR` | Build directory; also set by `--target-dir`. |
| `RUSTFLAGS` | Extra flags passed to `rustc`. |
| `RUSTDOCFLAGS` | Extra flags passed to `rustdoc`. |
| `CARGO_ENCODED_RUSTFLAGS` | NUL-separated `RUSTFLAGS`. |
| `RUSTC` / `RUSTDOC` | Override the compiler / documentation tool. |
| `CARGO_MANIFEST_DIR` | Directory containing `Cargo.toml`; useful for fixture paths. |
| `CARGO_PKG_*` | Package metadata. |
| `CARGO_BIN_EXE_<name>` | Absolute path to a binary; set only for integration tests and benches. |
| `CARGO_TARGET_TMPDIR` | Scratch directory inside `target/`; set only for integration tests and benches. |
| `RUST_TEST_THREADS` | Deprecated; sets `--test-threads`. |
| `RUST_TEST_NOCAPTURE` | Deprecated; sets `--no-capture`. |
| `RUST_TEST_SHUFFLE` / `RUST_TEST_SHUFFLE_SEED` | Deprecated; shuffle control. |
| `RUST_BACKTRACE` | `0`, `1`, or `full`; controls backtrace verbosity. |

### Ecosystem tooling

Add test-only crates under `[dev-dependencies]` in `Cargo.toml`. Keep test dependencies pinned to the repo's MSRV policy.

#### When to use which tool

| Problem | Default tool | Alternatives |
| --- | --- | --- |
| Property-based / invariant testing | `proptest` | `quickcheck` (legacy, simpler) |
| Mocking traits | `mockall` | Hand-written fakes, `wiremock` / `httpmock` for HTTP |
| Snapshot / structured output | `insta` | Manual assertions for small outputs |
| Async runtime tests | `tokio` (`#[tokio::test]`) | `tokio-test` for one-offs |
| Stable benchmarking | `criterion` | `divan` |
| Coverage | `cargo-llvm-cov` | `cargo-tarpaulin` |
| Fuzzing | `cargo-fuzz` + `libfuzzer-sys` | — |
| UI / compile-fail tests | `trybuild` | `libtest_mimic` for dynamic discovery |

#### Property-based testing

Use `proptest` as the default choice:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_roundtrip(s in "[a-z]+") {
        let encoded = format!("{s}\n");
        prop_assert_eq!(parse_line(&encoded).unwrap(), s);
    }
}
```

Use `prop_assert!` / `prop_assert_eq!` (NOT bare `assert!`) so failures shrink. Strategies include `any::<T>()`, `prop::collection::vec`, regex strings like `"[0-9]{4}"`, `.prop_filter("reason", pred)`, and `.prop_filter_map(...)`. Configure with `ProptestConfig`, `#![proptest_config(...)]`, or the `PROPTEST_CASES` environment variable. Persisted regressions live in `proptest-regressions/`; commit them. For timeout configuration, run cases in a subprocess.

`quickcheck` is legacy/simpler: put `#[quickcheck]` on a function returning `bool`; derive `Arbitrary`. The `QUICKCHECK_TESTS` environment variable defaults to 100. It has no constraint-aware generation, so it produces more discards. Prefer `proptest` for new code.

#### Mocking

Use `mockall` for trait-based dependencies:

```rust
#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, mockall::automock)]
pub trait Store {
    fn get(&self, key: &str) -> Option<String>;
}

#[test]
fn service_returns_default_when_missing() {
    let mut store = MockStore::new();
    store.expect_get()
        .with(mockall::predicate::eq("missing"))
        .times(1)
        .return_const(None);

    let service = Service::new(store);
    assert_eq!(service.value("missing"), "default");
}
```

`mockall` generates `Mock<Name>` from `#[automock]` traits. Predicates live in `mockall::predicate`. Use `mock!{}` for structs and external traits, and `#[double]` from `mockall_double` for module-level mocking. Call counts are verified on `Drop`. Requires Rust >= 1.77.

Prefer hand-written fakes for simple 1-2 method traits. Use `wiremock` or `httpmock` for HTTP-specific mocking.

#### Snapshot testing

Use `insta` for large or structured output:

```rust
#[test]
fn snapshot_config_display() {
    let config = Config::default();
    insta::assert_snapshot!(format!("{config:?}"));
}
```

Other macros: `assert_json_snapshot!`, `assert_yaml_snapshot!`, `assert_debug_snapshot!`, inline snapshots ` @"..."`. Snapshot files live in `tests/snapshots/`. CLI: `cargo insta test`, `cargo insta review`. In CI, fail on pending snapshots with `cargo insta test --check --unreferenced=reject`. Never auto-accept snapshots in CI. Sort maps and redact dynamic fields such as UUIDs and timestamps.

#### Async testing

Use `#[tokio::test]` to run async test functions:

```rust
#[tokio::test]
async fn fetch_returns_ok() {
    let client = build_client();
    let result = client.fetch("https://example.com").await;
    assert!(result.is_ok());
}
```

Default runtime flavor is `current_thread`. Use the attribute form for multi-threading or paused time:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_cache_access() { }

#[tokio::test(start_paused = true)] // requires tokio "test-util" feature
async fn time_based_behavior() { }
```

Without a runtime, async functions panic ("there is no reactor running"). Use `tokio-test` for one-off `block_on` / `spawn` helpers.

#### Benchmarking

Do NOT use libtest `#[bench]` in production; it requires nightly (`#![feature(test)]`).

Use `criterion` as the stable default. Set `harness = false` in `[[bench]]`:

```toml
[[bench]]
name = "fibonacci"
harness = false
```

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

Always use `black_box` on inputs or the optimizer removes the work. Use `bench_with_input` and `Throughput` for parameterized benchmarks. Results go to `target/criterion/`. Configuration can be provided via `criterion.toml`.

`divan` is an alternative with simpler `#[divan::bench]`, args/type parameters, and allocation tracking via `AllocProfiler`.

#### Coverage

Use `cargo-llvm-cov` for new projects:

```bash
# LCOV for Codecov / Coveralls
cargo llvm-cov --lcov --output-path lcov.info

# Cobertura for GitLab
cargo llvm-cov --cobertura

# HTML report
cargo llvm-cov --html

# CI gates
cargo llvm-cov --fail-under-lines N
cargo llvm-cov --fail-uncovered-regions N
```

`cargo-llvm-cov` works on the stable toolchain.

`cargo-tarpaulin` is an alternative:

```bash
cargo tarpaulin --out Xml --workspace
cargo tarpaulin --engine llvm  # outside Linux x86_64
cargo tarpaulin --fail-under N
```

The default ptrace engine works only on Linux x86_64. Branch coverage is not implemented. Pick ONE coverage tool per repo. Coverage is a signal, not a target.

#### Fuzzing

Use `cargo-fuzz` + `libfuzzer-sys` on nightly, Linux/macOS:

```bash
cargo fuzz init
cargo fuzz add parser
```

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = my_crate::parse(data);
});
```

Run: `cargo fuzz run <target>`. Minimize crashes: `cargo fuzz tmin <target> <crash>`. Minimize corpus: `cargo fuzz cmin <target>`. Use a seed corpus and bound CI with `-- -max_total_time=120`.

#### Custom harness / UI tests

Use `libtest_mimic` for dynamic test discovery with a libtest-compatible CLI:

```rust
fn main() {
    let args = libtest_mimic::Arguments::from_args();
    let tests = vec![
        libtest_mimic::Trial::test("smoke", || Ok(())),
    ];
    libtest_mimic::run(&args, tests).exit();
}
```

Use `trybuild` for compile-fail / pass tests with `.stderr` snapshot files:

```rust
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
```

Regenerate with `TRYBUILD=overwrite cargo test`. Pin `rust-src` in `rust-toolchain.toml` to avoid local-vs-CI diagnostic diffs. Run with `cargo test --test <name>`.

### Test naming conventions

Name tests after behavior + condition + expected result:

```rust
#[test]
fn withdraw_refuses_amount_greater_than_balance() { }
```

Avoid names that only mirror the function under test, such as `test_withdraw`. Group related tests in submodules when the file grows large.

### What to test

Cover the following categories in every non-trivial module:

- Happy path
- Edge cases (empty input, zero, maximum values)
- Boundary conditions
- Error paths and `Result::Err` variants
- Panic conditions, where appropriate, with `#[should_panic]`
- Invariants via property tests for parsers, serializers, and numeric code

## Review checklist

- [ ] Unit tests live in `#[cfg(test)] mod tests` blocks inside `src/`.
- [ ] Integration tests live under `tests/` and exercise only public APIs.
- [ ] Shared integration helpers live at `tests/common/mod.rs`, not `tests/common.rs`.
- [ ] Public methods have doctests where examples add clarity.
- [ ] `#[should_panic]` tests include an `expected` substring when possible.
- [ ] `#[ignore]` tests include a reason.
- [ ] Tests are isolated and do not rely on mutable global state.
- [ ] Async tests use `#[tokio::test]` or an equivalent runtime attribute.
- [ ] Test names describe the behavior, condition, and expected result.
- [ ] Error, boundary, and edge cases are covered.
- [ ] Property tests use `prop_assert!` / `prop_assert_eq!` for shrinking.
- [ ] Criterion benchmarks use `black_box` to prevent optimizer elision.
- [ ] Insta snapshots are not auto-accepted in CI.

## Implementation checklist

- [ ] Add unit tests for new pure functions and methods.
- [ ] Add integration tests for new public behavior.
- [ ] Add or update doctests for changed public APIs.
- [ ] Run `cargo test` before pushing.
- [ ] Run `cargo test --release` if the change touches unsafe or optimization-sensitive code.
- [ ] Run ignored tests locally if they touch external services.
- [ ] Verify benchmarks still compile with `cargo bench --no-run` when criterion is used.
- [ ] Regenerate `trybuild` `.stderr` snapshots when diagnostic output changes.
- [ ] Commit `proptest-regressions/` files when they change.
- [ ] Run `cargo clippy --all-targets -- -D warnings` to catch test-only lints.

## Validation hooks

Run these commands as appropriate for the change:

- `cargo test`
- `cargo test --lib`
- `cargo test --test NAME`
- `cargo test --doc`
- `cargo test --release`
- `cargo test -- --ignored`
- `cargo test --no-run` (compile-only check)
- `cargo clippy --all-targets -- -D warnings`
- `cargo bench --no-run` (verify benchmarks compile)
- `cargo llvm-cov --lcov --output-path lcov.info`
- `cargo tarpaulin --out Xml --workspace`
- `cargo test --test ui` (for `trybuild` UI tests)

## Examples

### Basic unit test module (with private-item access)

```rust
// src/lib.rs
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

fn internal_helper(x: usize) -> usize {
    x * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn private_helper_doubles() {
        assert_eq!(internal_helper(3), 6);
    }
}
```

### Result-returning test

```rust
#[test]
fn read_config_succeeds() -> std::io::Result<()> {
    let config = std::fs::read_to_string("fixtures/config.toml")?;
    assert!(!config.is_empty());
    Ok(())
}
```

### `#[should_panic]` test

```rust
#[test]
#[should_panic(expected = "division by zero")]
fn panics_when_dividing_by_zero() {
    divide(10, 0);
}
```

### Integration test with shared helper

File layout:

```text
tests/
  common/
    mod.rs
  api.rs
```

`tests/common/mod.rs`:

```rust
pub fn setup() {
    // shared setup
}
```

`tests/api.rs`:

```rust
mod common;
use my_crate::Config;

#[test]
fn config_has_sensible_defaults() {
    common::setup();
    let config = Config::default();
    assert_eq!(config.timeout_secs, 30);
}
```

### Doctest with hidden lines and `no_run`

```rust
/// Parses a comma-separated string into a vector of trimmed tokens.
///
/// ```
/// use my_crate::parse_csv;
/// assert_eq!(parse_csv("a, b, c"), vec!["a", "b", "c"]);
/// ```
///
/// This example has side effects, so it is marked `no_run`:
///
/// ```no_run
/// use my_crate::parse_csv;
/// std::fs::write("out.csv", "a, b, c").unwrap();
/// let data = std::fs::read_to_string("out.csv").unwrap();
/// assert_eq!(parse_csv(&data), vec!["a", "b", "c"]);
/// ```
pub fn parse_csv(input: &str) -> Vec<&str> {
    input.split(',').map(|s| s.trim()).collect()
}
```

### Async test with Tokio

```rust
#[tokio::test]
async fn cache_hits_are_fast() {
    let cache = Cache::new();
    cache.set("key", "value").await;
    assert_eq!(cache.get("key").await, Some("value".to_string()));
}
```

### Proptest property test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_roundtrip(s in "[a-z]+") {
        let encoded = format!("{s}\n");
        prop_assert_eq!(parse_line(&encoded).unwrap(), s);
    }
}
```

### Mockall automock test

```rust
#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, mockall::automock)]
pub trait Store {
    fn get(&self, key: &str) -> Option<String>;
}

#[test]
fn service_returns_default_when_missing() {
    let mut store = MockStore::new();
    store.expect_get()
        .with(mockall::predicate::eq("missing"))
        .times(1)
        .return_const(None);

    let service = Service::new(store);
    assert_eq!(service.value("missing"), "default");
}
```

### Insta snapshot test

```rust
#[test]
fn snapshot_config_display() {
    let config = Config::default();
    insta::assert_snapshot!(format!("{config:?}"));
}
```

### Criterion benchmark

`Cargo.toml`:

```toml
[[bench]]
name = "fibonacci"
harness = false
```

`benches/fibonacci.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

### cargo-fuzz target

`fuzz_targets/parser.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = my_crate::parse(data);
});
```

### trybuild UI test

```rust
// tests/ui.rs
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
```

## Common mistakes

- `cargo test --test-threads=1` is wrong; the flag must come after `--`: `cargo test -- --test-threads=1`.
- `tests/common.rs` for shared helpers produces a 0-test section; use `tests/common/mod.rs` instead.
- `#[should_panic]` on a `Result<T, E>` test will not compile.
- Using bare `assert!` to compare two values instead of `assert_eq!`/`assert_ne!`, losing the value diff.
- Comparing custom types with `assert_eq!` without `#[derive(PartialEq, Debug)]`.
- Assuming `left:`/`right:` ordering or expecting `expected`/`actual` labels.
- Tests depending on execution order or shared mutable state (env vars, files, CWD).
- `#[should_panic]` without `expected` substring, which passes on the wrong panic.
- Naming tests `test_foo` instead of behavior+condition+expected-result.
- Running slow or network-dependent tests unconditionally instead of marking them `#[ignore]`.
- Treating coverage percentage as a substitute for meaningful assertions.
- `proptest` with bare `assert!` instead of `prop_assert!`, losing shrinking.
- Doctests that reach for private items; doctests can only see public API.
- Criterion benchmarks without `black_box`, allowing the optimizer to remove the work.
- Auto-accepting `insta` snapshots in CI.
- Forgetting that integration test CWD is the package root, not the test file's directory.
- Using `#[bench]` on stable; it requires nightly.

## Strict vs contextual guidance

### Strict

- Every non-trivial public function must have at least one unit test or doctest.
- Integration tests must exercise only `pub` APIs.
- `#[cfg(test)]` must be used for test-only helpers and modules inside `src/`.
- Tests must not mutate shared global state.
- `assert!`, `assert_eq!`, and `assert_ne!` must include a custom message when the failure would be unclear.
- `#[should_panic]` must use `expected = "..."` unless any panic is truly acceptable.
- Shared integration-test helpers must live at `tests/common/mod.rs`, not `tests/common.rs`.
- `cargo test` flags that belong to libtest must appear after `--`.

### Contextual

- Property-based testing is valuable for parsers, serializers, and numeric code but may be overkill for trivial glue code.
- Snapshot testing is useful for large structured output but adds snapshot maintenance overhead.
- `mockall` is helpful for complex trait-based dependencies; hand-written fakes are acceptable for simple cases.
- Coverage gates are a policy decision for individual repositories.
- Whether ignored tests run in CI or only locally depends on the service availability of that repo.
- The choice between `criterion` and `divan` is project-specific.

## Policy decisions for individual repos

- Choose whether to require doctests for every public function or only non-trivial ones.
- Set a coverage threshold or leave coverage as informational only.
- Decide whether external-service tests run in CI, locally, or only on demand with `-- --ignored`.
- Choose a property-based testing crate (`proptest`, `quickcheck`, or none).
- Choose a mocking crate (`mockall` or hand-written fakes).
- Decide whether fuzz targets are required for parsers and protocol handlers.
- Define MSRV policy for `criterion` or other test-only dependencies.
- Decide whether to gate CI on `cargo clippy --all-targets -- -D warnings`.
- Decide whether to require `cargo bench --no-run` in CI to keep benchmarks compiling.

## Related docs

- `api-design.md`
- `async-tokio.md`
- `documentation-guidelines.md`
- `error-handling.md`
- `lints-clippy.md`
- `style-formatting.md`
- `editions-tooling.md`
- `modules-visibility.md`

## Related skills

- None defined yet.
