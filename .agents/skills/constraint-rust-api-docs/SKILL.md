---
name: constraint-rust-api-docs
description: |
  Enforces API documentation rules during Rust code execution. Load when adding
  or modifying pub items, crate roots, or public APIs. Does NOT cover internal
  implementation comments.
metadata:
  org.kind: constraint
---

# Constraint: Rust API Documentation

This constraint enforces documentation coverage and structure for public APIs.
Every `pub` item is part of the crate's contract and must be documented;
functions that can panic or fail must declare so in dedicated sections.

## Triggers

Load this skill when:

- Adding or modifying any `pub` item (fn, struct, enum, trait, const, type,
  module).
- Editing crate root (`lib.rs` / `main.rs`) module docs.
- Reviewing a PR that changes the public API surface.
- Adding a function that can `panic!` or return `Result`.
- Writing doc comments and intra-doc links.

## Rules

1. Every `pub` item must have a `///` doc comment.
2. Crate root must have `//!` module docs (crate-level documentation).
3. Functions that can panic must have a `# Panics` section explaining when.
4. Functions returning `Result` must have a `# Errors` section describing the
   error variants that can be returned.
5. `unsafe` functions must have a `# Safety` section describing caller
   obligations.
6. Non-trivial public functions should have a `# Examples` section with a
   runnable doctest.
7. Use intra-doc links `[Type]` / [`Type::method`] not bare URLs for std and
   crate items.

## References

- Operational skill: `rust-api-design`.
- Docs: `docs/rust/documentation-guidelines.md`.

## Out of scope

- Internal implementation comments (`//` inside function bodies).
- Commit messages and changelog entries.
- README and user-facing prose.

## Violation examples

### `pub fn` without a doc comment

```rust
pub fn parse(raw: &str) -> Config {  // FORBIDDEN: no doc comment
    // ...
}
```

Correct:

```rust
/// Parses a [`Config`] from `raw`.
///
/// # Errors
///
/// Returns [`ParseError`] if `raw` is not valid config syntax.
///
/// # Examples
///
/// ```
/// use mycrate::parse;
/// let cfg = parse("k=v").unwrap();
/// ```
pub fn parse(raw: &str) -> Result<Config, ParseError> {
    // ...
}
```

### `Result`-returning function without `# Errors`

```rust
/// Loads the config.
pub fn load() -> Result<Config, io::Error> {  // FORBIDDEN: no # Errors
    // ...
}
```

### Bare URL instead of intra-doc link

```rust
/// See https://doc.rust-lang.org/std/string/struct.String.html
pub fn name() -> String {  // FORBIDDEN: bare URL
    // ...
}
```

Correct:

```rust
/// Returns the name as a [`String`].
pub fn name() -> String {
    // ...
}
```

### Missing crate-level module docs

```rust
// lib.rs
pub mod api;   // FORBIDDEN: no //! crate docs at top of file
```

Correct:

```rust
//! My crate does X.
//!
//! See the [guide][guide] for a walkthrough.
//!
//! [guide]: guide.html
pub mod api;
```

## How to check

```bash
# Build docs and surface missing-doc warnings.
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items

# Enable the missing_docs lint in the crate or workspace:
#   #![deny(missing_docs)]
# or in Cargo.toml [lints.rust] missing_docs = "deny".

cargo clippy --workspace --all-targets -- -D warnings
# relevant lints: missing_errors_doc, missing_panics_doc,
# missing_safety_doc, doc_markdown, needless_doctest_main
```

Manual review: every `pub` item has a `///` comment; panicking functions have
`# Panics`; `Result` functions have `# Errors`; `unsafe fn` have `# Safety`;
links use intra-doc syntax.
