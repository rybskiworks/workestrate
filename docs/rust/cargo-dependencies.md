# Cargo, Manifests, Dependencies, Features, Resolver, Workspaces

## Purpose

Provide concrete, repo-independent guidance for writing, reviewing, and refactoring `Cargo.toml` manifests, managing dependencies, feature flags, the dependency resolver, workspaces, build profiles, and Cargo configuration. This document is intended as generic reference material for future AI coding agents working in any Rust codebase. It is not specific to the `ai-workbench` repository.

Agents should use this document as the authoritative reference when creating or modifying Cargo manifests, resolving dependency conflicts, configuring build profiles, or deciding whether to commit `Cargo.lock`.

## Sources used

- <https://doc.rust-lang.org/cargo/>
- <https://doc.rust-lang.org/cargo/reference/manifest.html>
- <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- <https://doc.rust-lang.org/cargo/reference/semver.html>
- <https://doc.rust-lang.org/cargo/reference/features.html>
- <https://doc.rust-lang.org/cargo/reference/features-examples.html>
- <https://doc.rust-lang.org/cargo/reference/resolver.html>
- <https://doc.rust-lang.org/cargo/reference/workspaces.html>
- <https://doc.rust-lang.org/cargo/reference/profiles.html>
- <https://doc.rust-lang.org/cargo/reference/config.html>
- <https://doc.rust-lang.org/cargo/reference/build-scripts.html>
- <https://doc.rust-lang.org/cargo/reference/source-replacement.html>
- <https://doc.rust-lang.org/cargo/reference/environment-variables.html>
- <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- <https://doc.rust-lang.org/cargo/reference/unstable.html>
- <https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html>
- <https://doc.rust-lang.org/cargo/faq.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-check.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-build.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-test.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-clippy.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-fmt.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-tree.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-update.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-add.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-remove.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-metadata.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-vendor.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-package.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-publish.html>
- <https://doc.rust-lang.org/clippy/>
- <https://github.com/rust-lang/rustfmt>
- <https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/resolve.rs>

### Crawl ledger

SEED:

- <https://doc.rust-lang.org/cargo/>
- <https://doc.rust-lang.org/cargo/reference/manifest.html>
- <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- <https://doc.rust-lang.org/cargo/reference/features.html>
- <https://doc.rust-lang.org/cargo/reference/resolver.html>
- <https://doc.rust-lang.org/cargo/reference/workspaces.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-check.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-build.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-test.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-clippy.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-fmt.html>

DISCOVERED & VISITED:

- <https://doc.rust-lang.org/cargo/reference/semver.html>
- <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- <https://doc.rust-lang.org/cargo/reference/profiles.html>
- <https://doc.rust-lang.org/cargo/reference/config.html>
- <https://doc.rust-lang.org/cargo/reference/build-scripts.html>
- <https://doc.rust-lang.org/cargo/reference/source-replacement.html>
- <https://doc.rust-lang.org/cargo/reference/environment-variables.html>
- <https://doc.rust-lang.org/cargo/reference/features-examples.html>
- <https://doc.rust-lang.org/cargo/reference/unstable.html>
- <https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html>
- <https://doc.rust-lang.org/cargo/faq.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-tree.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-update.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-add.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-remove.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-metadata.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-vendor.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-package.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-publish.html>
- <https://doc.rust-lang.org/cargo/reference/external-tools.html>
- <https://doc.rust-lang.org/clippy/>
- <https://github.com/rust-lang/rustfmt>
- <https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/resolve.rs>
- <https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/encode.rs>

SKIPPED (out of scope / tangential):

- <https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html> — `[patch]`/`[replace]` detail beyond corpus scope (covered at summary level)
- <https://doc.rust-lang.org/cargo/reference/registries.html> — registry internals; only surface config used
- <https://doc.rust-lang.org/cargo/reference/cargo-targets.html> — target discovery; tangential
- <https://doc.rust-lang.org/cargo/reference/pkgid-spec.html> — referenced for `-p`/`profile.package` syntax only
- <https://doc.rust-lang.org/cargo/reference/conditional-compilation.html> — `cfg` detail beyond scope
- <https://doc.rust-lang.org/cargo/reference/build-cache.html> — build cache internals
- <https://doc.rust-lang.org/cargo/appendix/glossary.html> — glossary
- <https://doc.rust-lang.org/cargo/reference/build-script.html> — 404 (correct URL is `build-scripts.html` plural)

## Core guidance

Cargo is the Rust build system and package manager. Every Rust crate is defined by a `Cargo.toml` manifest at its root. The manifest declares the crate's identity, dependencies, features, targets, and build configuration. Cargo resolves dependencies, downloads crates, invokes the compiler, and orchestrates the full build pipeline.

The [Cargo Book](https://doc.rust-lang.org/cargo/) and the [manifest reference](https://doc.rust-lang.org/cargo/reference/manifest.html) are the authoritative sources. When in doubt about a manifest key, consult the reference before guessing.

Key principles:

1. **One source of truth.** In a workspace, shared metadata and dependency versions belong in `[workspace.package]` and `[workspace.dependencies]`. Members inherit with `workspace = true`.
2. **SemVer discipline.** Dependency version requirements use caret (`^`) by default. Understand the semantics, especially for `0.x.y` and `0.0.x` versions. See [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) and [SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html).
3. **Additive features.** Feature flags must be additive — enabling more features must never break compilation. Mutually exclusive features are a design error.
4. **Lock file discipline.** Cargo's default and recommendation is to commit `Cargo.lock` to VCS. Always commit for binaries; libraries may omit it, but committing is still recommended when in doubt. See [Cargo.lock guidance](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) and the [Cargo FAQ](https://doc.rust-lang.org/cargo/faq.html).
5. **Resolver v2 or v3.** Use `resolver = "2"` (or `"3"` for edition 2024 / Rust 1.84+) in workspaces and standalone crates that use `[lints]`, dev-dependencies with features, or build-dependencies. See [resolver](https://doc.rust-lang.org/cargo/reference/resolver.html).
6. **MSRV discipline.** Declare `rust-version` and verify it in CI. See [rust-version](https://doc.rust-lang.org/cargo/reference/rust-version.html).

### Manifest (`[package]`)

The `[package]` table defines crate identity and metadata. Source: [manifest reference](https://doc.rust-lang.org/cargo/reference/manifest.html).

- The only field Cargo itself requires is `name`. `version` defaults to `0.0.0` but is **required** for publishing to crates.io.
- Required for crates.io publishing: `name`, `version`, `description`, and `license` (or `license-file`). `license` is interpreted as an SPDX 2.3 expression, e.g. `"MIT OR Apache-2.0"`.
- The `authors` field is **deprecated** (kept only for `CARGO_PKG_AUTHORS` backward compatibility). Do **not** add it to new manifests.
- `edition` defaults to `2015` if absent; `cargo new` uses the latest stable edition (currently `2024`). Supported: `2015`, `2018`, `2021`, `2024`.
- `rust-version` declares the MSRV. See the [MSRV section](#msrv).
- `keywords` (max 5, ASCII, ≤20 chars), `categories` (max 5, must match <https://crates.io/category_slugs> slugs exactly).
- `build` defaults to `"build.rs"`; set `build = false` to disable a build script.
- `links` declares a native library binding (see [build scripts](#build-scripts)).
- `exclude`/`include`: gitignore-style patterns for `cargo package`; `include` overrides `exclude`; if neither is set, `.gitignore` rules apply.
- `publish`: array of allowed registries, or `false` to block publishing. Omitting `version` also blocks publishing.
- `resolver`: `"1"`, `"2"`, or `"3"` at the package level; also set workspace-wide. Values in dependencies are ignored.
- `default-run`: default binary for `cargo run`.
- `package.metadata.*`: ignored by Cargo; used by tools such as `docs.rs` and `cross`.
- `[lints]` table: respected as of Rust 1.74; levels `forbid`/`deny`/`warn`/`allow`; supports `{ level, priority }` (priority is a signed int). Cargo applies `[lints]` only to the local package, **not** to dependencies. Requires resolver v2+.

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
license = "MIT OR Apache-2.0"
description = "A short summary"
repository = "https://github.com/org/my-crate"
readme = "README.md"
keywords = ["events", "processing"]
categories = ["command-line-utilities"]
resolver = "2"
```

### Dependency tables

Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

- `[dependencies]`: used when compiling the library, binaries, examples, tests, and benchmarks. **Not** shared with the build script.
- `[dev-dependencies]`: used **only** for tests, examples, and benchmarks. **Not** propagated to dependents. **Not** used for normal builds or publishing. On publish, only dev-deps that specify a `version` are included (crates.io forbids depending on non-crates.io code). Dev-deps of non-workspace-members are **always** ignored by the resolver. The resolver allows dev-dependency cycles.
- `[build-dependencies]`: used **only** by `build.rs`. Build scripts do **not** have access to `[dependencies]` or `[dev-dependencies]`. Build deps are **not** available to the package itself unless also listed in `[dependencies]`.
- `[target.'cfg(...)'.dependencies]` / `dev` / `build`: platform-specific. Operators: `not`, `any`, `all`; or full target triples. The resolver resolves **all** platform variants as if all platforms were enabled (`cfg` is ignored during resolution). You **cannot** use `cfg(debug_assertions)`, `cfg(test)`, `cfg(proc_macro)`, or `cfg(feature="...")` for dependency gating — use `[features]` for feature-based optional deps.

```toml
[dependencies]
serde = "1"

[dev-dependencies]
assert_cmd = "2"

[build-dependencies]
cc = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

### Version requirement syntax

Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) and [SemVer](https://doc.rust-lang.org/cargo/reference/semver.html).

| Syntax | Meaning | Example |
|--------|---------|---------|
| `1.2.3` | Caret/default: `>=1.2.3, <2.0.0` | `serde = "1"` → `>=1.0.0, <2.0.0` |
| `^1.2.3` | Explicit caret: same as above | `^1.2.3` → `>=1.2.3, <2.0.0` |
| `~1.2.3` | Tilde: `>=1.2.3, <1.3.0` | `~1.2.3` → `>=1.2.3, <1.3.0` |
| `~1.2` | Tilde: `>=1.2.0, <1.3.0` | `~1.2` → `>=1.2.0, <1.3.0` |
| `~1` | Tilde: `>=1.0.0, <2.0.0` | `~1` → `>=1.0.0, <2.0.0` |
| `*` | Wildcard: `>=0.0.0` (rejected by crates.io for published crates) | `foo = "*"` |
| `1.*` | Wildcard: `>=1.0.0, <2.0.0` | `foo = "1.*"` |
| `1.2.*` | Wildcard: `>=1.2.0, <1.3.0` | `foo = "1.2.*"` |
| `>=1.2.0` | Greater than or equal | `>=1.2.0` |
| `>1` | Greater than | `>1.0.0` |
| `<2` | Less than | `<2.0.0` |
| `=1.2.3` | Exact pin | only `1.2.3` |
| `>=1.2, <1.5` | Range | `>=1.2.0, <1.5.0` |

#### `0.x.y` caret semantics

Cargo differs from standard SemVer: the rightmost non-zero component governs compatibility.

| Requirement | Resolves to |
|-------------|-------------|
| `0.2.3` / `^0.2.3` | `>=0.2.3, <0.3.0` |
| `0.2` / `^0.2` | `>=0.2.0, <0.3.0` |
| `0` / `^0` | `>=0.0.0, <1.0.0` |
| `0.0.3` / `^0.0.3` | `>=0.0.3, <0.0.4` |
| `0.0` / `^0.0` | `>=0.0.0, <0.1.0` |

#### Pre-release rules

Pre-release versions are **excluded** unless explicitly named. Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

- `foo = "1.0"` will **not** match `1.0.0-alpha`.
- Naming `foo = "1.0.0-alpha"` allows upgrading to `1.0.0-beta` (same base release, newer pre-release) **and** to semver-compatible releases such as `1.0.0` / `1.2.0`, but **not** to `1.0.1-alpha` (different base release).
- `cargo install` avoids pre-releases unless explicitly requested.
- Build metadata such as `1.0.0+21AF26D3` is ignored and should not appear in requirements.

Cargo recommends: use caret/default; avoid upper bounds tighter than the next semver-incompatible version; use `=x.y.z` exact pins only for tightly-coupled pairs (e.g., a library + its proc-macro).

### Dependency sources

Sources: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html), [source replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html).

- **crates.io (default):** `serde = "1"`.
- **Other registry:** `some = { version = "1.0", registry = "my-registry" }`.
- **Git:** `regex = { git = "https://github.com/rust-lang/regex.git" }`; optional `branch` (default branch if omitted), `tag`, or `rev` (commit hash or e.g. `refs/pull/493/head`). Cargo traverses the file tree to find the crate's `Cargo.toml` anywhere in the repo (unlike path deps). Submodules are fetched recursively. Commits are locked in `Cargo.lock` and re-evaluated only on `cargo update`. `version` is **optional** with git; if present, Cargo checks compatibility but does not use it to select a commit. Fragment-style `git = "...#abc"` is ignored and warns.
- **Path:** `hello_utils = { path = "hello_utils" }`. Must point to the **exact** folder containing the dependency's `Cargo.toml` (no tree traversal). Path-only deps are **forbidden** on crates.io.
- **Multiple locations rule:** you **can** combine `version` with `git` **or** `version` with `path` (use local/git during dev, crates.io version when published):

  ```toml
  bitflags = { path = "my-bitflags", version = "1.0" }
  ```

  You **cannot** combine git+path, or two different sources for the same crate. To override the crates.io version of a crate already depended upon, use `[patch.crates-io]` rather than a second source line.

### Cargo.lock

Sources: [Cargo.toml vs Cargo.lock](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html), [Cargo FAQ](https://doc.rust-lang.org/cargo/faq.html).

The lock file records exact versions resolved for all dependencies. It ensures reproducible builds across machines and CI.

- Cargo's **default and recommendation** is to commit `Cargo.lock` to VCS. `cargo new` tracks it by default.
- Rationale: deterministic/reproducible builds, `git bisect`, consistent CI and contributor experience, and preserving lower versions for MSRV verification.
- Binary/library trade-off: binaries/apps → always commit. Libraries → `Cargo.lock` only affects the library's own development builds, **not** downstream consumers (a dependent's resolver re-resolves). Community convention: libraries **may** `.gitignore` it so the resolver picks newer transitive versions; Cargo's FAQ strongly recommends committing in all cases when in doubt. A library that also ships a binary → commit.
- `cargo install` **ignores** a dependency's published `Cargo.lock` unless run with `--locked`.

#### Lock file `version` field

The top-level `version = N` in `Cargo.lock` is a `ResolveVersion`, **not** the Cargo version that wrote it. It is selected from the **root package's `rust-version`** (not the running Cargo's version). Source: [`resolve.rs`](https://github.com/rust-lang/cargo/blob/master/src/cargo/core/resolver/resolve.rs).

| `rust-version` | Lockfile version |
|----------------|------------------|
| ≥ 1.83 | V4 |
| ≥ 1.53 | V3 |
| ≥ 1.41 | V2 |
| else | V1 |
| unset | V4 (current Cargo master) |

History: V2 introduced Cargo 1.38, default for new lockfiles 1.41–1.52; V3 introduced Cargo 1.47, default 1.53–1.82; V4 introduced Cargo 1.78, default from 1.83; V5 is nightly-only (`-Znext-lockfile-bump`).

Forward/backward compatibility: Cargo conservatively avoids lockfile churn; reading accepts known versions and upgrades the detected version; unknown higher versions error with "this version of Cargo does not understand this lock file". **Do not hand-edit the version field.**

### Features

Source: [features](https://doc.rust-lang.org/cargo/reference/features.html) and [features-examples](https://doc.rust-lang.org/cargo/reference/features-examples.html).

- `[features]` table: `name = ["other-feature", "dep:opt", "pkg/feat"]`. Empty array `[]` defines a leaf feature.
- Feature name charset: Unicode XID + `_` digits `-` `+` `.`; crates.io further restricts to ASCII alphanumerics + `_` `-` `+`. crates.io cap: ≤300 features per crate.
- `default = [...]` is conventional and treated specially (enabled unless `--no-default-features`). `default = []` is valid and explicit.
- `optional = true` deps implicitly create a same-named feature.
- `dep:<name>` syntax (since Rust 1.60) explicitly enables an optional dependency **and** suppresses the implicit same-named feature (avoids collisions). Prefer this.
- `<dep>/<feature>` enables a feature on a dependency **and** auto-enables the (optional) dep.
- `<dep>?/<feature>` (since Rust 1.60) enables the feature on a dep **only** if something else already enabled the dep (does **not** auto-enable).
- When a dep is renamed via `package = "..."`, feature syntax uses the renamed **local** name.
- `default-features = false` is per-declaration; due to feature **unification** it does **not** guarantee defaults are off (every declaration in the graph must opt out). Only the union of requested features is used.
- Feature unification: a dep used by multiple packages is built once with the **union** of all requested features. Hence features **must** be additive — enabling must never disable functionality. Mutually exclusive features are an explicit design error. Mitigations: `#[cfg(all(...))] compile_error!`, split into separate packages, runtime config, or `cfg-if` precedence.
- Document features in `lib.rs` doc comments / README / docs.rs; list only **public** features, not internal `dep:` opt-ins.
- `required-features` can gate build targets (`[[bin]]`, `[[example]]`, `[[test]]`, `[[bench]]`).
- When a feature is enabled, build scripts see `CARGO_FEATURE_<NAME>` (uppercased, `-` → `_`).

### Resolver

Source: [resolver](https://doc.rust-lang.org/cargo/reference/resolver.html).

- `"1"`: default for edition 2015/2018 (and unset). Unifies features globally across **all** targets/kinds: target-specific, build-dep, and dev-dep features leak into normal builds.
- `"2"` (default for edition 2021; MSRV Rust 1.51): feature unification scoped per kind. Three changes: (1) target-specific features only apply when that target is built; (2) build-dependencies and proc-macros are isolated from normal deps; (3) dev-dependencies do **not** activate features except when building tests/examples/benches (dev-deps of transitive deps are always ignored).
- `"3"` (default for edition 2024; MSRV Rust 1.84): **same** feature-unification semantics as v2, but changes the default of `resolver.incompatible-rust-versions` from `"allow"` to `"fallback"` (MSRV-aware version selection).
- CLI difference: under v1, `--features`/`--no-default-features` apply only to the package in CWD; under v2+, they apply to any `-p`/`--workspace` selected package.
- `[lints]` requires resolver v2+. Virtual workspaces (no `[package].edition`) **must** set `resolver` explicitly. `resolver` is workspace-wide; the value in dependencies is ignored. Set it at the workspace root or root `[package]`.
- `resolver.incompatible-rust-versions = "fallback"` (respected since 1.84): the resolver prefers dep versions whose `rust-version` ≤ your MSRV.

### Workspaces

Source: [workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) (workspace inheritance MSRV 1.64+).

A workspace is a collection of related crates that share a single `Cargo.lock` and `target/` directory.

- `[workspace]` keys: `members`, `exclude`, `default-members`, `resolver`. `members`/`exclude` accept globs. All path-deps inside the workspace become members automatically; `exclude` removes specific paths. `package.workspace = "path"` overrides auto-discovery.
- All members share **one** `Cargo.lock` and **one** `target/` at the workspace root.
- `[patch]`, `[replace]`, `[profile.*]` are honored **only** in the workspace-root manifest; ignored in members.
- `[workspace.package]` (MSRV 1.64+): inheritable keys exactly: `authors`, `categories`, `description`, `documentation`, `edition`, `exclude`, `homepage`, `include`, `keywords`, `license`, `license-file`, `publish`, `readme`, `repository`, `rust-version`, `version`. `license-file` and `readme` are relative to the workspace root; `include`/`exclude` are relative to the package root. Members opt in per-field: `version.workspace = true`.
- `[workspace.dependencies]` (MSRV 1.64+): members use `regex = { workspace = true }` in `[dependencies]` / `[dev-dependencies]` / `[build-dependencies]` / target deps. Two rules: (1) **cannot** declare `optional` here (members add `optional` themselves); (2) `features` declared here are **additive** with member-declared features — all other keys (`version`, `default-features`, `path`, `git`) come from the workspace table and are **not** re-settable in the inheriting declaration. Shorthand `cc.workspace = true` works.
- `[workspace.lints]` (respected since 1.74): members pull in via `[lints] workspace = true`.
- Root package vs virtual workspace: a root package has **both** `[package]` and `[workspace]`; a virtual manifest has `[workspace]` only and **must** set `resolver` explicitly. `default-members` chooses what runs from root with no `-p`/`--workspace`.

### Profiles

Source: [profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).

- There are exactly **four** built-in profiles: `dev`, `release`, `test`, `bench`. There is **no** `doc` profile — `cargo doc` uses `dev`/`release`.
- Command → profile mapping:
  - `cargo run`/`build`/`check`/`rustc` → `dev` (default); `--release` ≡ `--profile=release`
  - `cargo test` → `test` (inherits `dev`)
  - `cargo bench` → `bench` (inherits `release`)
  - `cargo install` → `release`
- Settings and values:
  - `opt-level`: `0`, `1`, `2`, `3`, `"s"`, `"z"`. (`"z"` also turns off loop vectorization.)
  - `debug`: `0`/`false`/`"none"`, `1`/`"limited"`, `2`/`true`/`"full"`, `"line-tables-only"`, `"line-directives-only"`. Named values need MSRV 1.71. Default: `true` (full) for `dev`, `false` (none) for `release`.
  - `split-debuginfo`: `packed`, `unpacked`, `off`, `dsym`, ... platform-specific. Default `unpacked` on macOS when debug is enabled.
  - `strip`: `"none"` (default), `"debuginfo"`, `"symbols"`. `true` ≡ `"symbols"`, `false` ≡ `"none"`.
  - `debug-assertions`: bool (true dev, false release).
  - `overflow-checks`: bool (true dev, false release).
  - `lto`: `false`/`"off"`, `true`/`"fat"`, `"thin"`. Note: `lto = false` produces **no** LTO if `codegen-units = 1` or `opt-level = 0`.
  - `panic`: `"unwind"` (default), `"abort"`. Tests/benches/build-scripts/proc-macros **ignore** panic setting (test harness needs unwind); with `panic = "abort"`, building a test forces all deps to `unwind`.
  - `incremental`: bool (true dev, false release). Only for workspace members and path deps.
  - `codegen-units`: int > 0. Default 256 for incremental builds, 16 for non-incremental.
  - `rpath`: bool (default false).
- Built-in defaults:
  - `dev`: `opt-level 0`, `debug true`, `incremental true`, `codegen-units 256`, `debug-assertions true`, `overflow-checks true`, `panic unwind`
  - `release`: `opt-level 3`, `debug false`, `incremental false`, `codegen-units 16`, `debug-assertions false`, `overflow-checks false`, `panic unwind`
  - `test` inherits `dev`; `bench` inherits `release`.
- Custom profiles require `inherits = "..."`. Output goes to `target/<profile-name>/`.
- Overrides:
  - `[profile.<name>.package.<pkg>]` (pkg is a Package ID Spec, so `[profile.dev.package."foo:2.1.0"]` works)
  - `[profile.<name>.package."*"]` = all non-workspace deps
  - `[profile.<name>.build-override]` = build scripts/proc-macros
  - Precedence (first match wins): named package > `"*"` > build-override > profile settings > Cargo defaults.
  - Overrides **cannot** specify `panic`, `lto`, or `rpath`.
- Profile settings are read **only** from the workspace-root `Cargo.toml` (ignored in dependency manifests).
- Generics caveat: at `opt-level` 2/3 monomorphized generics are **not** shared across crates; raising opt-level on a dep that defines generics may not speed them up. Use `opt-level = 1` for exported-generic workarounds.
- Release tuning: `lto = "thin"` is the typical sweet spot; `codegen-units = 1` is only effective **with** LTO; `strip = "symbols"` gives the smallest binary; `panic = "abort"` is smaller but breaks `catch_unwind`. **Measure** rather than guess.

### `.cargo/config.toml`

Source: [config](https://doc.rust-lang.org/cargo/reference/config.html).

- Probing (deepest first, deeper = higher precedence): walk from cwd up parents → `/.cargo/config.toml` → `$CARGO_HOME/config.toml`. Cargo does **not** read `config.toml` from crates inside a workspace (only workspace root + ancestors).
- Precedence (highest → lowest): `--config KEY=VALUE` CLI > environment variables > project `.cargo/config.toml` (deepest) > `$CARGO_HOME/config.toml`. Merge: scalars = deeper wins; arrays = joined (higher-precedence later).
- `.cargo/config` (no extension) is also supported; if both exist, the no-ext file wins (`.toml` preferred, added 1.39).
- Key sections:
  - `[build]`: `jobs`, `rustc`, `rustc-wrapper`, `rustc-workspace-wrapper`, `rustdoc`, `target`, `target-dir`, `build-dir`, `rustflags`, `rustdocflags`, `incremental`, `dep-info-basedir`
  - `[target.<triple>]` / `[target.'cfg(...)']`: `linker`, `runner`, `rustflags`, `rustdocflags`
  - `[target.<triple>.<links>]`: links build-script override
  - `[source.<name>]`: source replacement
  - `[net]`: `retry` (default 3), `git-fetch-with-cli` (default false), `offline`, `ssh.known-hosts`
  - `[registries.<name>]`: `index`, `token`, `credential-provider`
  - `[registries.crates-io] protocol`: default now `"sparse"` (alt `"git"`)
  - `[registry]`: `default`, `token`, `global-credential-providers`
  - `[alias]`: built-ins `b`/`c`/`d`/`t`/`r`/`rm`; recursive; cannot redefine built-ins
  - `[doc]`: `browser`
  - `[env]`: `KEY = "value"` does **not** override existing env; `{ value = "...", force = true }` overrides; `{ value = "...", relative = true }` resolves relative to parent of `.cargo/`
  - `[term]`: `quiet`, `verbose`, `color` (default `"auto"`), `hyperlinks`, `unicode`, `progress`
  - `[http]`: `proxy`, `timeout` (default 30), `cainfo`, `ssl-version`, `multiplexing` (default true), `user-agent`
  - `[install]`: `root`
  - `[cache]`: `auto-clean-frequency` (default `"1 day"`)
  - `[resolver]`: `incompatible-rust-versions`
  - `[patch.<registry>]`
  - `[profile.<name>]`: overrides `Cargo.toml`; package overrides have **no** env support
  - `[credential-alias]`
  - Top-level `paths` (no env), `include` (with optional)
- Config env var naming: `foo.bar` → `CARGO_FOO_BAR`. Not all keys support env.
- `rustflags` four mutually-exclusive sources (first match wins): (1) `CARGO_ENCODED_RUSTFLAGS`, (2) `RUSTFLAGS`, (3) `target.<triple>.rustflags` + `target.<cfg>.rustflags` joined, (4) `build.rustflags`. With `--target`, flags apply to target only.
- Credentials live in `$CARGO_HOME/credentials.toml`.

### Build scripts

Source: [build-scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) (note the plural URL; `build-script.html` is a 404).

- `build.rs` at package root runs before the package builds, compiled for the **host** (the machine running the script). Configurable via `package.build` (set to a name, or `false` to disable). Non-zero exit halts the build.
- Inputs: env vars + cwd = package root. Use `CARGO_CFG_*` env vars for target cfg (**not** `cfg!`/`#[cfg]`, which check the host).
- Outputs use the `cargo::` directive syntax (double-colon; MSRV 1.77). Legacy `cargo:` single-colon still works.
- Main directives:
  - `cargo::rerun-if-changed=PATH`
  - `cargo::rerun-if-env-changed=NAME`
  - `cargo::rustc-link-lib=LIB` (`KIND[:MODIFIERS]=]NAME[:RENAME]`; KIND `dylib`/`static`/`framework`)
  - `cargo::rustc-link-search=[KIND=]PATH`
  - `cargo::rustc-link-arg=FLAG` (+ per-target variants `-bin=BIN=`, `-bins`, `-tests`, `-examples`, `-benches`, `-cdylib`)
  - `cargo::rustc-flags=FLAGS` (only `-l` / `-L`)
  - `cargo::rustc-cfg=KEY[="VALUE"]` (register expected values via `cargo::rustc-check-cfg`, MSRV 1.80, to avoid `unexpected_cfgs`)
  - `cargo::rustc-env=VAR=VALUE` (read via `env!`)
  - `cargo::metadata=KEY=VALUE` (passed to immediate dependents as `DEP_<LINKS>_<KEY>`)
  - `cargo::warning=MESSAGE` (suppressed for crates.io deps unless `-vv`)
  - `cargo::error=MESSAGE` (MSRV 1.84)
- Directive order matters (affects rustc/linker arg order).
- Change detection: by default Cargo re-runs the script if **any** file in the package changes. Best practice: emit at least one `rerun-if-changed`/`rerun-if-env-changed` to narrow triggers; otherwise Cargo scans the whole package dir. Since 1.46, `env!`/`option_env!` in source auto-detect changes.
- `links` manifest key: declares the package links a native lib; package must have a build script that uses `rustc-link-lib`. **Hard rule:** at most **one** package per `links` value in the graph (prevents duplicate symbols). The `*-sys` convention (e.g. `foo-sys` linking `libfoo`, with a high-level `foo` wrapper depending on `foo-sys`) shares the single linker binding.
- `[target.<triple>.<links>]` config **overrides** a links build script (script not run): keys `rustc-link-lib`, `rustc-link-search`, `rustc-flags`, `rustc-cfg`, `rustc-env`, `rustc-cdylib-link-arg`, plus `metadata_*`; `warning`/`rerun-if-*` ignored.
- When **not** to use a build script: prefer the `cc` crate (build-dependency) for C/C++; `pkg-config` to find system libs; `cmake` crate for CMake; `bindgen` for FFI. For pure Rust, no build script is needed.
- **Important:** `RUSTFLAGS` is **not** available in build scripts since Rust 1.55 — use `CARGO_ENCODED_RUSTFLAGS` (`\x1f`-separated) instead.

### Source replacement

Source: [source replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html).

- Two strategies: **vendoring** (local `directory` of unpacked `*.crate` sources, via `cargo vendor`) and **mirroring** (a registry replacement).
- Core assumption: the replacement source must contain the **same** crates (no extra, none missing). Source replacement is **not** for patching a single crate (use `[patch]`) nor for private registries.
- Git sources **cannot** replace registry sources.
- `[source.crates-io] replace-with = "name"` + `[source.name] directory = "vendor"` (or `local-registry`/`registry`/`git`).
- When using source replacement, registry-contacting commands (e.g. `cargo publish`) require `--registry`.

### Environment variables

Source: [environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html).

Cargo reads:

- `CARGO_HOME`, `CARGO_TARGET_DIR`, `RUSTC`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `RUSTDOC`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `RUSTDOCFLAGS`, `CARGO_INCREMENTAL`, `CARGO_LOG`, `HTTPS_PROXY`/`HTTP_PROXY`, `BROWSER`, etc.

Cargo sets for the crate (read via `env!`):

- `CARGO_PKG_*`: `VERSION` + `_MAJOR`/`_MINOR`/`_PATCH`/`_PRE`, `AUTHORS`, `NAME`, `DESCRIPTION`, `HOMEPAGE`, `REPOSITORY`, `LICENSE`, `LICENSE_FILE`, `RUST_VERSION`, `README`
- `CARGO_MANIFEST_DIR`/`_PATH`, `CARGO_CRATE_NAME`, `CARGO_BIN_NAME`, `OUT_DIR`, `CARGO_BIN_EXE_<name>`, `CARGO_PRIMARY_PACKAGE`, `CARGO_TARGET_TMPDIR`

For build scripts:

- All `CARGO_PKG_*` + `CARGO_MANIFEST_LINKS`, `CARGO_MAKEFLAGS` (jobserver), `CARGO_FEATURE_<NAME>`, `CARGO_CFG_*` (`TARGET_OS`, `TARGET_ARCH`, `TARGET_FAMILY`, `UNIX`, `WINDOWS`, etc.; `cfg(test)` **not** exposed), `OUT_DIR`, `TARGET`, `HOST`, `NUM_JOBS`, `DEBUG`, `OPT_LEVEL`, `PROFILE` (not recommended for accurate checks — use `OPT_LEVEL`), `DEP_<links>_<key>`, `RUSTC`, `RUSTDOC`, `RUSTC_LINKER`, `CARGO_ENCODED_RUSTFLAGS`.

For `cargo test`:

- `CARGO_BIN_EXE_<name>` (absolute path to a bin target's executable, for integration tests/benches).

### Commands

Sources: the `cargo/commands/*.html` pages.

- `cargo check`: typecheck without codegen/linking. **Note:** some diagnostics only appear during codegen, so `cargo check` cannot catch link errors. Canonical fast validation: `cargo check --all-targets --all-features`. Flags: `-p/--package`, `--workspace`/`--all`(deprecated), `--exclude`, `--lib/--bin/--bins/--example/--examples/--test/--tests/--bench/--benches`, `--all-targets`, `-F/--features`, `--all-features`, `--no-default-features`, `--target`, `-r/--release`, `--profile`, `--timings`, `--target-dir`, `--message-format` (human/short/json/...), `-j/--jobs`, `--keep-going`, `--locked`, `--offline`, `--frozen`, `--ignore-rust-version`.
- `cargo build`: full codegen + linking. Same selection flags. `--release`.
- `cargo test`: compile + run unit/integration/doc tests. Test name and everything after `--` goes to libtest. Has **no** `--keep-going`; use `--no-fail-fast`. `--doc` cannot mix with other target flags. `-j` affects build only; libtest thread count via `-- --test-threads=N`. Show output via `-- --nocapture`. `--no-run` = compile only.
- `cargo clippy`: external subcommand; the Cargo Book page is a stub. Practical: `cargo clippy --workspace --all-targets --all-features -- -D warnings`. `--fix` auto-applies (use `--allow-dirty --allow-staged`). Relates to `[lints.clippy]` table (respected since 1.74; applies to local package only, not deps). See <https://doc.rust-lang.org/clippy/>.
- `cargo fmt`: external subcommand (rustfmt). Cargo Book page is a stub. `cargo fmt --all -- --check` is the CI gate. rustfmt config: `rustfmt.toml`/`.rustfmt.toml`. See <https://github.com/rust-lang/rustfmt>.
- `cargo tree`: defaults to edges `normal,build,dev` (≈ `cargo test` view); `-e normal,build` ≈ `cargo build` view. `-d`/`--duplicates` (semver-incompatible dupes, implies `--invert`); `-i`/`--invert` (reverse deps; great with `-e features` to explain why a feature is enabled); `-e features/no-dev/no-build/no-proc-macro`; `--target`; `--depth workspace`; `--no-dedupe`; `-f/--format` (`{p}` `{l}` `{r}` `{f}` `{lib}`).
- `cargo update`: conservative by default. `spec` limits to a package (Package ID Spec globs); transitive deps update only if required. `--recursive` forces transitive updates (**cannot** be used with `--precise`). `--precise <ver|sha|tag>` pins. `-w/--workspace` updates only workspace packages (useful after editing `Cargo.toml` version specs). `--dry-run`. (Yanked versions can be selected with `--precise`, though not recommended.)
- `cargo add`: adds to `Cargo.toml` (+ updates lock). Source: `crate@version` (registry, default), `--path`, `--git` (+branch/tag/rev), `--registry`. Section: `--dev`, `--build`, `--target <cfg>`. Dep: `--rename`, `--optional`/`--no-optional`, `--default-features`/`--no-default-features`, `-F/--features` (repeatable; supports `pkg/feat`). `-p/--package`, `--dry-run`.
- `cargo remove`: companion; `--dev/--build/--target`, `-p`, `--dry-run`.
- `cargo metadata`: **always** pass `--format-version=1` (the only value). Within v1, adding fields/values is **not** a breaking change. `--no-deps` (`resolve=null`), `--filter-platform <triple>` (`"host-tuple"` for host). Output: `packages[]`, `workspace_members[]`, `workspace_default_members[]`, `resolve`, `target_directory`, `workspace_root`, `metadata`. `id` is a parseable Package ID Spec since MSRV 1.77.
- `cargo vendor`: vendors crates.io + git deps; prints source-replacement TOML to stdout (redirect to `.cargo/config.toml`). Flags: `-s/--sync`, `--no-delete`, `--respect-source-config`, `--versioned-dirs`.
- `cargo package`: builds `.crate` tarball in `target/package`; always includes `Cargo.lock`; rewrites `Cargo.toml` (removes `[patch]`/`[replace]`/`[workspace]`); adds `.cargo_vcs_info.json`; verifies by extracting + building. Flags: `-l/--list`, `--no-verify`, `--no-metadata`, `--allow-dirty`, `--exclude-lockfile`.
- `cargo publish`: runs `cargo package` then uploads (default registry crates.io; needs `cargo login`/`CARGO_REGISTRY_TOKEN`). `--dry-run`, `--no-verify`, `--allow-dirty`, `--registry`.

### MSRV

Source: [rust-version](https://doc.rust-lang.org/cargo/reference/rust-version.html).

- `rust-version = "1.74"`: bare version, no semver operators or pre-release. Respected since Rust 1.56.
- Effects: clear error on unsupported toolchain (opt out via `--ignore-rust-version`); `cargo add` selects compatible versions; with `resolver.incompatible-rust-versions = "fallback"` the resolver prefers compatible versions.
- Changing `rust-version` is treated as a possibly-breaking/minor-incompatible change (SemVer).
- CI: `cargo +<msrv> check --all-targets`. Also drives lock file `version` selection (see [Cargo.lock](#cargolock)) and resolver v3 fallback.

## Practical rules

1. Always specify `edition` in `[package]`. Use the latest stable edition unless the project has an explicit reason not to.
2. Always specify `rust-version` in `[package]` to declare the Minimum Supported Rust Version (MSRV).
3. Use `cargo add <crate>` instead of hand-editing `[dependencies]` — it normalizes version syntax and updates `Cargo.lock`.
4. Commit `Cargo.lock` for binaries and applications. Libraries may omit it, but commit when in doubt (Cargo's recommendation).
5. Use `resolver = "2"` (or `"3"` for edition 2024 / Rust 1.84+) in every workspace and in standalone crates with `[lints]`, dev-dep feature splits, or build-deps.
6. Prefer caret requirements (`^`) for production dependencies. Use exact (`=`) only for known-incompatible upstream crates or tightly-coupled library/proc-macro pairs.
7. Never use path-only dependencies in published crates — they break consumers who lack the local path. You may combine `path` with `version` for workspace development.
8. Run `cargo tree -d` before merging to detect duplicate dependency versions.
9. Keep `default` features minimal. Consumers should opt into additional functionality, not opt out of bloat.
10. Document every public feature flag in the crate's README or module-level doc comment.
11. Use `[workspace.dependencies]` to centralize version requirements across workspace members.
12. Never hand-edit `Cargo.lock` — use `cargo update` or `cargo update -p <crate> --precise <version>`.
13. Use `[profile.release]` tuning (LTO, strip, codegen-units) for production binaries, but keep dev profiles fast. Measure rather than guess.
14. Pin git dependencies to a specific `rev` or `tag`, never a bare `branch`, in production manifests.
15. Run `cargo check --all-targets --all-features` as the fastest comprehensive validation before `cargo test` — but remember it cannot catch link errors.
16. Do not add the deprecated `authors` field to new manifests.
17. Use `dep:<name>` syntax (Rust 1.60+) for optional dependencies in feature definitions.
18. Ensure features are additive; never design mutually exclusive features.

## Review checklist

1. Does `Cargo.toml` specify `edition` and `rust-version`?
2. Are all dependency versions using caret requirements unless there is a documented reason for exact or tilde?
3. Is `Cargo.lock` committed for binaries? For libraries, is the lock-file policy documented?
4. Are there duplicate dependency versions? Run `cargo tree -d`.
5. Are features additive? No pair of features should cause a compilation error when both are enabled.
6. Is `default = []` declared explicitly if the crate has no default features?
7. Does the workspace use `resolver = "2"` or `"3"`?
8. Are workspace dependency versions centralized in `[workspace.dependencies]` with members using `workspace = true`?
9. Are git dependencies pinned to a specific `rev` or `tag`?
10. Are there any path-only dependencies that would break published crates?
11. Is `[profile.release]` configured for production binaries (LTO, strip, codegen-units)?
12. Are dev-dependencies only used for tests, benchmarks, and examples — not leaked into the public API?
13. Is `Cargo.lock` updated intentionally (not as a side-effect of an unrelated change)?
14. Does `cargo check --all-targets --all-features` pass cleanly?
15. Is `[lints]` used with resolver v2+?
16. Are optional dependencies declared with `dep:<name>` in feature definitions?

## Implementation checklist

1. Use `cargo add <crate>` to add dependencies — it handles version syntax and `Cargo.lock`.
2. After adding a dependency, run `cargo check` to verify resolution and compilation.
3. Run `cargo tree -d` to check for duplicate versions introduced by the new dep.
4. When adding a feature, ensure it is additive and documented in the README or `#[doc]`.
5. When adding a workspace member, add it to `workspace.members` and inherit shared metadata with `workspace = true`.
6. When changing a dependency version, run `cargo update -p <crate>` and verify the lock file diff.
7. When adding `[lints]`, ensure `resolver = "2"` (or `"3"`) is set.
8. When adding a git dependency, pin to `rev` or `tag` and add a comment explaining why crates.io is insufficient.
9. When adding optional dependencies, use the `dep:` prefix in feature definitions to avoid implicit feature names.
10. When configuring `[profile.release]`, benchmark before and after to justify LTO and codegen-unit changes.
11. When setting MSRV, verify with `cargo +<msrv> check` in CI.
12. When adding `[patch]` entries, add a TODO comment with the upstream issue or PR that will make the patch unnecessary.
13. Run `cargo tree -i <crate>` to understand the reverse dependency graph before removing or upgrading a crate.
14. When using workspace inheritance, remember that `optional` cannot be declared in `[workspace.dependencies]`; members add it themselves, and workspace-declared `features` are additive.

## Validation hooks

```bash
# Fast typecheck across all targets and features
cargo check --all-targets --all-features

# Full release build
cargo build --release

# Run all tests in the workspace
cargo test --workspace

# Find duplicate dependency versions
cargo tree -d

# Inverse dependency tree (who depends on this crate?)
cargo tree -i <crate>

# Show feature-driven dependency edges
cargo tree -e features

# Update a single dependency
cargo update -p <crate>

# Pin a dependency to an exact version
cargo update -p <crate> --precise <version>

# Update only workspace packages after version edits
cargo update --workspace

# Machine-readable manifest metadata
cargo metadata --format-version 1 | jq .

# Verify MSRV compatibility
cargo +<msrv> check

# Lint the workspace
cargo clippy --workspace --all-targets --all-features

# Format check
cargo fmt --all -- --check
```

See [cargo-check](https://doc.rust-lang.org/cargo/commands/cargo-check.html), [cargo-build](https://doc.rust-lang.org/cargo/commands/cargo-build.html), [cargo-test](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [cargo-clippy](https://doc.rust-lang.org/cargo/commands/cargo-clippy.html), and [cargo-fmt](https://doc.rust-lang.org/cargo/commands/cargo-fmt.html) for full CLI documentation.

## Examples

### Complete `Cargo.toml` for a binary

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
license = "MIT OR Apache-2.0"
description = "A production service for processing events"
repository = "https://github.com/org/my-app"
readme = "README.md"
keywords = ["events", "processing"]
categories = ["command-line-utilities"]
resolver = "2"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"

[features]
default = ["tls"]
tls = ["dep:tokio-rustls"]

[dependencies.tokio-rustls]
version = "0.26"
optional = true

[profile.release]
lto = "thin"
strip = "symbols"
codegen-units = 1
panic = "abort"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
```

### Workspace root `Cargo.toml` with `[workspace.dependencies]`

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
license = "MIT OR Apache-2.0"
repository = "https://github.com/org/my-workspace"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tracing = "0.1"
anyhow = "1"
thiserror = "1"

# Dev-only shared dependencies
assert_cmd = "2"
tempfile = "3"
```

### Member crate using workspace inheritance

```toml
[package]
name = "my-crate"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
assert_cmd = { workspace = true }
tempfile = { workspace = true }
```

### `[features]` table with additive features

```toml
[features]
default = ["json"]

# Each feature adds capability; any combination compiles correctly.
json = ["dep:serde_json", "serde/derive"]
yaml = ["dep:serde_yaml", "serde/derive"]
tls = ["dep:tokio-rustls"]
observability = ["dep:tracing-subscriber"]

# Optional dependency — only pulled in when the feature is enabled.
[dependencies]
serde = { version = "1", optional = true }
serde_json = { version = "1", optional = true }
serde_yaml = { version = "0.9", optional = true }
tokio-rustls = { version = "0.26", optional = true }
tracing-subscriber = { version = "0.3", optional = true }
```

### `[profile.release]` tuning

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "thin"         # Thin link-time optimization (good size/speed trade-off)
codegen-units = 1    # Single codegen unit; most effective when lto is enabled
strip = "symbols"    # Strip symbols from binary
panic = "abort"      # Smaller binary, no unwinding
incremental = false  # Disable incremental for reproducible builds

# Custom profile inheriting from release but with debug info
[profile.release-with-debug]
inherits = "release"
debug = "line-tables-only"
strip = "none"
```

See [profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) for all available settings.

### `.cargo/config.toml` snippet

```toml
[build]
target = "x86_64-unknown-linux-gnu"
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-unknown-linux-gnu]
linker = "gcc"

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
git-fetch-with-cli = true
retry = 3

[alias]
check-all = "check --all-targets --all-features"
lint = "clippy --workspace --all-targets --all-features"
```

See [config](https://doc.rust-lang.org/cargo/reference/config.html) and [source replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html).

### `cargo tree -d` output interpretation

```text
cargo tree -d
serde v1.0.195
├── serde_json v1.0.111
│   └── my-crate v0.1.0 (/path/to/my-crate)
└── my-crate v0.1.0 (/path/to/my-crate)
serde v1.0.130
└── legacy-dep v0.2.0
    └── my-crate v0.1.0 (/path/to/my-crate)
```

This output shows two versions of `serde` in the dependency graph. `legacy-dep` requires `serde v1.0.130` while the rest of the graph uses `v1.0.195`. Resolution: upgrade `legacy-dep` to a version compatible with the newer `serde`, or if impossible, use `[patch]` to override. See [cargo-tree](https://doc.rust-lang.org/cargo/commands/cargo-tree.html).

### Combining `version` with `path` for workspace development

```toml
[dependencies]
my-shared = { path = "../my-shared", version = "0.1.0" }
```

This uses the local path during development and the crates.io version when the crate is published. Path-only deps are forbidden on crates.io; the `version` key makes the declaration publishable.

### `[patch.crates-io]` override

```toml
[patch.crates-io]
serde = { git = "https://github.com/serde-rs/serde", rev = "abc123" }
```

Use `[patch]` to temporarily replace a registry dependency with a git or path source. Always add a TODO comment with the upstream issue or PR that will remove the need for the patch.

## Common mistakes

### 1. Adding a crate to both `[dependencies]` and `[dev-dependencies]`

```toml
# WRONG: serde appears in both tables with different versions
[dependencies]
serde = "1"

[dev-dependencies]
serde = "1.0.130"  # Unnecessary — dev-deps can use the [dependencies] version
```

Dev-dependencies are only compiled for tests, benchmarks, and examples. If the crate already depends on `serde` in `[dependencies]`, tests can use it directly. Only add to `[dev-dependencies]` if the crate is not needed at runtime.

### 2. Using `features = ["full"]` on large crates

```toml
# WRONG: pulls in every optional feature of tokio
tokio = { version = "1", features = ["full"] }
```

The `full` feature in tokio enables every sub-feature, increasing compile time and binary size. Enable only the features you use: `features = ["rt-multi-thread", "macros", "net"]`. Source: [features](https://doc.rust-lang.org/cargo/reference/features.html).

### 3. Publishing breaking changes under `^0.x.y`

```toml
# DANGEROUS: caret requirement ^0.1.0 means >=0.1.0, <0.2.0
# If upstream publishes 0.1.1 with a breaking change, your code breaks
some-crate = "0.1.0"
```

Per Cargo's version semantics, `^0.1.0` allows `0.1.z` but not `0.2.0`. However, some `0.x` crates still make breaking changes in patch versions. Pin with `=0.1.2` if the upstream is unreliable. Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

### 4. Mutually exclusive features

```toml
# WRONG: enabling both "backend-a" and "backend-b" causes compile errors
[features]
backend-a = ["dep:backend-a-lib"]
backend-b = ["dep:backend-b-lib"]
```

Features are additive. If both `backend-a` and `backend-b` are enabled simultaneously (which Cargo allows), the crate must still compile. Use a single feature with values, or use `cfg` guards that handle the "both enabled" case gracefully. See [features](https://doc.rust-lang.org/cargo/reference/features.html).

### 5. Forgetting `resolver = "2"` when using `[lints]`

```toml
# WRONG: [lints] requires resolver = "2" but the workspace uses the default v1
[workspace]
members = ["crates/*"]
# Missing: resolver = "2"

[workspace.lints.clippy]
unwrap_used = "warn"
```

The `[lints]` table requires the v2+ feature resolver. Without `resolver = "2"` or `"3"`, Cargo will emit an error. See [resolver](https://doc.rust-lang.org/cargo/reference/resolver.html).

### 6. Hand-editing `Cargo.lock`

Never edit `Cargo.lock` by hand. The lock file is machine-generated and has a specific format. Use `cargo update -p <crate>` to update a single dependency or `cargo update` to update all. Use `--precise <version>` to pin to a specific version. Do not edit the top-level `version` field. See [cargo-update](https://doc.rust-lang.org/cargo/commands/cargo-update.html) and [Cargo.lock](#cargolock).

### 7. Using path-only dependencies in published crates

```toml
# WRONG: consumers cannot resolve "../my-shared-lib"
[dependencies]
my-shared-lib = { path = "../my-shared-lib" }
```

Path-only dependencies work locally but break when the crate is published to crates.io. Either add a `version` key (`{ path = "...", version = "..." }`) for workspace dev + publish, publish the dependency separately and use a version requirement, or restructure into a workspace. See [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

### 8. Mixing git and registry versions of the same crate

```toml
# WRONG: duplicate TOML key; even if rewritten, two sources for the same crate
# are forbidden. Use [patch.crates-io] instead.
[dependencies]
serde = "1"
serde = { git = "https://github.com/serde-rs/serde", branch = "main" }
```

Cargo cannot resolve the same crate from two different sources. If you need a git version of a crate already on crates.io, use `[patch]` to replace the registry version:

```toml
[patch.crates-io]
serde = { git = "https://github.com/serde-rs/serde", rev = "abc123" }
```

Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

### 9. Not specifying `rust-version` and breaking users on older toolchains

Without `rust-version`, Cargo cannot warn users that their toolchain is too old. Always set it:

```toml
[package]
rust-version = "1.74"
```

Source: [rust-version](https://doc.rust-lang.org/cargo/reference/rust-version.html).

### 10. Using bare `branch` for git dependencies in production

```toml
# DANGEROUS: "main" moves — builds are not reproducible
my-dep = { git = "https://github.com/org/my-dep", branch = "main" }
```

Always pin to a `rev` or `tag` for reproducibility:

```toml
my-dep = { git = "https://github.com/org/my-dep", rev = "a1b2c3d" }
```

Source: [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

### 11. Assuming `cargo check` catches all build errors

`cargo check` only typechecks; it does not codegen or link. Some diagnostics (especially link errors and certain monomorphization errors) appear only during `cargo build`. Always run `cargo build` (or `cargo test`) before considering a change validated. Source: [cargo-check](https://doc.rust-lang.org/cargo/commands/cargo-check.html).

### 12. Relying on `default-features = false` to keep defaults off

```toml
# WRONG assumption: serde's default features are disabled globally just because
# one crate opts out. Feature unification means the union wins.
[dependencies]
serde = { version = "1", default-features = false }
```

`default-features = false` is per-declaration. If any other package in the graph enables serde's defaults, serde is built with defaults enabled. To truly disable defaults, every declaration must opt out. Source: [features](https://doc.rust-lang.org/cargo/reference/features.html).

## Strict vs contextual guidance

### Strict guidance

- **Commit `Cargo.lock` for binaries and applications.** Libraries may omit it, but Cargo recommends committing when in doubt. See [Cargo.lock guidance](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) and [Cargo FAQ](https://doc.rust-lang.org/cargo/faq.html).
- **Use `^` (caret) SemVer requirements for production dependencies.** Exact pins (`=`) are acceptable only with documented justification.
- **One source of truth for lint config and dependency versions.** Use `[workspace.dependencies]` and `[workspace.lints]` at the workspace level; members inherit with `workspace = true`.
- **Use `resolver = "2"` or `"3"`.** Required for `[lints]`, dev-dep feature resolution, and build-dep isolation. See [resolver](https://doc.rust-lang.org/cargo/reference/resolver.html).
- **Specify `rust-version` in `[package]`.** Without it, users on older toolchains get confusing compiler errors instead of a clear Cargo warning.
- **Never use `cargo update` blindly.** Always specify `-p <crate>` or review the full diff. Blind updates can pull in SemVer-compatible but behavior-changing patch versions.
- **Never use path-only dependencies in published crates.** They are invisible to crates.io consumers. Combine `path` with `version` for workspace dev/publish workflows.
- **Features must be additive.** Any combination of enabled features must compile. Mutually exclusive features are a design error.
- **Do not add the `authors` field to new manifests.** It is deprecated.
- **Never hand-edit `Cargo.lock`.** Use `cargo update` and `--precise`.

### Common convention

- Keep `default` features minimal. Consumers should opt in, not opt out.
- Document every public feature flag in the crate's README or module-level doc comment.
- Use `cargo add` instead of hand-editing `Cargo.toml` — it normalizes syntax and updates `Cargo.lock`.
- Use `cargo tree -d` to find duplicate versions before merging.
- Pin git dependencies to `rev` or `tag`, not `branch`.
- Use `[profile.release]` with `lto = "thin"` and `codegen-units = 1` for production binaries, but measure rather than guess.
- Run `cargo check --all-targets --all-features` as the fastest pre-test validation.
- Use `dep:<name>` syntax (Rust 1.60+) for optional dependencies in features.

### Contextual tradeoffs

- **Git deps vs crates.io.** Git dependencies are appropriate for unreleased upstream work or urgent hotfixes. They trade reproducibility (moving `branch`) and supply-chain assurance for access to unreleased code. Pin to `rev` and add a TODO for migration to crates.io.
- **`~` vs `^` version requirements.** Tilde (`~1.2.3` → `>=1.2.3, <1.3.0`) is more restrictive than caret (`^1.2.3` → `>=1.2.3, <2.0.0`). Use tilde when the upstream crate has a history of breaking changes in minor versions within a major version. Use caret otherwise.
- **`lto = "fat"` vs `lto = "thin"` vs no LTO.** Fat LTO produces the smallest, fastest binaries but has the longest build time. Thin LTO is a good middle ground. No LTO is fastest to build. Choose based on deployment frequency and binary size constraints.
- **Path deps for in-monorepo components vs published crates.** Path dependencies are natural in a monorepo workspace. When a crate needs to be published, convert path-only deps to version deps (or add `version` alongside `path`) and ensure the workspace publishes in dependency order.
- **Vendoring (`cargo vendor`) vs live resolution.** Vendoring provides offline builds and supply-chain auditing but increases repo size. Live resolution is simpler but requires network access and trust in registries. See [source replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html).
- **`resolver.incompatible-rust-versions = "fallback"`.** With resolver v3 (or explicit config), Cargo prefers dependency versions whose `rust-version` ≤ your MSRV. Use this to keep CI green on older toolchains, but be aware it may select older crates.

## Policy decisions for individual repos

Each repository should explicitly decide and document the following:

1. **Approved dependency list.** Which crates are permitted based on license compatibility, maintenance activity, and supply-chain risk? Maintain an allow-list or use `cargo vet` for auditing. See `supply-chain-security.md`.

2. **Required feature flags for the default build.** Which features must always be enabled? For example, a service that always uses TLS should include `tls` in `default` features rather than requiring consumers to remember to enable it.

3. **MSRV policy.** How far back does the project support? This determines `rust-version` in `[package]` and the CI matrix. Common choices: current stable minus 2, or the version shipped in the oldest supported distribution.

4. **Vendoring policy.** Does the project vendor dependencies for offline/reproducible builds (`cargo vendor`), or rely on live resolution from crates.io? Vendoring increases repo size but enables air-gapped builds and supply-chain auditing.

5. **`[patch]` policy.** Under what circumstances are `[patch]` entries acceptable? Typical policy: only for urgent upstream fixes, with a tracked upstream issue or PR, and a TODO comment for removal.

6. **Profile defaults.** Does the project standardize on `lto = "thin"` for all release builds, or allow per-crate tuning? Document the default and any exceptions.

7. **Custom registry usage.** Does the project use a private registry for internal crates? If so, document the registry URL and authentication in `.cargo/config.toml`. See [config](https://doc.rust-lang.org/cargo/reference/config.html).

8. **Lock file policy for mixed crates.** If the workspace contains both libraries and binaries, commit the lock file (the binary requirement wins). Document this decision.

9. **Git dependency policy.** When are git dependencies allowed? Require pinning to `rev` or `tag` and a documented migration plan to crates.io.

10. **Feature-additivity enforcement.** How will the project detect mutually exclusive features? Options: CI matrix building `--all-features`, `compile_error!` guards, or feature-design review.

## Related docs

- `unsafe-security.md` — security considerations for unsafe code, including dependency auditing
- `lints-clippy.md` — detailed clippy lint configuration and `[lints]` table usage
- `supply-chain-security.md` — dependency supply-chain security, `cargo vet`, and auditing
- `editions-tooling.md` — edition migration, toolchain management, and `rustup` configuration
- `testing.md` — Rust testing strategy and test target configuration

## Related skills

No repo-specific skills for this topic.
