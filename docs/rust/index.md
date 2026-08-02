# Rust Guidance Index

## Purpose

This corpus is a set of repo-independent Rust guidance documents for future AI coding agents that write, review, refactor, debug, and validate Rust code. The 18 files cover the language, standard library, ecosystem tooling, and project mechanics needed to produce idiomatic, safe, and maintainable Rust. Each document derives its rules from official Rust project sources and records the policy decisions that individual repositories must still make for themselves.

## How to use this corpus

Start with the scenario that matches your task, then read the recommended docs in order. Each doc is self-contained for its topic but cross-references siblings where mechanics overlap. For implementation work, read the relevant language or API doc first, then consult the matching workflow file (generated under `docs/rust/workflows/`) for step-by-step execution. For reviews, read the relevant docs and the code-review workflow. For CI or toolchain setup, read the Cargo, edition, lint, style, and supply-chain docs together. The corpus is also the source for future skills under `.agents/skills/` and workflows under `docs/rust/workflows/`.

## Generated files

### 1. `docs/rust/api-design.md`

- **Purpose:** Generic guidance for designing public Rust APIs, derived from the official Rust API Guidelines and RFC 1105.
- **Main topics:**
  - Naming conventions (RFC 430, `as_`/`to_`/`into_`)
  - Interoperability and common traits
  - Macros, predictability, flexibility, and type safety
  - Dependability, debuggability, future-proofing, and necessities
  - Builders, newtypes, sealed traits, and `#[non_exhaustive]`
- **When a future agent should read it:** When designing or reviewing a public API surface, crate naming, semver-sensitive changes, or trait/exposure choices.
- **Related skill files:** `.agents/skills/rust-api-design/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 2. `docs/rust/documentation-guidelines.md`

- **Purpose:** Conventions for writing rustdoc-compatible documentation, doctests, intra-doc links, and the C-CRATE-DOC through C-HIDDEN checklist items.
- **Main topics:**
  - Doc comment forms (`///` vs `//!`)
  - Per-item documentation structure
  - Doctest attributes and fenced examples
  - Intra-doc links and namespace disambiguators
  - `#[doc]` attributes and `Cargo.toml` metadata
  - Release notes, hidden implementations, and lint enforcement
- **When a future agent should read it:** When writing crate-level docs, public examples, `# Safety` sections, or setting rustdoc lint policy.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 3. `docs/rust/design-patterns.md`

- **Purpose:** A catalog of Rust design patterns, idioms, anti-patterns, and functional patterns drawn from the Rust Design Patterns book and ecosystem conventions.
- **Main topics:**
  - Newtype, builder, RAII guards, strategy, visitor, command
  - Functional patterns (lenses/optics, generics as type-classes)
  - Idioms (`Default`, `Deref`, `mem::replace`, `Cow`, FFI string handling)
  - Anti-patterns (`borrow_clone`, `deny-warnings`, `Deref` abuse)
  - Pattern decision frameworks and type-state machines
- **When a future agent should read it:** When choosing a pattern-level abstraction, refactoring toward idiomatic Rust, or reviewing for pattern misuse.
- **Related skill files:** `.agents/skills/rust-api-design/SKILL.md` (generated, partial)
- **Related workflow files:** See `docs/rust/workflows/`

### 4. `docs/rust/style-formatting.md`

- **Purpose:** Canonical formatting guidance centered on `rustfmt`, the Rust Style Guide, and the 2024 style edition.
- **Main topics:**
  - `cargo fmt` and CI enforcement
  - `rustfmt.toml` stable vs nightly options
  - `edition` vs `style_edition`
  - Import grouping, trailing commas, and `#[rustfmt::skip]` policy
  - 2024 style-edition formatting changes
  - rust-analyzer / VS Code integration
- **When a future agent should read it:** When setting up formatting, creating `rustfmt.toml`, or reviewing formatting-only PRs.
- **Related skill files:** `.agents/skills/rust-lints-and-clippy/SKILL.md` (generated, partial)
- **Related workflow files:** See `docs/rust/workflows/`

### 5. `docs/rust/ownership-lifetimes.md`

- **Purpose:** Canonical reference for Rust's ownership, borrowing, lifetimes, variance, `Pin`, and the compile-time aliasing discipline.
- **Main topics:**
  - Ownership rules, moves, `Copy`, `Clone`, and `Drop`
  - Place vs value expressions and partial moves
  - References, mutable borrows, and non-lexical lifetimes
  - Slices, lifetime elision, and `'static`
  - Variance, higher-ranked trait bounds, and `Pin`
- **When a future agent should read it:** When debugging borrow-checker errors, refactoring ownership, or reviewing lifetime annotations.
- **Related skill files:** `.agents/skills/rust-ownership-borrowing/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 6. `docs/rust/types-traits-generics.md`

- **Purpose:** Guidance on structs, enums, traits, generics, conversions, pattern matching, and `PhantomData` variance.
- **Main topics:**
  - Structs, enums, and pattern matching
  - Traits, object safety, and sealed traits
  - Generics, dispatch, and `impl Trait`
  - `From`/`Into`/`TryFrom`/`AsRef`/`FromStr` conversions
  - Newtypes, `PhantomData`, const generics, and derive policy
- **When a future agent should read it:** When modeling a domain in the type system, choosing generics vs `dyn Trait`, or implementing conversions.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 7. `docs/rust/modules-visibility.md`

- **Purpose:** Authoritative guidance on packages, crates, modules, visibility, path qualifiers, re-exports, and Cargo project layout.
- **Main topics:**
  - Package/crate/module hierarchy
  - Visibility grammar (`pub`, `pub(crate)`, `pub(in path)`)
  - Absolute vs relative paths and `use` idioms
  - File resolution, `#[path]`, and auto-discovery opt-outs
  - Library + binary pattern and facade design
- **When a future agent should read it:** When structuring a new crate, reorganizing modules, or fixing visibility errors.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 8. `docs/rust/iterators-closures.md`

- **Purpose:** Guidance on closures, iterators, functional combinators, capture modes, and zero-cost abstraction patterns.
- **Main topics:**
  - Closure syntax, capture modes, and `move`
  - `Fn`/`FnMut`/`FnOnce` hierarchy
  - Iterator constructors, combinators, and lazy evaluation
  - `collect`, `try_fold`, and error handling in chains
  - `for_each` policy, string iteration, and `rayon` thresholds
- **When a future agent should read it:** When writing iterator chains, choosing closure capture modes, or reviewing functional-style code.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 9. `docs/rust/smart-pointers-memory.md`

- **Purpose:** Guidance on choosing and using smart pointers and memory-management primitives (`Box`, `Rc`, `Arc`, `Cell`, `RefCell`, `Mutex`, `RwLock`, `Cow`, etc.).
- **Main topics:**
  - Ownership-mutability matrix
  - `Box<T>`, `Rc<T>`, `Arc<T>`, `Weak<T>`
  - `Cell<T>`, `RefCell<T>`, `UnsafeCell<T>`
  - `OnceCell<T>`, `OnceLock<T>`, `Mutex<T>`, `RwLock<T>`
  - `Cow<'a, B>`, `Deref`, `Drop`, `MaybeUninit`, `ManuallyDrop`
- **When a future agent should read it:** When choosing a heap/shared pointer, adding interior mutability, or reviewing custom destructors.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 10. `docs/rust/error-handling.md`

- **Purpose:** Comprehensive reference on `Option`, `Result`, `?`, panic semantics, error-type design, and ecosystem crates (`thiserror`, `anyhow`, `miette`, `color-eyre`).
- **Main topics:**
  - `Option` vs `Result` and `#[must_use]`
  - Panic vs recoverable errors
  - Combinator catalogs and short-circuiting `collect`
  - Implementing `std::error::Error`
  - `unwrap`/`expect` conventions and backtraces
- **When a future agent should read it:** When designing error types, converting errors, or reviewing `unwrap`/`expect`/`panic` usage.
- **Related skill files:** `.agents/skills/rust-error-handling/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 11. `docs/rust/async-tokio.md`

- **Purpose:** Reference for async/await, the `Future` trait, pinning, wakers, and the Tokio runtime ecosystem.
- **Main topics:**
  - Async/await mental model and futures as state machines
  - `Pin`, wakers, executors, and `Send` + `'static`
  - Tokio runtime flavors, spawning, and `JoinHandle`
  - `join!`, `select!`, channels, and synchronization
  - Cancellation, blocking work, time, process, signal, and net APIs
- **When a future agent should read it:** When writing, reviewing, or refactoring async Rust code with Tokio.
- **Related skill files:** `.agents/skills/rust-async-tokio/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 12. `docs/rust/std-runtime-apis.md`

- **Purpose:** Practical guidance on synchronous standard-library runtime APIs: env, path, fs, io, process, and time.
- **Main topics:**
  - `std::env` variables and constants
  - `std::path` portability and sandboxing
  - `std::fs` operations and atomic writes
  - `std::io` buffering, readers, writers, and error kinds
  - `std::process` subprocess policy and `std::time` monotonic timeouts
- **When a future agent should read it:** When writing CLI tools, file handling, subprocess spawning, or portable path logic.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

### 13. `docs/rust/testing.md`

- **Purpose:** Guidance for writing, organizing, and running Rust tests, doctests, integration tests, and ecosystem test tooling.
- **Main topics:**
  - Test attributes and assertion macros
  - Returning `Result<T, E>` from tests and `#[should_panic]`
  - Unit, integration, and doctest organization
  - `cargo test` flags, environment variables, and target configuration
  - Property-based testing, mocking, fuzzing, and async tests
- **When a future agent should read it:** When adding tests, setting up test helpers, or configuring the test harness.
- **Related skill files:** `.agents/skills/rust-testing/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 14. `docs/rust/unsafe-security.md`

- **Purpose:** Practical guidance on `unsafe` Rust: soundness invariants, UB catalog, FFI, raw pointers, atomics, `Pin`, and validation tooling.
- **Main topics:**
  - What `unsafe` permits and the two roles of the keyword
  - Safety invariants and the complete undefined-behavior catalog
  - 2024 edition unsafe changes
  - FFI, ABI, `no_mangle`, and `unsafe extern`
  - `Send`/`Sync`, `transmute`, raw pointers, `MaybeUninit`, atomics, Miri
- **When a future agent should read it:** When writing, reviewing, or refactoring any `unsafe` code, FFI boundary, or manual `Send`/`Sync` impl.
- **Related skill files:** `.agents/skills/rust-unsafe-review/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 15. `docs/rust/lints-clippy.md`

- **Purpose:** Comprehensive guidance on Clippy, rustc lints, the `[lints]` table, workspace inheritance, `clippy.toml`, and CI integration.
- **Main topics:**
  - Lint levels (`allow`, `warn`, `deny`, `forbid`, `expect`, `force-warn`)
  - Clippy groups (`all`, `pedantic`, `nursery`, `cargo`, `restriction`)
  - Notable individual lints and restriction lints
  - `[lints]` table, workspace inheritance, and scoped allows
  - `clippy.toml` configuration and MSRV interaction
- **When a future agent should read it:** When configuring lint policy, setting up `cargo clippy` in CI, or reviewing lint suppressions.
- **Related skill files:** `.agents/skills/rust-lints-and-clippy/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 16. `docs/rust/cargo-dependencies.md`

- **Purpose:** Guidance on `Cargo.toml`, dependency management, features, resolver behavior, workspaces, profiles, and Cargo commands.
- **Main topics:**
  - `[package]`, dependency tables, and version requirements
  - Dependency sources, `Cargo.lock`, and additive features
  - Resolver versions, workspaces, and profiles
  - `.cargo/config.toml`, build scripts, and source replacement
  - MSRV, `cargo add`/`remove`/`tree`/`vendor`, and complete manifest examples
- **When a future agent should read it:** When creating or modifying manifests, resolving dependency issues, or configuring workspaces.
- **Related skill files:** `.agents/skills/rust-cargo-and-deps/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 17. `docs/rust/supply-chain-security.md`

- **Purpose:** Guidance on vulnerability auditing (`cargo-audit`), supply-chain linting (`cargo-deny`), upstream review (`cargo-vet`), SBOMs, and source restrictions.
- **Main topics:**
  - Threat model and RustSec advisory database
  - `cargo-audit`, yanked crates, and `audit.toml`
  - `cargo-deny` checks (advisories, licenses, bans, sources)
  - `cargo-vet`, `cargo-auditable`, and SBOM generation
  - Dependency pinning, trusted publishing, and incident response
- **When a future agent should read it:** When setting up supply-chain security, reviewing dependencies, or responding to advisories.
- **Related skill files:** `.agents/skills/rust-supply-chain/SKILL.md` (generated)
- **Related workflow files:** See `docs/rust/workflows/`

### 18. `docs/rust/editions-tooling.md`

- **Purpose:** Guidance on Rust editions, rustdoc tooling, `rust-toolchain.toml`, MSRV policy, and toolchain management.
- **Main topics:**
  - Edition semantics (2015/2018/2021/2024) and migration
  - `edition` and `rust-version` fields in `Cargo.toml`
  - `unsafe_op_in_unsafe_fn` edition-specific default
  - rustfmt style edition alignment
  - rustdoc, `cargo doc`, rustup components, and target support
- **When a future agent should read it:** When choosing an edition, migrating crates, pinning toolchains, or configuring rustdoc.
- **Related skill files:** To be generated under `.agents/skills/`
- **Related workflow files:** See `docs/rust/workflows/`

## Recommended reading paths

### New to Rust / writing first Rust code

1. `docs/rust/ownership-lifetimes.md`
2. `docs/rust/types-traits-generics.md`
3. `docs/rust/modules-visibility.md`
4. `docs/rust/error-handling.md`
5. `docs/rust/iterators-closures.md`
6. `docs/rust/style-formatting.md`

### Reviewing a Rust PR

1. `docs/rust/style-formatting.md`
2. `docs/rust/lints-clippy.md`
3. `docs/rust/api-design.md`
4. `docs/rust/error-handling.md`
5. `docs/rust/testing.md`
6. `docs/rust/unsafe-security.md` (if any `unsafe` is present)

### Debugging borrow checker errors

1. `docs/rust/ownership-lifetimes.md`
2. `docs/rust/types-traits-generics.md`
3. `docs/rust/smart-pointers-memory.md`
4. `docs/rust/iterators-closures.md`

### Writing async/concurrent code

1. `docs/rust/ownership-lifetimes.md`
2. `docs/rust/smart-pointers-memory.md`
3. `docs/rust/async-tokio.md`
4. `docs/rust/error-handling.md`
5. `docs/rust/testing.md`

### Setting up CI/lint/formatting policy

1. `docs/rust/style-formatting.md`
2. `docs/rust/lints-clippy.md`
3. `docs/rust/editions-tooling.md`
4. `docs/rust/cargo-dependencies.md`
5. `docs/rust/supply-chain-security.md`
6. `docs/rust/testing.md`

### Implementing a new module/crate

1. `docs/rust/modules-visibility.md`
2. `docs/rust/api-design.md`
3. `docs/rust/types-traits-generics.md`
4. `docs/rust/error-handling.md`
5. `docs/rust/documentation-guidelines.md`
6. `docs/rust/testing.md`

### Refactoring existing Rust code

1. `docs/rust/ownership-lifetimes.md`
2. `docs/rust/types-traits-generics.md`
3. `docs/rust/design-patterns.md`
4. `docs/rust/iterators-closures.md`
5. `docs/rust/smart-pointers-memory.md`
6. `docs/rust/error-handling.md`

### Adding unsafe code

1. `docs/rust/unsafe-security.md`
2. `docs/rust/ownership-lifetimes.md`
3. `docs/rust/smart-pointers-memory.md`
4. `docs/rust/lints-clippy.md`
5. `docs/rust/editions-tooling.md`
6. `docs/rust/supply-chain-security.md`

## Skill derivation map

The following skills have been generated under `.agents/skills/` from the corpus. The 9 currently-generated skills are listed with their paths below; remaining categories are still to be generated.

### Coding skills

- `rust-ownership-borrowing` → `.agents/skills/rust-ownership-borrowing/SKILL.md` (generated) — derived from `ownership-lifetimes.md`
- `rust-types-traits-generics` — derived from `types-traits-generics.md` (to be generated)
- `rust-error-handling` → `.agents/skills/rust-error-handling/SKILL.md` (generated) — derived from `error-handling.md`
- `rust-iterators-closures` — derived from `iterators-closures.md` (to be generated)
- `rust-async-tokio` → `.agents/skills/rust-async-tokio/SKILL.md` (generated) — derived from `async-tokio.md`
- `rust-std-runtime-apis` — derived from `std-runtime-apis.md` (to be generated)
- `rust-smart-pointers-memory` — derived from `smart-pointers-memory.md` (to be generated)
- `rust-unsafe-security` — derived from `unsafe-security.md` (to be generated)
- `rust-api-design` → `.agents/skills/rust-api-design/SKILL.md` (generated) — derived from `api-design.md` and `design-patterns.md`
- `rust-documentation` — derived from `documentation-guidelines.md` (to be generated)

### Reviewing skills

- `rust-code-review` — derived from `api-design.md`, `style-formatting.md`, `lints-clippy.md`, and `error-handling.md` (to be generated)
- `rust-unsafe-review` → `.agents/skills/rust-unsafe-review/SKILL.md` (generated) — derived from `unsafe-security.md`, `ownership-lifetimes.md`, and `smart-pointers-memory.md`
- `rust-dependency-review` — derived from `cargo-dependencies.md` and `supply-chain-security.md` (to be generated)
- `rust-doc-review` — derived from `documentation-guidelines.md` (to be generated)

### Refactoring skills

- `rust-refactoring-patterns` — derived from `design-patterns.md`, `types-traits-generics.md`, and `iterators-closures.md` (to be generated)
- `rust-memory-refactoring` — derived from `smart-pointers-memory.md` and `ownership-lifetimes.md` (to be generated)

### Debugging skills

- `rust-borrow-checker-debugging` — derived from `ownership-lifetimes.md` (to be generated)
- `rust-lifetime-debugging` — derived from `ownership-lifetimes.md` and `types-traits-generics.md` (to be generated)
- `rust-async-debugging` — derived from `async-tokio.md` (to be generated)

### Validating skills

- `rust-testing` → `.agents/skills/rust-testing/SKILL.md` (generated) — derived from `testing.md`
- `rust-linting` — derived from `lints-clippy.md` and `style-formatting.md`; generated skill is `rust-lints-and-clippy` at `.agents/skills/rust-lints-and-clippy/SKILL.md` (generated)
- `rust-supply-chain-validation` — derived from `supply-chain-security.md`; generated skill is `rust-supply-chain` at `.agents/skills/rust-supply-chain/SKILL.md` (generated)
- `rust-ci-validation` — derived from `editions-tooling.md`, `cargo-dependencies.md`, `lints-clippy.md`, `style-formatting.md`, and `testing.md`; partial coverage via `rust-lints-and-clippy` (`.agents/skills/rust-lints-and-clippy/SKILL.md`) and `rust-cargo-and-deps` (`.agents/skills/rust-cargo-and-deps/SKILL.md`) skills (to be generated as a dedicated skill)

## Workflow map

The following workflow files have been generated under `docs/rust/workflows/`:

- `docs/rust/workflows/index.md` — overview of all Rust workflows and when to use each one
- `docs/rust/workflows/implementation.md` — step-by-step workflow for implementing new Rust code
- `docs/rust/workflows/code-review.md` — workflow for reviewing Rust pull requests
- `docs/rust/workflows/refactoring.md` — workflow for refactoring existing Rust code
- `docs/rust/workflows/debugging.md` — workflow for debugging Rust compile-time and runtime issues
- `docs/rust/workflows/validation.md` — workflow for validating Rust code (lint, test, typecheck, CI)

## Open policy decisions

Every Rust repository using this corpus must make and document its own choices for the items below. They are aggregated from the "Policy decisions for individual repos" sections of all 18 docs.

### Language and edition policy

- Target Rust edition (`2015` / `2018` / `2021` / `2024`); new crates should use 2021 or 2024.
- MSRV support policy: version, cadence for raising the floor, and CI enforcement.
- Toolchain pinning policy: exact version, channel, or nightly; whether to commit `rust-toolchain.toml`.
- Required rustup components (clippy, rustfmt, rust-src, rust-analyzer, etc.).
- Cross-compilation targets and linker configuration.

### Formatting and style

- `max_width` (default 100).
- `style_edition` (pin explicitly; 2027 is nightly-only).
- Stable vs nightly rustfmt in CI.
- Nightly-only `rustfmt.toml` options (`imports_granularity`, `group_imports`, `wrap_comments`, etc.).
- Crate-level `#[rustfmt::skip]` policy.
- Format-on-save policy.

### Lint and doc quality

- Clippy group levels: `clippy::pedantic`, `clippy::cargo`, `clippy::nursery`.
- `unwrap_used`/`expect_used`/`panic` policy in production code and tests.
- `missing_docs` level for public API.
- `unsafe_op_in_unsafe_fn` level.
- `indexing_slicing` policy.
- Nightly Clippy in CI (blocking or informational).
- Whether broken intra-doc links are denied or warned in CI.
- Doctest strictness (hard CI gate vs soft warning).
- rustdoc JSON generation (requires nightly).
- `#[doc(cfg(...))]` requirement for conditional public APIs.

### Error handling

- Approved error crates (`thiserror`, `anyhow`, `eyre`, `miette`, `color-eyre`, `snafu`).
- `unwrap`/`expect` conventions and required clippy lints.
- `Box<dyn Error>` usage in binaries or prototypes.
- Error message style (lowercase, no trailing period, etc.).
- Error reporting channel (CLI stderr, structured logs, HTTP bodies).
- `panic = "abort"` for release builds.
- Backtrace environment variables (`RUST_BACKTRACE`, `RUST_LIB_BACKTRACE`).
- Whether public error enums must be `#[non_exhaustive]`.
- Whether `anyhow::Error` is permitted in public APIs.

### Type-system and API conventions

- Mandatory `#[derive]` traits for public types.
- `#[non_exhaustive]` requirements.
- `dyn Trait` vs generics threshold.
- Error conversion strategy (`From` vs `Into`, `TryFrom`, `AsRef`, `FromStr`).
- Newtype policy (IDs, units of measure).
- Sealed trait policy.
- `impl Trait` in return position policy.
- Pattern-matching style preferences (`let-else`, `if let`, `_ => ()`).
- `Default` implementation policy.
- `PhantomData` variance choice.
- Const-generic policy and nightly-feature prohibition.

### Ownership, memory, and smart pointers

- Clone threshold in hot paths.
- Default owned vs borrowed struct data.
- Lifetime annotation style (elision vs explicit).
- `Copy` derivation policy.
- Error ownership (`String` vs `&str`).
- Async ownership defaults.
- Shared ownership threshold (`Rc` vs `Arc`).
- Interior-mutability primitives permitted (`RefCell`, `Cell`, `Mutex`, `RwLock`, atomics).
- `Box<dyn Trait>` vs generics threshold.
- `RwLock` vs `Mutex` default.
- `Cow<'a, B>` usage policy.
- Custom `Drop` policy and cleanup patterns.
- Smart-pointer documentation requirements.

### Modules, visibility, and project layout

- Module file convention (`foo.rs` vs `foo/mod.rs`).
- Re-export / facade strategy and prelude module contents.
- Visibility granularity (`pub(super)`, `pub(in path)` vs `pub(crate)`).
- Module depth limit.
- `#[path]` usage policy.
- Generated code placement.
- Helper-module location for integration tests.
- Error-type organization (per-module vs dedicated module).
- Binary naming convention in `src/bin/`.
- Auto-discovery opt-outs.

### Iterators and closures

- Maximum iterator chain length before extracting a helper or `for` loop.
- `collect()` annotation style (turbofish vs variable annotation).
- `for_each` policy for side-effect chains.
- Custom iterator naming convention.
- Error handling in iterator chains.
- String iteration default (`chars()` vs `bytes()`).
- `into_iter()` usage in function bodies.
- Parallel iteration threshold (`rayon`).
- `try_*` combinators vs collecting `Result`.
- `copied` vs `cloned` default.

### Async and concurrency

- Default runtime flavor and `worker_threads` count.
- `async-trait` allowance vs native async traits and MSRV.
- Maximum blocking-work duration before `spawn_blocking`.
- Default mpsc channel capacity and backpressure strategy.
- Default `MissedTickBehavior` for intervals.
- Canonical shutdown primitive (`tokio-util::sync::CancellationToken`).
- Logging/metrics around cancellations, timeouts, spawn failures, aborts, and `JoinError` panics.
- `kill_on_drop(true)` for spawned subprocesses.

### Standard runtime APIs

- Async runtime choice (sync, tokio, other).
- Error handling library for runtime code.
- Path validation rules (relative paths, sandbox roots, symlink resolution, traversal).
- Subprocess policy (allowed binaries, `PATH` sanitization, shell execution).
- Temporary file strategy.
- Test isolation rules for filesystem/process tests.
- Unsafe extension traits policy (`pre_exec`, raw fd/handle, `set_var`).

### Testing

- Doctest requirements (every public function vs non-trivial only).
- Coverage threshold or informational-only.
- External-service test policy (CI, local, `--ignored`).
- Property-based testing crate (`proptest`, `quickcheck`, none).
- Mocking crate (`mockall` or hand-written fakes).
- Fuzz target requirements for parsers/protocol handlers.
- MSRV policy for test-only dependencies.
- `cargo clippy --all-targets -- -D warnings` CI gate.
- `cargo bench --no-run` in CI.

### Unsafe and security

- Whether `unsafe` is allowed and under what conditions.
- `// SAFETY:` comment format and CI enforcement.
- Miri in CI requirements and targets.
- `unsafe_op_in_unsafe_fn` level.
- Unsafe code review requirements.
- Dependency unsafe budget.
- Tooling requirements (`cargo geiger`, AddressSanitizer).
- Edition policy for new unsafe code (2024 `unsafe extern`, `#[unsafe(attr)]`).

### Dependencies, Cargo, and supply chain

- Approved dependency list and allow-list policy.
- Required default feature flags.
- Vendoring policy (`cargo vendor`).
- `[patch]` policy.
- Profile defaults (`lto = "thin"`, etc.).
- Custom registry usage and authentication.
- Lock file policy for mixed library/binary workspaces.
- Git dependency policy (pin to `rev`/`tag`, migration plan).
- Feature-additivity enforcement and mutually exclusive feature detection.
- License allow-list.
- Advisory ignore approval process.
- Source allow-list (crates.io only vs git).
- Duplicate versions policy.
- SBOM generation tool.
- `cargo-vet` adoption level.
- Trusted publishing method.

### Documentation and release

- Whether `# Examples` is required on all public methods or only non-trivial ones.
- Where docs are hosted.
- Whether README is tested via `include_str!` + `#[cfg(doctest)]`.
- Public API diffing tool (`cargo semver-checks`, `cargo public-api`, manual review).
- Release notes convention (C-RELNOTES).
- License choice (default recommendation `MIT OR Apache-2.0`).
