# Rust Validation Workflow

## Purpose

Process for running the full validation gate suite on Rust code: compilation,
lints, tests, doctests, formatting, doc build, supply chain, Miri (if
applicable), and coverage (if configured). The workflow reports pass/fail per
gate with evidence. It is the final gate before merge/release.

## When to use

Use this workflow as the final validation before merge or release, or as a
periodic health gate on the whole workspace. Do not use it as a substitute for
`implementation.md`, `code-review.md`, `refactoring.md`, or `debugging.md`;
run it after one of those workflows has produced code. If a gate fails, hand
off to the appropriate workflow (`debugging.md` for a defect,
`implementation.md` for missing tests/docs) to fix, then re-run validation.

## Order of operations

Run each gate in order. Record pass/fail and the relevant output for each. If
a gate fails, you may continue to the remaining gates to give a full picture,
but the overall result is fail.

### 1. Full compilation check

```sh
cargo check --all-targets
```

`--all-targets` covers lib, bins, tests, examples, and benches. A failure
here is a blocker.

### 2. Lint check

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

`--workspace` covers all members. Any warning treated as error is a blocker.
Consult `docs/rust/lints-clippy.md` and the
`.agents/skills/rust-lints-and-clippy/SKILL.md` skill for lint policy.

### 3. Test suite

```sh
cargo test --all-features
```

`--all-features` exercises the full feature matrix (adjust if the repo uses
mutually exclusive features; see `docs/rust/cargo-dependencies.md`). All tests
must pass. Consult `docs/rust/testing.md` and the
`.agents/skills/rust-testing/SKILL.md` skill.

### 4. Doctest check

```sh
cargo test --doc
```

Doctests are part of `cargo test` but running `--doc` separately gives a
clearer pass/fail signal for documentation examples. Consult
`docs/rust/documentation-guidelines.md`.

### 5. Format check

```sh
cargo fmt --all -- --check
```

Formatting must be clean across the workspace. Consult
`docs/rust/style-formatting.md`.

### 6. Doc build

```sh
cargo doc --no-deps --document-private-items
```

`--document-private-items` ensures internal docs build too. Broken intra-doc
links or rustdoc warnings are failures (configure `RUSTDOCFLAGS="-D warnings"`
if the repo policy denies them). Consult `docs/rust/documentation-guidelines.md`
and `docs/rust/editions-tooling.md`.

### 7. Supply chain

```sh
cargo audit
cargo deny check
```

`cargo audit` checks the RustSec advisory database; `cargo deny check` runs
advisories, licenses, bans, and sources checks. Consult
`docs/rust/supply-chain-security.md` and the
`.agents/skills/rust-supply-chain/SKILL.md` skill. If `cargo deny` is not
configured, record that and run `cargo audit` alone.

### 8. Unsafe code review (if applicable)

If the workspace contains `unsafe` code, run Miri on the test suite:

```sh
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test
```

Consult `docs/rust/unsafe-security.md` and the
`.agents/skills/rust-unsafe-review/SKILL.md` skill. If no `unsafe` code is
present, record "not applicable" and skip.

### 9. Coverage report (if configured)

If the repo has a coverage tool configured (for example, `cargo-llvm-cov` or
`tarpaulin`), run it:

```sh
cargo llvm-cov --all-features
```

Record the coverage percentage and whether it meets the repo's threshold (see
the "Testing" section of `docs/rust/index.md` open policy decisions). If no
coverage tool is configured, record "not configured" and skip.

### 10. Report

Report pass/fail per gate with evidence for each.

## Docs consulted

- `docs/rust/lints-clippy.md`
- `docs/rust/style-formatting.md`
- `docs/rust/testing.md`
- `docs/rust/cargo-dependencies.md`
- `docs/rust/supply-chain-security.md`
- `docs/rust/editions-tooling.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/documentation-guidelines.md`

## Skills loaded

- `.agents/skills/rust-lints-and-clippy/SKILL.md`
- `.agents/skills/rust-testing/SKILL.md`
- `.agents/skills/rust-cargo-and-deps/SKILL.md`
- `.agents/skills/rust-supply-chain/SKILL.md`
- (conditional) `.agents/skills/rust-unsafe-review/SKILL.md`

## Commands run (in order)

```sh
cargo check --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --all-features
cargo test --doc
cargo fmt --all -- --check
cargo doc --no-deps --document-private-items
cargo audit
cargo deny check
```

If `unsafe` code is present:

```sh
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test
```

If coverage is configured:

```sh
cargo llvm-cov --all-features
```

## Evidence to report

A per-gate table:

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Compilation | `cargo check --all-targets` | pass/fail | error count or clean |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | pass/fail | warning count or clean |
| Tests | `cargo test --all-features` | pass/fail | passed/failed counts |
| Doctests | `cargo test --doc` | pass/fail | passed/failed counts |
| Format | `cargo fmt --all -- --check` | pass/fail | diff or clean |
| Doc build | `cargo doc --no-deps --document-private-items` | pass/fail | warning count or clean |
| Supply chain | `cargo audit` / `cargo deny check` | pass/fail | advisory/violation count or clean |
| Miri | `cargo +nightly miri test` | pass/fail/N-A | UB report or clean |
| Coverage | `cargo llvm-cov --all-features` | pass/fail/N-A | percentage and threshold |

Plus an overall verdict: pass (all applicable gates green) or fail (any
applicable gate red), with the list of failing gates.

## When human judgment is needed

- Deciding whether a supply-chain advisory is exploorable in this repo's usage
  (consult `docs/rust/supply-chain-security.md` and the `rust-supply-chain`
  skill; escalate if unclear).
- Judging whether a Miri finding is a true soundness bug or a Miri limitation
  (escalate to a human `unsafe` reviewer; consult the `rust-unsafe-review`
  skill).
- Deciding whether a coverage dip below threshold is acceptable for a given
  change.
- Choosing whether to block on a `cargo deny` license/ban violation that the
  repo policy does not explicitly address.
- Deciding whether a doctest failure caused by an environment dependency (for
  example, a missing binary) is a blocker or an environment issue.

## Avoiding scope creep

- Validation is a gate, not a fix. If a gate fails, record the failure and hand
  off to the appropriate workflow (`debugging.md`, `implementation.md`,
  `refactoring.md`); do not fix in the validation pass.
- Do not add tests, docs, or lint suppressions during validation; those are
  implementation changes that must go through their own workflow and re-run
  validation.
- Do not change `Cargo.toml`, `clippy.toml`, `rustfmt.toml`, or `deny.toml` to
  make a gate pass; policy changes are separate decisions.
- Run only the gates applicable to the workspace. If a gate is not configured
  (for example, no `unsafe` code, no coverage tool), record "not applicable"
  or "not configured" rather than setting it up as part of validation.
- Keep the report to the per-gate table and overall verdict; do not expand it
  into a code review or implementation summary.
