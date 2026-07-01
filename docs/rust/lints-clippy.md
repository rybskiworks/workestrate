# Lints, Clippy, and Lint Configuration

## Purpose

This document provides comprehensive guidance on Rust linting for future AI coding agents. It covers Clippy, rustc built-in lints, the `[lints]` table in `Cargo.toml`, workspace-level lint inheritance, attribute syntax, `clippy.toml` configuration, CI integration, and common anti-patterns. The goal is to produce code that is lint-clean, consistently configured, and easy to review.

Linting (Clippy/rustc) is distinct from formatting (rustfmt). Formatting policy—`cargo fmt`, `rustfmt.toml`, `.rustfmt.toml`—is owned by [`style-formatting.md`](style-formatting.md). Keep formatting concerns out of lint configuration.

## Sources used

Primary sources:

- https://doc.rust-lang.org/stable/clippy/ — Clippy documentation home
- https://doc.rust-lang.org/stable/clippy/usage.html — How to invoke Clippy
- https://doc.rust-lang.org/stable/clippy/lints.html — Clippy lint list
- https://rust-lang.github.io/rust-clippy/master/index.html — Full searchable lint index (live count)
- https://github.com/rust-lang/rust-clippy — Clippy source repository
- https://doc.rust-lang.org/rustc/lints/index.html — rustc lint overview
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section — `[lints]` in `Cargo.toml`
- https://doc.rust-lang.org/cargo/reference/lints.html — Cargo lint configuration reference
- https://doc.rust-lang.org/cargo/reference/workspaces.html#the-lints-table — Workspace lint inheritance
- https://rust-lang.github.io/rfcs/3389-manifest-lint.html — RFC 3389: manifest-level lint configuration

Additional authoritative references:

- https://doc.rust-lang.org/rustc/lints/levels.html — Lint levels and precedence
- https://doc.rust-lang.org/rustc/lints/groups.html — Built-in lint groups
- https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html — Allowed-by-default rustc lints
- https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html — Warn-by-default rustc lints
- https://doc.rust-lang.org/cargo/reference/resolver.html — Cargo dependency/feature resolver
- https://rust-lang.github.io/rust-clippy/master/ — Clippy master docs
- https://rust-lang.github.io/rust-clippy/master/#configuration — `clippy.toml` configuration overview
- https://rust-lang.github.io/rust-clippy/master/lint_configuration.html — Clippy lint configuration options

## Core guidance

### Clippy overview

[Clippy](https://github.com/rust-lang/rust-clippy) is a collection of **over 800 lints** that run on top of the rustc compiler. The live [master index](https://rust-lang.github.io/rust-clippy/master/index.html) shows the current total (e.g., "Total number: 822"); the count drifts upward as lints are added or deprecated. Clippy catches common mistakes, enforces idiomatic style, identifies performance issues, and flags suspicious patterns that the compiler itself does not warn about. Clippy is distributed via rustup and is the standard linting tool for Rust projects.

Invoke Clippy through Cargo:

```bash
cargo clippy                          # Lint the default target
cargo clippy --all-targets            # Lint all targets (lib, bins, tests, examples, benches)
cargo clippy --all-targets -- -D warnings  # Promote all warnings to errors
cargo clippy --fix                    # Auto-apply machine-applicable suggestions
```

See [Clippy usage](https://doc.rust-lang.org/stable/clippy/usage.html) for the full set of invocation options.

By default, `cargo clippy` runs the `clippy::all` group, which includes `correctness`, `suspicious`, `style`, `complexity`, and `perf`. The `pedantic`, `nursery`, `cargo`, and `restriction` groups are opt-in.

### Lint levels

Rust lints have six levels, documented at [lint levels](https://doc.rust-lang.org/rustc/lints/levels.html):

| Level | Behavior | Overridable? |
|-------|----------|--------------|
| `allow` | Suppress the lint entirely | Yes |
| `expect` | Suppress, but warn at the lint's default level if the lint is **not** triggered (Rust 1.81+, attribute-only) | Yes |
| `warn` | Emit a warning; compilation succeeds | Yes |
| `force-warn` | Always warn; cannot be overridden by attributes or other flags (CLI-only via `--force-warn <lint>`) | No |
| `deny` | Emit an error; compilation fails | Yes, by `forbid` |
| `forbid` | Emit an error; compilation fails | **No** — cannot be overridden by inner attributes; only `--cap-lints` can demote it |

Precedence (highest to lowest):

1. `--force-warn <lint>` (highest; cannot be demoted by anything except nothing)
2. `--cap-lints <level>` (sets the maximum lint level; Cargo passes `--cap-lints allow` for dependencies)
3. CLI level flags (`-W`/`-A`/`-D`/`-F`), with right-most winning
4. Inner attributes / lower-in-syntax-tree attributes
5. Outer attributes
6. Defaults (lowest)

`--cap-lints` cannot demote `force-warn`. Cargo passes `--cap-lints allow` when compiling dependencies so their lints do not pollute the build; this is why `[lints]` applies only to the local package.

The `warnings` lint group is special: it ignores attribute/CLI order and applies to all lints that would otherwise warn within the entity. This is why `#[deny(warnings)]` promotes all warnings to errors.

Use `forbid` only for lints that must never be relaxed anywhere in the crate (e.g., `unsafe_code` in a crate that must be safe). Use `deny` for lints that should fail CI but can be locally overridden with a documented reason. Prefer `expect(...)` over `allow(...)` when the suppression is intentional and should remain relevant.

### Clippy lint groups

Clippy organizes its lints into groups. See the [full lint list](https://doc.rust-lang.org/stable/clippy/lints.html) and the [searchable index](https://rust-lang.github.io/rust-clippy/master/index.html).

| Group | Default? | Default level | Description |
|-------|----------|---------------|-------------|
| `clippy::correctness` | Yes | **deny** | Code that is outright wrong or very likely a bug. Never suppress without a very good reason. |
| `clippy::suspicious` | Yes | **warn** | Code that is probably wrong or misleading. High signal, but some false positives. |
| `clippy::style` | Yes | **warn** | Non-idiomatic code. Following these makes code more readable to experienced Rust developers. |
| `clippy::complexity` | Yes | **warn** | Code that is more complex than necessary. Simplification improves readability and reduces bugs. |
| `clippy::perf` | Yes | **warn** | Code that can be written to run faster. Usually zero-cost abstractions or unnecessary allocations. |
| `clippy::pedantic` | No | **allow** | Strict, opinionated lints. High signal but noisy. Opt-in for libraries that want maximum rigor. |
| `clippy::nursery` | No | **allow** | Experimental lints that may have false positives. Review individually before enabling. |
| `clippy::cargo` | No | **allow** | Lints for Cargo manifest issues (duplicate dependencies, version mismatches). Opt-in for published crates. |
| `clippy::restriction` | No | **allow** | Very strict lints that forbid specific patterns. Enable **one at a time**, never as a group. |

The `clippy::all` group is an alias for the five default-on groups (`correctness` + `suspicious` + `style` + `complexity` + `perf`). Its effective default level is a mix of `deny` (from `correctness`) and `warn` (from the other four groups).

`clippy::blanket_clippy_restriction_lints` is a `suspicious` / `warn` lint that flags enabling the whole `clippy::restriction` category. Restriction lints are intentionally meant to be enabled individually; this lint is the authoritative reason to never enable `clippy::restriction` as a whole group. See the [master index](https://rust-lang.github.io/rust-clippy/master/index.html).

### rustc built-in lints and groups

The Rust compiler itself provides lints documented at [rustc lints](https://doc.rust-lang.org/rustc/lints/index.html) and [lint groups](https://doc.rust-lang.org/rustc/lints/groups.html).

Built-in lint groups include:

- **`warnings`**: Applies to all warn-by-default lints.
- **`unused`**: `unused_imports`, `unused_variables`, `dead_code`, `unused_must_use`, `unused_unsafe`, and others.
- **`nonstandard_style`** (hyphenated group name): `non_camel_case_types`, `non_snake_case`, `non_upper_case_globals`. The alias `bad_style` is deprecated.
- **`rust_2018_compatibility`**: Lints for migrating to the 2018 edition.
- **`rust_2018_idioms`**: `bare_trait_objects`, `elided_lifetimes_in_paths`, and others.
- **`rust_2021_compatibility`**: Lints for migrating to the 2021 edition.
- **`rust_2024_compatibility`**: Includes `unsafe_op_in_unsafe_fn`, `static_mut_refs`, `missing_unsafe_on_extern`, and others.

Common individual rustc lints and their default levels:

Warn-by-default:

- `unused_imports`
- `unused_variables`
- `dead_code`
- `unused_must_use`
- `unused_unsafe`
- `non_snake_case`
- `non_camel_case_types`
- `non_upper_case_globals`
- `private_interfaces`
- `warnings` (the group)

Allow-by-default (useful opt-ins):

- `missing_docs` (public items; most useful in library crates)
- `unsafe_code`
- `unsafe_op_in_unsafe_fn` (allow-by-default in pre-2024 editions; warn-by-default in the 2024 edition; member of `rust_2024_compatibility`)
- `elided_lifetimes_in_paths`
- `unreachable_pub`
- `unnameable_types`
- `unused_qualifications`
- `variant_size_differences`

For `unsafe_op_in_unsafe_fn`: in editions before 2024 it is allow-by-default; in the 2024 edition it is warn-by-default. For safe encapsulation of unsafe code, consider enabling it explicitly (`warn` or `deny`) in all editions. See [`unsafe-security.md`](unsafe-security.md) for unsafe-code policy and safety-comment requirements.

### Relationship between rustc lints and Clippy lints

rustc lints are built into the compiler and run on every compilation. Clippy lints are external, distributed as a separate tool via rustup, and run via `cargo clippy`. The `[lints]` table in `Cargo.toml` configures both: rustc lints use unprefixed names (`unsafe_code`, `warnings`), while Clippy lints use the `clippy::` prefix (`clippy::pedantic`, `clippy::unwrap_used`). In the manifest this appears as `[lints.rust]` for rustc lints and `[lints.clippy]` for Clippy lints.

There is **no `clippy::must_use` lint**. The must_use-related Clippy lints are:

- `clippy::must_use_candidate` (pedantic / allow)
- `clippy::must_use_unit` (style / warn)
- `clippy::double_must_use` (style / warn)

rustc's own `unused_must_use` and the `#[must_use]` attribute are separate from Clippy.

## Practical rules

1. **Configure lints in `[lints]` tables in `Cargo.toml`** — not scattered `#![deny(...)]` crate attributes — so the policy is reviewable in one place.
2. **Run `cargo clippy --all-targets -- -D warnings` in CI** to catch all lint violations at every merge.
3. **Deny `clippy::correctness` and `clippy::suspicious`** — these are almost always real issues. (Correctness is already deny-by-default; explicitly deny suspicious if you override it.)
4. **Opt in to `clippy::pedantic` for library crates** — the signal-to-noise ratio is high for public APIs; allow individually noisy lints.
5. **Never use `#[forbid(...)]` for lints you might need to scope-allow** — `forbid` cannot be overridden; use `deny` instead.
6. **Scope `#[allow(...)]` to the narrowest possible item** — never allow a lint at the crate root when a single function or line suffices.
7. **Add a `reason` parameter to every `#[allow(...)]` and `#[expect(...)]`** — future readers (including agents) need to know why the lint was suppressed.
8. **Set `msrv` in `clippy.toml`** — Clippy will skip lints that suggest features unavailable at your minimum supported Rust version.
9. **Use `resolver = "2"` (or `"3"` for edition 2024 / Rust 1.84+) in `Cargo.toml`** when adding `[lints]` — the feature resolver v2 or newer is required.
10. **Forbid `unsafe_code` in crates that must be safe** — use `#![forbid(unsafe_code)]` or `[lints.rust] unsafe_code = "forbid"`.
11. **Allow `clippy::unwrap_used` and `clippy::expect_used` only in test code** — use scoped allows or `clippy.toml` `allow-unwrap-in-tests` / `allow-expect-in-tests`, not crate-wide allows.
12. **Version-control `clippy.toml`** — lint configuration is project policy and must be tracked in the repository.
13. **Review `cargo clippy --fix` output before committing** — automatic fixes are not always semantically identical to the original code.
14. **Never enable `clippy::restriction` as a whole group** — restriction lints are opt-in one at a time; enabling the group triggers `clippy::blanket_clippy_restriction_lints`.
15. **Prefer `clippy.toml` `allow-*-in-tests` options** over scoped `#[allow(...)]` attributes where possible (e.g., `allow-unwrap-in-tests = true`).
16. **Use `priority = -1` (or lower) for group-level lint entries** so individual lint overrides at default priority 0 are emitted later and win.

## Key lints to enable

### Default groups (enabled by `clippy::all`)

These run automatically with `cargo clippy`. Do not suppress them without strong justification:

- `clippy::correctness` — bugs and likely-bugs (e.g., `clippy::eq_op`, `clippy::drop_copy`). Default level: **deny**.
- `clippy::suspicious` — suspicious code (e.g., `clippy::arc_with_non_send_sync`, `clippy::suspicious_else_formatting`). Default level: **warn**.
- `clippy::style` — idiomatic style (e.g., `clippy::ptr_arg`, `clippy::needless_return`, `clippy::redundant_closure`). Default level: **warn**.
- `clippy::complexity` — overly complex code (e.g., `clippy::clone_on_copy`, `clippy::vec_box`, `clippy::too_many_arguments`). Default level: **warn**.
- `clippy::perf` — performance issues (e.g., `clippy::needless_collect`). Default level: **warn**.

### Opt-in groups

- **`clippy::pedantic`**: Strict lints with high signal. Examples: `clippy::must_use_candidate`, `clippy::missing_errors_doc`, `clippy::single_char_pattern`, `clippy::redundant_closure_for_method_calls`, `clippy::manual_let_else`. Enable for library crates; suppress individual lints that are too noisy.
- **`clippy::cargo`**: Catches manifest issues like duplicate dependencies and version inconsistencies. Enable for published crates.
- **`clippy::nursery`**: Experimental lints. Review individually; some have false positives. Example: `clippy::option_if_let_else`.

### Notable individual lints

| Lint | Group | Default level | Description |
|------|-------|---------------|-------------|
| `clippy::eq_op` | correctness | deny | Same expression on both sides of a binary operator (e.g., `a == a`). |
| `clippy::ptr_arg` | style | warn | Using `&Vec<T>` or `&String` where `&[T]` or `&str` suffices. |
| `clippy::needless_collect` | perf | warn | `collect()` when an iterator method would suffice. |
| `clippy::clone_on_copy` | complexity | warn | `clone()` on a `Copy` type; use `*` or copy. |
| `clippy::vec_box` | complexity | warn | `Vec<Box<T>>` where `Vec<T>` is usually better. |
| `clippy::needless_return` | style | warn | Explicit `return` at the end of a function. |
| `clippy::arc_with_non_send_sync` | suspicious | warn | `Arc<T>` where `T` is not `Send + Sync`. |
| `clippy::must_use_candidate` | pedantic | allow | Suggests `#[must_use]` for functions whose return value should not be ignored. |
| `clippy::missing_errors_doc` | pedantic | allow | Public `Result`-returning functions should document errors. |
| `clippy::single_char_pattern` | pedantic | allow | `.split("x")` could be `.split('x')`. |
| `clippy::redundant_closure_for_method_calls` | pedantic | allow | `\|x\| x.method()` where `Type::method` suffices. |
| `clippy::manual_let_else` | pedantic | allow | Manual `match`/`if let` that could use `let ... else`. |
| `clippy::option_if_let_else` | nursery | allow | `if let`/`else` on `Option` that could use combinator methods. |
| `clippy::module_name_repetitions` | restriction | allow | Module name repeated in item names; opt-in individually. |

### Restriction lints (enable individually, never as a group)

All restriction lints are **allow-by-default** and opt-in individually:

- `clippy::unwrap_used`
- `clippy::expect_used`
- `clippy::panic`
- `clippy::todo`
- `clippy::unimplemented`
- `clippy::dbg_macro`
- `clippy::print_stdout`
- `clippy::print_stderr`
- `clippy::clone_on_ref_ptr`
- `clippy::indexing_slicing`
- `clippy::unwrap_in_result`
- `clippy::float_arithmetic`
- `clippy::arithmetic_side_effects`

## Lint configuration via Cargo.toml `[lints]` table

[RFC 3389](https://rust-lang.github.io/rfcs/3389-manifest-lint.html) introduced the `[lints]` table in `Cargo.toml`, documented at [the lints section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section) and [Cargo lint reference](https://doc.rust-lang.org/cargo/reference/lints.html). This is the preferred way to configure lint levels — it is declarative, version-controlled, and visible in code review.

### Stabilization and resolver requirement

- The `[lints]` table was stabilized in **Rust/Cargo 1.74**.
- It requires feature resolver **v2 or newer**. Edition 2021 defaults to resolver v2; edition 2024 defaults to resolver v3 (resolver `"3"` requires Rust 1.84+).
- For **virtual workspaces** (no `[package]` table), `resolver` must be set explicitly in `[workspace]` because there is no `package.edition` to infer from.
- Without resolver v2 or newer, Cargo errors when `[lints]` is used.
- `[lints]` applies **only to the local package**, not to dependencies. Dependency lints are capped via `--cap-lints allow`.
- `RUSTFLAGS` and `.cargo/config.toml` `rustflags` **override** `[lints]`, because manifest lints are emitted before `RUSTFLAGS` on the command line and rustc uses last-write-wins.

Set `resolver` in `[package]` or `[workspace]`:

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
resolver = "2"
```

### Lint name prefixes

- rustc lints: no prefix — `unsafe_code`, `warnings`, `dead_code`, `missing_docs`
- Clippy lints: `clippy::` prefix — `clippy::pedantic`, `clippy::unwrap_used`

In the manifest:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
```

### Priority mechanism

Cargo sorts lint specs by `priority` (signed integer; default 0) and passes them to rustc on the command line, with **lower priority first**. Because rustc uses "last-write-wins" for lint flags, entries emitted **later** (higher priority) override earlier ones. Therefore groups must be given `priority = -1` (or lower) so individual lints at default priority 0 are emitted after and override the group level. Within the same priority the order is unspecified — never rely on it.

See [RFC 3389](https://rust-lang.github.io/rfcs/3389-manifest-lint.html) and [the manifest lints section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section).

### Example `[lints]` table

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
elided_lifetimes_in_paths = "warn"
unreachable_pub = "warn"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
nursery = { level = "allow", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
todo = "warn"
unimplemented = "warn"
indexing_slicing = "warn"
clone_on_ref_ptr = "warn"
```

## Workspace-level lint inheritance

For workspaces, define shared lint policy in `[workspace.lints]` and have crates opt in with `lints.workspace = true`. See [workspace lints table](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-lints-table).

### Workspace root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
dbg_macro = "deny"
```

### Crate-level `Cargo.toml` (inherit from workspace)

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[lints]
workspace = true
```

This mirrors the `[workspace.dependencies]` pattern: define once at the workspace root, inherit in each crate. Adding or changing a lint in the workspace root propagates to all member crates.

### Important: do not mix `workspace = true` with local overrides

Per [RFC 3389](https://rust-lang.github.io/rfcs/3389-manifest-lint.html), when `workspace = true` is present, **no other fields are allowed** in the `[lints]` table. A member crate cannot currently override workspace lints locally. If a crate needs different lints, either (a) do not inherit (`workspace = true`) and define a complete local `[lints]` table, or (b) add the override to `[workspace.lints]` at the root so it applies to all members. Future Cargo may support per-package overrides, but today mixing the two is a hard error.

## Attribute syntax

### Item-level attributes

Apply lint levels to specific items (functions, structs, modules, etc.):

```rust
#[allow(clippy::unwrap_used)]
fn init_logger() {
    // unwrap is safe here because this runs before any user code
    env_logger::Builder::from_default_env().init().unwrap();
}

#[deny(clippy::todo)]
fn critical_path() {
    // todo!() will cause a compile error here
}
```

### Crate-level attributes

Use `#!` (inner attribute) to apply to the entire crate:

```rust
#![forbid(unsafe_code)]
#![warn(missing_docs)]
```

Prefer the `[lints]` table over crate-level attributes. The `[lints]` table is more visible, easier to review, and supports workspace inheritance.

### Module-level attributes

Apply at the top of a module file to affect all items in that module:

```rust
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    // unwrap and expect are acceptable in test code
}
```

### Scoped allows with `reason` parameter

Rust 1.81+ supports a `reason` parameter on lint attributes. Always include a reason when suppressing a lint:

```rust
#[expect(clippy::unwrap_used, reason = "invariant: config is always valid after parse")]
fn get_timeout(config: &Config) -> Duration {
    config.timeout.unwrap()
}
```

The `expect(...)` attribute is stronger than `allow(...)`: it warns at the lint's default level if the lint is *not* triggered, ensuring the suppression remains relevant. Use `expect` when you are intentionally relying on a pattern that a lint would flag.

## Clippy configuration (`clippy.toml`)

Clippy reads project-level configuration from `clippy.toml` or `.clippy.toml`. Lookup order: `CLIPPY_CONF_DIR` env var → `CARGO_MANIFEST_DIR` env var → current working directory; if not found, Clippy walks **up** the directory tree until found or reaching the root. Recommend placing `clippy.toml` at the workspace root. Note that `clippy.toml` is an **unstable API**: options may change between versions. See [Clippy configuration](https://rust-lang.github.io/rust-clippy/master/#configuration) and [lint configuration](https://rust-lang.github.io/rust-clippy/master/lint_configuration.html).

### MSRV discovery precedence

Clippy MSRV is discovered in this order (highest precedence first):

1. Inner attribute `#![clippy::msrv = "x.y"]` (nightly feature)
2. `msrv` in `clippy.toml`
3. `rust-version` in `Cargo.toml`
4. Current compiler version (default if nothing is set)

Setting MSRV correctly makes MSRV-aware lints suppress or adjust suggestions that require a newer Rust. `--cap-lints` is not related to MSRV.

### Key settings

| Setting | Default | Description |
|---------|---------|-------------|
| `msrv` | current compiler version | Minimum Supported Rust Version. Clippy skips lints that suggest features unavailable at this version. |
| `cognitive-complexity-threshold` | 25 | Threshold for `cognitive_complexity` lint. |
| `too-many-arguments-threshold` | 7 | Threshold for `too_many_arguments` lint. |
| `too-many-lines-threshold` | 100 | Threshold for `too_many_lines` lint. |
| `disallowed-names` | `["foo", "baz", "quux"]` | Names that `disallowed_names` lint flags. `blacklisted-names` is the deprecated alias. |
| `doc-valid-idents` | large built-in list (e.g., GitHub, OAuth2) | Valid identifiers for `doc_markdown` lint. |
| `arithmetic-side-effects-allowed` | `+`, `-binary`, `-unary` | Operators allowed by `arithmetic_side_effects`. |
| `standard-macro-braces` | — | Enforce specific brace styles for macros. |
| `allowed-idents-below-min-chars` | `["i", "j", "x", "y", "z", "w", "n"]` | Allow short identifiers below the minimum character threshold. |
| `enum-variant-name-threshold` | 3 | Threshold for `enum_variant_names` lint. |
| `single-char-binding-names-threshold` | 4 | Threshold for `single_char_binding_names`. |
| `vec-box-size-threshold` | 4096 | Threshold for `vec_box` lint. |
| `pass-by-value-size-limit` | 256 | Size limit for `large_types_passed_by_value`. |
| `type-complexity-threshold` | 250 | Threshold for `type_complexity` lint. |
| `array-size-threshold` | 16384 | Threshold for `large_stack_arrays`. |

Use the `disallowed-methods` / `disallowed-types` / `disallowed-macros` / `disallowed-fields` family for org-specific standards. The `allow-*-in-tests` family (`allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-dbg-in-tests`, `allow-print-in-tests`) is a cleaner alternative to scoped `#[allow]` in tests.

### Example `clippy.toml`

```toml
msrv = "1.74"
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7
disallowed-names = ["foo", "bar", "baz", "quux"]
doc-valid-idents = ["OAuth2", "GraphQL", "NaN", "IPv4", "IPv6"]
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

Always version-control `clippy.toml`. It is project policy, not a personal preference file.

## cargo clippy invocation reference

Common invocations (from [Clippy usage](https://doc.rust-lang.org/stable/clippy/usage.html)):

```bash
cargo clippy                                # Default: runs clippy::all
cargo clippy --all-targets                  # Lint lib, bins, tests, examples, benches
cargo clippy --workspace                    # Lint all workspace members
cargo clippy --all-features                 # Lint with all features enabled
# Everything after -- is forwarded to rustc/clippy-driver:
cargo clippy -- -D warnings                 # Deny all warnings (rustc + clippy)
cargo clippy -- -D clippy::all              # Deny only clippy warnings
cargo clippy -- -W clippy::pedantic         # Warn on pedantic lints
cargo clippy -- -A clippy::all -W clippy::useless_format  # Allow all but one
cargo clippy --fix                          # Auto-apply machine-applicable fixes
cargo clippy --fix --allow-dirty --allow-staged             # Fix with uncommitted/staged changes
cargo clippy -p <crate> -- --no-deps        # Lint only the named crate, not path/workspace deps
```

Notes:

- `cargo clippy --fix` implies `--all-targets` (fixes tests/examples/benches too).
- `--allow-dirty` and `--allow-staged` are forwarded from `cargo fix`.
- Not all lints have machine-applicable fixes. Always review the diff.
- Without `--no-deps`, `cargo clippy -p <crate>` also lints path/workspace dependency crates.
- `-D warnings` denies **all** warnings including rustc's; to deny only Clippy use `-D clippy::all`.

## CI integration

### Baseline CI command

The standard CI invocation denies all warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

For workspaces:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Additional flags

- `--all-features`: Lint with all feature combinations enabled. Catches conditional compilation issues.
- `--keep-going`: Continue linting after errors (Rust 1.74+). Useful for seeing all violations in one pass.
- `--target <triple>`: Lint for a specific target. Useful for cross-compilation projects.

### Caching and incremental lint runs

Clippy benefits from Cargo's build cache. In CI, cache `target/` and `~/.cargo/` between runs. Incremental compilation means re-running Clippy after a small change is fast.

### MSRV-specific lint jobs

If your project supports multiple Rust versions, run Clippy at the MSRV to ensure lint configuration is compatible:

```bash
cargo +1.74 clippy --all-targets -- -D warnings
```

### Nightly Clippy

Some lints are only available on nightly. Run nightly Clippy in a separate CI job to catch forward-compatibility issues:

```bash
cargo +nightly clippy -- -D warnings
```

Do not make nightly Clippy a blocking job — nightly lints can change — but use it as an informational check.

## When to allow a lint

### Legitimate use cases

- **`unwrap_used` / `expect_used` in tests**: Panicking on failure is the correct behavior in test code. Allow these at the test-module level, not globally.
- **`print_stdout` in CLI binaries**: Command-line tools are expected to print to stdout. Allow at the module or function level.
- **`too_many_arguments` in FFI bindings**: External APIs cannot be refactored. Allow on specific functions.
- **`cognitive_complexity` in parser code**: Parsers are inherently complex. Allow on specific functions with a reason.

### Scoped allows with comments

Always scope the allow to the narrowest item and explain why:

```rust
#[allow(clippy::unwrap_used, reason = "invariant: env var set by CI")]
fn get_ci_token() -> String {
    env::var("CI_TOKEN").unwrap()
}
```

### Per-module vs per-line scope

- **Per-module**: When the entire module legitimately violates a lint (e.g., test module with `unwrap_used`).
- **Per-function**: When a specific function has a justified reason.
- **Per-line**: When a single expression needs the suppression. Use `#[allow]` on the statement or expression.

### Using `expect` for known-safe operations

Prefer `expect(...)` over `allow(...)` when the suppression is intentional and should remain relevant:

```rust
#[expect(clippy::unwrap_used, reason = "invariant: parse validated above")]
fn get_value(parsed: &Parsed) -> &str {
    parsed.field.unwrap()
}
```

If the code changes such that the lint is no longer triggered, `expect` will produce a warning, prompting removal of the now-unnecessary suppression.

### Crate-level forbids for absolute no-go lints

Use `forbid` for lints that must never be violated anywhere in the crate:

```rust
#![forbid(unsafe_code)]
```

Or in `[lints]`:

```toml
[lints.rust]
unsafe_code = "forbid"
```

Do not use `forbid` for lints that might need scoped exceptions — `forbid` cannot be overridden by inner attributes.

## Review checklist

- [ ] Does the PR introduce new `#[allow(...)]` attributes? If so, do they include a `reason` parameter and are they scoped to the narrowest possible item?
- [ ] Are there any `unwrap()`, `expect()`, or `panic!()` calls in non-test production code?
- [ ] Does the PR modify `[lints]` or `clippy.toml`? If so, is the change consistent with workspace policy?
- [ ] Does CI pass `cargo clippy --all-targets -- -D warnings`?
- [ ] Are there any `todo!()` or `unimplemented!()` macros in code that is not explicitly marked as in-progress?
- [ ] Are there `dbg!()`, `print!()`, or `println!()` calls that should use structured logging?
- [ ] Does the PR add `unsafe` code? If so, is it accompanied by a safety comment and is `unsafe_op_in_unsafe_fn` addressed?
- [ ] Are new public items documented? If `missing_docs` is `warn`, undocumented public items will fail CI.
- [ ] Are there any `#[forbid(...)]` attributes that might prevent legitimate scoped exceptions?
- [ ] If the PR adds a new crate, does it inherit workspace lints with `lints.workspace = true`?
- [ ] Are there any `#[deny(warnings)]` crate-level attributes that should be replaced with `[lints]` table entries?
- [ ] Does the PR pass `cargo clippy --all-targets --all-features -- -D warnings` (not just default features)?

## Implementation checklist

- [ ] Add `resolver = "2"` (or `"3"` for edition 2024 / Rust 1.84+) to `[package]` or `[workspace]` in `Cargo.toml` before adding `[lints]`.
- [ ] Define `[lints.rust]` and `[lints.clippy]` sub-tables in `Cargo.toml` (or `[workspace.lints]` for workspaces).
- [ ] Set `clippy::pedantic` and `clippy::cargo` to `warn` with `priority = -1` so individual overrides work.
- [ ] Deny `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` for production code paths.
- [ ] Allow `clippy::unwrap_used` and `clippy::expect_used` in test modules with scoped attributes or `clippy.toml` `allow-*-in-tests`.
- [ ] Create `clippy.toml` at the workspace root with `msrv` set to the project's minimum supported Rust version.
- [ ] Add `#![forbid(unsafe_code)]` or `[lints.rust] unsafe_code = "forbid"` for crates that must be safe.
- [ ] Remove any scattered `#![deny(...)]` or `#![allow(...)]` crate-level attributes and consolidate into `[lints]`.
- [ ] Add `cargo clippy --all-targets -- -D warnings` to CI pipeline.
- [ ] Run `cargo clippy --fix --allow-dirty` to auto-fix existing violations, then review the diff.
- [ ] Verify workspace member crates use `lints.workspace = true` to inherit shared policy.
- [ ] Add a nightly Clippy CI job as an informational (non-blocking) check.
- [ ] Document any repo-specific lint deviations in this guidance file or a `LINT-POLICY.md`.

## Validation hooks

```bash
# Standard CI gate: deny all warnings across all targets
cargo clippy --all-targets -- -D warnings

# Full workspace with all features
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Lint only one crate without its path/workspace dependencies
cargo clippy -p my-crate -- --no-deps

# Auto-fix machine-applicable suggestions (review diff before committing)
cargo clippy --fix --allow-dirty --allow-staged

# Nightly Clippy for forward-compatibility (informational, not blocking)
cargo +nightly clippy -- -D warnings

# MSRV-specific: ensure lint config is compatible with minimum supported version
cargo +1.74 clippy --all-targets -- -D warnings

# Verify [lints] table is picked up (should show configured levels)
cargo clippy --all-targets 2>&1 | head -20

# Check that workspace members inherit lints
cargo metadata --format-version=1 | jq '.packages[] | {name, lints}'
```

## Examples

### 1. Complete `[lints]` table in Cargo.toml

```toml
[package]
name = "my-lib"
version = "0.1.0"
edition = "2021"
resolver = "2"

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
elided_lifetimes_in_paths = "warn"
unreachable_pub = "warn"
unused_must_use = "deny"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
nursery = { level = "allow", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
todo = "warn"
unimplemented = "warn"
indexing_slicing = "warn"
clone_on_ref_ptr = "warn"
```

### 2. Workspace lint inheritance (correct pattern)

**Workspace root `Cargo.toml`:**

```toml
[workspace]
resolver = "2"
members = ["crates/core", "crates/cli", "crates/server"]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
dbg_macro = "deny"
# If the CLI crate legitimately prints to stdout, allow it here for all members
# or do not inherit and define a local [lints] table in crates/cli.
```

**Crate `crates/cli/Cargo.toml` (inherits workspace lints):**

```toml
[package]
name = "cli"
version = "0.1.0"
edition = "2021"

[lints]
workspace = true
```

**Crate `crates/cli/Cargo.toml` (does NOT inherit; defines its own lints):**

```toml
[package]
name = "cli"
version = "0.1.0"
edition = "2021"

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
print_stdout = "allow"   # CLI-specific override
print_stderr = "allow"
```

### 3. `clippy.toml` with MSRV and thresholds

```toml
msrv = "1.74"
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7
too-many-lines-threshold = 100
disallowed-names = ["foo", "bar", "baz", "quux"]
doc-valid-idents = ["OAuth2", "GraphQL", "NaN", "IPv4", "IPv6"]
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

### 4. Scoped `#[allow]` and `#[expect]` with `reason`

```rust
#[allow(clippy::unwrap_used, reason = "invariant: config validated at startup")]
fn get_database_url(config: &Config) -> &str {
    config.database_url.as_deref().unwrap()
}

#[expect(clippy::too_many_arguments, reason = "FFI: matches C ABI signature")]
#[no_mangle]
pub extern "C" fn process_request(
    id: u64,
    name: *const c_char,
    value: f64,
    flags: u32,
    timeout: u32,
    retry: u32,
    callback: extern "C" fn(u64),
    user_data: *mut c_void,
) -> i32 {
    // ...
}
```

### 5. `cargo clippy --fix` workflow

```bash
# Apply safe auto-fixes
cargo clippy --fix --allow-dirty --allow-staged

# Review the changes
git diff

# Verify no remaining violations
cargo clippy --all-targets -- -D warnings
```

### 6. Crate-level `#![forbid(unsafe_code)]`

```rust
//! # my-safe-lib
//!
//! A library that guarantees no unsafe code.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Adds two numbers. Cannot panic, cannot be unsafe.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Prefer the `[lints]` table equivalent:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
```

### 7. CI step snippet

**GitHub Actions:**

```yaml
- name: Clippy
  run: cargo clippy --all-targets -- -D warnings

- name: Clippy (all features)
  run: cargo clippy --all-targets --all-features -- -D warnings

- name: Clippy (nightly, informational)
  continue-on-error: true
  run: cargo +nightly clippy -- -D warnings
```

**GitLab CI:**

```yaml
clippy:
  stage: check
  script:
    - cargo clippy --workspace --all-targets -- -D warnings
  cache:
    key: ${CI_COMMIT_REF_SLUG}
    paths:
      - target/
      - ~/.cargo/registry/
```

## Common mistakes

### 1. Using `#[deny(warnings)]` in a library

```rust
// BAD: denies ALL warnings, including new ones added by future compiler versions
// This breaks downstream users when new lints are added to rustc.
#![deny(warnings)]

// GOOD: use [lints] table to deny specific lints, or let CI enforce -D warnings
```

Libraries should not use `#![deny(warnings)]` because future Rust versions may add new warnings that break the library's compilation. See [rustc lint levels](https://doc.rust-lang.org/rustc/lints/levels.html). Use `[lints]` to deny specific lints, and enforce `cargo clippy -- -D warnings` in CI instead.

### 2. Lint config scattered across `lib.rs` attributes

```rust
// BAD: lint policy is invisible in code review and cannot be inherited by workspace members
#![deny(clippy::unwrap_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
```

```toml
# GOOD: all lint policy in one reviewable location
[lints.clippy]
unwrap_used = "deny"
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
```

### 3. Allowing a lint at the crate root instead of scoping it

```rust
// BAD: allows unwrap everywhere in the crate
#![allow(clippy::unwrap_used)]

// GOOD: allow only where justified
#[allow(clippy::unwrap_used, reason = "invariant: validated at startup")]
fn get_config_value(config: &Config) -> &str {
    config.value.unwrap()
}
```

### 4. Forgetting `resolver = "2"` or `"3"` when adding `[lints]`

```toml
# BAD: [lints] errors without resolver v2+
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[lints.clippy]
unwrap_used = "deny"
```

```toml
# GOOD: resolver v2+ is required for [lints]
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
resolver = "2"

[lints.clippy]
unwrap_used = "deny"
```

See [Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html) and [`cargo-dependencies.md`](cargo-dependencies.md).

### 5. Allowing `clippy::unwrap_used` globally instead of just in tests

```rust
// BAD: unwrap is now allowed everywhere, including production code
#![allow(clippy::unwrap_used)]
```

```rust
// GOOD: allow only in test modules, or use clippy.toml allow-unwrap-in-tests
#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "panicking on test failure is correct")]
mod tests {
    #[test]
    fn test_parse() {
        let result = parse("42").unwrap();
        assert_eq!(result, 42);
    }
}
```

### 6. Using `cargo clippy --fix` without reviewing the diff

```bash
# BAD: blindly committing auto-fixes without review
cargo clippy --fix --allow-dirty --allow-staged
git add -A && git commit -m "fix clippy lints"

# GOOD: review the diff before committing
cargo clippy --fix --allow-dirty --allow-staged
git diff  # review each change
# Only commit after verifying semantic correctness
```

### 7. Using `forbid` for lints that need scoped exceptions

```rust
// BAD: cannot allow unwrap even in tests
#![forbid(clippy::unwrap_used)]

// GOOD: use deny so scoped allows work
#![deny(clippy::unwrap_used)]

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    // This works with deny, but NOT with forbid
}
```

### 8. Missing `priority = -1` on group-level lint configuration

```toml
# BAD: without priority, individual lint overrides may not win
[lints.clippy]
pedantic = "warn"
unwrap_used = "deny"  # order within same priority is unspecified

# GOOD: set group priority lower so individual overrides take precedence
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"  # priority 0 > -1, so this wins
```

### 9. Not setting `msrv` in `clippy.toml`

Without `msrv`, Clippy may suggest features that are unavailable at the project's minimum supported Rust version, leading to CI failures or incorrect suggestions. Always set `msrv` to match the project's actual MSRV.

### 10. Using `blacklisted-names` instead of `disallowed-names` in `clippy.toml`

`blacklisted-names` is a deprecated alias for `disallowed-names`. Use `disallowed-names` in new configuration:

```toml
# BAD (deprecated)
blacklisted-names = ["foo", "bar"]

# GOOD
disallowed-names = ["foo", "bar"]
```

### 11. Mixing `[lints] workspace = true` with local lint overrides

This is a **hard error** in current Cargo. You cannot combine workspace inheritance with per-crate lint overrides.

```toml
# BAD: hard error
[lints]
workspace = true

[lints.clippy]
print_stdout = "allow"
```

```toml
# GOOD: inherit everything from workspace
[lints]
workspace = true
```

```toml
# GOOD: define a complete local [lints] table and do NOT inherit
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
print_stdout = "allow"
```

Per [RFC 3389](https://rust-lang.github.io/rfcs/3389-manifest-lint.html), "Packages overriding inherited lints" is a known future possibility, but today `workspace = true` precludes any other `[lints]` fields.

### 12. Mis-grouping lints by intuition

Clippy lint groups change over time. Do not assume a lint's group from its name. For example, `clippy::arc_with_non_send_sync` is in `suspicious` (not `correctness`), `clippy::single_char_pattern` and `clippy::manual_let_else` are in `pedantic` (not `style` or `perf`), and `clippy::module_name_repetitions` is in `restriction` (not `pedantic`). Always verify the current group and default level against the live [master index](https://rust-lang.github.io/rust-clippy/master/index.html).

## Strict vs contextual guidance

### Strict guidance

- **Use `[lints]` table in `Cargo.toml`** instead of scattered `#![deny(...)]` crate attributes. The policy must be reviewable in one place.
- **CI must run `cargo clippy -- -D warnings`** as a blocking gate. No lint warnings in the main branch.
- **No `unwrap()` or `expect()` in production code paths.** Deny `clippy::unwrap_used` and `clippy::expect_used`; allow only in test modules.
- **Do not use `#[deny(warnings)]` in library crates.** This breaks downstream users when new compiler warnings are added. Deny specific lints instead.
- **Lint configuration must be version-controlled.** `clippy.toml` and `[lints]` tables are project policy, not personal preferences.
- **Always include a `reason` parameter** on `#[allow(...)]` and `#[expect(...)]` attributes so future readers understand the justification.
- **Never enable `clippy::restriction` as a whole group.** Enable restriction lints individually; enabling the group triggers `clippy::blanket_clippy_restriction_lints`.

### Common convention

- **Opt in to `clippy::pedantic` for library crates.** The signal-to-noise ratio is high for public APIs.
- **Opt in to `clippy::cargo` for published crates.** Catches manifest issues early.
- **Use `missing_docs = "warn"` for public APIs.** Documentation is part of the API contract.
- **Use scoped `#[allow(...)]` with reason** instead of crate-wide allows.
- **Set `msrv` in `clippy.toml`** to match the project's minimum supported Rust version.
- **Use `expect(...)` attribute** (Rust 1.81+) for intentionally-suppressed lints that should remain relevant.
- **Prefer `clippy.toml` `allow-*-in-tests`** over scoped `#[allow(...)]` for test-only exceptions.

### Contextual tradeoffs

- **`clippy::pedantic` is opinionated.** Teams differ on whether it is a net positive. Some lints (e.g., `must_use_candidate`, `redundant_closure_for_method_calls`) are noisy. Enable the group at `warn` and allow individual lints that are too aggressive.
- **`clippy::nursery` lints can be flaky.** They are experimental and may have false positives. Review individually before enabling; do not enable as a group.
- **`clippy::restriction` is opt-in only for specific cases.** Never enable the entire restriction group. Enable individual lints (e.g., `unwrap_used`, `dbg_macro`) based on project needs.
- **`clippy::cargo` may flag intentional duplicate dependencies.** Some duplicates are unavoidable due to transitive dependency version conflicts. Allow specific instances with documented reasons.
- **`unsafe_op_in_unsafe_fn` default level depends on edition.** It is allow-by-default before the 2024 edition and warn-by-default in the 2024 edition. Consider enabling it explicitly in all editions for safe unsafe encapsulation.

### Repo-policy decision points

Each repository must decide and document:

1. **Which lint groups are deny vs warn.** E.g., is `clippy::pedantic` `deny` or `warn`?
2. **MSRV for `clippy.toml`.** What is the minimum supported Rust version?
3. **Whether `panic`/`unwrap` are allowed in tests.** Most projects allow them; some enforce `Result`-based testing.
4. **Whether `missing_docs` is enforced.** Libraries typically enforce it; applications may not.
5. **Whether `unsafe_op_in_unsafe_fn` is deny or warn.** The 2024 edition makes this default `warn`; some projects deny it explicitly across all editions.
6. **Whether `clippy::indexing_slicing` is enabled.** Some projects prefer `v[i]` for readability; others require `v.get(i)`.
7. **Whether `clippy::float_arithmetic` is enabled.** Only relevant for projects that need deterministic numerics.
8. **Whether nightly Clippy is run in CI.** Informational vs blocking.

## Policy decisions for individual repos

Document repo-specific lint policy in a `LINT-POLICY.md` or in this file's repo-specific appendix. At minimum, record:

| Decision | Value | Rationale |
|----------|-------|-----------|
| `clippy::pedantic` level | warn/deny | ... |
| `clippy::cargo` level | warn/deny/allow | ... |
| `clippy::nursery` level | allow/warn | ... |
| `unwrap_used` in tests | allow/deny | ... |
| `missing_docs` for public API | warn/allow | ... |
| `unsafe_op_in_unsafe_fn` | warn/deny | ... |
| MSRV | e.g., "1.74" | ... |
| Nightly Clippy in CI | yes/no, blocking/informational | ... |
| `indexing_slicing` | warn/allow | ... |

## Related docs

- `unsafe-security.md` — `unsafe` code policy, safety comments, and `unsafe_op_in_unsafe_fn` guidance
- `cargo-dependencies.md` — dependency management, `[lints]`/`resolver` at a high level, `[workspace.lints]`, and duplicate dependency handling
- `style-formatting.md` — rustfmt / `cargo fmt` / `rustfmt.toml`; formatting is separate from linting
- `testing.md` — clippy as a CI gate and test-module lint exceptions
- `editions-tooling.md` — edition-specific lint behavior, resolver versions, and 2024 edition changes
- `supply-chain-security.md` — cargo-audit/vet and supply-chain auditing (not a linting topic; cross-reference only)

## Related skills

No repo-specific skills for this topic.
