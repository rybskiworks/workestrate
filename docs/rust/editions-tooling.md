# Rust Editions, rustdoc Tooling, and Toolchain Management

## Purpose

Document Rust editions, rustdoc tooling, rustup toolchain management, and Minimum Supported Rust Version (MSRV) policy so that future agents can:

- Set `edition` and `rust-version` correctly in `Cargo.toml`.
- Migrate crates between editions with `cargo fix` and manual review.
- Pin toolchains reproducibly with `rust-toolchain.toml`.
- Generate, validate, and link rustdoc documentation.
- Keep language `edition`, rustfmt `style_edition`, and rustdoc `--edition` aligned.

This file focuses on **mechanics and policy**. Concrete rustfmt configuration tables and style-edition formatting changes live in `docs/rust/style-formatting.md`; doc-comment structure, doctest fence attributes, and intra-doc link forms live in `docs/rust/documentation-guidelines.md`.

## Sources used

### Seed URLs (required citations)

- https://doc.rust-lang.org/edition-guide/
- https://doc.rust-lang.org/edition-guide/rust-2024/
- https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html
- https://doc.rust-lang.org/rustdoc/
- https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
- https://doc.rust-lang.org/rustdoc/write-documentation/what-is-rustdoc.html (content folded into the top-level rustdoc book; cite as historical seed)
- https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html
- https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
- https://doc.rust-lang.org/rust-by-example/meta/doc.html

### Edition semantics and migration

- https://doc.rust-lang.org/edition-guide/rust-2015/index.html
- https://doc.rust-lang.org/edition-guide/rust-2018/index.html
- https://doc.rust-lang.org/edition-guide/rust-2021/index.html
- https://doc.rust-lang.org/edition-guide/rust-2024/index.html
- https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html
- https://doc.rust-lang.org/edition-guide/editions/index.html
- https://doc.rust-lang.org/edition-guide/editions/advanced-migrations.html
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-edition-field
- https://doc.rust-lang.org/cargo/reference/rust-version.html
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
- https://doc.rust-lang.org/cargo/commands/cargo-fix.html
- https://doc.rust-lang.org/cargo/reference/semver.html

### rustdoc and rustdoc JSON

- https://doc.rust-lang.org/cargo/commands/cargo-doc.html
- https://doc.rust-lang.org/cargo/commands/cargo-rustdoc.html
- https://doc.rust-lang.org/rustdoc/command-line-arguments.html
- https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html
- https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html
- https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html
- https://doc.rust-lang.org/rustdoc/lints.html
- https://doc.rust-lang.org/rustdoc/unstable-features.html
- https://doc.rust-lang.org/rustdoc/advanced-features.html
- https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/

### Lint groups

- https://doc.rust-lang.org/rustc/lints/groups.html

### rustup and components

- https://rust-lang.github.io/rustup/
- https://rust-lang.github.io/rustup/overrides.html
- https://rust-lang.github.io/rustup/concepts/components.html
- https://rust-lang.github.io/rustup/concepts/profiles.html
- https://rust-lang.github.io/rustup/concepts/channels.html
- https://rust-lang.github.io/rustup/concepts/toolchains.html
- https://rust-lang.github.io/rustup/cross-compilation.html
- https://rust-lang.github.io/rustup/environment-variables.html

### Style edition

- https://rust-lang.github.io/rfcs/3338-style-evolution.html

### Target support

- https://doc.rust-lang.org/rustc/platform-support.html

## Core guidance

### What Rust editions are

Editions are Rust's mechanism for backwards-incompatible language changes **without** splitting the ecosystem. They are opt-in, per-crate, and cross-edition compatible. Editions are **not SemVer** — they are a separate compatibility dimension. As the Edition Guide states: "When there are backwards-incompatible changes, they are pushed into the next edition. Since editions are opt-in, existing crates won't use the changes unless they explicitly migrate." All Rust code compiles to the same internal representation regardless of edition.

Key properties:

- Declared per-crate in `Cargo.toml` via `[package] edition`.
- Different crates in one dependency graph may use different editions; the compiler handles interop.
- Default when omitted: **2015**. `cargo new` writes the newest stable edition explicitly.
- Macro tokens carry their edition marker, so macros can be called from crates of any edition.
- Upcoming/unreleased editions require a **nightly** toolchain. Once stable, stable Rust compiles them.

### Editions and their stable releases

| Edition | Stable release | Theme |
|---------|----------------|-------|
| 2015 | Rust 1.0 (May 2015) | Baseline; stability |
| 2018 | Rust 1.31.0 (Dec 6 2018) | Created the edition system; productivity |
| 2021 | Rust 1.56.0 (Oct 21 2021) | Ergonomics |
| 2024 | Rust 1.85.0 (Feb 20 2025) | Safety and precision |

### Edition 2015 (baseline)

The original Rust 1.0, retroactively designated "edition 2015". New crates should not use it.

- `extern crate` required.
- Module paths needed leading `::` for crate-relative references.
- No `async`/`await`, no `?` in `main`, no Non-Lexical Lifetimes (NLL).

### Edition 2018 changes

- Path clarity / module system: `extern crate` no longer required; `crate::`, `super::`, `self::` paths; no leading `::` for crate-relative refs; `mod foo;` can use `foo.rs`.
- `dyn` keyword for trait objects (bare `Box<Trait>` becomes a hard error; use `Box<dyn Trait>`).
- `async`/`await` keywords stabilized.
- Non-Lexical Lifetimes (NLL).
- `impl Trait` in return and argument position.
- Raw identifiers `r#name`.
- `?` operator usable in `fn main() -> Result<(), E>` and tests.
- New reserved keywords: `async`, `await`, `dyn`, `try` (reserved), `gen` (reserved).
- Anonymous params in trait methods (`fn foo(usize)`) → hard error (use `_: usize`).

### Edition 2021 changes

- Disjoint closure captures (RFC 2229): closures capture individual fields, not whole structs.
- `IntoIterator` for arrays by value: `for x in [1,2,3]` consumes the array. Method-call `.into_iter()` on older editions still resolves to the `(&[T;N])` impl for back-compat (lint: `array_into_iter`).
- `panic!` consistency: always uses `fmt::Display` semantics (lint: `non_fmt_panics`).
- Or-patterns in `macro_rules!` matchers.
- Reserved syntax `prefix"..."` and `prefix#...` (lint: `rust_2021_prefixes_incompatible_syntax`).
- Prelude additions: `TryFrom`, `TryInto`, `FromIterator`.
- Default Cargo resolver = "2" (`resolver = "2"` in `[package]` or workspace).
- Bare trait objects promoted to hard error.

### Edition 2024 changes (comprehensive)

Released with Rust 1.85.0 (Feb 2025):

- `unsafe_op_in_unsafe_fn` becomes **warn-by-default** in 2024. See the dedicated section below.
- `unsafe extern` blocks required: `extern "C" { ... }` must be `unsafe extern "C" { ... }` (lint `missing_unsafe_on_extern` becomes a hard error).
- `unsafe impl` for traits: marker/autotraits require `unsafe impl` where the trait declares it.
- `gen` blocks and the `gen` keyword reserved (RFC 3513) — generator/iterator blocks; `gen` is now a hard keyword (lint `keyword_idents_2024`).
- RPIT lifetime capture rules: `impl Trait` in return position captures all in-scope lifetimes by default in 2024; use `use<'a, T>` capture syntax for precise control (lints `impl_trait_overcaptures`, `impl_trait_redundant_captures`).
- Never type (`!`) fallback changes: `!` no longer falls back to `()` in more positions (lint `dependency_on_unit_never_type_fallback`).
- Tail-expression temporary drop order changed (lint `tail_expr_drop_order`).
- `if let` temporary rescope: temporaries in `if let` conditions drop at a different point (lint `if_let_rescope`).
- Macro fragment specifiers: `expr` now also matches `const { ... }`; new `expr_2021` fragment preserves old behavior (lint `edition_2024_expr_fragment_specifier`).
- `||` closure-type syntax removed from generics (was rarely used).
- Newly-marked-`unsafe` standard-library functions: `std::env::set_var`, `std::env::remove_var`, `std::process::abort`, `std::arch::asm!` etc. (lint `deprecated_safe_2024`, allow-by-default).
- `offset_of!` macro stabilized.
- `IntoIterator` impls for `Box<[T]>` / `Box<[(K,V)]>` re-dispatch changes.
- A new rustfmt style edition (2024) — see the style edition section below.

### The corrected fact about `unsafe_op_in_unsafe_fn`

**Default levels:**

- Editions 2015, 2018, 2021: **allow-by-default**.
- Edition 2024: **warn-by-default**.

**Mechanism:** rustc's `Lint` struct has two fields: `default_level: Level` (= `Allow`) **and** `edition_lint_opts: Option<(Edition, Level)>` (= `Some((Edition2024, Warn))`). The `default_level(edition)` method returns the edition-specific override when the crate edition ≥ the configured edition, else the static default:

```rust
pub fn default_level(&self, edition: Edition) -> Level {
    self.edition_lint_opts
        .filter(|(e, _)| *e <= edition)
        .map(|(_, l)| l)
        .unwrap_or(self.default_level)
}
```

The rustc rustdoc for the lint states verbatim: "This lint is 'allow' by default on editions up to 2021, from 2024 it is 'warn' by default."

**Why the listing page is misleading:** the rustc "Allowed-by-default Lints" page lists `unsafe_op_in_unsafe_fn` as allowed because the docs generator reads only the first level keyword (`Allow,`) and ignores the `@edition Edition2024 => Warn;` clause. It does not document edition-specific overrides. Do **not** cite that page as evidence the 2024 default is allow.

**Why `core`/`std` carry explicit `#![deny(...)]`:** at Rust 1.85.0 the standard library crates are on **edition 2021**, so they need the explicit `deny` because the 2021 default is `allow`. This does **not** contradict the 2024 warn default.

The lint is a member of the `rust_2024_compatibility` lint group. Running `cargo fix --edition` promotes that group, surfacing migration warnings even on older editions.

**Recommendation:** use `#![warn(unsafe_op_in_unsafe_fn)]` (or `deny`) for all editions, and always wrap unsafe operations inside `unsafe fn` bodies in `unsafe {}` blocks.

### The `edition` Cargo.toml field

```toml
[package]
edition = "2024"
```

Valid values: `"2015"`, `"2018"`, `"2021"`, `"2024"`. Affects all targets in the package (lib, bins, tests, benches, examples). Per-target override possible via `[[bin]] edition = "..."` (rare/advanced). Default 2015 if omitted.

### The `rust-version` (MSRV) Cargo.toml field

```toml
[package]
rust-version = "1.85"
```

- Bare version (≥1 component); no semver operators or pre-release.
- Respected as of cargo **1.56**. `1.56` and `1.56.0` both valid.
- On violation, cargo reports an error. Override with `cargo build --ignore-rust-version`.
- `cargo add` auto-selects dep versions compatible with `rust-version`.
- Resolver knob (cargo ≥1.84): `resolver.incompatible-rust-versions = "fallback"` in `.cargo/config.toml` makes the resolver fall back to older deps that satisfy the MSRV.

SemVer impact: bumping `rust-version` is classified as a **minor incompatibility** (Cargo semver: "Possibly-breaking: changing the minimum version of Rust required"). Treat an MSRV bump as a minor version bump for 1.x crates (and per Cargo 0.x convention, the leftmost-non-zero component for 0.y.z).

### Edition migration

The canonical loop:

1. `cargo fix --edition` — applies machine-applicable fixes to migrate to the **next** edition. Does **not** edit `edition` in Cargo.toml; you must do that manually afterward.
2. Manually change `edition` in `Cargo.toml`.
3. `cargo build --all-targets` to verify.
4. `cargo fix --edition-idioms` — applies current-edition idiom suggestions.
5. Run tests, including doctests.
6. Run `cargo fmt` with the new style edition.

Notes:

- Edition 2021 has **no** idiom lints.
- 2018 idiom lints are `unused-extern-crates` and `explicit-outlives-requirements` and have known problems; review suggestions.
- `cargo fix --broken-code` keeps going even with pre-existing errors.
- `cargo fix --allow-dirty` / `--allow-staged` / `--allow-no-vcs` for non-clean trees.
- `cargo fix` only operates on code actually compiled by `cargo check` — use `--all-features` and `--target <triple>` to reach cfg-gated code.
- Migration is per-crate & incremental; migrate one crate at a time in a workspace.

### rustfmt style edition relationship

Style editions are an **independent** versioning axis from language editions, defined by RFC 3338 ("Style Evolution"). Language editions 2015, 2018, 2021 share one style edition; 2024 introduces a distinct style edition. Stabilized in rustfmt 1.8.0 / Rust 1.85.0.

- `cargo fmt` uses the style edition matching the language `edition` by default.
- Override via `style_edition = "2024"` in `rustfmt.toml` or `rustfmt --style-edition 2024`.
- Set **both** `edition` (Cargo.toml) and `style_edition` (rustfmt.toml) explicitly so editor format-on-save, local `cargo fmt`, and CI match.
- Run `cargo fmt` after edition migration; formatting output may change.

For the full rustfmt stable/nightly option tables, the 2024 concrete formatting changes, and the nightly-only mechanism, see `docs/rust/style-formatting.md`.

### rustdoc at a glance

rustdoc turns doc comments into HTML and runs code examples as tests. Use `cargo doc` for the normal workflow; use `cargo rustdoc -- <rustdoc-flags>` to pass flags directly.

Key principles:

- `cargo doc` output is cumulative across runs; `cargo clean --doc` clears it.
- rustdoc sets `#[cfg(doc)]` during doc generation but **not** during doctests.
- Use intra-doc links for internal references; they are checked by rustdoc.
- JSON output is unstable and requires nightly.

### `cargo doc` and `cargo rustdoc` command surface

`cargo doc` builds crate + dependency docs into `target/doc`. Output is cumulative across runs; `cargo clean --doc` clears it.

Common `cargo doc` options:

| Option | Effect |
|--------|--------|
| `--open` | Open docs in browser after building. |
| `--no-deps` | Skip dependency docs. |
| `--document-private-items` | Include non-public items; default-on for binary targets. |
| `-p/--package <spec>` | Document a specific workspace package. |
| `--workspace` | Document the whole workspace. |
| `--exclude <spec>` | Exclude a package from `--workspace`. |
| `--lib` / `--bin <name>` / `--bins` | Document only the lib or specific binary. |
| `--example <name>` / `--examples` | Document examples. |
| `--features`/`-F`, `--all-features`, `--no-default-features` | Feature control. |
| `--target <triple>` | Cross-compile target for docs. |
| `-r/--release`, `--profile <name>` | Build profile. |
| `--target-dir <dir>` | Custom target directory. |
| `--ignore-rust-version` | Skip `rust-version` check. |
| `--locked`, `--offline`, `--frozen` | Network/reproducibility modes. |
| `-j/--jobs` | Parallel job limit. |
| `+<toolchain>` | Use a specific rustup toolchain. |

`cargo rustdoc -- <rustdoc-flags>` passes flags straight to rustdoc. Examples:

| Flag | Status | Effect |
|------|--------|--------|
| `--crate-name`, `--crate-type` | stable | Crate metadata. |
| `--edition` | stable | Affects docs and doctests (default 2015). |
| `--document-private-items` | stable | Include private items. |
| `-o/--out-dir` | stable | Output directory (default `doc/` in cwd). |
| `--extern NAME=PATH` | stable | Link an extern crate. |
| `--cfg`, `--check-cfg` | stable | Conditional compilation flags. |
| `--test`, `--test-args` | stable | Run doctests with args. |
| `--html-in-header` | stable | Inject HTML into `<head>`. |
| `--theme`, `--extend-css` | stable | Custom themes/CSS. |
| `--output-format html|json|doctest` | nightly (`-Z unstable-options`) | Choose output format. |
| `--extern-html-root-url NAME=URL` | nightly | Link extern crate to external docs. |
| `--show-coverage` | nightly | Emit doc-coverage data. |
| `--generate-link-to-definition` | nightly | Link types to their definitions. |
| `--scrape-examples-*` | nightly | Scrape code examples (RFC 3123). |

Disable docs for a target via `doc = false` in its manifest section.

## Practical rules

1. **Set `edition` explicitly in every `Cargo.toml`.** The default is 2015 if omitted.
2. **Use edition 2021 or 2024 for all new crates.** Edition 2018 is acceptable for existing crates; 2015 should be migrated.
3. **Set `rust-version` in `[package]`** to declare MSRV and test it in CI.
4. **Pin the toolchain via `rust-toolchain.toml`.** Commit this file so contributors and CI use the same compiler.
5. **Set `style_edition` explicitly in `rustfmt.toml`.** Match it intentionally to the language edition or pin separately.
6. **Run `cargo fmt` after edition migration.** Formatting output can change when `edition` or `style_edition` changes.
7. **Run `cargo doc` as part of CI.** Broken intra-doc links and failing doctests are bugs.
8. **All `///` examples must compile via doctests.** Mark non-runnable examples with `no_run`, `ignore`, `should_panic`, or `compile_fail`. See `docs/rust/documentation-guidelines.md` for the fence-attribute table.
9. **Use intra-doc links (`[``Type``]`) instead of raw URLs.** They are verified at doc time and survive refactoring. See `docs/rust/documentation-guidelines.md` for link forms and namespace disambiguators.
10. **Migrate editions with `cargo fix --edition`, then review manually.** Follow with `cargo build --all-targets` and tests.
11. **Test MSRV in CI.** Add the declared `rust-version` compiler to the CI matrix and verify `cargo +<msrv> build` passes.
12. **Install required components explicitly.** Use `rustup component add` or list them in `rust-toolchain.toml`.
13. **Never mix language edition and rustfmt style edition without intent.** They are independent knobs; set both explicitly.
14. **Use `#[cfg(doc)]` for platform-gated items you want visible in docs.** Combine as `#[cfg(any(windows, doc))]`. Remember it is **not** passed to doctests.
15. **Use `#[doc(cfg(...))]` for conditional APIs** so rendered docs show the conditions under which an item exists.
16. **Prefer `rust-toolchain.toml` (TOML) over the legacy `rust-toolchain` (plain text) file.** The TOML format supports components, targets, and profiles.
17. **Document edition and MSRV decisions in the repo README or contributing guide.** Contributors must know which compiler to install and which edition is in use.
18. **In edition 2024 crates, wrap unsafe operations inside `unsafe fn` bodies in `unsafe {}` blocks.** This is warn-by-default in 2024; consider `#![warn(unsafe_op_in_unsafe_fn)]` or `deny` for all editions.
19. **For re-exports, choose `#[doc(inline)]`, `#[doc(no_inline)]`, or `#[doc(hidden)]` intentionally.** Defaults depend on source-module visibility; see the re-exports section in Examples.
20. **Use `--extern-html-root-url NAME=URL` (nightly) to link extern crates to external docs** when local docs are not on disk.

## Review checklist

1. Does every `Cargo.toml` set `edition` explicitly (not relying on the 2015 default)?
2. Is `rust-version` declared in `[package]` and does CI test against it?
3. Does `rust-toolchain.toml` exist and is it committed?
4. Do all public items have `///` or `//!` doc comments?
5. Do all doctest examples compile (`cargo test --doc` passes)?
6. Are intra-doc links used instead of raw URLs for internal references?
7. Is `cargo fmt --check` enforced in CI?
8. Is `style_edition` set in `rustfmt.toml` if the project uses edition 2024?
9. Are `#[doc(cfg(...))]` attributes present for platform- or feature-gated public items?
10. Does `cargo doc --no-deps` complete without warnings about broken links?
11. Are `unsafe` functions inside edition 2024 crates wrapping their bodies in `unsafe {}` blocks?
12. Is the MSRV tested in CI (not just declared)?
13. Are required rustup components listed in `rust-toolchain.toml` or documented?
14. Has edition migration been reviewed manually after `cargo fix --edition`?
15. Are re-export visibility and inlining choices intentional (`#[doc(inline)]` / `#[doc(no_inline)]` / `#[doc(hidden)]`)?
16. Is the rustdoc `--edition` used for doctests aligned with the crate's language edition?

## Implementation checklist

1. Set `edition = "2021"` (or `"2024"`) in every `Cargo.toml` in the workspace.
2. Set `rust-version` in `[package]` to the minimum compiler the crate supports.
3. Create `rust-toolchain.toml` with `channel`, `components`, and `targets`.
4. Add `rustfmt.toml` with `edition` and `style_edition` matching the language edition (or an intentional separate pin).
5. Run `cargo fix --edition` to apply automatic migration fixes.
6. Run `cargo fix --edition-idioms` for additional idiomatic changes (none for 2021).
7. Run `cargo build --all-targets` after each migration step.
8. Run `cargo fmt` and review the diff.
9. Add `cargo fmt --check` to CI.
10. Add `cargo doc --no-deps` and `cargo test --doc` to CI.
11. Add MSRV build to the CI matrix: `cargo +<msrv> build`.
12. Install required components: `rustup component add clippy rustfmt rust-src`.
13. Add `#[doc(cfg(...))]` to any conditional public APIs.
14. Replace raw doc URLs with intra-doc links.
15. Verify `cargo doc --open` renders correctly locally.
16. Document edition, MSRV, and toolchain decisions in the README.

## Validation hooks

```bash
# Build docs for current crate only, including private items
cargo doc --no-deps --document-private-items

# Build and open docs in browser
cargo doc --open

# Run doctests
cargo test --doc

# Check formatting
cargo fmt --all -- --check

# Attempt automatic edition migration (use +nightly for upcoming edition)
cargo +nightly fix --edition --edition-idioms

# Show active toolchain and overrides
rustup show

# List installed components
rustup component list --installed

# Print compiler version
rustc --version

# Verify MSRV builds
cargo +1.85.0 build

# Build rustdoc JSON (nightly only)
cargo +nightly rustdoc -- --output-format json -Z unstable-options

# Run docs with all features enabled
cargo doc --all-features

# Check rustdoc lints at deny level
cargo doc --no-deps -- -D rustdoc::broken_intra_doc_links
```

## Examples

### `rust-toolchain.toml`

```toml
# Pin the toolchain for reproducible builds.
# See https://rust-lang.github.io/rustup/overrides.html
[toolchain]
channel = "1.85.0"
components = ["clippy", "rustfmt", "rust-src", "rust-analyzer"]
targets = ["wasm32-unknown-unknown"]
profile = "default"
```

### Edition migration with `cargo fix`

```bash
# Step 1: Automatic fixes for edition migration (does NOT edit Cargo.toml)
cargo fix --edition

# Step 2: Edit Cargo.toml edition manually
# edition = "2024"

# Step 3: Apply idiomatic changes for the new edition
cargo fix --edition-idioms

# Step 4: Verify everything compiles
cargo build --all-targets

# Step 5: Run all tests including doctests
cargo test --all-targets

# Step 6: Reformat with the new style edition
cargo fmt --check
```

### `Cargo.toml` with edition 2024 and `rust-version`

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "A demonstration crate"

[dependencies]
serde = { version = "1", features = ["derive"] }
```

### `#[doc(...)]` attribute catalog

Crate-level (`#![doc(...)]` in lib.rs/main.rs):

```rust
#![doc(html_favicon_url = "https://example.com/favicon.ico")]
#![doc(html_logo_url = "https://example.com/logo.png")]
#![doc(html_playground_url = "https://play.rust-lang.org")]
#![doc(issue_tracker_base_url = "https://github.com/org/my-crate/issues/")]
#![doc(html_root_url = "https://docs.rs/my-crate/0.1.0")]
#![doc(html_no_source)]
#![doc(test(no_crate_inject))]
#![doc(test(attr(allow(dead_code))))]
```

Item-level:

```rust
#[doc(hidden)]
pub mod __internal;

#[doc(alias = "map")]
#[doc(alias("transform", "convert"))]
pub fn transform<T, F>(input: T, f: F) -> T { f(input) }

#[doc(inline)]
pub use std::io::Error as IoError;
```

### Re-exports: inline, no_inline, hidden

Default inlining rules:

- A `pub use` from a **private** module is auto-inlined.
- A `pub use` from an ancestor that is `#[doc(hidden)]` is inlined.
- If the item itself is `#[doc(hidden)]`, it is not inlined and not visible.
- Edition 2018+: `pub use` of an external dependency is **not** eagerly inlined unless `#[doc(inline)]` is added.

Force inline (even when source module is public):

```rust
#[doc(inline)]
pub use std::collections::HashMap;
```

Prevent inline (even when source module is private):

```rust
#[doc(no_inline)]
pub use crate::internal::helper;
```

Hide from docs:

```rust
#[doc(hidden)]
pub use crate::internal::macro_support;
```

On inline, attributes like `cfg` are unioned; re-export doc-comments concatenate. `#[doc(alias)]` / `#[doc(inline)]` / `#[doc(hidden)]` on the re-export are **not** inherited by the inlined item.

### rustdoc LINTS table

Set via `#![allow|warn|deny(rustdoc::NAME)]`. Except `missing_docs`, these lints only run under rustdoc (not plain rustc).

| Lint | Default level | Scope |
|------|---------------|-------|
| `rustdoc::broken_intra_doc_links` | WARN | Unresolved intra-doc links. |
| `rustdoc::private_intra_doc_links` | WARN | Public docs linking private items. |
| `missing_docs` | ALLOW | Also a rustc lint. |
| `rustdoc::missing_crate_level_docs` | ALLOW | Crate root has no docs. |
| `rustdoc::missing_doc_code_examples` | ALLOW | Nightly-only. |
| `rustdoc::private_doc_tests` | ALLOW | Doctests on private items. |
| `rustdoc::invalid_codeblock_attributes` | WARN | Mistyped fence attrs. |
| `rustdoc::invalid_html_tags` | WARN | Invalid HTML in docs. |
| `rustdoc::invalid_rust_codeblocks` | WARN | Unparsable Rust blocks. |
| `rustdoc::bare_urls` | WARN | URLs not wrapped in link syntax. |
| `rustdoc::redundant_explicit_links` | WARN | Explicit link duplicating an intra-doc link. |
| `rustdoc::unescaped_backticks` | ALLOW | Unescaped backticks in prose. |

For the doctest fence attribute table (`ignore`, `should_panic`, `no_run`, `compile_fail`, edition fences, `standalone_crate`, `ignore-<target>`, `custom`), see `docs/rust/documentation-guidelines.md`.

### rustdoc JSON output

Unstable; requires nightly:

```bash
cargo +nightly rustdoc -- --output-format json -Z unstable-options
```

Tracking issue: rust-lang/rust#76578. The schema is explicitly experimental; do not rely on stability across versions. Schema lives at https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/.

Install toolchain docs JSON:

```bash
rustup component add --toolchain nightly rust-docs-json
```

Consumers include docs.rs, cargo-semverchecks, rust-analyzer, and IDE tooling.

`--output-format doctest` (nightly) emits JSON describing each doctest. `--show-coverage --output-format json` emits doc-coverage JSON.

### `#[cfg(doc)]` for cross-platform docs

```rust
#[cfg(any(windows, doc))]
#[doc(cfg(windows))]
pub fn windows_only() {}

#[cfg(any(unix, doc))]
#[doc(cfg(unix))]
pub fn unix_only() {}
```

CAVEAT: `cfg(doc)` is **not** passed to doctests. If a doctest needs the same gating, use explicit cfg or feature flags.

### `--extern-html-root-url` (nightly)

```bash
cargo +nightly rustdoc -- \
  --extern-html-root-url serde=https://docs.rs/serde/latest \
  --extern-html-root-url tokio=https://docs.rs/tokio/latest
```

Use this to link extern crates to external docs when local docs are not on disk. rustdoc link resolution order: local on-disk docs → `--extern-html-root-url` → the crate's `html_root_url` → no link.

### `--html-in-header` for custom CSS/JS (stable)

```bash
cargo rustdoc -- --html-in-header header.html
```

Inject custom HTML/CSS/JS into `<head>`. Common uses: analytics, theme loader, kaTeX CSS/JS, mermaid.js diagrams. `--html-before-content` and `--html-after-content` are similar.

### rustup commands and override precedence

```bash
# Install a toolchain
rustup toolchain install 1.85.0

# Install a pinned nightly
rustup toolchain install nightly-2024-01-01

# Set default
rustup default 1.85.0

# Directory override
rustup override set 1.85.0 --path ./my-crate

# Add components/targets
rustup component add clippy rustfmt --toolchain 1.85.0
rustup target add wasm32-unknown-unknown --toolchain 1.85.0

# Show active toolchain
rustup show
```

Override precedence (first match wins):

1. Command-line shorthand: `cargo +beta build` / `rustc +beta`.
2. `RUSTUP_TOOLCHAIN` environment variable.
3. Directory override via `rustup override`.
4. `rust-toolchain.toml` (or legacy `rust-toolchain`) file.
5. The default toolchain.

EXCEPTION: directory overrides and the `rust-toolchain.toml` file are preferred by **proximity** — walking up the directory tree toward root, a closer `rust-toolchain.toml` beats a farther directory override.

### rustup components and profiles

Profiles (exact contents):

- `minimal` — rustc, rust-std, cargo. Recommended in CI and on Windows without local docs.
- `default` — minimal + rust-docs + rustfmt + clippy. Recommended for general use; the rustup default.
- `complete` — every component. "This should never be used, as it includes every component ever included in the metadata and thus will almost always fail." Use `default` and add components explicitly.

Common components:

| Component | Purpose |
|-----------|---------|
| `rustc` | Compiler + rustdoc. |
| `cargo` | Package manager. |
| `rust-std` | Standard library per target triple. |
| `rust-docs` | Local docs; `rustup doc`. |
| `rustfmt` | Formatter. |
| `clippy` | Linter. |
| `rust-analyzer` | LSP server. |
| `miri` | UB-checking interpreter. |
| `rust-src` | stdlib source for rust-analyzer / miri / build-std. |
| `rust-mingw` | Windows-gnu linker libs. |
| `llvm-tools` | LLVM tooling (unstable). |
| `rustc-dev` | Compiler-as-library. |
| `rustc-codegen-cranelift` | Cranelift backend (nightly). |

DEPRECATED/REMOVED: `rls` (replaced by rust-analyzer), `rust-analysis` (RLS metadata). Target `wasm32-wasi` was renamed to `wasm32-wasip1`.

### MSRV testing and `cargo-msrv`

```bash
# Test MSRV in CI
rustup toolchain install 1.85.0
cargo +1.85.0 build --all-targets
```

Third-party helper:

```bash
cargo install cargo-msrv --locked
cargo msrv find
cargo msrv verify
cargo msrv list
cargo msrv show
cargo msrv set 1.85.0
```

`cargo-msrv` recognizes `package.rust-version` and `package.metadata.msrv`.

Clippy MSRV:

```toml
# clippy.toml
msrv = "1.85"
```

**CRITICAL CAVEAT:** `cfg(version("1.X"))` and `cfg!(version(...))` are **nightly-only** (feature `cfg_version`, tracking rust-lang/rust#64796). On stable, use the `version_check` crate in `build.rs` (emit `cargo:rustc-cfg=...`) for compile-time version gating.

## Common mistakes

- **Forgetting to set `edition` explicitly.** The default is 2015, which lacks modern ergonomics. Always set it.

- **Believing `unsafe_op_in_unsafe_fn` is allow-by-default in edition 2024.** It is **warn-by-default** in 2024. The rustc "Allowed-by-default Lints" page lists it as allowed because it reads only the static `default_level` and ignores the `@edition Edition2024 => Warn;` override. `core` and `std` carry explicit `#![deny(unsafe_op_in_unsafe_fn)]` because they are on edition 2021. Use `#![warn(unsafe_op_in_unsafe_fn)]` or `deny` for all editions.

- **Omitting `unsafe {}` wrapper inside `unsafe fn` in edition 2024.** An `unsafe fn` body is no longer itself an `unsafe` block — wrap unsafe operations explicitly.

- **Doctest examples that panic or perform I/O without `no_run`.** A bare ` ```rust ` block is compiled and executed. Network calls, file I/O, and blocking operations will fail in `cargo test --doc`. Use `no_run` for examples that should compile but not execute.

- **Ambiguous intra-doc links.** Writing `[`Result`]` when both `std::io::Result` and `std::fmt::Result` are in scope causes a rustdoc warning. Use the full path or a disambiguator. See `docs/rust/documentation-guidelines.md`.

- **Mixing language edition and rustfmt style edition unintentionally.** The `edition` key in `Cargo.toml` controls language semantics. The `style_edition` key in `rustfmt.toml` controls formatting. They are independent. Set both explicitly.

- **Pinning the toolchain to a specific old version indefinitely.** `rust-toolchain.toml` should track a reasonably current stable release. Pinning to an ancient version blocks security patches, new lints, and edition migrations. Update the channel regularly.

- **Not testing MSRV in CI.** Declaring `rust-version = "1.85"` in `Cargo.toml` is meaningless if CI only tests on `stable`. Add the MSRV compiler to the CI matrix.

- **Using ` ```ignore ` without a reason.** `ignore` skips compilation entirely, so the example may silently rot. Prefer `no_run` (compiles but doesn't run) or `compile_fail` (verifies the example is intentionally invalid).

- **Running `cargo fix --edition` without manual review.** Automatic fixes handle syntax changes but cannot judge semantic correctness. Always review the diff and run `cargo build --all-targets` afterward.

- **Forgetting `#[doc(cfg(...))]` on conditional items.** Without it, docs show items that may not exist on the reader's platform, causing confusion.

- **Assuming `cfg(doc)` is passed to doctests.** It is not. Doctests run under the normal compilation cfg set.

- **Treating `--extern-html-root-takes-precedence` as a real rustdoc flag.** It is not documented in the current rustdoc book. Use `--extern-html-root-url` instead.

- **Presenting `cfg(version(...))` as stable.** It is nightly-only. On stable, use `version_check` in `build.rs`.

- **Using `style_edition = "2027"` on stable.** It is nightly-only (marked `#[unstable_variant]` in rustfmt).

- **Forgetting to run `cargo fmt` after an edition migration.** Output can change; review the diff separately.

## Strict vs contextual guidance

### Strict guidance

- The `edition` field must be set explicitly in every `Cargo.toml`. Never rely on the 2015 default.
- `cargo doc` must be part of CI. Broken doc builds are release blockers.
- All `///` examples must compile via doctests. Use `no_run`, `ignore`, `should_panic`, or `compile_fail` for non-runnable examples — never leave a broken code block unannotated.
- Pin the toolchain via `rust-toolchain.toml` committed to the repository.
- Set `rust-version` in `[package]` of `Cargo.toml`.
- In edition 2024 crates, `unsafe` operations inside `unsafe fn` must be wrapped in `unsafe {}` blocks.
- Set both `edition` and `style_edition` explicitly; do not let them drift.
- `unsafe_op_in_unsafe_fn` must be at least `warn` for all editions.

### Common convention

- `cargo fmt --check` runs in CI.
- New crates use `edition = "2021"` or `"2024"`.
- `rustfmt.toml` configuration is committed to the repo.
- Doc tests are part of `cargo test` in CI.
- `rust-toolchain.toml` (TOML format) is preferred over the legacy `rust-toolchain` plain-text file.
- Intra-doc links are preferred over raw URLs.
- MSRV is tested in CI with `cargo +<msrv> build`.
- `#[doc(cfg(...))]` is used for platform- or feature-gated public APIs.

### Contextual tradeoffs

- **Edition migration timing.** Migrating during a refactor sprint spreads the risk; a standalone PR is cleaner but blocks other work. Choose based on release cadence and team capacity.
- **Toolchain pinning: exact version vs channel.** Pinning to `1.85.0` is maximally reproducible but requires manual updates. Pinning to `stable` auto-updates but may introduce unexpected breakage. Choose based on CI stability requirements.
- **`style_edition` per-crate vs per-workspace.** A workspace with mixed editions may need per-crate `rustfmt.toml` files. Most workspaces should use a single style edition for consistency.
- **2024 edition migration.** The 2024 edition introduces `unsafe_op_in_unsafe_fn` as a default warn and other breaking changes. Migrate when the toolchain is stable and the team has bandwidth for the `unsafe {}` wrapper changes.
- **`format_code_in_doc_comments`**. Enabling it keeps doctests formatted consistently, but it may reformat hand-tuned examples. Decide per project.
- **rustdoc JSON consumers.** Only enable JSON output if the tooling chain (docs.rs, cargo-semverchecks, etc.) accepts the experimental schema churn.

## Policy decisions for individual repos

Each repository should decide and record the following:

| Decision | Options | Notes |
|----------|---------|-------|
| Target edition | `"2015"` / `"2018"` / `"2021"` / `"2024"` | New crates should use 2021 or 2024. |
| MSRV support policy | Version and cadence for raising the floor | Bumping MSRV is a minor incompatibility per Cargo semver. |
| rustfmt `style_edition` | `"2015"` / `"2018"` / `"2021"` / `"2024"` | Pin explicitly; `"2027"` is nightly-only. |
| Toolchain pinning policy | exact version / channel / nightly | Document rationale in README. |
| `rust-toolchain.toml` vs rustup default | committed / omitted | Committing strongly recommended for CI reproducibility. |
| Required rustup components | list in `rust-toolchain.toml` or docs | clippy, rustfmt, rust-src, rust-analyzer, etc. |
| Doc test strictness | hard CI gate / soft warning | Hard gate recommended for libraries. |
| Intra-doc link failure policy | warning / error in CI | Error recommended for libraries. |
| rustdoc JSON generation | enabled / disabled | Requires nightly; schema is unstable. |
| Cross-compilation targets | list required triples | Also configure linker in `.cargo/config.toml`. |
| `#[doc(cfg(...))]` requirement | required / recommended | Required for conditional public APIs. |
| MSRV test matrix | stable + MSRV / + beta / + nightly | At minimum test stable and declared MSRV. |

## Related docs

- `docs/rust/style-formatting.md` — rustfmt configuration tables, stable/nightly options, RFC 3338 style-evolution deep-dive, and the concrete list of 2024 style-edition formatting changes.
- `docs/rust/documentation-guidelines.md` — doc-comment structure (`///`/`//!`), doctest fence attribute table, intra-doc link forms and namespace disambiguators, C-METADATA, C-RELNOTES, C-HIDDEN, and C-EXAMPLE/C-FAILURE heading conventions.
- `docs/rust/unsafe-security.md` — unsafe code guidance, including the `unsafe_op_in_unsafe_fn` edition-specific default (allow pre-2024, warn in 2024).
- `docs/rust/lints-clippy.md` — lint levels, MSRV interaction with `clippy.toml`, and rustdoc lint policy.
- `docs/rust/cargo-dependencies.md` — `Cargo.toml` fields, resolver behavior, and dependency management.
- `docs/rust/supply-chain-security.md` — toolchain pinning for reproducible builds and MSRV enforcement.

## Related skills

No repo-specific skills for this topic. The `nix-usage` and `frontend-command-discovery` skills are not relevant to Rust edition/toolchain documentation.
