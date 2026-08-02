# Rust Style and Formatting

## Purpose

This document provides canonical Rust style and formatting guidance for future AI agents. It is centered on the official [Rust Style Guide](https://doc.rust-lang.org/style-guide/) and on [rustfmt](https://rust-lang.github.io/rustfmt/), the official Rust formatter, invoked through `cargo fmt`. The goal is to produce code that is consistent, reviewable, and mechanically verifiable, with minimal manual formatting decisions.

Because rustfmt is the reference implementation of the style guide, rustfmt output is treated as the practical authority in this repo. Where rustfmt and the prose style guide differ, follow rustfmt unless the project has an explicit, documented exception. Format code before committing and enforce formatting in CI so diffs remain focused on semantic changes.

## Sources used

- https://doc.rust-lang.org/style-guide/
- https://rust-lang.github.io/rustfmt/
- https://github.com/rust-lang/rustfmt
- https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html

Additional references:

- https://doc.rust-lang.org/style-guide/items.html
- https://doc.rust-lang.org/style-guide/statements.html
- https://doc.rust-lang.org/style-guide/expressions.html
- https://doc.rust-lang.org/style-guide/types.html
- https://doc.rust-lang.org/style-guide/advice.html
- https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-formatting-fixes.html
- https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-version-sorting.html
- https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-raw-identifier-sorting.html
- https://rust-lang.github.io/rfcs/3338-style-evolution.html
- https://rust-lang.github.io/rustfmt/?version=v1.8.0&search=#Configuration
- https://github.com/rust-lang/rustfmt/blob/master/Configurations.md
- https://github.com/rust-lang/rustfmt/blob/master/src/config.rs
- https://rust-lang.github.io/api-guidelines/naming.html
- https://rust-analyzer.github.io/manual.html#rustfmt
- https://doc.rust-lang.org/cargo/commands/cargo-fmt.html

## Core guidance

[rustfmt](https://rust-lang.github.io/rustfmt/), invoked via `cargo fmt`, is the canonical formatter for Rust. The [Rust Style Guide](https://doc.rust-lang.org/style-guide/) is the normative reference that defines the default Rust style. As the style guide states, it "defines the default Rust style, and *recommends* that developers and tools follow the default style"; "Tools such as `rustfmt` use the style guide as a reference for the default style"; and differences between the guide and rustfmt "may represent a bug in rustfmt, or a bug in the style guide". In practice, rustfmt output is the canonical formatting for this repo.

Style changes are versioned by *style editions*, independent of language editions. Rust 2015, Rust 2018, and Rust 2021 all share the same style edition. The 2024 style edition is distinct and is the default for `edition = "2024"` projects in Rust 1.85.0. Set both `edition` and `style_edition` explicitly in `rustfmt.toml` so editor format-on-save, local `cargo fmt`, and CI produce identical output.

Run `cargo fmt` before committing and enforce `cargo fmt --check` in CI. Avoid manual formatting that rustfmt would undo on the next run. Use `#[rustfmt::skip]` only rarely and with justification.

## Practical rules

1. **Run `cargo fmt` before opening a PR.** This is the baseline formatting command for the current crate/workspace.

2. **Enforce `cargo fmt --check` in CI.** Use `cargo fmt --all -- --check` or `cargo fmt --check` for the workspace. The check exits `0` if clean and exits `1` with a diff if there is drift.

3. **Commit a `rustfmt.toml` at the workspace root.** Even if it only pins `edition` and `style_edition`, the file makes formatting deterministic across machines and editors.

4. **Set both `edition` and `style_edition` explicitly.** They are independent knobs. The language `edition` lives in `Cargo.toml`; the formatting `style_edition` lives in `rustfmt.toml`. Pinning both prevents editor-on-save from diverging from `cargo fmt`.

5. **Prefer a stable-only `rustfmt.toml`.** Many commonly desired options (for example `imports_granularity`, `group_imports`, `trailing_comma`, `wrap_comments`, `format_code_in_doc_comments`, `format_strings`) are **nightly-only** and are silently ignored by stable rustfmt. If you use them, you must also use a nightly toolchain and set `unstable_features = true`.

6. **Use 4-space indentation and `max_width = 100` unless the repo decides otherwise.** Never use tabs. These are the rustfmt defaults and match the style guide.

7. **Use trailing commas in multi-line comma-separated lists.** The style guide says "use a trailing comma when followed by a newline". rustfmt applies this by default.

8. **Group imports as `std`/`core`/`alloc`, then external crates, then local `crate::`/`super::`/`self::`, separated by blank lines.** On nightly, `group_imports = "StdExternalCrate"` enforces this automatically. On stable, the grouping must be maintained by hand or with a nightly formatting job.

9. **Apply `#[rustfmt::skip]` only to small, justified constructs.** Matrices, alignment-sensitive tables, and generated-looking blocks are reasonable. Never apply it at module or crate level without an explicit repo policy.

10. **Do not disable rustfmt broadly.** Avoid `disable_all_formatting = true` or crate-level skips. Prefer configuring rustfmt or accepting its output.

11. **Distinguish stable from nightly options in documentation and CI.** If a `rustfmt.toml` contains nightly-only keys, label it clearly and run `cargo +nightly fmt --all -- --check` in CI.

12. **Use `#[rustfmt::skip::macros(name)]` or `skip_macro_invocations` sparingly** for macros whose internal layout is intentional (for example test matrices).

13. **Set `format_generated_files = false` if generated files carry an `@generated` marker** and should not be reformatted.

14. **Configure format-on-save to call rustfmt through rust-analyzer.** Set `editor.formatOnSave = true` in VS Code and choose `rust-analyzer` or `rustfmt` as the formatter. Ensure rust-analyzer uses the same rustfmt binary and configuration as CI.

15. **Prefer `style_edition` over the deprecated `version` key.** `version = "One"`/`"Two"` is a soft-deprecated alias for style edition. Do not use it in new configuration.

16. **Prefer `imports_granularity` over the deprecated `merge_imports` key.** `merge_imports = true` is equivalent to `imports_granularity = "Crate"`.

17. **Do not cite RFC 3306 or RFC 3390 for style edition.** The style-edition mechanism is defined by [RFC 3338 — "Style Evolution"](https://rust-lang.github.io/rfcs/3338-style-evolution.html).

18. **Run `cargo fmt` after edition migrations.** Changing `edition` or `style_edition` can change rustfmt output. Reformat and review the diff as a separate commit.

## Review checklist

1. [ ] Code is formatted with `cargo fmt` using the committed `rustfmt.toml`.
2. [ ] CI enforces `cargo fmt --check` (or `cargo +nightly fmt --all -- --check` if nightly options are used).
3. [ ] `rustfmt.toml` sets `edition` and `style_edition` explicitly.
4. [ ] Any nightly-only `rustfmt.toml` options are labeled and paired with a nightly CI job.
5. [ ] `#[rustfmt::skip]` is rare, scoped, and justified with a comment.
6. [ ] Import grouping follows the configured style (`std`/`core`/`alloc`, external, local).
7. [ ] Line width stays within the configured `max_width`.
8. [ ] No deprecated rustfmt keys (`version`, `merge_imports`) are used.
9. [ ] No tabs or mixed indentation are present.
10. [ ] Generated files with `@generated` markers are excluded from formatting if configured.
11. [ ] rust-analyzer format-on-save settings match the repo's rustfmt configuration.
12. [ ] The chosen `style_edition` is documented and intentional.

## Implementation checklist

1. [ ] Create or update `rustfmt.toml` at the workspace root.
2. [ ] Set `edition` in `rustfmt.toml` to match the crate language edition.
3. [ ] Set `style_edition` in `rustfmt.toml` explicitly (for example `"2024"` or `"2021"`).
4. [ ] Choose stable-only options unless the project accepts a nightly toolchain dependency.
5. [ ] Add a stable CI step: `cargo fmt --all -- --check`.
6. [ ] Add a nightly CI step only if nightly options are required: `cargo +nightly fmt --all -- --check`.
7. [ ] Run `cargo fmt` once and commit the resulting diff as a formatting-only commit.
8. [ ] Document any non-default options with comments in `rustfmt.toml`.
9. [ ] Configure editor format-on-save to use the same rustfmt binary/configuration.
10. [ ] Verify that `cargo fmt -- --files-with-diff` reports no unexpected files.
11. [ ] Remove any crate-level or module-level `#[rustfmt::skip]` unless explicitly approved.
12. [ ] Update this guidance if the repo adopts nightly-only formatting options.

## Validation hooks

```bash
# Stable formatting check for the whole workspace (preferred CI command)
cargo fmt --all -- --check

# Stable formatting check (shorthand; requires cargo-fmt >= 1.4.38, Oct 2021)
cargo fmt --check

# Nightly-only check when rustfmt.toml contains unstable options
cargo +nightly fmt --all -- --check

# Show the effective configuration for a file
rustfmt --print-config current src/lib.rs

# List files that differ from the configured format
cargo fmt -- --files-with-diff

# Format a single file in place (useful for pre-commit hooks)
rustfmt --edition 2024 --style-edition 2024 src/lib.rs

# Format stdin and emit to stdout
rustfmt --edition 2024

# Check formatting via rustfmt directly (legacy -- passthrough)
cargo fmt -- --check
```

## Examples

### 1. Stable-only `rustfmt.toml`

This configuration works with plain `cargo fmt` on a stable toolchain. No nightly options are present.

```toml
# Stable rustfmt configuration.
# Works with: cargo fmt --all -- --check
# See https://rust-lang.github.io/rustfmt/
edition = "2024"
style_edition = "2024"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
match_arm_leading_pipes = "Never"
match_block_trailing_comma = false
merge_derives = true
use_try_shorthand = false
use_field_init_shorthand = false
force_explicit_abi = true
hex_literal_case = "Preserve"
fn_params_layout = "Tall"
```

### 2. Nightly `rustfmt.toml` (requires nightly + `unstable_features = true`)

This configuration uses nightly-only options. On a stable toolchain these options are silently dropped.

```toml
# NIGHTLY-ONLY rustfmt configuration.
# Requires: rustfmt nightly + cargo +nightly fmt --all -- --check
# See https://rust-lang.github.io/rustfmt/
unstable_features = true

edition = "2024"
style_edition = "2024"
max_width = 100
tab_spaces = 4

imports_granularity = "Crate"
group_imports = "StdExternalCrate"
imports_layout = "Mixed"
wrap_comments = true
comment_width = 80
format_code_in_doc_comments = true
doc_comment_code_block_width = 100
format_strings = false
trailing_comma = "Vertical"
blank_lines_upper_bound = 1
blank_lines_lower_bound = 0
```

### 3. Import grouping before/after

Before (ungrouped):

```rust
use crate::error::Error;
use std::collections::HashMap;
use serde::Deserialize;
use std::fmt;
use crate::config::Config;
use tokio::sync::mpsc;
```

After nightly `group_imports = "StdExternalCrate"` + `imports_granularity = "Crate"`:

```rust
use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::error::Error;
```

### 4. Long expression / call-chain break

```rust
// One line if it fits
let ok = items.iter().map(|x| x.id).collect::<Vec<_>>();

// Broken before each `.` when it does not fit
let ids = items
    .iter()
    .filter(|item| item.active)
    .map(|item| item.id)
    .collect::<Vec<_>>();
```

### 5. Match arms

```rust
match status {
    Status::Ok => process(),
    Status::Err(e) if e.retryable() => {
        log::warn!("retrying: {}", e);
        retry()
    }
    Status::Err(e) => {
        log::error!("failed: {}", e);
        Err(e.into())
    }
}
```

### 6. Closures

```rust
// Short closure: no braces
let doubled: Vec<i32> = values.iter().map(|x| x * 2).collect();

// Multi-line body: braces
let mapped: Vec<i32> = values.iter().map(|x| {
    let y = x * 2;
    y + 1
}).collect();

// 2024 style edition forces a block around a single `loop` body
let run = || {
    loop {
        tick();
    }
};
```

### 7. 2024 style edition before/afters

Nested tuple indexing (stray space removed):

```rust
// 2015/2018/2021 style edition
((1,),).0 .0

// 2024 style edition
((1,),).0.0
```

`overflow_delimited_expr` flips to `true` by default in 2024, so struct literals "hug":

```rust
// 2015/2018/2021 style edition (overflow_delimited_expr = false)
do_thing(
    x,
    Bar {
        x: value,
        y: value2,
    },
);

// 2024 style edition (overflow_delimited_expr = true)
do_thing(x, Bar { x: value, y: value2 });
```

Version sorting for imports (`NonZeroU8` before `NonZeroU16` before `NonZeroU32`):

```rust
// 2015/2018/2021 style edition (ASCIIbetical)
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroU8;

// 2024 style edition (version sorting)
use std::num::NonZeroU8;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
```

Raw identifier sorting (`r#` ignored):

```rust
// 2015/2018/2021 style edition
use crate::r#async;
use crate::await;

// 2024 style edition
use crate::await;
use crate::r#async;
```

### 8. `#[rustfmt::skip]` matrix

```rust
// A small, justified skip: the visual rows are meaningful.
#[rustfmt::skip]
const IDENTITY: [[f64; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
```

### 9. CI YAML

GitHub Actions:

```yaml
jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      - name: Check formatting
        run: cargo fmt --all -- --check

  fmt-nightly:
    runs-on: ubuntu-latest
    if: false  # enable only if rustfmt.toml uses nightly-only options
    steps:
      - uses: actions/checkout@v4
      - name: Install nightly Rust
        uses: dtolnay/rust-action@nightly
      - name: Check formatting with nightly options
        run: cargo +nightly fmt --all -- --check
```

GitLab CI:

```yaml
fmt:
  stage: check
  image: rust:1.85
  script:
    - cargo fmt --all -- --check
  cache:
    key: ${CI_COMMIT_REF_SLUG}
    paths:
      - target/
      - ~/.cargo/registry/
```

### 10. rust-analyzer / VS Code settings

```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.rustfmt.extraArgs": [],
  "rust-analyzer.rustfmt.overrideCommand": null,
  "rust-analyzer.rustfmt.rangeFormatting.enable": false
}
```

If you enable `rust-analyzer.rustfmt.rangeFormatting.enable`, rust-analyzer uses rustfmt's nightly `--file-lines` option, which requires a nightly toolchain.

## Common mistakes

### 1. Using nightly-only options on a stable toolchain

```toml
# BAD: stable rustfmt silently drops these options
edition = "2024"
style_edition = "2024"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
trailing_comma = "Vertical"
wrap_comments = true
```

```toml
# GOOD (stable): only stable options
edition = "2024"
style_edition = "2024"
max_width = 100
tab_spaces = 4
reorder_imports = true
reorder_modules = true
```

```toml
# GOOD (nightly): label the file and enable the master gate
# NIGHTLY-ONLY: run with cargo +nightly fmt --all -- --check
unstable_features = true
edition = "2024"
style_edition = "2024"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
trailing_comma = "Vertical"
wrap_comments = true
```

### 2. Assuming `cargo fmt` honors `imports_granularity` without nightly

```bash
# BAD: this check will pass even if imports are unmerged,
# because stable rustfmt silently ignores imports_granularity.
cargo fmt --all -- --check
```

```bash
# GOOD: use nightly for nightly-only options
cargo +nightly fmt --all -- --check
```

### 3. Crate-level `#[rustfmt::skip]`

```rust
// BAD: disables formatting for the entire crate
#![rustfmt::skip]
```

```rust
// GOOD: skip only a small, justified construct
#[rustfmt::skip]
const LOOKUP: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03,
    0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0A, 0x0B,
    0x0C, 0x0D, 0x0E, 0x0F,
];
```

### 4. Not pinning `style_edition`, so editor-on-save diverges from CI

```toml
# BAD: editor uses its own default, which may differ from cargo fmt
max_width = 100
tab_spaces = 4
```

```toml
# GOOD: explicit edition and style_edition
edition = "2024"
style_edition = "2024"
max_width = 100
tab_spaces = 4
```

### 5. Running `cargo fmt` without a CI `--check`

```yaml
# BAD: CI never checks formatting, so drift accumulates
- name: Build
  run: cargo build --all-targets
```

```yaml
# GOOD: formatting is a blocking gate
- name: Check formatting
  run: cargo fmt --all -- --check
- name: Build
  run: cargo build --all-targets
```

### 6. Using deprecated `version` or `merge_imports`

```toml
# BAD (deprecated)
version = "Two"
merge_imports = true
```

```toml
# GOOD
style_edition = "2024"
imports_granularity = "Crate"
```

### 7. Mixing tabs and spaces

```rust
// BAD: hard_tabs = false will convert these, but the source is inconsistent
fn main() {
\tprintln!("tabs and spaces");
}
```

```rust
// GOOD
fn main() {
    println!("spaces only");
}
```

### 8. Citing nonexistent RFC 3306 or RFC 3390

```markdown
<!-- BAD -->
The style edition mechanism was introduced in RFC 3306 / RFC 3390.

<!-- GOOD -->
The style edition mechanism was introduced in [RFC 3338 — "Style Evolution"](https://rust-lang.github.io/rfcs/3338-style-evolution.html).
```

### 9. Forgetting that `style_edition = "2027"` is nightly-only

```toml
# BAD on stable: 2027 is marked #[unstable_variant] and requires nightly
style_edition = "2027"
```

```toml
# GOOD on stable
style_edition = "2024"
```

## Rust Style Guide — detailed reference

### What the style guide is

The [Rust Style Guide](https://doc.rust-lang.org/style-guide/) is the official prose reference for Rust formatting. It "defines the default Rust style, and recommends that developers and tools follow the default style". The guide's chapters are: Introduction, Items, Statements, Expressions, Types and Bounds, "Other style advice" (`advice.md`), Cargo.toml conventions, Guiding principles, Rust style editions, and Nightly-only syntax. There is no separate `naming.md` or `attributes.md` chapter. Richer naming conventions (for example `as_`/`to_`/`into_` prefixes, `new`, getters/setters, single-letter type parameters, and lifetime names such as `'a`) come from the [rustc API guidelines](https://rust-lang.github.io/api-guidelines/naming.html), not the style guide proper.

### Status and relationship to rustfmt

The style guide is normative but aspirational for users who do not use rustfmt. rustfmt is the practical authority: it is the reference implementation of the guide, and its output is what most projects enforce in CI. The style guide acknowledges that differences between rustfmt and the guide "may represent a bug in rustfmt, or a bug in the style guide". Style changes are gated behind style editions, not arbitrary rustfmt releases. Rust 2015, Rust 2018, and Rust 2021 share the same style edition; Rust 2024 introduces a new style edition.

### Basic formatting rules

From the [style guide introduction](https://doc.rust-lang.org/style-guide/):

- Use 4 spaces for indentation. Do not use tabs.
- Maximum line width is 100 characters.
- Use block indent style, not visual indent style. For example:

```rust
// Block indent (preferred)
a_function_call(
    foo,
    bar,
);

// Visual indent (not preferred)
a_function_call(foo,
                bar);
```

- Use Unix-style line endings (LF). The rustfmt default `newline_style = "Auto"` preserves existing line endings; set it to `"Unix"` to force LF.
- Do not leave trailing whitespace.
- Use a trailing comma when a comma-separated list is followed by a newline.
- Separate items and statements by zero or one blank line.
- Version sorting is the default for imports in the 2024 style edition.

### Items

Per the [Items chapter](https://doc.rust-lang.org/style-guide/items.html):

- `extern crate` statements come first, sorted alphabetically.
- `use` and `mod foo;` declarations come before other items; imports precede modules.
- Within a group, imports are sorted. In 2024 this is version-sorted; `self` and `super` come first.
- An import group is a sequence of `use` lines with no blank line between them. Blank lines or other items separate groups. Groups are not merged or reordered. `#[macro_use]` starts a new group.
- Function definitions are formatted so that `fn name` is greppable. A multi-line signature breaks after `(` and before `)`, with one argument per line and a trailing comma.
- Structs/unions: name on the same line as `struct`/`union`, opening brace on the same line, fields block-indented with a trailing comma, closing brace on its own line. Prefer unit struct `struct Foo;` over an empty struct.
- Tuple structs stay on one line if they fit.
- Enums: each variant on its own line, block-indented. A small struct variant may be one line, but if any struct variant is multi-line, all such variants must be multi-line.
- Traits and impls: items inside are block-indented. Empty trait/impl: `trait Foo {}` / `impl Foo {}`. Non-inherent impl breaks before `for`.
- Always specify the ABI on `extern` items: `extern "C" fn`.
- Only one `derive` attribute. rustfmt merges multiple derives.

### Statements

Per the [Statements chapter](https://doc.rust-lang.org/style-guide/statements.html):

- `let` spacing: `let pattern: Type = expr;`.
- `let` line-breaking: split after `=`, then after `:` if still too long.
- `let ... else`: keep on one line only if the pattern is short and the `else` block is a single expression with no comments:

```rust
let Some(1) = opt else { return };
```

Otherwise put `else {` on the same line as the initializer and break before `}`:

```rust
let Some(value) = compute() else {
    log::error!("compute failed");
    return;
};
```

- Macros in statement position keep their invocation form: `name!(...);`.
- Expressions in statement position end with `;` unless the expression ends in a block.

### Expressions

Per the [Expressions chapter](https://doc.rust-lang.org/style-guide/expressions.html):

- Blocks are formatted with opening and closing braces on their own lines, contents block-indented.
- Closures: prefer no braces for a single expression; add braces for a return type, multiple statements, comments, or control-flow bodies. Use `|args| expr` with a space after the pipes. The 2024 style edition forces a block around a single `loop` body.
- Struct literals: keep on one line with no trailing comma if small; otherwise put each field on its own line with a trailing comma.
- Tuple/unit/array/enum literals follow the same comma rules.
- Indexing: no spaces, never break between the target and `[`.
- Unary operators: no space before the operand, but a space after `&mut`.
- Binary operators: spaces around the operator, including `=` and `+=`. Assignment operators break AFTER the operator; other operators break BEFORE the operator. Prefer dereferencing over taking a reference.
- `as` casts: formatted like binary operators; break BEFORE `as`. Chained casts have a special-case layout.
- Function calls: no space before `(`. Nullary calls `func()` are never broken. Multi-line calls put each argument on its own line with a trailing comma.
- Method calls: no spaces around `.`.
- Chains: one line if small; otherwise each element on its own line, breaking BEFORE `.` and AFTER `?`. rustfmt uses a combine-if-fits rule.
- `match`: break after `{` and before `}`; arms are block-indented. Trailing comma after an arm expression unless the body is a block. Never start a pattern with `|`. Or-patterns break before `|`. Guards put `if` on its own line when the body is a block.
- Ranges: no spaces (`0..10`); break before the operator.
- Single-line `if`/`else` is allowed only in expression context when small: `let y = if x { 0 } else { 1 };`.

### Types and bounds

Per the [Types and Bounds chapter](https://doc.rust-lang.org/style-guide/types.html):

- No spaces around `<>`/`::`/`[]`/`()`. Space after each comma.
- Slices `[T]`, arrays `[T; expr]`, pointers `*const T` / `*mut T`, references `&'a T` / `&T` / `&mut T`, function types `fn(...) -> T`, tuples `(A, B, C)`, paths `Foo::Bar<T, U>`, and bounds `T + T` / `impl T + T` follow these rules.
- Line breaks prefer the outermost scope.
- `+` bounds break before each `+`.
- Generics (also covered in Items): no space before/after `<>`, space after comma, no trailing comma on a single line. Multi-line generics break after `<` and before `>`, with a trailing comma.
- `where` clauses: `where` on the same line as the preceding `>` / `)` if it fits; otherwise on a new line. Each clause is on its own line, block-indented, with a trailing comma unless terminated by `;`. Prefer single-letter generic parameter names.

### Other style advice (naming)

Per the [Other style advice chapter](https://doc.rust-lang.org/style-guide/advice.html):

- Types and enum variants: `UpperCamelCase`.
- Struct fields, functions/methods, local variables, macros: `snake_case`.
- Constants (`const` and immutable `static`): `SCREAMING_SNAKE_CASE`.
- Reserved words used as identifiers: use a raw identifier (`r#crate`) or a trailing underscore (`crate_`). Never misspell the word.
- Avoid `#[path]` on modules.

For richer naming conventions — crate names, type parameters (`T`), lifetimes (`'a`), acronyms (`Uuid`), `as_`/`to_`/`into_` conversions, `new`, getters/setters, and `is_` predicates — see the [rustc API guidelines](https://rust-lang.github.io/api-guidelines/naming.html).

### Comments

Per the [Introduction §Comments and §Doc comments](https://doc.rust-lang.org/style-guide/):

- Prefer line comments `//` over block comments `/* */`.
- Use a single space after the comment sigil.
- Prefer comments on their own line; trailing comments are okay with a single space before them.
- Use complete sentences, capitalized and punctuated.
- Comment lines are ideally capped at 80 characters (or `max_width`, whichever is smaller).
- Prefer outer doc comments `///` over `/** */`.
- Use `///` and `//!` for outer and inner doc comments; use inner forms `//!` / `/*! */` only for module/crate-level docs.
- Doc comments precede attributes.

These are recommendations. A mechanical formatter may skip or only partially reformat comments.

### Attributes

Per the [Introduction §Attributes](https://doc.rust-lang.org/style-guide/):

- Each attribute on its own line at the item's indentation.
- Inner attributes (`#!`) are indented to the inside of the item.
- Prefer outer attributes (`#[...]`) where possible.
- Attribute argument lists are formatted like function calls: `#[foo(a, b)]`.
- Single space around `=` in attributes: `#[foo = 42]`.
- Only one `derive` attribute. When merging multiple derives, preserve their ordering.

## rustfmt CLI and configuration — detailed reference

### CLI overview

`cargo fmt` reads `Cargo.toml` and `rustfmt.toml` to format the workspace. `rustfmt <file>` formats a single file in place, or reads stdin if no file is given. The [`cargo fmt` man page](https://doc.rust-lang.org/cargo/commands/cargo-fmt.html) and [rustfmt repository](https://github.com/rust-lang/rustfmt) are the authoritative references.

Common flags:

| Command | Description |
|---------|-------------|
| `cargo fmt` | Format the current crate via cargo metadata. |
| `cargo fmt --all` | Format the entire workspace. |
| `cargo fmt --check` | Verify formatting; exits `0` if clean, `1` + diff if drift (added in rustfmt 1.4.38, Oct 2021). |
| `cargo fmt -- --check` | Legacy equivalent; args after `--` are passed to rustfmt. |
| `cargo fmt -p <pkg>` | Format a specific package. |
| `cargo fmt --manifest-path <path>` | Use a specific `Cargo.toml`. |
| `cargo fmt +nightly` | Run with the nightly toolchain. |
| `rustfmt <file>` | Format a single file in place. |
| `rustfmt --config k=v,k=v` | Override config options for this run. |
| `rustfmt --config-path <path>` | Use a specific `rustfmt.toml`. |
| `rustfmt --edition 2015\|2018\|2021\|2024` | Set the edition for this run. |
| `rustfmt --style-edition 2015\|2018\|2021\|2024` | Set the style edition for this run (stable since rustfmt 1.8.0). |
| `rustfmt --print-config default\|minimal\|current` | Print the default/minimal/current config. |
| `rustfmt -l` / `--files-with-diff` | List files that differ from the configured format. |
| `rustfmt --emit files\|stdout\|diff` | Stable emit modes. |
| `rustfmt --emit coverage\|checkstyle\|json` | Nightly-only emit modes. |
| `rustfmt --unstable-features` / `-Z unstable-options` | Nightly-only. |

The `RUSTFMT` environment variable selects a custom rustfmt binary.

### rustfmt skip attributes and config

- `#[rustfmt::skip]` skips the attached item.
- `#![rustfmt::skip]` skips the entire file/module.
- `#[rustfmt::skip::macros(name)]` skips formatting of the named macro's invocations.
- `#[rustfmt::skip::attributes(name)]` skips formatting of the named attribute.
- `skip_macro_invocations = ["name"]` or `["*"]` in `rustfmt.toml` skips macro invocations globally.
- `format_generated_files = false` skips files that contain an `@generated` marker within the first `generated_marker_line_search_limit` lines.

### Stable `rustfmt.toml` options

These options work with plain `cargo fmt` on a stable toolchain. Defaults and allowed values are current as of rustfmt 1.8.0.

| Name | Default | Allowed values | Stable? | Source |
|------|---------|----------------|---------|--------|
| `max_width` | `100` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `hard_tabs` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `tab_spaces` | `4` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `newline_style` | `"Auto"` | `Auto` / `Unix` / `Windows` / `Native` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `use_small_heuristics` | `"Default"` | `Default` / `Off` / `Max` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `fn_call_width` | `60` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `attr_fn_like_width` | `70` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `struct_lit_width` | `18` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `struct_variant_width` | `35` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `array_width` | `60` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `chain_width` | `60` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `single_line_if_else_max_width` | `50` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `reorder_imports` | `true` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `reorder_modules` | `true` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `remove_nested_parens` | `true` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `short_array_element_width_threshold` | `10` | integer | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `fn_params_layout` | `"Tall"` | `Compressed` / `Tall` / `Vertical` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `match_arm_leading_pipes` | `"Never"` | `Always` / `Never` / `Preserve` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `match_block_trailing_comma` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `merge_derives` | `true` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `use_try_shorthand` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `use_field_init_shorthand` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `force_explicit_abi` | `true` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `hex_literal_case` | `"Preserve"` | `Preserve` / `Upper` / `Lower` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `disable_all_formatting` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `edition` | `"2015"` | `2015` / `2018` / `2021` / `2024` | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `style_edition` | `"2015"` | `2015` / `2018` / `2021` / `2024` (`2027` is nightly-only) | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `make_backup` | `false` | bool | stable | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |

`use_small_heuristics` controls the derived width heuristics: `Default` scales the widths proportionally to `max_width` (relative to the default `max_width` of 100); `Max` sets all derived widths equal to `max_width`; `Off` forces every construct to break across multiple lines.

### Unstable / nightly-only `rustfmt.toml` options

These options require `unstable_features = true` in `rustfmt.toml` and `cargo +nightly fmt`. On a stable toolchain they are silently dropped. Tracking issues are from the rustfmt repository.

| Name | Default | Allowed values | Tracking | Source |
|------|---------|----------------|----------|--------|
| `imports_granularity` | `"Preserve"` | `Preserve` / `Crate` / `Module` / `Item` / `One` | [#4991](https://github.com/rust-lang/rustfmt/issues/4991) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `group_imports` | `"Preserve"` | `Preserve` / `StdExternalCrate` / `One` | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `imports_layout` | `"Mixed"` | `Mixed` / `Horizontal` / `HorizontalVertical` / `Vertical` | [#3361](https://github.com/rust-lang/rustfmt/issues/3361) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `imports_indent` | `"Block"` | — | [#3360](https://github.com/rust-lang/rustfmt/issues/3360) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `wrap_comments` | `false` | bool | [#3349](https://github.com/rust-lang/rustfmt/issues/3349) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `format_code_in_doc_comments` | `false` | bool | [#3348](https://github.com/rust-lang/rustfmt/issues/3348) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `doc_comment_code_block_width` | `100` | integer | [#5359](https://github.com/rust-lang/rustfmt/issues/5359) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `comment_width` | `80` | integer | [#3349](https://github.com/rust-lang/rustfmt/issues/3349) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `normalize_comments` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `normalize_doc_attributes` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `format_strings` | `false` | bool | [#3353](https://github.com/rust-lang/rustfmt/issues/3353) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `format_macro_matchers` | `false` | bool | [#3354](https://github.com/rust-lang/rustfmt/issues/3354) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `format_macro_bodies` | `true` | bool | [#3355](https://github.com/rust-lang/rustfmt/issues/3355) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `skip_macro_invocations` | `[]` | array of strings | [#5346](https://github.com/rust-lang/rustfmt/issues/5346) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `indent_style` | `"Block"` | `Block` / `Visual` | [#3346](https://github.com/rust-lang/rustfmt/issues/3346) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `binop_separator` | `"Front"` | `Front` / `Back` | [#3368](https://github.com/rust-lang/rustfmt/issues/3368) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `brace_style` | `"SameLineWhere"` | `AlwaysNextLine` / `PreferSameLine` / `SameLineWhere` | [#3376](https://github.com/rust-lang/rustfmt/issues/3376) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `control_brace_style` | `"AlwaysSameLine"` | `AlwaysNextLine` / `AlwaysSameLine` / `ClosingNextLine` | [#3377](https://github.com/rust-lang/rustfmt/issues/3377) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `trailing_comma` | `"Vertical"` | `Always` / `Never` / `Vertical` | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `trailing_semicolon` | `true` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `space_before_colon` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `space_after_colon` | `true` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `spaces_around_ranges` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `type_punctuation_density` | `"Wide"` | — | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `empty_item_single_line` | `true` | bool | [#3356](https://github.com/rust-lang/rustfmt/issues/3356) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `fn_single_line` | `false` | bool | [#3358](https://github.com/rust-lang/rustfmt/issues/3358) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `where_single_line` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `struct_lit_single_line` | `true` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `single_line_let_else_max_width` | `50` | integer | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `match_arm_blocks` | `true` | bool | [#3373](https://github.com/rust-lang/rustfmt/issues/3373) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `match_arm_indent` | `true` | bool | [#6533](https://github.com/rust-lang/rustfmt/issues/6533) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `force_multiline_blocks` | `false` | bool | [#3374](https://github.com/rust-lang/rustfmt/issues/3374) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `combine_control_expr` | `true` | bool | [#3369](https://github.com/rust-lang/rustfmt/issues/3369) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `condense_wildcard_suffixes` | `false` | bool | [#3384](https://github.com/rust-lang/rustfmt/issues/3384) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `overflow_delimited_expr` | `false` (`true` when `style_edition = "2024"`) | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `struct_field_align_threshold` | `0` | integer | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `enum_discrim_align_threshold` | `0` | integer | [#3372](https://github.com/rust-lang/rustfmt/issues/3372) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `inline_attribute_width` | `0` | integer | [#3343](https://github.com/rust-lang/rustfmt/issues/3343) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `blank_lines_upper_bound` | `1` | integer | [#3381](https://github.com/rust-lang/rustfmt/issues/3381) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `blank_lines_lower_bound` | `0` | integer | [#3382](https://github.com/rust-lang/rustfmt/issues/3382) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `reorder_impl_items` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `format_generated_files` | `true` | bool | [#5080](https://github.com/rust-lang/rustfmt/issues/5080) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `generated_marker_line_search_limit` | `5` | integer | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `float_literal_trailing_zero` | `"Preserve"` | — | [#6471](https://github.com/rust-lang/rustfmt/issues/6471) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `color` | `"Auto"` | — | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `required_version` | — | version string | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `unstable_features` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `skip_children` | `false` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `show_parse_errors` | `true` | bool | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `hide_parse_errors` | `false` | bool (legacy) | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `error_on_line_overflow` | `false` | bool | [#3391](https://github.com/rust-lang/rustfmt/issues/3391) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `error_on_unformatted` | `false` | bool | [#3392](https://github.com/rust-lang/rustfmt/issues/3392) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `ignore` | `[]` | array of strings/globs | [#3395](https://github.com/rust-lang/rustfmt/issues/3395) | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |
| `version` | `"One"` | `One` / `Two` | — | [rustfmt docs](https://rust-lang.github.io/rustfmt/) |

`version` is soft-deprecated since rustfmt 1.8.0; it is an alias for `style_edition` (`One` maps to 2015/2018/2021, `Two` maps to 2024). Do not use `version` in new configuration. `merge_imports` is a deprecated alias for `imports_granularity = "Crate"`.

### The nightly-only mechanism

To use any unstable option:

1. Use a nightly toolchain.
2. Set `unstable_features = true` in `rustfmt.toml`.
3. Run `cargo +nightly fmt --all -- --check`.

Stable rustfmt silently ignores unstable keys and their values, so a CI job that runs `cargo fmt --check` on a stable toolchain will pass even if the `rustfmt.toml` contains nightly-only options. Always pair nightly-only config with a nightly CI job. The value `style_edition = "2027"` is also nightly-only (marked `#[unstable_variant]` in rustfmt).

### Style edition concept and 2024 changes

Style editions are defined by [RFC 3338 — "Style Evolution"](https://rust-lang.github.io/rfcs/3338-style-evolution.html). They allow rustfmt to change default formatting without affecting existing code. The 2024 style edition became the default for `edition = "2024"` projects in Rust 1.85.0 and was stabilized in rustfmt 1.8.0 (released 2024-09-20). `style_edition = "2027"` is the only nightly-only value.

The [Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html) documents the 2024 style edition changes. Important: the 2024 style edition does **not** change the import grouping order; it only changes the sorting algorithm within groups. Concrete 2024 changes include:

- Nested tuple indexing: `((1,),).0 .0` → `((1,),).0.0`.
- Match block arms: trailing `;` added after `return`/`break`/`continue` in block arms; fixed indentation when `=>` appears in a comment; correct indentation of multiple inner attributes `#![attr]` in a match.
- `let`-else with an attribute: an attributed `let` is kept on one logical line when possible.
- Long array/slice destructuring patterns are wrapped one element per line with a trailing comma.
- Trailing-comment alignment is removed; unrelated trailing comments are no longer vertically aligned.
- String literals embedded in comments are left untouched.
- `overflow_delimited_expr` defaults to `true`: struct literals "hug" their call arguments.
- Closures with a single `loop` body are forced into block form: `|| loop { ... }` → `|| { loop { ... } }`.
- Raw identifier sorting ignores the `r#` prefix.
- Version sorting replaces ASCIIbetical sorting for imports.
- Generics indentation in `impl` blocks, complex `fn` return-type chains, empty lines in `where` clauses, `format_macro_matchers` off-by-one, last-expression `{ T }` collapsing, and long strings no longer blocking expression formatting.

Pin `style_edition` explicitly in `rustfmt.toml` so that editor format-on-save and CI use the same rules.

## Strict vs contextual guidance

### Strict guidance

- `cargo fmt --all -- --check` must pass in CI.
- Indentation is 4 spaces. `hard_tabs = false`.
- `rustfmt.toml` is committed at the workspace root.
- `edition` and `style_edition` are set explicitly in `rustfmt.toml`.
- `#[rustfmt::skip]` is rare, scoped, and justified with a comment.
- No crate-level or module-level `#[rustfmt::skip]` without an explicit repo policy.
- No deprecated rustfmt keys (`version`, `merge_imports`) in new configuration.

### Common convention

- `max_width = 100` unless the project documents a different value.
- `style_edition = "2024"` for new projects; `"2021"` for projects that have not migrated.
- `reorder_imports = true` and `reorder_modules = true`.
- `merge_derives = true`.
- `newline_style = "Unix"` to force LF line endings.
- Generated files with `@generated` markers are excluded via `format_generated_files = false`.
- Editor format-on-save is enabled and configured to match CI.

### Contextual tradeoffs

- **`max_width`**: 80, 100, or 120 depending on project conventions and review tooling. The style guide default is 100.
- **`style_edition`**: 2015/2018/2021 share one style edition; 2024 introduces new formatting. Match the language edition or pin separately based on team readiness.
- **Stable-only vs nightly rustfmt in CI**: Nightly options (`imports_granularity`, `group_imports`, `trailing_comma`, `wrap_comments`, etc.) produce nicer output but require a nightly toolchain in CI. Many teams prefer stable-only to avoid toolchain complexity.
- **`imports_granularity`**: `Preserve`, `Crate`, `Module`, `Item`, or `One`. `Crate` is a common default for nightly users.
- **`trailing_comma`**: `"Vertical"` encodes the style guide rule; `"Always"` or `"Never"` are stricter but nightly-only.
- **`format_code_in_doc_comments`**: keeps doctests tidy but can reformat hand-tuned examples. Decide per project.
- **`wrap_comments` / `comment_width`**: useful for long prose comments, but requires nightly.

### Repo-policy decision points

- Choose and document `max_width`.
- Choose and document `style_edition`.
- Decide whether CI uses stable-only or nightly rustfmt.
- Decide whether to use nightly-only options such as `imports_granularity`, `group_imports`, `wrap_comments`.
- Decide whether crate-level or module-level `#[rustfmt::skip]` is ever allowed.
- Decide whether format-on-save is required, recommended, or optional.
- Decide whether to enforce `newline_style = "Unix"`.

## Policy decisions for individual repos

Record repo-specific formatting policy in this file's repo-specific appendix or in a `FORMATTING.md` at the repo root. At minimum, capture:

| Decision | Options | Note |
|----------|---------|------|
| `max_width` | `80` / `100` / `120` / other | Default is 100. |
| `style_edition` | `"2015"` / `"2018"` / `"2021"` / `"2024"` | Pin explicitly. `"2027"` is nightly-only. |
| Stable vs nightly rustfmt in CI | stable only / nightly required | Nightly required for `imports_granularity`, `group_imports`, `trailing_comma`, `wrap_comments`, etc. |
| `imports_granularity` | `"Preserve"` / `"Crate"` / `"Module"` / `"Item"` / `"One"` | Nightly-only. |
| `group_imports` | `"Preserve"` / `"StdExternalCrate"` / `"One"` | Nightly-only. |
| `wrap_comments` | `true` / `false` | Nightly-only. |
| Crate-level `#[rustfmt::skip]` | allowed / forbidden | Forbidden by default. |
| Format-on-save | required / recommended / optional | Typically required with rust-analyzer. |

## Related docs

- `docs/rust/editions-tooling.md` — Rust editions, `rust-toolchain.toml`, and the relationship between language edition and rustfmt style edition.
- `docs/rust/lints-clippy.md` — Lint configuration, CI integration, and repo policy tables.
- `docs/rust/documentation-guidelines.md` — Doc comment conventions and doctest practices.
- `docs/rust/api-design.md` — API design guidance, including naming conventions from the API guidelines.
- `docs/rust/modules-visibility.md` — Module structure and visibility, which interacts with import ordering.

## Related skills

No repo-specific skills for this topic.
