---
name: rust-api-design
description: |
  Operational checklist for designing public Rust APIs: naming items, choosing
  visibility, trait design, conversions, builders, newtypes, sealed traits,
  #[non_exhaustive], and semver implications. Load when designing public APIs,
  naming pub items, deciding what to make public, designing traits, or reviewing
  a crate's public surface before release. Distilled from the Rust API
  Guidelines, rustdoc conventions, and the Rust design patterns catalog. Does
  NOT re-teach language mechanics; see docs/rust/*.md for full detail.
---

# Rust API Design

Distilled operational guidance for public Rust API surfaces. A **public API**
is any `pub` item a downstream crate can import; every `pub` item is part of
the semver contract. Full detail lives in:

- `docs/rust/api-design.md` — naming, conversions, traits, builders, sealed
  traits, `#[non_exhaustive]`, review/implementation checklists.
- `docs/rust/documentation-guidelines.md` — `///` vs `//!`, `# Examples` /
  `# Errors` / `# Panics` / `# Safety`, intra-doc links, doctest attributes.
- `docs/rust/design-patterns.md` — newtype, builder, RAII, strategy, type-state,
  sealed traits, idioms, and anti-patterns (Deref polymorphism, borrow-clone).

## Triggers

Load this skill when:

- Designing or reviewing a public API surface (`pub` items, traits, error
  types, constructors).
- Naming items (functions, types, traits, variants, features) or choosing
  `as_` / `to_` / `into_` / `with_` prefixes.
- Deciding what to make `pub` vs `pub(crate)` vs `#[doc(hidden)]`.
- Designing traits (object safety, sealing, bounds).
- Adding conversions (`From`/`TryFrom`/`AsRef`/`AsMut`/`FromStr`).
- Reviewing a crate before release (semver, docs, clippy).

## Naming (RFC 430 casing)

| Item | Convention |
|------|------------|
| Modules, functions, methods, fields, locals | `snake_case` |
| Types, traits, enum variants | `UpperCamelCase` |
| Statics, constants | `SCREAMING_SNAKE_CASE` |
| Macros | `snake_case!` |
| Type parameters | concise `UpperCamelCase` (`T`) |
| Lifetimes | short lowercase (`'a`, `'de`) |

Acronyms are one word (`Uuid`, not `UUID`); no `-rs`/`-rust` crate suffix;
word order consistent with std (verb-object-error: `ParseAddrError`).

### Conversion prefixes (C-CONV)

| Prefix | Cost | Pattern |
|--------|------|---------|
| `as_` | Free | `borrowed -> borrowed` |
| `to_` | Expensive | `borrowed -> borrowed`; `borrowed -> owned` (non-`Copy`); `owned -> owned` (`Copy`) |
| `into_` | Variable | `owned -> owned` (non-`Copy`) |
| `with_` | — | Secondary constructors |

`as_`/`into_` decrease abstraction; `to_` stays level. Wrappers expose
`into_inner()`. `mut` appears in the return type, not the name
(`as_mut_slice`, not `as_slice_mut`).

### Getters (C-GETTER)

No `get_` prefix; name getter after the field; mutable accessor is
`field_mut`. Reserve `get` for validated lookup (`get(&self, k) -> Option<&V>`,
`get_unchecked`).

### Collections (C-ITER, C-ITER-TY)

`iter() -> Iter`, `iter_mut() -> IterMut`, `into_iter() -> IntoIter`. Prefix
the type with the owning module in signatures (`vec::IntoIter<T>`).

### Features (C-FEATURE)

Name `abc`, never `use-abc`/`with-abc`/`no-abc`. Features are additive.
Serde support is feature `serde` exactly (`serde = ["dep:serde"]`).

## Trait Impl Checklist (public types)

For every public type, eagerly derive/implement where applicable (orphan rule
blocks downstream impls):

- `Debug` (always; non-empty repr — C-DEBUG, C-DEBUG-NONEMPTY).
- `Clone`, `Copy` (if cheap and value-like).
- `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`.
- `Default` (often identical to `new`).
- `Display` (if it has a user-facing string form).
- `Send` + `Sync` (assert via compile-time tests for raw-pointer types).
- Error types: `std::error::Error + Send + Sync + 'static`; use `source()`,
  not deprecated `description()`/`cause()`. Mark `#[non_exhaustive]`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Point { x: f64, y: f64 }
```

## Conversions (C-CONV-TRAITS)

Implement `From`, `TryFrom`, `AsRef`, `AsMut`. **Never** implement
`Into`/`TryInto` directly (blanket impls exist). `From` must not fail; use
`TryFrom` when fallible. Implement `FromStr` for text parsing (paired with a
`Parse*Error`). Collections implement `FromIterator` + `Extend`.

`from_` inherent constructors differ from `From<T>`: may be `unsafe`
(`Box::from_raw`), may take extra args (`u64::from_str_radix`), or used when
input type does not fully determine output.

## Documentation Requirements

- `///` on every public item; `//!` for crate/module level (never `///!`).
- First line = one-sentence summary (used in search/tooltips).
- `# Examples` on non-trivial public items; use `?`, not `unwrap`/`try!`
  (C-QUESTION-MARK). Hide setup with `# ` lines.
- `# Errors` required when returning `Result`; `# Panics` when panics possible;
  `# Safety` required for `unsafe fn`/`unsafe trait`. No invented `# Guarantees`.
- Prefer intra-doc links (`` [`Type`] ``, `[Type::method]`) over raw URLs.
- `Cargo.toml` C-METADATA: `authors`, `description`, `license`, `repository`,
  `keywords`, `categories`.
- Libraries: `#![deny(missing_docs)]` + `#![deny(rustdoc::broken_intra_doc_links)]`.

## Patterns (when to apply)

| Pattern | When | Notes |
|---|---|---|
| **Newtype** (C-NEWTYPE) | Domain IDs, units, trait isolation | Zero-cost; derive common traits. |
| **Builder** (C-BUILDER) | Many optional/validated fields | Non-consuming preferred (`&mut self` → `&mut Self`, `build(&self)`); consuming for one-liners. |
| **Sealed trait** (C-SEALED) | Forbid downstream impls | Private `Sealed` supertrait; document "sealed". Still breaking to remove/change public methods. |
| **`#[non_exhaustive]`** | Public enums/structs in published libs | Adding variants/fields is non-breaking. Required for published libraries; optional for internal binaries. |
| **Type-state** | Compile-time state transitions | PhantomData markers; consider data-carrying enum for runtime state. |
| **RAII guard** | Resource acquire/release | `Drop` must never panic/block; expose `close() -> Result<...>` for fallible teardown. |
| **`#[doc(hidden)]`** | Macro-support internals, deprecated items | Never use to hide intentionally public APIs. |

## Semver Implications (RFC 1105)

- Every `pub` item is part of the semver contract.
- **Breaking**: removing a public item; changing a public signature; adding a
  trait bound to a data struct; adding a field to a non-`#[non_exhaustive]`
  struct; adding an enum variant without `#[non_exhaustive]`.
- **Non-breaking**: adding new `pub` items; deriving more traits; adding
  `#[non_exhaustive]` variants/fields; adding default trait methods.
- A crate at `>= 1.0.0` requires all public dependencies stable (C-STABLE).
  `impl From<foreign::Error>` puts that foreign type in your public API.
- Struct bounds: derive without duplicating bounds (`pub struct Good<T> {
  value: T }`, not `<T: Clone + Debug>`). Adding a bound is breaking.## Anti-patterns

- `unwrap()`/`expect()`/`panic!()` in public API code (clippy
  `unwrap_used`/`expect_used`/`panic`).
- Implementing `Into`/`TryInto` directly — use `From`/`TryFrom`.
- `Deref` to share methods (only smart pointers `Deref`) — Deref polymorphism.
- `get_` prefix on field getters.
- `bool`/raw numeric args where enum/newtype is clearer (boolean blindness).
- `()` as a public error type.
- Feature names `use-x`/`with-x`/`no-x`; serde feature not named `serde`.
- `#[deny(warnings)]` in source (breaks forward compat) — enforce in CI.
- `static mut` for global state — use `Atomic*`/`OnceLock`/`LazyLock`.

## Verification Commands

```bash
cargo check --all-features
cargo test --all-features
cargo test --doc
cargo doc --no-deps --all-features
cargo clippy --all-features -- -D warnings
cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
cargo semver-checks        # before release; or: cargo public-api diff
```

Compile-time `Send`/`Sync` assertions for raw-pointer types:

```rust
#[test] fn assert_send_sync() {
    fn assert_send<T: Send>() {} fn assert_sync<T: Sync>() {}
    assert_send::<MyType>(); assert_sync::<MyType>();
}
```

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `missing_docs` warning | Public item undocumented | Add `///`; enable `#![deny(missing_docs)]`. |
| Broken intra-doc link | Unresolved `[Type]` reference | Use correct path/namespace disambiguator; run `cargo doc`. |
| Doctest fails | Example uses `unwrap` or won't compile | Use `?` + hidden `# fn main`; add `no_run` for I/O. |
| `cargo semver-checks` fails | Breaking change since last release | Restore signature, add `#[non_exhaustive]`, or bump major version. |
| Downstream cannot impl trait | Orphan rule | Implement standard traits upstream; or seal and impl yourself. |
| Adding enum variant breaks downstream | Missing `#[non_exhaustive]` | Add `#[non_exhaustive]` before `1.0.0`. |
| `From` impl panics | Fallible conversion as `From` | Switch to `TryFrom`. |

## Related Docs

- `docs/rust/api-design.md`
- `docs/rust/documentation-guidelines.md`
- `docs/rust/design-patterns.md`
