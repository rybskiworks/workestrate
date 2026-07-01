---
name: constraint-rust-error-propagation
description: |
  Enforces error handling rules during Rust code execution. Load when writing
  or reviewing code that returns Result, uses the ? operator, or handles
  failures. Does NOT cover panic/invariant design philosophy (only the rule
  that panics are for invariant violations).
metadata:
  org.kind: constraint
---

# Constraint: Rust Error Propagation

This constraint enforces the project's error-handling discipline: expected and
runtime failures flow through `Result<T, E>` and the `?` operator, while
`panic!`/`unwrap`/`expect` are reserved for invariant violations, tests, and
provably-impossible cases.

## Triggers

Load this skill when:

- Writing or reviewing a function that returns `Result<T, E>`.
- Using or introducing the `?` operator.
- Defining a custom error type or implementing `From` for error conversion.
- Reviewing code that calls `unwrap()`, `expect()`, or `panic!`.
- Designing the boundary between recoverable failures and invariant violations.

## Rules

1. Expected and runtime errors must use `Result<T, E>`, NOT `panic!`.
2. Use the `?` operator to propagate errors; do not match-and-return manually
   when `?` suffices.
3. No `unwrap()` / `expect()` on recoverable conditions in production code.
4. Implement `From` for error conversion so `?` works seamlessly across error
   types.
5. `panic!` is only for invariant violations and bugs, never for expected
   failures.
6. Reserve `unwrap()` / `expect()` for tests and provably-impossible cases
   (with a comment explaining why the case is impossible).

## References

- Operational skill: `rust-error-handling`.
- Docs: `docs/rust/error-handling.md`.

## Out of scope

- Panic/invariant design philosophy beyond the rule that panics are for
  invariant violations.
- Logging/tracing strategy for errors.
- Async cancellation semantics — see `docs/rust/async-tokio.md`.

## Violation examples

### `unwrap()` on a recoverable condition

```rust
fn parse_config(raw: &str) -> Config {
    serde_json::from_str(raw).unwrap()   // FORBIDDEN: parse can fail
}
```

Correct:

```rust
fn parse_config(raw: &str) -> Result<Config, serde_json::Error> {
    serde_json::from_str(raw)
}
```

### `panic!` for an expected failure

```rust
fn open_input(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => panic!("could not read input"),  // FORBIDDEN
    }
}
```

Correct: return `Result<String, io::Error>` and let the caller decide.

### Missing `From` impl blocking `?`

```rust
fn load() -> Result<Config, AppError> {
    let raw = fs::read_to_string("cfg.json")?;  // FORBIDDEN: io::Error
                                                  // does not convert to AppError
    Ok(serde_json::from_str(&raw)?)
}
```

Correct: `impl From<io::Error> for AppError` and
`impl From<serde_json::Error> for AppError`, then `?` works seamlessly.

### `unwrap()` without a justification comment

```rust
let n: u32 = items.len().try_into().unwrap();  // FORBIDDEN: no justification
```

Acceptable only with a comment:

```rust
// items is bounded to MAX_ITEMS <= u32::MAX by the constructor
let n: u32 = items.len().try_into().expect("bounded by constructor");
```

## How to check

```bash
cargo clippy --workspace --all-targets -- -D warnings \
  -W clippy::unwrap_used \
  -W clippy::expect_used
# or set these lints in the [lints] table / #![deny(...)] in the crate root.
cargo test --all-features   # runtime validation of error paths
```

Manual review: grep for `unwrap()`, `expect(`, `panic!`, `unreachable!`,
`todo!`, `unimplemented!` and justify each occurrence.
