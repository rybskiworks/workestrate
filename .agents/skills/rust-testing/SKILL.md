---
name: rust-testing
description: |
  Operational guide for writing, organizing, and running Rust tests. Load when
  writing tests, running tests, setting up test organization, or choosing
  testing tools (cargo test flags, #[test]/#[should_panic]/#[ignore],
  Result-returning tests, doctests, unit vs integration tests, proptest,
  mockall, tokio::test, coverage, fuzzing). Distilled from docs/rust/testing.md;
  consult that doc for full detail and source URLs.
---

# Rust Testing

Distilled from [`docs/rust/testing.md`](../../../docs/rust/testing.md). That doc
holds the canonical rules and upstream source links; this skill is the
actionable subset.

## Triggers

Load this skill when:

- Writing or modifying `#[test]` functions, `mod tests`, integration tests,
  or doctests.
- Running `cargo test` and choosing flags (`--lib`, `--doc`, `--test NAME`,
  `-- --nocapture`, `-- --test-threads`, `-- --ignored`).
- Setting up test organization (unit/integration/doctest layout, shared
  helpers, bin+lib split).
- Choosing test tooling: `proptest`/`quickcheck`, `mockall`, `insta`,
  `#[tokio::test]`, `criterion`, `cargo-llvm-cov`/`tarpaulin`, `cargo-fuzz`,
  `trybuild`.
- Reviewing a test diff for correctness and idioms.

## Test Organization

| Kind | Location | Notes |
|---|---|---|
| Unit | `#[cfg(test)] mod tests { use super::*; ... }` inside `src/` | Child module can access private items of ancestor. |
| Integration | `tests/<name>.rs` at crate root | Each file = own crate; public API only; NO `#[cfg(test)]`. |
| Doctest | `///` and `//!` code blocks | Public items only; run via `cargo test --doc`. |

- Shared integration helpers: `tests/common/mod.rs` (NOT `tests/common.rs`).
  Reference via `mod common;` then `common::setup();`.
- Binary-only crate (`src/main.rs`, no `src/lib.rs`) cannot have integration
  tests; split into bin+lib with `src/main.rs` as a thin shim.

## Test Attributes

| Attribute | Rule |
|---|---|
| `#[test]` | Free fn, monomorphic, no args, returns `()` or `Result<(), E: Debug>`. |
| `#[should_panic]` | Return type MUST be `()`. Use `expected = "..."` (substring match) whenever possible. |
| `#[ignore]` | Use `#[ignore = "reason"]`; still compiled; run with `-- --ignored`. |
| `#[cfg(test)]` | Gates test-only code; do NOT apply to integration test files. |

## Assertions

- `assert_eq!`/`assert_ne!` for value comparison (shows both values); types
  need `#[derive(PartialEq, Debug)]`.
- `assert!(cond, "msg {args}")` for booleans; custom message when failure is
  unclear.
- Failure labels are `left:`/`right:` — NOT `expected`/`actual`; order is
  irrelevant.
- `debug_assert*` elided in release; never use to guard `unsafe`.

## Result-Returning Tests

```rust
#[test]
fn read_config_succeeds() -> std::io::Result<()> {
    let config = std::fs::read_to_string("fixtures/config.toml")?;
    assert!(!config.is_empty());
    Ok(())
}
```

- Do NOT combine `#[should_panic]` with `Result`-returning tests (won't compile).
- To assert an error, use `assert!(value.is_err())`, NOT `?`.

## cargo test Flags

The `--` separator is critical: Cargo flags before, libtest flags after.

```bash
cargo test                       # all tests
cargo test --lib                 # unit tests only
cargo test --test integration    # one integration file
cargo test --doc                 # doctests only
cargo test -- --test-threads=1   # serial (NOT `cargo test --test-threads=1`)
cargo test -- --nocapture        # stream stdout live (alias of --no-capture)
cargo test -- --show-output      # show passing output after run
cargo test -- --ignored          # run only #[ignore] tests
cargo test withdraw_refuses      # substring filter on full path
cargo test --no-run              # compile only
cargo test --no-fail-fast        # keep running after a binary fails
```

## Test Isolation

- Each test runs in its own thread; no ordering guarantee; no shared mutable
  state (env vars, files, CWD).
- Serialize with `-- --test-threads=1` only when unavoidable.
- Passing-test stdout is captured; failing-test stdout always shown.
- CWD is the package root, not the test file dir; use `env!("CARGO_MANIFEST_DIR")`
  for fixture paths.
- Tests require `panic = "unwind"`; `abort` is unsupported by stable libtest.

## Tooling Decision Table

| Problem | Default | Alternatives / Notes |
|---|---|---|
| Property testing | `proptest` | `quickcheck` (legacy). Use `prop_assert!`/`prop_assert_eq!` for shrinking; commit `proptest-regressions/`. |
| Mocking traits | `mockall` | Hand-written fakes for 1-2 method traits; `wiremock`/`httpmock` for HTTP. |
| Snapshot testing | `insta` | Never auto-accept in CI: `cargo insta test --check --unreferenced=reject`. |
| Async tests | `#[tokio::test]` | Default `current_thread`; use `flavor = "multi_thread"` or `start_paused = true` as needed. |
| Benchmarking | `criterion` (`harness = false`) | Always `black_box` inputs. `divan` is alternative. NO libtest `#[bench]` (nightly only). |
| Coverage | `cargo-llvm-cov` (stable) | `cargo-tarpaulin` (Linux x86_64 default). Pick ONE per repo. |
| Fuzzing | `cargo-fuzz` + `libfuzzer-sys` (nightly) | `cargo fuzz run/tmin/cmin <target>`. |
| UI / compile-fail | `trybuild` | Regenerate: `TRYBUILD=overwrite cargo test`. |

Add test-only crates under `[dev-dependencies]`.

## Doctest Quick Rules

- Public items only; use unit tests for private items.
- Fenced block with no language = Rust. Use `# Examples` header by convention.
- Annotations: `no_run`, `should_panic`, `compile_fail`, `ignore`,
  `edition2021`, `text`.
- Hidden lines start with `# `; escape a literal leading `#` with `##`.
- `?` in doctest: write `fn main() -> io::Result<()>` or end with
  `Ok::<(), io::Error>(())`.
- Disable for a lib: `[lib] doctest = false`.

## Review Checklist

- [ ] Unit tests in `#[cfg(test)] mod tests` inside `src/`.
- [ ] Integration tests under `tests/`, public API only.
- [ ] Shared helpers at `tests/common/mod.rs`, not `tests/common.rs`.
- [ ] `#[should_panic]` uses `expected`; `#[ignore]` has a reason.
- [ ] Tests isolated; no shared mutable global state.
- [ ] Async tests use `#[tokio::test]` or equivalent.
- [ ] Names describe behavior + condition + expected result (not `test_foo`).
- [ ] Error, boundary, edge cases covered.
- [ ] `proptest` uses `prop_assert!`; criterion uses `black_box`.
- [ ] Insta snapshots not auto-accepted in CI.

## Validation Commands

Run as appropriate to the change:

```bash
cargo test
cargo test --lib
cargo test --test NAME
cargo test --doc
cargo test --release            # for unsafe / optimization-sensitive code
cargo test -- --ignored         # if touching external-service tests
cargo test --no-run             # compile-only gate
cargo clippy --all-targets -- -D warnings
cargo bench --no-run            # if criterion used
cargo llvm-cov --lcov --output-path lcov.info
cargo tarpaulin --out Xml --workspace
```

## Common Mistakes

- `cargo test --test-threads=1` — must be `cargo test -- --test-threads=1`.
- `tests/common.rs` produces a 0-test section — use `tests/common/mod.rs`.
- `#[should_panic]` on a `Result`-returning test won't compile.
- Bare `assert!` for value comparison loses the diff — use `assert_eq!`.
- `#[should_panic]` without `expected` passes on the wrong panic.
- Tests depending on order or shared mutable state (env, files, CWD).
- Doctests reaching for private items — only public API is visible.
- Criterion without `black_box` — optimizer removes the work.
- `#[bench]` on stable — requires nightly; use `criterion` with `harness = false`.

## Strict Rules

- Every non-trivial public function has a unit test or doctest.
- Integration tests exercise only `pub` APIs.
- `#[cfg(test)]` for test-only helpers/modules inside `src/`.
- Tests must not mutate shared global state.
- `#[should_panic]` must use `expected = "..."` unless any panic is acceptable.
- Libtest flags must appear after `--`.

## Related Docs

- Full reference: [`docs/rust/testing.md`](../../../docs/rust/testing.md) (canonical; upstream source URLs there).
- `docs/rust/async-tokio.md`, `docs/rust/error-handling.md`,
  `docs/rust/lints-clippy.md`, `docs/rust/modules-visibility.md`.
