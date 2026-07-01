---
name: rust-cargo-and-deps
description: |
  Operational reference for writing, reviewing, and refactoring Cargo.toml
  manifests, managing dependencies, features, the dependency resolver,
  workspaces, build profiles, MSRV, and edition selection. Load when setting
  up Cargo.toml, adding dependencies, configuring features, or doing
  workspace setup. Cites docs/rust/cargo-dependencies.md and
  docs/rust/editions-tooling.md as the authoritative long-form references.
---

# Cargo, Dependencies, Features, Workspaces, Editions

Compact operational skill. Full detail in the two source docs (upstream
Cargo/rustc URLs cited through them — do not link upstream from here):
`docs/rust/cargo-dependencies.md` (manifests, version specifiers, features,
resolver, workspaces, profiles, commands, lock files) and
`docs/rust/editions-tooling.md` (editions, MSRV, edition migration, rustdoc,
rustup toolchains).

## Triggers

Load when: creating/editing a `Cargo.toml`; adding/removing/upgrading/pinning
dependencies; configuring `[features]`/`default`/optional deps/`dep:` syntax;
setting up or refactoring a workspace; choosing `edition`/`rust-version`/
`resolver`; tuning `[profile.release]`; resolving dep conflicts (`cargo tree
-d`); reviewing a Cargo manifest PR.

Do NOT load for: pure source edits with no manifest change, Nix flake work
(use `nix-usage`), or rustfmt/rustdoc detail (see `docs/rust/style-formatting.md`
/ `documentation-guidelines.md`).

## Manifest Structure

```toml
[package]
name = "my-crate"          # required
version = "0.1.0"          # required for publish
edition = "2021"          # ALWAYS set; default 2015 if omitted
rust-version = "1.74"     # MSRV; bare version, no operators
license = "MIT OR Apache-2.0"  # SPDX; required for publish
resolver = "2"            # or "3" for edition 2024 / Rust 1.84+
# authors = ...           # DEPRECATED — do not add to new manifests

[dependencies]             # lib + bins + tests + benches; NOT build.rs
serde = { version = "1", features = ["derive"] }
[dev-dependencies]        # tests/examples/benches only; not propagated
assert_cmd = "2"
[build-dependencies]      # build.rs only; no access to [dependencies]
cc = "1"
[target.'cfg(unix)'.dependencies]
libc = "0.2"
[features]
default = ["json"]
json = ["dep:serde_json", "serde/derive"]   # dep: prefix (Rust 1.60+) preferred
[dependencies.serde_json]
version = "1"
optional = true
```

## Version Specifiers

| Syntax | Meaning | Use when |
|---|---|---|
| `1.2.3` / `^1.2.3` | caret: `>=1.2.3, <2.0.0` | **default** for production deps |
| `~1.2.3` | tilde: `>=1.2.3, <1.3.0` | lock to minor |
| `=1.2.3` | exact pin | tightly-coupled lib+proc-macro pairs only |
| `*` / `1.*` | wildcard | never for published crates (crates.io rejects `*`) |
| `>=1.2, <1.5` | range | rare; explicit bounds |

`0.x.y` caret: rightmost non-zero governs — `0.2.3` → `>=0.2.3, <0.3.0`;
`0.0.3` → `>=0.0.3, <0.0.4`. Pre-releases **excluded** unless explicitly named.
Prefer `cargo add` over hand-editing.

## Features Best Practices

1. **Additive only** — enabling more must never break compilation.
2. **No mutually exclusive features** — if unavoidable, use
   `#[cfg(all(...))] compile_error!`, split crates, or `cfg-if`.
3. **`default` minimal** — consumers opt in, not out. Declare `default = []`
   explicitly if no default features.
4. **`dep:<name>`** (Rust 1.60+) for optional deps — suppresses implicit
   same-named feature, avoids collisions. Prefer it.
5. **`<dep>/<feature>`** enables a feature on a dep and auto-enables the dep.
6. **`default-features = false`** is per-declaration; feature **unification**
   means it does NOT guarantee defaults off across the graph.
7. **Document public features** in `lib.rs`/README; list only public ones.

## Workspace Inheritance (MSRV 1.64+)

```toml
# workspace root
[workspace]
members = ["crates/*"]
resolver = "2"
[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
license = "MIT OR Apache-2.0"
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
# member crate:
#   [package] name = "my-crate"; version.workspace = true; edition.workspace = true
#   [dependencies] serde = { workspace = true }
```

Rules: `[workspace.dependencies]` **cannot** declare `optional` (members add it); `features` there are **additive** with member features; other keys (`version`/`path`/`git`/`default-features`) come from the workspace table and are NOT re-settable in the inheriting declaration. `[patch]`/`[profile.*]` honored **only** at workspace root. Virtual workspaces (no `[package]`) **must** set `resolver` explicitly.

## Edition, MSRV, Resolver

| Field | Values / Effect |
|---|---|
| `edition` | `"2015"` (default if omitted — avoid), `"2018"`, `"2021"`, `"2024"`. New crates: `2021`/`2024`. |
| `rust-version` | Bare version (e.g. `"1.74"`); since Cargo 1.56. Bumping = minor SemVer break. |
| `resolver` | `"1"` (2015/2018), `"2"` (default 2021; MSRV 1.51), `"3"` (default 2024; MSRV 1.84, MSRV-aware dep selection). |

Always set `edition` + `rust-version` explicitly. Use `resolver = "2"` (or `"3"` for edition 2024); `[lints]` requires v2+. Verify MSRV in CI: `cargo +<msrv> check --all-targets`. Edition migration: `cargo fix --edition` → edit `edition` → `cargo build --all-targets` → `cargo fix --edition-idioms` → `cargo fmt` (see `docs/rust/editions-tooling.md`).

## Profiles (release tuning)

```toml
[profile.release]
lto = "thin"          # typical sweet spot; "fat" slower build
codegen-units = 1     # only effective WITH lto enabled
strip = "symbols"     # smallest binary
panic = "abort"       # smaller; breaks catch_unwind; forces test deps to unwind
incremental = false
```

Only four built-in profiles: `dev`, `release`, `test`, `bench` (no `doc`). Custom profiles require `inherits`. Settings read **only** from workspace-root `Cargo.toml`. **Measure** before/after — don't guess.

## Cargo.lock Discipline

Commit for binaries/apps **always**; libraries may omit but commit when in doubt. Never hand-edit — use `cargo update` / `cargo update -p <crate> --precise <ver>`. Lock `version` field is a `ResolveVersion` derived from `rust-version`, not the writing Cargo version — do not hand-edit it.

## Commands & Verification Hooks

```bash
cargo add <crate> [--dev|--build] [-F feat] [--optional]  # add; normalizes syntax + lock
cargo remove <crate>                                      # companion
cargo check --all-targets --all-features                 # fast typecheck (no link errors)
cargo build --release                                     # full codegen + link
cargo test --workspace                                    # all tests
cargo tree -d                                             # duplicate versions — run before merge
cargo tree -i <crate>                                     # reverse deps before remove/upgrade
cargo tree -e features                                    # why is a feature enabled?
cargo update -p <crate> [--precise <ver>]                 # update/pin one dep
cargo update --workspace                                  # only workspace pkgs after version edits
cargo +<msrv> check --all-targets                         # MSRV verification
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check                                # CI format gate
```

## Review Checklist

1. `edition` + `rust-version` set explicitly in every `[package]`?
2. Caret requirements unless documented reason for `=`/`~`?
3. `Cargo.lock` committed for binaries; policy documented for libraries?
4. `cargo tree -d` clean (no duplicate versions)?
5. Features additive? No pair breaks when both enabled?
6. `default = []` declared explicitly if no default features?
7. `resolver = "2"`/`"3"` set (workspace + standalone with `[lints]`)?
8. Workspace dep versions centralized in `[workspace.dependencies]`?
9. Git deps pinned to `rev`/`tag` (never bare `branch`)?
10. No path-only deps in publishable crates (combine `path` + `version`)?
11. Optional deps use `dep:<name>` in feature definitions?
12. `cargo check --all-targets --all-features` passes cleanly?

## Anti-patterns

Hand-editing `[dependencies]` (use `cargo add`); `features = ["full"]` on
large crates (tokio) — enable only what you use; same crate in both
`[dependencies]` and `[dev-dependencies]`; path-only deps in published crates;
mutually exclusive features; bare `branch` git deps in production; hand-editing
`Cargo.lock`; omitting `edition` (silently defaults to 2015); adding the
deprecated `authors` field.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `cargo tree -d` shows 2+ versions | semver-incompatible transitive reqs | upgrade the dep pulling the old version, or `[patch]` |
| `default-features = false` didn't disable defaults | feature unification across graph | every declaration must opt out; only union is used |
| `package specifies bad resolver` | `resolver` in a dependency | set only at workspace root or root `[package]` |
| `cfg(feature=...)` ignored in `[target]` deps | not allowed for dep gating | use `[features]` + optional deps instead |
| Path dep breaks publish | path-only forbidden on crates.io | add `version = "..."` alongside `path` |
| `[lints]` silently ignored | resolver v1 | set `resolver = "2"` or `"3"` |
| Virtual workspace resolver error | no `[package].edition` | set `resolver` explicitly in `[workspace]` |

## Related Docs

- `docs/rust/cargo-dependencies.md` — full manifest/dependency/features/resolver/workspace/profiles/commands reference.
- `docs/rust/editions-tooling.md` — editions, MSRV, edition migration, rustdoc, rustup toolchains, `rust-toolchain.toml`.
- `docs/rust/style-formatting.md` — rustfmt config tables, style editions.
- `docs/rust/lints-clippy.md` — lint levels, `[lints]`, clippy MSRV.
