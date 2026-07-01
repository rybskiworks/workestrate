# Rust documentation guidelines

## Purpose

Establish generic, repo-independent conventions for writing, organizing, and maintaining Rust documentation so that future agents produce docs that render correctly in rustdoc, include runnable examples, and follow the official Rust API Guidelines documentation chapter. The two pillars are the rustdoc book for mechanics and the API Guidelines documentation chapter for content quality.

## Sources used

### Primary guidelines

- https://rust-lang.github.io/api-guidelines/documentation.html — Rust API Guidelines documentation chapter.
- https://rust-lang.github.io/api-guidelines/ — root index of the API Guidelines.
- https://rust-lang.github.io/api-guidelines/checklist.html — API Guidelines checklist.

### Rustdoc book

- https://doc.rust-lang.org/rustdoc/ — rustdoc home.
- https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html — how to write documentation.
- https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html — documentation tests.
- https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html — the `#[doc]` attribute.
- https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html — intra-doc links.
- https://doc.rust-lang.org/rustdoc/lints.html — rustdoc lints.

### RFCs referenced by the guidelines

- https://github.com/rust-lang/rfcs/pull/1687 — RFC 1687, referenced by C-CRATE-DOC.
- https://github.com/rust-lang/rfcs/blob/master/text/1574-more-api-documentation-conventions.md — RFC 1574, "Link all the things", referenced by C-LINK.
- https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md — RFC 1105, API evolution and breaking changes, referenced by C-RELNOTES.

## Core guidance

Rustdoc turns doc comments into HTML and runs code examples as documentation tests, so documentation is both human-readable reference and executable validation. Future agents should treat every public doc comment as an API contract: it tells users what an item does, why they would use it, and how to use it correctly.

The Rust API Guidelines documentation chapter defines eight documentation recommendations (verified IDs):

- **C-CRATE-DOC** — crate-level docs are thorough and include examples.
- **C-EXAMPLE** — all public items have a rustdoc example.
- **C-QUESTION-MARK** — examples use `?`, not `try!` or `unwrap`.
- **C-FAILURE** — function docs include error, panic, and safety considerations.
- **C-LINK** — prose contains hyperlinks to relevant things.
- **C-METADATA** — `Cargo.toml` includes all common metadata.
- **C-RELNOTES** — release notes document all significant changes.
- **C-HIDDEN** — rustdoc does not show unhelpful implementation details.

These are the official recommendation IDs from https://rust-lang.github.io/api-guidelines/documentation.html. Do not invent alternative codes such as `C-DOC`; the crate-level-docs recommendation is `C-CRATE-DOC`, not `C-DOC`.

## Practical rules

### Doc comment forms

Use `///` to document the next item and `//!` to document the enclosing module or crate. Crate-level documentation belongs at the top of `lib.rs` or `main.rs` and uses inner doc comments (`//!`). Module-level documentation uses `//!` at the top of `mod.rs` or the named module file. All other public items use outer doc comments (`///`).

```rust
//! A tiny logging crate.
//!
//! Use [`Logger`] to write timestamped messages.

/// The main logger.
pub struct Logger;
```

Do not write `///!` for module docs; the correct token is `//!`.

### Recommended per-item structure

Follow the structure recommended by the rustdoc book at https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html:

1. A short sentence explaining what the item is.
2. A more detailed explanation, if needed.
3. At least one code example users can copy and paste.
4. Advanced explanations, if necessary.

The first paragraph (before the first blank line) becomes the summary in module overviews, search results, and tooltips. Keep that summary to one line. Do not restate the type system in prose; rustdoc auto-links types that appear in signatures.

```rust
/// Returns the number of active connections.
///
/// This counts only connections that have completed the handshake.
///
/// # Examples
///
/// ```
/// let pool = Pool::new();
/// assert_eq!(pool.active_count(), 0);
/// ```
pub fn active_count(&self) -> usize { ... }
```

### Required documentation (C-CRATE-DOC, C-EXAMPLE)

The API Guidelines documentation chapter at https://rust-lang.github.io/api-guidelines/documentation.html requires crate-level docs to be thorough and include examples (C-CRATE-DOC, referencing RFC 1687 at https://github.com/rust-lang/rfcs/pull/1687). It also recommends that every public module, trait, struct, enum, function, method, macro, and type definition have an example (C-EXAMPLE). Document public fields when their meaning is not obvious from the type.

Apply C-EXAMPLE within reason. A link to an applicable example elsewhere may suffice. The purpose of an example is usually to show why someone would use the item, not merely how.

Minimum documented surface:

- Crate-level docs (`//!`).
- Every `pub` module.
- Every `pub` struct, enum, trait, type alias, constant, static, function, and method.
- Public macros.
- Public fields if their meaning is not obvious.

### Standard sections (C-FAILURE)

Use the standard section headings recognized by rustdoc and the API Guidelines:

- `# Examples` — standard location for runnable examples.
- `# Errors` — error conditions; required when the function returns `Result`. This applies to trait methods as well.
- `# Panics` — panic conditions. It is not necessary to document every conceivable panic, especially when the panic originates in caller-provided logic, but err on the side of documenting more panic cases.
- `# Safety` — required for `unsafe fn` and `unsafe trait`; explains every invariant the caller must uphold.

There is no official `# Guarantees` section. Do not invent it.

```rust
/// Parses a string as a decimal number.
///
/// # Errors
///
/// Returns [`ParseIntError`] if the string is not a valid decimal integer.
///
/// # Examples
///
/// ```
/// let n: i32 = "42".parse().unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn parse(s: &str) -> Result<i32, ParseIntError> { ... }
```

### Examples use `?` (C-QUESTION-MARK)

The C-QUESTION-MARK recommendation says examples should use `?`, not `try!` and not `unwrap`. Example code is often copied verbatim; unwrapping an error should be a conscious decision. Hide the supporting `main` function with `#`-prefixed lines so the rendered docs show only the meaningful example.

````rust
/// ```rust
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// your;
/// example?;
/// code;
/// #
/// #     Ok(())
/// # }
/// ```
````

A line starting with `# ` is compiled but hidden in the rendered docs. To render a literal leading `#`, start the line with `## `.

```rust
/// ```
/// ## This renders as a literal leading `#`.
/// ```
```

### Doctest attributes

Documentation tests are described at https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html. A bare triple-backtick is treated the same as `rust`. Multiple attributes are comma-separated on the opening fence.

| Attribute | Meaning | When to use |
| --- | --- | --- |
| `ignore` | Skip entirely. | Rarely; prefer other attributes. |
| `should_panic` | Must compile and panic. | Examples that demonstrate panic conditions. |
| `no_run` | Compile but do not execute. | Network, file, or external-resource code. |
| `compile_fail` | Compilation must fail. | Demonstrating invalid usage; may pass in future Rust versions. |
| `edition2015` / `edition2018` / `edition2021` / `edition2024` | Set the edition. | When an example needs a specific edition. |
| `standalone_crate` | Do not merge with other doctests. | 2024-edition addition; use when isolation is required. |
| `ignore-<target>` | Skip on a target triple. | Examples that only make sense on some platforms. |
| `custom` | Non-Rust block with custom CSS classes. | Special formatting, not a doctest. |

Doctest preprocessing: common `allow` attributes are inserted automatically; crate-level attributes can be set with `#![doc(test(attr(...)))]`; if there is no `extern crate` and no `no_crate_inject`, rustdoc injects `extern crate <mycrate>;`; if there is no `fn main`, the example is wrapped in `fn main() {}`.

````rust
/// ```no_run
/// let listener = std::net::TcpListener::bind("0.0.0.0:80")?;
/// # Ok::<(), std::io::Error>(())
/// ```
````

### Cross-references and intra-doc links (C-LINK)

RFC 1574 "Link all the things" (https://github.com/rust-lang/rfcs/blob/master/text/1574-more-api-documentation-conventions.md), referenced by C-LINK, endorses hyperlinks in prose. Prefer intra-doc links over raw URLs. Full details are at https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html.

Intra-doc link forms that resolve include:

- `[Bar]`
- `[bar](Bar)`
- `` [`Bar`] ``
- `[bar][Bar]`
- `[bar][b]` with a reference definition `[b]: Bar`

Backticks are stripped in display: `` [`Option`] `` renders as `Option`. Paths such as `Self`, `self`, `super`, and `crate` work, as do generics like `Vec<T>`.

Namespace disambiguators (stripped in display) include:

- `struct@`
- `enum@`
- `trait@`
- `mod@` / `module@`
- `const@` / `constant@`
- `fn@` / `function@`
- `field@`
- `variant@`
- `method@`
- `derive@`
- `type@` / `tyalias@`
- `value@`
- `macro@`
- `prim@` / `primitive@`
- `union@`

Functions append `()`; macros append `!`. Rustdoc auto-disambiguates common trait-vs-derive conflicts such as `Clone`.

```rust
/// See also [`Client::connect`], [`Error`], and [`std::vec::Vec`].
///
/// Use [`crate::Config::build`] to construct a configuration.
///
/// This type implements [`Clone`](derive@Clone).
pub struct Client;
```

### The `#[doc]` attribute

The `#[doc]` attribute controls rendering and indexing. See https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html.

Common item-level uses:

- `#[doc(hidden)]` — hide from rendered docs.
- `#[doc(inline)]` — inline a re-exported module's items.
- `#[doc(no_inline)]` — do not inline a re-export.
- `#[doc(alias = "...")]` — add a search alias.

Crate-level uses inside `lib.rs`:

- `#![doc(html_root_url = "...")]`
- `#![doc(html_logo_url = "...")]`
- `#![doc(html_favicon_url = "...")]`
- `#![doc(html_playground_url = "...")]`
- `#![doc(html_no_source)]`
- `#![doc(test(no_crate_inject))]`
- `#![doc(test(attr(...)))]`
- `#![doc = include_str!("../../README.md")]` — inline an external file as crate docs.

```rust
#[doc(hidden)]
pub mod __internal;

#[doc(alias = "map")]
pub fn transform<T, F>(input: T, f: F) -> T { ... }
```

### Cargo.toml metadata (C-METADATA)

The C-METADATA recommendation from https://rust-lang.github.io/api-guidelines/documentation.html says `Cargo.toml` should include all common metadata. Required `[package]` fields are:

- `authors`
- `description`
- `license`
- `repository`
- `keywords`
- `categories`

Optional fields with rules:

- `documentation` — only set if docs are not hosted on docs.rs.
- `homepage` — only set if there is a unique website that is not redundant with `documentation` or `repository`.

Note that `rust-version` (MSRV) is not on the C-METADATA required-field list. It is an ecosystem convention, not part of this official recommendation.

```toml
[package]
name = "my_crate"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "A tiny HTTP client."
license = "MIT OR Apache-2.0"
repository = "https://github.com/org/my_crate"
keywords = ["http", "client", "network"]
categories = ["network-programming", "web-programming::http-client"]
```

### Release notes (C-RELNOTES)

C-RELNOTES from https://rust-lang.github.io/api-guidelines/documentation.html requires release notes to document all significant changes. A link to release notes should appear in crate-level docs and/or the `Cargo.toml` repository. Breaking changes must be clearly identified, following RFC 1105 (https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md). Every crates.io release should have a Git tag, with annotated tags preferred.

```rust
//! ## Release notes
//!
//! See [CHANGELOG.md](https://github.com/org/my_crate/blob/main/CHANGELOG.md).
//!
//! ### Breaking changes in 0.2.0
//!
//! - `Client::new` now returns `Result<Client, Error>`.
```

### Hidden implementations (C-HIDDEN)

C-HIDDEN from https://rust-lang.github.io/api-guidelines/documentation.html says rustdoc should include everything users need to use the crate fully and nothing more. Use `#[doc(hidden)]` and `pub(crate)` (per RFC 1422) to remove implementation details from the public API.

Appropriate uses:

- Implementation modules that must be public for macro expansion.
- Internal traits that support a public derive macro.
- Deprecated items kept for semver compatibility.

Do not use `#[doc(hidden)]` to hide intentionally public APIs.

```rust
#[doc(hidden)]
pub mod __internal;

pub(crate) fn helper() { ... }
```

### Doc comment style

Write in present tense, third person, and imperative. Start with a one-sentence summary. Add detail in later paragraphs.

```rust
/// Returns the number of active connections.
pub fn active_count(&self) -> usize { ... }

/// Build the request.
pub fn build(self) -> Request { ... }
```

### README-as-doctest pattern

To test the README as a doctest without publishing it as crate docs, use `#[doc = include_str!("...")]` together with `#[cfg(doctest)]`.

```rust
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
```

### Lint enforcement

For libraries, enforce documentation with `#![deny(missing_docs)]`. While ramping up, use `#![warn(missing_docs)]`. See the rustdoc lints page at https://doc.rust-lang.org/rustdoc/lints.html.

Default rustdoc lint levels include:

| Lint | Default level | Notes |
| --- | --- | --- |
| `rustdoc::broken_intra_doc_links` | WARN | Unresolved intra-doc links. |
| `rustdoc::private_intra_doc_links` | WARN | Public docs linking private items. |
| `missing_docs` | ALLOW | Also a rustc lint. |
| `rustdoc::missing_crate_level_docs` | ALLOW | Crate-level docs missing. |
| `rustdoc::missing_doc_code_examples` | ALLOW | Nightly-only. |
| `rustdoc::private_doc_tests` | ALLOW | Doctests on private items. |
| `rustdoc::invalid_codeblock_attributes` | WARN | Invalid attributes on code blocks. |
| `rustdoc::invalid_html_tags` | WARN | Invalid HTML tags. |
| `rustdoc::invalid_rust_codeblocks` | WARN | Rust code blocks that cannot be parsed. |
| `rustdoc::bare_urls` | WARN | URLs not turned into links. |
| `rustdoc::unescaped_backticks` | ALLOW | Unescaped backticks in prose. |
| `rustdoc::redundant_explicit_links` | WARN | Links that could be intra-doc links. |

Do not present `rustdoc::missing_doc_code_examples` as a stable enforcement tool; it is nightly-only.

## Review checklist

- [ ] Every public item has a doc comment.
- [ ] Crate-level and module-level docs use `//!`.
- [ ] Non-trivial public functions and methods include `# Examples`.
- [ ] Functions returning `Result` include `# Errors`.
- [ ] Functions that can panic include `# Panics`.
- [ ] `unsafe fn` and `unsafe trait` include `# Safety`.
- [ ] Intra-doc links use `` [`Target`] `` syntax and resolve.
- [ ] `#[doc(hidden)]` is applied only to intentional internals.
- [ ] Doctests pass (`cargo test --doc`).
- [ ] `Cargo.toml` includes all C-METADATA fields.
- [ ] Release notes and Git tags exist for releases.

## Implementation checklist

- [ ] Write thorough crate-level docs in `lib.rs` or `main.rs` using `//!`.
- [ ] Add `# Examples` sections to public functions and methods.
- [ ] Add `# Errors` sections to functions returning `Result`.
- [ ] Add `# Panics` sections where panics are possible.
- [ ] Add `# Safety` sections for every `unsafe fn` and `unsafe trait`.
- [ ] Replace raw URLs with intra-doc links where possible.
- [ ] Run `cargo doc --all-features` and fix broken links.
- [ ] Run `cargo test --doc` and fix failing examples.

## Validation hooks

- `cargo doc --all-features` — build docs with all features enabled.
- `cargo test --doc` — run documentation tests.
- `cargo doc --document-private-items` — render private items for internal review.
- `cargo doc --no-deps` — document only the current crate.
- `cargo clippy --all-features -- -W clippy::missing_docs_in_private_items -W clippy::missing_errors_doc -W clippy::missing_panics_doc` — surface doc-related Clippy warnings.
- Library crates: add `#![deny(rustdoc::broken_intra_doc_links)]` and `#![deny(missing_docs)]` at the crate root; also consider `#![warn(rustdoc::private_intra_doc_links)]`.

## Examples

### Crate-level docs

````rust
//! # my_crate
//!
//! `my_crate` provides a tiny HTTP client.
//!
//! ## Quick start
//!
//! ```rust
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use my_crate::Client;
//!
//! let client = Client::new();
//! let body = client.get("https://example.com")?;
//! #
//! #     Ok(())
//! # }
//! ```
//!
//! ## Release notes
//!
//! See the [changelog](https://github.com/org/my_crate/blob/main/CHANGELOG.md).
````

### Function with `# Errors` and `# Examples` using `?`

````rust
/// Sends a GET request and returns the response body as a string.
///
/// # Errors
///
/// Returns [`Error::Timeout`] if the request exceeds the configured timeout,
/// or [`Error::Network`] for connection failures.
///
/// # Examples
///
/// ```rust
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use my_crate::Client;
///
/// let client = Client::new();
/// let body = client.get("https://example.com")?;
/// #
/// #     Ok(())
/// # }
/// ```
pub fn get(&self, url: &str) -> Result<String, Error> { ... }
````

### Unsafe function with `# Safety`

````rust
/// Reads a value from a raw pointer.
///
/// # Safety
///
/// `ptr` must be non-null, properly aligned, and must point to a valid,
/// initialized value of type `T`. The caller must ensure that no mutable
/// reference to the same memory is active for the duration of this call.
///
/// # Examples
///
/// ```
/// let x = 5;
/// let ptr = &x as *const i32;
/// unsafe { assert_eq!(read(ptr), 5); }
///
/// unsafe fn read<T>(ptr: *const T) -> T {
///     *ptr
/// }
/// ```
unsafe fn read<T>(ptr: *const T) -> T { ... }
````

### Doctest using `no_run`

````rust
/// Binds a TCP listener on the given address.
///
/// # Examples
///
/// ```no_run
/// let listener = std::net::TcpListener::bind("0.0.0.0:8080")?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn bind(addr: &str) -> Result<std::net::TcpListener, std::io::Error> { ... }
````

### Intra-doc links

```rust
/// A client for the service.
///
/// Use [`Client::new`] to construct a client and [`Client::get`] to send
/// requests. Errors are reported via [`Error`].
///
/// [`Error`]: crate::Error
pub struct Client;

impl Client {
    /// Creates a new [`Client`].
    pub fn new() -> Client { ... }

    /// Sends a GET request. See [`Client`] for an overview.
    pub fn get(&self, url: &str) -> Result<String, Error> { ... }
}
```

### Cargo.toml `[package]` with all C-METADATA fields

```toml
[package]
name = "my_crate"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "A tiny HTTP client."
license = "MIT OR Apache-2.0"
repository = "https://github.com/org/my_crate"
keywords = ["http", "client", "network"]
categories = ["network-programming", "web-programming::http-client"]
```

## Common mistakes

- Forgetting `# Examples` on public methods.
- Omitting `# Errors` on functions that return `Result`.
- Omitting `# Panics` on functions that can panic.
- Omitting `# Safety` on `unsafe fn` or `unsafe trait`.
- Inventing a `# Guarantees` section; this is not a standardized rustdoc heading.
- Using raw URLs instead of intra-doc links.
- Writing `///!` instead of `//!` for module docs.
- Using `ignore` without a reason when `no_run` or `compile_fail` would be more precise.
- Using `unwrap` in examples instead of `?` (C-QUESTION-MARK).
- Omitting C-METADATA fields from `Cargo.toml`.
- Writing a multi-line first paragraph; the summary must be one line.
- Relying on `rustdoc::missing_doc_code_examples` as a stable enforcement tool; it is nightly-only.
- Treating `C-DOC` as a real recommendation ID; the correct crate-level-docs ID is `C-CRATE-DOC`.
- Listing `rust-version` as a C-METADATA required field; it is not part of the official list.

## Strict vs contextual guidance

### Strict

- All public items must have doc comments.
- `unsafe fn` and `unsafe trait` must have `# Safety` documentation.
- Functions returning `Result` must document error conditions in `# Errors`.
- Functions that can panic must document panic conditions in `# Panics`.
- Doctests must compile unless explicitly opted out with a valid attribute.
- Intra-doc links must resolve.

### Contextual

- `# Examples` is strongly recommended but may be omitted for trivial getters or obvious constructors.
- Module-level prose can be brief for small internal modules.
- `--document-private-items` is optional and usually only run locally.
- The exact tone and depth of crate-level docs may vary by audience.

## Policy decisions for individual repos

Each repository should decide and record:

- Whether `# Examples` is required on all public methods or only non-trivial ones.
- Whether to deny `missing_docs` via `#![deny(missing_docs)]` or only warn.
- Where docs are hosted: docs.rs, GitHub Pages, or an internal site.
- Whether broken intra-doc links are denied or only warned in CI.
- Whether to enforce a specific rustdoc test edition globally.
- Whether to test the README via `include_str!` + `#[cfg(doctest)]`.

## Related docs

- `docs/rust/api-design.md`
- `docs/rust/style-formatting.md`
- `docs/rust/testing.md`
- `docs/rust/lints-clippy.md`

## Related skills

None defined yet.
