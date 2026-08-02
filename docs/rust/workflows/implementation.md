# Rust Implementation Workflow

## Purpose

Process for implementing new Rust code — features, modules, functions, and
public API surfaces — from requirements to validated, documented, tested code.
The workflow enforces a deterministic order: understand requirements, load
skills, set up structure, write code, design errors, write tests, run gates,
document, and report.

## When to use

Use this workflow when adding new Rust code: a new feature, a new module or
crate, a new public function or trait, or the first implementation of a
capability. Do not use it for reviewing a diff (use `code-review.md`),
restructuring without behavior change (use `refactoring.md`), fixing a defect
(use `debugging.md`), or running a final gate (use `validation.md`).

## Order of operations

### 1. Understand requirements

Before writing code, capture the intended behavior, inputs, outputs, error
cases, and any public API implications. Read the relevant topic docs:

- `docs/rust/api-design.md` — naming, trait exposure, `#[non_exhaustive]`,
  builder/newtype choices.
- `docs/rust/types-traits-generics.md` — struct/enum/trait/generics modeling,
  `dyn Trait` vs generics, conversions.
- `docs/rust/error-handling.md` — `Option`/`Result`, panic vs recoverable,
  error-type design.

Record a one-paragraph summary of the intended behavior and the public surface
so later steps can be checked against it.

### 2. Load relevant skills

Load the operational skills that match the implementation area:

- `.agents/skills/rust-ownership-borrowing/SKILL.md` — borrow rules, smart
  pointer selection, lifetime annotation.
- `.agents/skills/rust-error-handling/SKILL.md` — `Result`/`?` patterns,
  `thiserror`/`anyhow` decision, `unwrap`/`expect` policy.
- `.agents/skills/rust-api-design/SKILL.md` — naming, trait exposure,
  semver-sensitive changes.

Load additional skills only if the implementation area requires them (for
example, `.agents/skills/rust-async-tokio/SKILL.md` for async code).

### 3. Set up module structure

Read `docs/rust/modules-visibility.md` for package/crate/module hierarchy,
visibility grammar, and the library + binary pattern. Consult the
`.agents/skills/rust-cargo-and-deps/SKILL.md` skill for manifest changes,
feature flags, and workspace layout. Decide module file convention
(`foo.rs` vs `foo/mod.rs`) and visibility granularity up front and apply it
consistently.

### 4. Write code following ownership/borrowing rules

Implement the body of the code applying the borrow rules from the
`rust-ownership-borrowing` skill: one owner, one `&mut` or many `&`, references
must outlive borrowed data, non-`Copy` values move. Choose smart pointers
(`Box`, `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`, `Cow`) per the skill's
selection guidance rather than reaching for `clone()` reflexively.

### 5. Design error types

Decide the error strategy per `docs/rust/error-handling.md` and the
`rust-error-handling` skill:

- Library crate with a public error enum → `thiserror`, `#[non_exhaustive]`,
  implement `std::error::Error`.
- Application/binary boundary → `anyhow::Result` (or repo-approved equivalent).
- Never use `unwrap`/`expect`/`panic!` for expected runtime conditions; reserve
  them for genuine invariants and tests.

### 6. Write tests alongside code

Write unit tests in the same module (`#[cfg(test)] mod tests`) and integration
tests under `tests/` per `docs/rust/testing.md` and the
`.agents/skills/rust-testing/SKILL.md` skill. Cover the happy path, each error
branch, and edge cases (empty input, boundary values, concurrent access for
async code). Add doctests for non-trivial public items.

### 7. Run checks

Run the gates in this exact order; stop and fix before proceeding if a gate
fails:

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

If any gate fails, fix the root cause (do not suppress with `#[allow]` unless
the lint policy explicitly permits it). Re-run from `cargo check`.

### 8. Document public API

Document every public item per `docs/rust/documentation-guidelines.md`: `///`
on items, `//!` at crate root, `# Examples` for non-trivial public functions,
`# Safety` sections on `unsafe` items, and intra-doc links. Ensure
`cargo doc --no-deps --document-private-items` builds without warnings.

### 9. Report evidence

Report what was implemented, the public surface added, the tests added, and the
output of each gate in step 7 plus the doc build in step 8.

## Docs consulted

- `docs/rust/api-design.md`
- `docs/rust/types-traits-generics.md`
- `docs/rust/error-handling.md`
- `docs/rust/modules-visibility.md`
- `docs/rust/documentation-guidelines.md`
- `docs/rust/testing.md`

## Skills loaded

- `.agents/skills/rust-ownership-borrowing/SKILL.md`
- `.agents/skills/rust-error-handling/SKILL.md`
- `.agents/skills/rust-api-design/SKILL.md`
- `.agents/skills/rust-cargo-and-deps/SKILL.md`
- `.agents/skills/rust-testing/SKILL.md`
- (conditional) `.agents/skills/rust-async-tokio/SKILL.md`

## Commands run (in order)

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
cargo doc --no-deps --document-private-items
```

## Evidence to report

- One-paragraph summary of intended behavior and public surface (from step 1).
- List of files added/modified.
- List of tests added (unit, integration, doctest) with what each covers.
- Raw output (or pass/fail) of each command in step 7 and the doc build in
  step 8.
- Any policy decisions deferred to the repo (for example, approved error
  crate, `#[non_exhaustive]` requirement).

## When human judgment is needed

- Choosing an error crate when the repo has no recorded policy (`thiserror` vs
  `anyhow` vs `eyre`/`miette`).
- Deciding whether a new public type needs `#[non_exhaustive]` or a sealed
  trait.
- Choosing `dyn Trait` vs generics when the performance/ergonomics tradeoff is
  not clear from the skill guidance.
- Approving a new dependency or feature flag (supply-chain and Cargo policy
  decisions belong to the repo owner).
- Deciding whether an `unsafe` block is justified; if so, hand off to the
  `rust-unsafe-review` skill and `docs/rust/unsafe-security.md`.

## Avoiding scope creep

- Implement only the requirements captured in step 1. If a related cleanup
  appears, record it as a follow-up rather than folding it into this task.
- Do not refactor unrelated modules that the new code touches only at a call
  site; restrict changes to the new code and its direct integration points.
- Do not add lints, feature flags, or dependencies beyond what the new code
  requires; if a manifest change is needed, keep it minimal and note it in the
  report.
- If a gate fails because of pre-existing code, fix only the new code's
  contribution and surface the pre-existing issue as a follow-up.
