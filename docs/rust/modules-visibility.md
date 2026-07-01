# Modules, Visibility, and Project Layout

## Purpose

This document provides authoritative, AI-agent-oriented guidance on Rust's package/crate/module hierarchy, the module-to-filesystem mapping, the visibility grammar and access rules, `use`/`pub use`/re-export mechanics, path qualifiers, and Cargo project directory layout. It is repo-independent and intended as a reference for writing, reviewing, and refactoring Rust code. Rust's module system is not merely organizational — it directly controls encapsulation, API boundaries, and compilation units. Incorrect module structure or visibility annotations produce code that is inaccessible where needed or inappropriately exposed, and both errors are costly to fix late in a project's lifecycle.

This doc owns the mechanics of packages, crates, and modules, the module↔filesystem mapping, visibility grammar/access rules, `use`/`pub use`/re-export mechanics, paths & path qualifiers, and Cargo directory layout + target auto-discovery. It intentionally defers manifest internals, dependency/feature resolver mechanics, workspace inheritance, profiles, API naming conventions, `#[non_exhaustive]`, sealed traits, builder patterns, test-writing style, and formatting to sibling docs.

## Sources used

- Rust Book ch07 (packages, crates, modules, paths, `use`, visibility):
  - https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
  - https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html
  - https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html
  - https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html
  - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
  - https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html
- Rust Reference:
  - https://doc.rust-lang.org/reference/items/modules.html
  - https://doc.rust-lang.org/reference/visibility-and-privacy.html
  - https://doc.rust-lang.org/reference/items/use-declarations.html
  - https://doc.rust-lang.org/reference/items/extern-crates.html
  - https://doc.rust-lang.org/reference/paths.html
  - https://doc.rust-lang.org/reference/names/preludes.html
- Cargo Book:
  - https://doc.rust-lang.org/cargo/guide/project-layout.html
  - https://doc.rust-lang.org/cargo/reference/cargo-targets.html
  - https://doc.rust-lang.org/cargo/reference/build-scripts.html
  - https://doc.rust-lang.org/cargo/reference/workspaces.html
  - https://doc.rust-lang.org/cargo/guide/tests.html

## Core guidance

Rust's organizational hierarchy has three levels (Rust Book ch07-01, ch07-02):

- **Package** — a bundle of one or more crates that provides a set of functionality. A package contains a `Cargo.toml` file. A package can contain as many binary crates as you like, but at most only one library crate, and it must contain at least one crate.
- **Crate** — the smallest amount of code the Rust compiler considers at a time. A binary crate has a `fn main` and compiles to an executable. A library crate has no `main`; most of the time when Rustaceans say "crate" they mean a library crate.
- **Module** — the namespace and visibility unit within a crate. Modules organize code for readability and reuse and control privacy because code within a module is private by default.

The crate root is the source file the compiler starts from and is the root module of the crate. `src/main.rs` is the binary crate root; `src/lib.rs` is the library crate root; both are named after the package. The contents of `src/main.rs` / `src/lib.rs` form a module named `crate`, and the entire module tree is rooted under that implicit module (Rust Book ch07-02).

The single most important mental model for visibility is: **privacy is per-link**. An item is externally reachable only if **every** module on the path from the crate root is `pub` (or reachable via a chain of `pub use` re-exports), and the item itself is `pub`. Adding `pub` to a deeply nested item does nothing if an intermediate module is private. This transitive requirement is the source of most visibility confusion.

### Visibility grammar and access rules

The visibility grammar is (Rust Reference: visibility-and-privacy.html):

```text
Visibility →
    pub
  | pub ( crate )
  | pub ( self )
  | pub ( super )
  | pub ( in SimplePath )
```

- `pub` — public everywhere (within the crate and to downstream crates).
- `pub(crate)` — within the current crate.
- `pub(super)` — visible to the parent module, equivalent to `pub(in super)`.
- `pub(self)` — visible to the current module, equivalent to `pub(in self)` and equivalent to omitting `pub`.
- `pub(in path)` — visible within `path`; the path must be a `SimplePath`, resolve to an **ancestor** module of the item, and each identifier must refer directly to a module (not via a `use` alias). In edition 2018+ the path must start with `crate`, `self`, or `super`.

All items (functions, methods, structs, enums, modules, constants) are private to parent modules by default. There are exactly two exceptions (Rust Reference: visibility-and-privacy.html):

1. Associated items in a `pub` trait are public by default.
2. Enum variants in a `pub` enum are public by default.

The two access rules are (Rust Reference: visibility-and-privacy.html):

1. A **public** item is accessible from module `m` iff **every** ancestor module from `m` to the item is itself accessible. It may also be reachable via re-exports.
2. A **private** item is accessible by the current module and its descendants.

Critical corollary: **`pub(crate)` does not bypass a private ancestor**. Visibility is a per-link restriction. A `pub` item behind a private ancestor module is not externally reachable.

Re-exports short-circuit the privacy chain: "When re-exporting a private item, it can be thought of as allowing the 'privacy chain' being short-circuited through the reexport" (Rust Reference: visibility-and-privacy.html). So `pub use` can make an item externally nameable even if its canonical path is private, provided each `pub use` link is public.

### Path kinds

Paths are absolute or relative (Rust Book ch07-03):

- **Absolute** — starts from the crate root. For the current crate use the literal `crate`; for an external crate use the crate name.
- **Relative** — starts from the current module. Uses `self`, `super`, or an identifier. `super` is the parent module, repeatable (`super::super::foo`).

The `crate` keyword in paths is analogous to `/` in a shell; `super` is analogous to `..`. Prefer absolute paths because code definitions and item calls are more likely to move independently (Rust Book ch07-03).

In expression position the turbofish `::<...>` is required before `<`: `Vec::<u8>::with_capacity(1024)` (Rust Reference: paths.html). Use `<S as T1>::f()` to disambiguate trait methods. Path qualifiers are `crate::`, `self::`, `super::`, `::` (extern prelude in 2018+), `Self` (type-system only), and `$crate` (macro transcribers only).

## Practical rules

### File resolution

For `mod garden;` declared in the crate root, the compiler looks for, in order: an inline body; `src/garden.rs`; `src/garden/mod.rs`. For `mod vegetables;` declared in `src/garden.rs`, the compiler looks for: inline; `src/garden/vegetables.rs`; `src/garden/vegetables/mod.rs` (Rust Book ch07-02, ch07-05).

| Situation | Looks for | Notes |
|-----------|-----------|-------|
| `mod garden;` in `src/lib.rs` | `src/garden.rs` or `src/garden/mod.rs` | Modern style is `src/garden.rs`. |
| `mod vegetables;` in `src/garden.rs` | `src/garden/vegetables.rs` or `src/garden/vegetables/mod.rs` | Same logic, nested. |
| Both `foo.rs` and `foo/mod.rs` | — | Compiler error. Pick one. |
| `mod foo;` inside a function body | — | Error; `mod foo { ... }` inline is allowed, but the file form is not. |

Rustc <1.30 only supported the `mod.rs` form. Modern Rust (2018+) supports both. Both `foo.rs` and `foo/mod.rs` for the **same** module is an error; mixing styles across **different** modules is legal but confusing. The main downside of `foo/mod.rs` is many files named `mod.rs` open at once in an editor (Rust Book ch07-02, ch07-05).

### `mod` is not `include`

Load a file with `mod` **once** in the tree; other files refer to it via paths. `use`/`pub use` have no impact on what files are compiled as part of the crate. The `mod` keyword declares modules. If you create `src/foo.rs` but never write `mod foo;`, the file is dead code (Rust Book ch07-05).

### `use` idioms

Idioms from Rust Book ch07-04:

- **Functions** — bring the parent module into scope, then call `hosting::add_to_waitlist()`. Do **not** `use ...::add_to_waitlist` directly. This makes clear the function is not locally defined while minimizing repetition.
- **Structs, enums, and other items** — bring the full path: `use std::collections::HashMap;`.
- **Exception** — when two items share a name, bring their parent modules (`use std::fmt; use std::io;`) or use `as`.
- **Nested paths** — `use std::{cmp::Ordering, io};`; combine with `self` as `use std::io::{self, Write};`.
- **Glob** — `use std::collections::*;` is acceptable for test modules and designed preludes, but avoid in library code because it makes it harder to tell what names are in scope and imports change if the dependency changes.

`use` is scope-local: a `use` in a parent module does **not** apply in a child module (E0433). `use` imports are private by default; re-export with `pub use`. `pub use` is useful when the internal structure of your code differs from how programmers calling your code think about the domain — write code with one structure but expose a different one (Rust Book ch07-04).

### `use` restrictions

The following are disallowed and require `as`-renaming (Rust Reference: items/use-declarations.html#restrictions):

- `use crate;`, `use crate::{self};`
- `use self;`, `use self::{self};`
- `use super;`, `use super::{self};`
- `use self::super;`, `use super::super;`, `use super::super::{self};`

Rename instead: `use crate as root;`, `use {self as this_module};`, `use super as parent;`. `use TypeAlias::Variant` is an error — enum variants cannot be reached via a type alias. `as _` imports without binding a name, useful for importing a trait for its methods. Duplicate bindings in the same namespace are errors.

### Restricted visibility selection

- `pub` — external API.
- `pub(crate)` — internal API shared across modules within the same crate. Use this by default for cross-module helpers.
- `pub(super)` — shared only with the parent module and its descendants. Useful for tightly coupled sibling modules.
- `pub(in path)` — shared only within a specific ancestor module. Use for the narrowest meaningful boundary.

### Field visibility

For structs, `pub struct` keeps fields private; mark each field `pub` individually. A struct with a private field needs a public constructor function or it cannot be constructed externally. Example: `Breakfast { pub toast: String, seasonal_fruit: String }` with `pub fn summer(...) -> Breakfast` (Rust Book ch07-03).

For enums, if the enum is public, all variants are public by default. Per-variant visibility is parsed but rejected at validation; variants inherit the enum's visibility (Rust Reference: items/enumerations.html). The syntax remains so `$vis:vis` macro fragments work.

### Library + binary pattern

A package may have both `src/lib.rs` and `src/main.rs`. The binary crate then becomes a user of the library crate. The module tree should be defined in `src/lib.rs`; public items can be used in the binary crate by starting paths with the package name (Rust Book ch07-03). Prefer this for non-trivial packages to avoid duplicating logic between library and binary.

### `#[path]` escape hatch

`#[path = "..."]` changes only the filesystem location, never the Rust module name (Rust Reference: items/modules.html). Outside inline module blocks the path is relative to the directory of the file containing the attribute. Inside an inline `mod {}` block in a `mod.rs` file, it is relative to that `mod.rs` file's directory with inline module segments appended. Inside an inline `mod {}` block in a non-`mod.rs` file, the path is prefixed with a directory named after that module. Reserve `#[path]` for exceptional cases (shared code, generated code) and document every use with a comment.

### Test helpers

Put shared test utilities in `tests/common/mod.rs` (a subdirectory, **not** a `tests/*.rs` file) and `mod common;` from each test that needs them. Every top-level `tests/*.rs` file is auto-compiled as its own test crate; a helper placed at `tests/helpers.rs` would be built and run as an empty test crate. Directories are not treated as test files, so `tests/common/mod.rs` is safe (Cargo Reference: cargo-targets.html; Cargo Guide: tests.html).

### Auto-discovery opt-outs

Cargo auto-determines targets from file layout by default. Disable per-kind with `[package] autolib = false` (MSRV 1.83), `autobins = false`, `autoexamples = false`, `autotests = false`, `autobenches = false` (MSRV 1.27). Use cases: a library wanting an internal module literally named `bin` (`src/bin/mod.rs`) must set `autobins = false` so Cargo doesn't treat it as a binary target (Cargo Reference: cargo-targets.html#target-auto-discovery).

### Domain-based organization

Organize modules by domain, not by kind. A `users` module containing the model, service, and error types for user management is better than separate `models/`, `services/`, and `errors/` modules. Domain-based organization keeps related code together and reduces cross-module visibility gymnastics.

### Build script placement

Placing `build.rs` at the package root auto-detects/compiles/runs it before the build. `[package].build` defaults to `"build.rs"`; set `build = false` to disable, or `build = "custom.rs"` for a different file. The build script CWD is the package root; it communicates with Cargo via `cargo::KEY=VALUE` stdout lines (MSRV 1.77 for the `cargo::` prefix). Defer the full instruction catalog to a build-scripts reference (Cargo Reference: build-scripts.html, manifest.html).

## Review checklist

- [ ] Every `.rs` file in `src/` is reachable via a `mod` declaration chain from the crate root.
- [ ] No orphan files exist (`.rs` files not referenced by any `mod` declaration).
- [ ] No `pub` item exists that could be `pub(crate)` or `pub(super)`.
- [ ] Public API types are re-exported from the crate root or a dedicated `prelude` module.
- [ ] No glob imports in library code except preludes or test modules.
- [ ] Module organization follows domain boundaries, not kind-based grouping.
- [ ] `mod.rs` files are used only if the repo has explicitly chosen that convention; otherwise `foo.rs` is used.
- [ ] Struct fields have intentional visibility — private by default, `pub` only when needed.
- [ ] Enum variants are public only because the enum itself is public; no per-variant visibility attempts.
- [ ] Integration tests in `tests/` only test the public API; unit tests are inline.
- [ ] Any use of `#[path]` is documented with a justification comment.
- [ ] Binary + library packages define the module tree in `src/lib.rs` and consume it from `src/main.rs`.
- [ ] Shared test helpers live in `tests/common/mod.rs` or a dev-dependency crate, not as a top-level `tests/*.rs` file.

## Implementation checklist

- [ ] Decide on `foo.rs` vs `foo/mod.rs` convention and apply it consistently.
- [ ] Create the module file and add the `mod` declaration in the parent immediately.
- [ ] Set visibility on each item: private (default), `pub(super)`, `pub(crate)`, or `pub`.
- [ ] For structs, decide per-field visibility; provide constructors when fields are private.
- [ ] Add `use` imports for items needed in the current module; prefer specific imports over globs.
- [ ] Add `pub use` re-exports at the crate root or facade module for items that form the public API.
- [ ] Verify the module compiles: `cargo check` or `cargo build`.
- [ ] Verify visibility is correct: try using the item from an integration test or a downstream crate.
- [ ] Run `cargo doc --no-deps` and inspect the generated documentation to confirm the public API surface matches intent.
- [ ] Run `cargo clippy` and address visibility-related lints.

## Validation hooks

- **`cargo check`**: Catches missing `mod` declarations, visibility errors, unresolved imports, and invalid `pub(in path)` targets. Fastest validation loop.
- **`cargo doc --no-deps`**: Generates documentation for the crate. Items that appear in the docs but should not be public indicate over-publishing. Items that are missing but should be public indicate under-publishing.
- **`cargo test`**: Integration tests in `tests/` are compiled as separate crates and can only access the library's `pub` items. If an integration test cannot reach an item you intended to be public, the visibility chain is broken somewhere.
- **`cargo clippy`**: Watches for redundant or excessive visibility such as `redundant_pub_crate`, `unreachable_pub`, and `pub_use_of_private_extern_crate`. Enable and address these lints.
- **Manual orphan-file audit**: List every `.rs` file under `src/` and verify it is reachable via a `mod` chain. Look for files that exist but are never declared.
- **Import audit**: Grep for `use .*::{` and `use .*::\*;` to find nested imports and globs; verify each is justified.
- **`pub(in path)` audit**: Verify every `pub(in ...)` path points to an ancestor module and uses `crate`/`self`/`super` in edition 2018+.

## Examples

### Example 1: Backyard — canonical end-to-end file mapping

```text
backyard
├── Cargo.lock
├── Cargo.toml
└── src
    ├── garden
    │   └── vegetables.rs
    ├── garden.rs
    └── main.rs
```

```rust
// src/main.rs
use crate::garden::vegetables::Asparagus;

pub mod garden;

fn main() {
    let plant = Asparagus {};
    println!("I'm growing {plant:?}!");
}

// src/garden.rs
pub mod vegetables;

// src/garden/vegetables.rs
#[derive(Debug)]
pub struct Asparagus {}
```

This is the canonical example from Rust Book ch07-02. `main.rs` declares `pub mod garden;`, `garden.rs` declares `pub mod vegetables;`, and the actual vegetable type lives in `src/garden/vegetables.rs`.

### Example 2: Restaurant module tree and transitive visibility

Module tree (Rust Book ch07-02, ch07-03):

```text
crate
 └── front_of_house
     ├── hosting
     │   ├── add_to_waitlist
     │   └── seat_at_table
     └── serving
         ├── take_order
         ├── serve_order
         └── take_payment
```

```rust
// src/lib.rs
mod front_of_house;

pub use front_of_house::hosting;

pub fn eat_at_restaurant() {
    hosting::add_to_waitlist();
}

// src/front_of_house.rs
pub mod hosting;
pub mod serving;

// src/front_of_house/hosting.rs
pub fn add_to_waitlist() {}
pub fn seat_at_table() {}
```

For `eat_at_restaurant` to call `add_to_waitlist`, both `mod hosting` must be `pub mod` and `fn add_to_waitlist` must be `pub fn`. Making a module public does **not** make its contents public.

### Example 3: `pub(crate)` cross-module access

```rust
// src/parser.rs
pub(crate) fn parse_internal(input: &str) -> ast::Node {
    // Available anywhere in this crate, not to external crates.
}

// src/engine.rs
use crate::parser::parse_internal;

pub fn run(input: &str) {
    let node = parse_internal(input);
    // ...
}
```

`pub(crate)` is the right choice for implementation details shared between modules but not part of the public API.

### Example 4: `pub(super)` sibling access

```rust
// src/ast.rs
mod expr;
mod stmt;

// src/ast/expr.rs
pub(super) fn visit_expr(e: &Expr) {
    // Visible to ast::stmt and ast itself, but not the rest of the crate.
}

// src/ast/stmt.rs
use super::expr::visit_expr;

pub fn visit_stmt(s: &Stmt) {
    visit_expr(&s.expr);
}
```

`pub(super)` restricts visibility to the parent module and its descendants. Useful when modules within a group share helpers that should not leak further.

### Example 5: `pub use` facade for a flat public API

```rust
// src/lib.rs
mod error;
mod client;
mod request;

pub use error::Error;
pub use client::Client;
pub use request::Request;

// Users write: use my_crate::Client;
// Not:       use my_crate::client::Client;
```

This pattern decouples internal organization from the public API. If `client` is later split into submodules, the re-export at the crate root stays the same.

### Example 6: Modern file-based modules with submodules

```rust
// src/lib.rs
pub mod api;       // src/api.rs
pub mod models;    // src/models.rs

// src/api.rs
pub mod handlers;  // src/api/handlers.rs
pub mod routes;    // src/api/routes.rs

pub fn init() { /* ... */ }
```

With the modern convention, `api.rs` sits alongside the `api/` directory. This avoids the older `api/mod.rs` pattern.

### Example 7: Struct with private field and public constructor

```rust
// src/breakfast.rs
pub struct Breakfast {
    pub toast: String,
    seasonal_fruit: String,
}

impl Breakfast {
    pub fn summer(toast: &str) -> Breakfast {
        Breakfast {
            toast: String::from(toast),
            seasonal_fruit: String::from("peaches"),
        }
    }
}
```

External code can construct `Breakfast` only through `Breakfast::summer` because `seasonal_fruit` is private (Rust Book ch07-03).

### Example 8: `use` idioms

```rust
// Idiomatic for functions: bring parent module.
use crate::front_of_house::hosting;

pub fn eat() {
    hosting::add_to_waitlist(); // clear it isn't locally defined
}

// Idiomatic for structs/enums: bring full path.
use std::collections::HashMap;

// Collision: bring parent modules or use `as`.
use std::fmt;
use std::io;
use std::io::Result as IoResult;

// Nested paths.
use std::{cmp::Ordering, io};
use std::io::{self, Write};

// Glob in tests only.
#[cfg(test)]
mod tests {
    use super::*;
}
```

### Example 9: `pub(in path)` restricted visibility

```rust
// src/lib.rs
pub mod outer;

// src/outer.rs
pub mod inner;

pub(in crate::outer) fn restricted_helper() {}

// src/outer/inner.rs
use crate::outer::restricted_helper;

pub fn do_work() {
    restricted_helper();
}
```

`restricted_helper` is visible within `crate::outer` and its descendants, but not elsewhere in the crate. The path must refer to an ancestor module directly, not via a `use` alias.

### Example 10: Full Cargo project layout

```text
my-package/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs              # Library crate root
│   ├── main.rs             # Default binary crate root
│   ├── bin/
│   │   ├── admin.rs        # cargo run --bin admin
│   │   ├── worker.rs       # cargo run --bin worker
│   │   └── multi-file-tool/# Directory name = target name
│   │       ├── main.rs
│   │       └── support.rs
│   ├── config.rs           # Internal module
│   └── db/
│       ├── mod.rs          # (or db.rs at src/ level)
│       ├── pool.rs
│       └── migrations.rs
├── tests/
│   ├── integration.rs      # Separate test crate
│   └── common/
│       └── mod.rs          # Shared test helpers, not a test file
├── benches/
│   └── perf.rs             # Criterion or #[bench]
├── examples/
│   └── basic.rs            # cargo run --example basic
└── build.rs                # Optional build script
```

This layout follows Cargo Book: guide/project-layout.html and reference/cargo-targets.html.

### Example 11: `tests/common/mod.rs` helper

```text
tests/
├── common/
│   └── mod.rs
└── integration.rs
```

```rust
// tests/common/mod.rs
pub fn setup() -> TestContext {
    // ...
}

// tests/integration.rs
mod common;

#[test]
fn it_works() {
    let ctx = common::setup();
    // ...
}
```

Never place shared helpers at `tests/helpers.rs`; Cargo would compile and run it as an empty test crate.

### Example 12: Workspace layout

```text
PROJECT_DIR/
├── Cargo.toml      # workspace root (virtual, or root package)
├── Cargo.lock
├── target/         # shared
└── hello_world/
    ├── Cargo.toml
    └── src/
```

A workspace is one or more packages sharing a single `Cargo.lock` and `target/` at the workspace root. Members are subdirectories each with their own `Cargo.toml` + `src/`. `[patch]`, `[replace]`, and `[profile.*]` are recognized only at the workspace root. For workspace inheritance (`workspace.package`, `workspace.dependencies`, `workspace.lints`) and resolver mechanics, see `docs/rust/cargo-dependencies.md`.

## Common mistakes

### 1. Orphan file with no `mod` declaration

You add `src/utils.rs` but forget `mod utils;` in `src/lib.rs`. The file is valid Rust but is never included in the crate. No error is produced — the file is dead code. **Fix**: Add the `mod` declaration immediately after creating the file. Run `cargo check` and audit every `.rs` file under `src/`.

### 2. Using `pub` when `pub(crate)` suffices

A helper is marked `pub` because it needs to be called from another module within the crate. This unnecessarily adds it to the public API and creates a stability commitment. **Fix**: Use `pub(crate)` for items shared within the crate but not intended for external consumers.

### 3. Broken visibility chain

A nested item is `pub`, but one of its ancestor modules is private. External crates cannot reach the item despite the `pub` annotation. The code compiles because it works within the crate. **Fix**: Verify the entire path from the crate root is `pub`, or use `pub use` to re-export the item at a visible location.

### 4. Making a module `pub` but forgetting its contents are still private

`pub mod hosting;` makes the module name reachable, but `fn add_to_waitlist` inside remains private unless also marked `pub fn`. **Fix**: Apply visibility to each item that should be reachable.

### 5. `pub` struct with private field and no constructor

External code cannot construct the struct because all fields are not `pub` and no constructor is provided. **Fix**: Mark necessary fields `pub`, or provide a `pub fn new(...)` constructor.

### 6. Both `foo.rs` and `foo/mod.rs` for the same module

This is a compiler error. **Fix**: Choose one convention and delete the other. If `foo` has submodules, use `src/foo.rs` (modern) and place children in `src/foo/`.

### 7. Overusing glob imports

`use my_module::*;` in library code pulls in every public item. New items added later silently enter scope, risking collisions. **Fix**: Import specific items. Reserve globs for test modules and preludes.

### 8. `use` scope-locality across modules

A `use` in `src/lib.rs` does not apply in `src/foo.rs`. Each module must import what it needs. **Fix**: Add imports in each module, or use `pub use` at the crate root.

### 9. Unit tests placed in `tests/`

The `tests/` directory is for integration tests compiled as separate crates; they can access only the public API. Unit tests needing private items should live in `#[cfg(test)] mod tests { ... }` within the source file. Putting them in `tests/` often leads to over-publishing items just for testing. **Fix**: Keep unit tests inline; use `tests/` only for public-API tests.

### 10. Helper file as `tests/helpers.rs`

Cargo treats every top-level `tests/*.rs` as its own test crate. `tests/helpers.rs` compiles and runs as an empty test. **Fix**: Use `tests/common/mod.rs` or a dev-dependency crate.

### 11. `#[path]` without comment

The attribute overrides the standard file mapping, making navigation non-obvious. **Fix**: Add a comment such as `// #[path] used to share types between bin and lib crates — see docs/rust/modules-visibility.md`.

### 12. Declaring `mod foo;` inside a function

While inline `mod foo { ... }` inside a function body is legal, the file form `mod foo;` is not. **Fix**: Declare modules at module level; use `#[cfg(...)]` if conditional inclusion is needed.

### 13. `use crate;` / `use self;` / `use super;` without `as`

These are disallowed by the grammar. **Fix**: Rename: `use crate as root;`, `use super as parent;`, `use {self as this_module};`.

### 14. `pub(in path)` pointing at a non-ancestor or via a `use` alias

The path in `pub(in path)` must resolve to an ancestor module, and each segment must refer directly to a module, not through a `use` alias. **Fix**: Use a canonical module path starting with `crate`, `self`, or `super`.

### 15. Library + binary duplication instead of bin-consumes-lib

Non-trivial packages define logic in `src/lib.rs` and call it from `src/main.rs`. Defining the same logic twice leads to drift. **Fix**: Put the module tree in the library; make the binary a consumer.

## Strict vs contextual guidance

| Rule | Strictness | Rationale |
|------|-----------|-----------|
| The two access rules: public items reachable only when every ancestor is accessible; private items visible to current module and descendants | **Strict** | These are enforced by the compiler and are the foundation of all visibility reasoning (Rust Reference: visibility-and-privacy.html). |
| Default privacy, with the two exceptions (pub trait items, pub enum variants) | **Strict** | Compiler-enforced defaults; exceptions are fixed (Rust Reference: visibility-and-privacy.html). |
| Every `.rs` file must have a `mod` declaration chain to the crate root | **Strict** | Orphan files are dead code; the compiler does not warn about unused files. |
| Items are private by default; use the narrowest visibility that works | **Strict** | Over-publishing creates API commitments that are hard to retract. |
| `use` idioms by item kind (functions: parent module; structs/enums: full path) | **Contextual** | Idiomatic but not enforced; use `as` or parent modules when names collide. |
| `pub use` facade at the crate root | **Contextual** | Essential for libraries with downstream consumers; less important for application-only crates. |
| `pub(crate)` for internal cross-module access | **Strict** | `pub` should be reserved for the public API. |
| Prefer `foo.rs` over `foo/mod.rs` | **Contextual** | Both valid; 2018+ convention is `foo.rs`, but repos with existing `mod.rs` usage may choose consistency. |
| Avoid glob imports in library code | **Strict** | Glob imports create silent name collisions and reduce readability; test code and preludes are exceptions. |
| `tests/common/mod.rs` for shared helpers | **Strict** | Cargo convention; any top-level `tests/*.rs` becomes its own test crate. |
| `build = false` opt-out when no build script is needed | **Contextual** | Only relevant when a file named `build.rs` exists but should not run. |
| `#[path]` only with documented justification | **Strict** | It breaks the standard file mapping and must be justified. |
| `pub(in path)` for narrow visibility boundaries | **Contextual** | Use when `pub(super)` or `pub(crate)` is too broad; adds verbosity. |
| Struct fields private by default | **Strict** | Public fields lock in representation; make them `pub` only when direct access is part of the API contract. |
| Integration tests in `tests/`, unit tests inline | **Strict** | Cargo convention; ensures unit tests can access private items without over-publishing. |
| Auto-discovery opt-outs (`autobins = false`, etc.) when internal modules conflict with target names | **Strict** | Required to prevent Cargo from misinterpreting `src/bin/`, `tests/`, etc. |
| Library + binary pattern: module tree in `src/lib.rs` | **Contextual** | Strongly recommended for non-trivial packages; trivial binaries may keep everything in `main.rs`. |

## Policy decisions for individual repos

Each repository should make and document the following decisions in its contributing guide or architecture documentation:

1. **Module file convention**: Whether to use `foo.rs` (modern) or `foo/mod.rs` (legacy) for modules with submodules. Choose one and enforce it consistently. Mixed usage within a repo is confusing.

2. **Re-export / facade strategy**: Whether the crate root re-exports all public types, whether a `prelude` module is provided, or whether consumers navigate the module tree directly. For libraries, re-exporting is strongly recommended. For application crates, it is less important.

3. **Prelude module**: If provided, document which items belong in it and the criteria for inclusion. Preludes are designed for glob use; library preludes should be conservative.

4. **Visibility granularity**: Whether to use `pub(super)` and `pub(in path)` actively, or default to `pub(crate)` for all internal sharing. More granular visibility is better for large crates but adds verbosity.

5. **Module depth limit**: Whether to enforce a maximum nesting depth (e.g., no more than 3 levels). Deep nesting makes paths long and visibility harder to manage.

6. **`#[path]` usage policy**: Whether `#[path]` is allowed at all, and if so, under what circumstances (e.g., only for sharing code between `src/bin/` and `src/lib.rs`, or for generated code).

7. **Generated code placement**: Where generated code lives and how it is included. Common choices are `src/generated/` with a `#[path]` or `include!` directive.

8. **Helper-module location**: Whether shared test helpers live in `tests/common/mod.rs`, a `tests/common.rs` that is explicitly excluded via `[[test]]` config, or a dev-dependency crate.

9. **Error-type organization**: Whether error types live in each module (with re-exports) or in a dedicated `error`/`errors` module. This affects how `pub(crate)` and `pub use` are applied.

10. **Binary naming convention**: If using `src/bin/`, whether names follow a pattern (e.g., `myapp-server`, `myapp-worker`). Remember binary names use kebab-case while internal modules use snake_case per RFC 430.

11. **Auto-discovery opt-outs**: Document when and how to use `autobins`, `autotests`, `autoexamples`, `autobenches`, and `autolib` to avoid conflicts between target auto-discovery and internal module names.

## Related docs

- `docs/rust/api-design.md` — public-API naming conventions per RFC 430, `#[non_exhaustive]`, sealed traits, builder patterns, and the API Guidelines checklist.
- `docs/rust/cargo-dependencies.md` — `Cargo.toml` manifest internals, `[dependencies]`/`[dev-dependencies]`/`[build-dependencies]`, features & the resolver, profiles, `[lints]`, workspace inheritance (`workspace.package`/`workspace.dependencies`/`workspace.lints`), and `[patch]`/`[replace]` root-only rules.
- `docs/rust/testing.md` — test-writing style and test organization guidance.
- `docs/rust/style-formatting.md` — formatting conventions and style rules.
- `docs/rust/editions-tooling.md` — edition-specific path-resolution differences (2015 vs 2018+), `extern crate` idioms, and tooling.
- `docs/rust/documentation-guidelines.md` — doc comments and rustdoc guidance for public items.
- Official Rust Book ch07 — packages, crates, modules, paths, `use`, and visibility: https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- Rust Reference: items/modules — module grammar and `#[path]` semantics: https://doc.rust-lang.org/reference/items/modules.html
- Rust Reference: visibility-and-privacy — visibility grammar and access rules: https://doc.rust-lang.org/reference/visibility-and-privacy.html
- Rust Reference: items/use-declarations — `use` grammar and restrictions: https://doc.rust-lang.org/reference/items/use-declarations.html
- Rust Reference: items/extern-crates — `extern crate` grammar and 2018+ idiom: https://doc.rust-lang.org/reference/items/extern-crates.html
- Rust Reference: paths — path grammar, qualifiers, turbofish, and canonical paths: https://doc.rust-lang.org/reference/paths.html
- Rust Reference: names/preludes — prelude layers and `#![no_implicit_prelude]`: https://doc.rust-lang.org/reference/names/preludes.html
- Cargo Book: guide/project-layout — canonical directory layout: https://doc.rust-lang.org/cargo/guide/project-layout.html
- Cargo Book: reference/cargo-targets — target kinds, auto-discovery, and opt-outs: https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- Cargo Book: reference/build-scripts — `build.rs` placement and Cargo communication: https://doc.rust-lang.org/cargo/reference/build-scripts.html
- Cargo Book: reference/workspaces — workspace layout and root-level tables: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Cargo Book: guide/tests — integration test helpers and the `tests/common/mod.rs` convention: https://doc.rust-lang.org/cargo/guide/tests.html

## Related skills

- `nix-usage` — for building, checking, and testing Rust code within the ai-workbench Nix development shell.
- No React/front-end-specific skills apply to this Rust guidance.
