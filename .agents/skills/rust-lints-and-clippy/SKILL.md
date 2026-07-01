---
name: rust-lints-and-clippy
description: |
  Operational guide for Rust lint policy: Clippy/rustc lints via the
  `[lints]` table in Cargo.toml, workspace inheritance, clippy.toml,
  rustfmt.toml basics, and CI gates. Load when establishing lint policy,
  configuring clippy/rustc lints, reviewing clippy or rustfmt output, adding
  `[lints]` tables, or wiring `cargo clippy` / `cargo fmt --check` into CI.
  Links back to docs/rust/lints-clippy.md and docs/rust/style-formatting.md.
---

# Rust Lints, Clippy, and Formatting Gates

Linting (Clippy/rustc) and formatting (rustfmt) are distinct. Configure lints
in `[lints]` tables; formatting in `rustfmt.toml`. Full docs:
[`docs/rust/lints-clippy.md`](../../../docs/rust/lints-clippy.md),
[`docs/rust/style-formatting.md`](../../../docs/rust/style-formatting.md).

## Triggers

- Setting up or revising a Rust lint policy (`[lints]` table, `clippy.toml`).
- Configuring Clippy groups (`all`/`pedantic`/`nursery`/`cargo`/`restriction`).
- Reviewing clippy/rustc lint output or triaging CI lint failures.
- Adding `cargo clippy -- -D warnings` or `cargo fmt --check` to CI.
- Deciding lint levels (`allow`/`warn`/`deny`/`forbid`/`expect`) for a crate.
- Wiring workspace lint inheritance or reviewing `#[allow]`/`#[expect]` attrs.

## Lint Levels (precedence high → low)

| Level | Behavior | Overridable? |
|-------|----------|--------------|
| `allow` | Suppress entirely | Yes |
| `expect` | Suppress; warn if lint NOT triggered (1.81+, attr-only) | Yes |
| `warn` | Warning; build succeeds | Yes |
| `force-warn` | Always warn; CLI-only (`--force-warn <lint>`) | No |
| `deny` | Error; build fails | Yes, by `forbid` |
| `forbid` | Error; build fails | **No** — only `--cap-lints` demotes |

Precedence: `--force-warn` > `--cap-lints` > CLI flags (right-most wins) >
inner attrs > outer attrs > defaults. Cargo passes `--cap-lints allow` for
dependencies, so `[lints]` applies only to the local package. Use `forbid`
only for lints that must NEVER relax (e.g., `unsafe_code`); `deny` for
CI-blocking lints that may need scoped exceptions. Prefer `expect(...)` over
`allow(...)` for intentional suppressions.

## Clippy Groups

| Group | Default? | Default level | When to enable |
|-------|----------|---------------|----------------|
| `clippy::correctness` | Yes | deny | Always (already on) |
| `clippy::suspicious` | Yes | warn | Always; explicitly deny if overridden |
| `clippy::style` / `complexity` / `perf` | Yes | warn | Always |
| `clippy::pedantic` | No | allow | Library crates; allow noisy individuals |
| `clippy::cargo` | No | allow | Published crates |
| `clippy::nursery` | No | allow | Review individually; flaky |
| `clippy::restriction` | No | allow | **Never as a group** — one at a time |

`clippy::all` = the five default-on groups. Never enable
`clippy::restriction` as a whole (triggers `blanket_clippy_restriction_lints`);
enable restriction lints individually.

## Key Lints to Enable

**rustc** (`[lints.rust]`): `unsafe_code = "forbid"` (safe crates; `deny` if
scoped unsafe needed), `unsafe_op_in_unsafe_fn = "warn"` (allow-by-default
pre-2024; warn in 2024), `missing_docs = "warn"` (libs; public items),
`elided_lifetimes_in_paths`/`unreachable_pub = "warn"`, `unused_must_use = "deny"`.
**clippy** (`[lints.clippy]`): `unwrap_used`/`expect_used`/`panic`/`dbg_macro`
= `deny` (prod; allow in tests only), `print_stdout`/`print_stderr` = `deny`
(libs; allow in CLI bins), `indexing_slicing` = `warn` (prefer `v.get(i)`),
`todo`/`unimplemented`/`clone_on_ref_ptr` = `warn`.

## `[lints]` Table in Cargo.toml

Requires `resolver = "2"` (or `"3"` for edition 2024 / Rust 1.84+);
stabilized in Rust/Cargo 1.74. Without resolver v2+, `[lints]` is a hard
error. **Priority rule**: set `priority = -1` on group entries so individual
lints at default priority 0 win (rustc = last-write-wins).

```toml
[package]
name = "my-lib"
edition = "2021"
resolver = "2"                       # REQUIRED for [lints]

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unsafe_op_in_unsafe_fn = "warn"
unused_must_use = "deny"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }   # priority = -1 on groups!
cargo = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "warn"
```

## Workspace Inheritance

Define shared policy in `[workspace.lints.rust]` / `[workspace.lints.clippy]`
at the workspace root; member crates opt in with `[lints] workspace = true`.
**Hard rule**: when `workspace = true` is present, NO other `[lints]` fields
are allowed — a crate needing different lints must define a complete local
`[lints]` table and NOT inherit.

## clippy.toml (workspace root)

Version-control it. Unstable API; options may change between versions. MSRV
discovery: `#![clippy::msrv]` (nightly) > `clippy.toml msrv` >
`Cargo.toml rust-version` > compiler version.

```toml
msrv = "1.74"                          # match project MSRV
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7
disallowed-names = ["foo", "bar", "baz"]   # NOT blacklisted-names (deprecated)
allow-unwrap-in-tests = true           # cleaner than scoped #[allow]
allow-expect-in-tests = true
```

## rustfmt.toml Basics

Formatting is separate from linting — see
[`docs/rust/style-formatting.md`](../../../docs/rust/style-formatting.md).
Pin both `edition` and `style_edition` so editor-on-save matches CI:

```toml
edition = "2024"
style_edition = "2024"      # pin both; editor-on-save must match CI
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
```

Nightly-only options (`imports_granularity`, `group_imports`, `wrap_comments`,
`trailing_comma`) require `unstable_features = true` + `cargo +nightly fmt`
(stable silently drops them). Avoid deprecated keys: `version` (use
`style_edition`), `merge_imports` (use `imports_granularity`).
`#[rustfmt::skip]` only on small, justified constructs; never crate-level.

## CI & Verification Commands

```bash
# Lint gate (blocking)
cargo clippy --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Formatting gate (blocking)
cargo fmt --all -- --check
cargo +nightly fmt --all -- --check   # if rustfmt.toml uses nightly-only keys

# Auto-fix (review diff) / verify inheritance / formatting drift
cargo clippy --fix --allow-dirty --allow-staged && git diff
cargo metadata --format-version=1 | jq '.packages[] | {name, lints}'
cargo fmt -- --files-with-diff
```

`-D warnings` denies ALL warnings (rustc + clippy); use `-D clippy::all` for
clippy only. MSRV/nightly: `cargo +1.74 clippy -- -D warnings`,
`cargo +nightly clippy -- -D warnings` (informational, non-blocking).

## Review Checklist

- [ ] New `#[allow(...)]`/`#[expect(...)]` include a `reason`, scoped to the
      narrowest item (never crate-root `#![allow]`).
- [ ] No `unwrap()`/`expect()`/`panic!()` in non-test production code.
- [ ] CI passes `cargo clippy --all-targets -- -D warnings` and
      `cargo fmt --all -- --check`.
- [ ] New `unsafe` code has a safety comment; `unsafe_op_in_unsafe_fn` set;
      new public items documented (if `missing_docs = "warn"`).
- [ ] No `#![deny(warnings)]` in libraries; no `#[forbid]` on lints needing
      scoped exceptions.
- [ ] New crates inherit via `lints.workspace = true`; group entries have
      `priority = -1`; `clippy.toml` has `msrv` set.

## Anti-patterns & Failure Modes

| Anti-pattern / Symptom | Cause | Fix |
|---|---|---|
| `#![deny(warnings)]` in a library | Breaks downstream on new rustc lints | Use `[lints]` + CI `-D warnings` |
| Scattered `#![deny(...)]` crate attrs | Invisible in review; no inheritance | Consolidate into `[lints]` table |
| `#![allow(unwrap_used)]` at crate root | Allows unwrap everywhere | Scope to tests or `allow-unwrap-in-tests` |
| `forbid` on lints needing scoped exceptions | Cannot be overridden | Use `deny` instead |
| `failed to parse manifest` near `[lints]` | Missing resolver v2+ | Add `resolver = "2"` |
| `cannot be overridden` error | Used `forbid` then tried `#[allow]` | Change `forbid` → `deny` |
| `workspace = true` + override error | Mixed inheritance with overrides | Inherit fully OR complete local `[lints]` |
| Individual lint override ignored | Group entry missing `priority = -1` | Add `priority = -1` to group entry |
| Clippy suggests unavailable feature | `msrv` not set in `clippy.toml` | Set `msrv` to project MSRV |
| `cargo fmt --check` passes but imports ungrouped | Nightly-only option on stable | Use `cargo +nightly fmt` or drop it |
| `blanket_clippy_restriction_lints` fires | Enabled `clippy::restriction` group | Enable restriction lints individually |
| `cargo clippy --fix` committed unreviewed | Auto-fix not semantically identical | Review `git diff` before committing |

## Related Docs
- [`docs/rust/lints-clippy.md`](../../../docs/rust/lints-clippy.md) — full lint policy, attribute syntax, clippy.toml options, CI integration.
- [`docs/rust/style-formatting.md`](../../../docs/rust/style-formatting.md) — rustfmt, `cargo fmt`, `rustfmt.toml`, style editions, import grouping.
