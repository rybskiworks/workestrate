---
name: rust-error-handling
description: |
  Operational guide for Rust error handling: choosing Option vs Result, using
  the `?` operator, designing error types, choosing thiserror (libraries) vs
  anyhow (binaries), panic policy, `#[must_use]`, unwrap/expect conventions,
  `#[non_exhaustive]` for public error enums, and backtraces. Load when
  designing error types, writing `Result`/`Option` code, deciding panic vs
  Result, or choosing between thiserror/anyhow/miette/color-eyre. Does NOT
  cover general Rust tutorials, async error handling, or clippy configuration
  beyond error-specific lints.
---

# Rust Error Handling

Distilled from `docs/rust/error-handling.md` and `docs/rust/api-design.md`.
Cite those docs (not upstream URLs) when linking to sources.

## Triggers

Load this skill when:

- Designing or revising a public error enum / `Result<T, E>` return type.
- Deciding between `Option` and `Result`, or between `panic!` and `Result`.
- Choosing `thiserror`, `anyhow`, `miette`, or `color-eyre` for a crate.
- Writing or reviewing `?` propagation, `unwrap`/`expect`, or `#[must_use]`.
- Adding `#[non_exhaustive]`, `#[from]`, `#[source]`, or backtrace capture.
- Reviewing whether `anyhow::Error` leaked into a public library API.

## Option vs Result

| Need | Use | Convert |
|------|-----|---------|
| Ordinary absence (key missing, end of iter) | `Option<T>` | to Result: `ok_or` / `ok_or_else` |
| Failure with a cause the caller may react to | `Result<T, E>` | to Option: `.ok()` (loses `Err`) |

Never smuggle error semantics into `Option` via side channels. If the caller
needs to know *why*, it is `Result`.

## Panic vs Result

Panic **only** on contract violations / bugs: out-of-bounds index, division by
zero, violated constructor invariant, logically unreachable branch. Return
`Result` for expected runtime failures: missing file, network timeout,
malformed input, rate limit.

> Library code should return an error if you can; panic only where continuing
> could be insecure or harmful. — `docs/rust/error-handling.md` §Panic vs Result

## Error Type Choice

| Context | Approach |
|---------|----------|
| Stable public library | `thiserror` enum, `Error + Send + Sync + 'static`, `#[non_exhaustive]` |
| Internal/workspace crate | `thiserror` enum or `Box<dyn Error + Send + Sync>` |
| Application / CLI / binary | `anyhow::Result<T>` + `.context()`; consider `miette`/`color-eyre` |
| Prototype / quick script | `Box<dyn Error>` (limits callers) |

Rule: libraries expose structured, matchable errors; applications report to
humans. **Never expose `anyhow::Error` in a public library API** (erases
structured info; see `docs/rust/api-design.md` C-GOOD-ERR, C-STABLE).

## `?` Operator

- Propagates `Err`/`None` early; auto-converts via `From` to the function's
  error type.
- Only valid in functions returning `Result`/`Option` (or a `FromResidual`
  type — nightly `try_trait_v2` only; do not impl `Try` on stable).
- Cannot mix `Result` and `Option` freely: convert with `ok_or`/`ok_or_else`,
  `Result::ok`, or `transpose`.
- `fn main() -> Result<(), E>` is idiomatic (`anyhow::Result<()>` for apps).

## `unwrap` / `expect` Conventions

- `unwrap`/`expect` panic on failure — discouraged in production code.
- Prefer `?`, combinators (`unwrap_or`, `unwrap_or_else`, `unwrap_or_default`),
  or explicit `match`/`let-else`.
- If `expect` is justified, the message describes why the value *should* be
  `Ok`/`Some` (e.g. `"env PORT should be set by the deployment script"`).
- Eager vs lazy: `unwrap_or`/`ok_or`/`map_or` evaluate eagerly; use
  `unwrap_or_else`/`ok_or_else`/`map_or_else` when the fallback is expensive.
- `Result` is `#[must_use]`; never silently discard — handle, propagate, or
  `let _ = ...` when intentional.

## `thiserror` Quick Reference

Attributes: `#[error("...")]` (Display), `#[from]` (generates `From`, implies
`#[source]`), `#[source]` (returned by `source()`), `#[error(transparent)]`
(forwards Display + source), `#[backtrace]` (nightly 1.73+). `#[from]` variant
must contain no other fields except a backtrace. thiserror does not appear in
the public API — switching to/from handwritten impls is not a breaking change.

## `anyhow` Quick Reference

`anyhow::Result<T>` = `Result<T, anyhow::Error>` (one-word pointer, `Send +
Sync + 'static`, guaranteed backtrace). `.context(C)` / `.with_context(|| …)`
attach context at boundaries. Macros: `anyhow!`, `bail!`, `ensure!`. Display:
`{}` = outermost layer, `{:#}` = full cause chain, `{:?}` = chain + backtrace.

## Public Error Enum Checklist

- [ ] Derive/implement `Debug`, `Display`, `std::error::Error`.
- [ ] `Send + Sync + 'static` (enables `downcast_ref`); see C-GOOD-ERR.
- [ ] `#[non_exhaustive]` on published enums (adding variants is non-breaking).
- [ ] `From` impls / `#[from]` for every `?` propagation path.
- [ ] `source()` chains inner errors; do NOT also render inner in `Display`.
- [ ] `Display` messages: lowercase, concise, no trailing punctuation.
- [ ] User-facing output from `Display`, not `Debug` (derived `Debug` is unstable).
- [ ] No `()` or `String` as a public error type.

## Panic Policy & Backtraces

- `panic!` = bugs/invariant violations only; `todo!`/`unimplemented!`/
  `unreachable!` are `panic!` shorthands — treat `todo!` as a release blocker.
- `assert!` always runs; `debug_assert!` compiled out in release.
- `panic = "abort"` in `[profile.release]` is a deliberate size/safety choice;
  tests/benches/build-scripts/proc-macros force unwind regardless.
- `std::process::abort()` runs no destructors, flushes nothing, no panic hook.
- `catch_unwind` only at FFI boundaries / test harnesses, never general
  try/catch; closure must be `UnwindSafe` (use `AssertUnwindSafe` to opt in).
- Backtraces: `std::backtrace::Backtrace` stable since 1.65. `RUST_LIB_BACKTRACE`
  consulted first, then `RUST_BACKTRACE`; `force_capture()` ignores env. For
  panics set `RUST_BACKTRACE=1` (debug symbols on by default without `--release`).
- `Error::backtrace()` does not exist on stable; store a `Backtrace` field and
  print in `Display`/`Debug`. `Error::provide()` backtrace access is nightly.

## Review Checklist

- [ ] Expected failures return `Result`, not `panic!`.
- [ ] `Option` for absence; `Result` for failure-with-cause.
- [ ] `unwrap`/`expect` in production justified with a documented invariant.
- [ ] Error types impl `Display` + `std::error::Error`; public ones `Send + Sync + 'static`.
- [ ] `From` conversions cover every `?` path.
- [ ] `source()` chains inner errors; inner not also in `Display`.
- [ ] User-facing messages from `Display`, not `Debug`.
- [ ] Domain boundaries attach context (`.context()`, `map_err`, wrapper).
- [ ] No `anyhow::Error` in a public library API.
- [ ] No `todo!`/`unimplemented!`/`unreachable!` in shipped code without reason.
- [ ] `catch_unwind` only at FFI/test boundaries.
- [ ] Public error enums `#[non_exhaustive]`.
- [ ] `panic = "abort"` is deliberate, not accidental.

## Verification Commands

```bash
cargo check
cargo test
cargo test --doc
cargo clippy -- -D warnings
cargo fmt --check
# libraries, before release:
cargo doc --no-deps
cargo semver-checks
```

Error-relevant clippy lints (enable per repo policy): `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::panic`, `clippy::result_unit_err`,
`clippy::option_option`.

## Anti-patterns

- `unwrap`/`expect` in production without a documented invariant.
- `catch_unwind` as general error handling.
- Returning `Err(())`, bare `String`, or opaque errors without `source()`/context.
- `()` or `String` as a public error type.
- `anyhow::Error` in a public library API.
- Panicking on expected I/O / parse / network errors.
- `todo!` left in shipped code.
- Mixing `?` on `Result` and `Option` without explicit conversion.
- `Box<dyn Error>` where downstream needs to match variants.
- Rendering inner error in both `Display` and `source()`.
- `unreachable!()` for conditions that are only *probably* false.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `cannot use ? on Option in fn returning Result` | Mixed carrier types | Convert with `ok_or_else` / `transpose` |
| `the trait From<X> is not implemented for E` | Missing conversion | Add `#[from]` or manual `From` impl |
| `anyhow::Error` in public API | Used anyhow in a library | Switch to `thiserror` enum |
| Backtrace empty | `RUST_BACKTRACE` unset | `RUST_BACKTRACE=1` (or `RUST_LIB_BACKTRACE=1`) |
| `todo!()` panic at runtime | Unfinished code shipped | Implement or gate behind a feature |
| Adding error variant breaks downstream | Enum not `#[non_exhaustive]` | Add `#[non_exhaustive]` to public enums |

## Related Docs

- `docs/rust/error-handling.md` — full reference, combinator catalogs, examples.
- `docs/rust/api-design.md` — C-GOOD-ERR, C-STABLE, `#[non_exhaustive]`, no
  `unwrap`/`expect`/`panic!` in public APIs.

## Related Skills

- `nix-usage` — Rust toolchain and `just shell` workflows for running the
  verification commands above.
