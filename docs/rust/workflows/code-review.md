# Rust Code Review Workflow

## Purpose

Process for reviewing a Rust PR/diff for correctness, safety, idioms, and
documentation quality. The workflow enforces a deterministic order: understand
the change, load review skills, run the automated gates, then perform the
human-judgment review using the per-doc checklists, and report findings with
severity levels.

## When to use

Use this workflow when reviewing a pull request, auditing a diff, or
performing a pre-merge gate on someone else's Rust changes. Do not use it for
implementing new code (use `implementation.md`), refactoring without behavior
change (use `refactoring.md`), fixing a defect (use `debugging.md`), or running
the full validation suite (use `validation.md`). If the review surfaces a
defect, hand off to `debugging.md` for the fix; do not fix in the review pass.

## Order of operations

### 1. Understand the change

Before judging the code, capture what the change is and what it intends. Read
the PR description and the diff. Record: which files changed, what the stated
intent is, whether the change touches public API, `unsafe`, async, FFI, or
dependencies. This summary drives which docs and skills to consult in later
steps.

### 2. Load review skills

Load the operational skills that match a Rust review:

- `.agents/skills/rust-api-design/SKILL.md` — naming, trait exposure,
  semver-sensitive changes, `#[non_exhaustive]`.
- `.agents/skills/rust-unsafe-review/SKILL.md` — soundness invariants, UB
  catalog, `// SAFETY:` comment requirements, `Send`/`Sync` impls.
- `.agents/skills/rust-lints-and-clippy/SKILL.md` — lint levels, `#[allow]`
  justification, clippy group policy.

Load `.agents/skills/rust-ownership-borrowing/SKILL.md` and
`.agents/skills/rust-error-handling/SKILL.md` as well if the diff is large or
touches ownership/error paths.

### 3. Check compilation

```sh
cargo check
```

A clean compile is the baseline. If it does not compile, report a blocker and
stop the review.

### 4. Check lints

```sh
cargo clippy --all-targets -- -D warnings
```

Any warning treated as error is a blocker. Review any new `#[allow(...)]` or
`#[expect(...)]` for justification against the repo lint policy in
`docs/rust/lints-clippy.md`.

### 5. Check tests

```sh
cargo test
```

All tests must pass. If a test was deleted or weakened, flag it. New public
items should have accompanying tests (see `docs/rust/testing.md`).

### 6. Check formatting

```sh
cargo fmt --all -- --check
```

Formatting must be clean. Do not accept hand-formatted code that contradicts
`rustfmt` output (see `docs/rust/style-formatting.md`).

### 7. Review for correctness and design

With the automated gates green, perform the human-judgment review across these
dimensions, consulting the per-doc review checklists:

- **Ownership/borrowing correctness** — `docs/rust/ownership-lifetimes.md` and
  the `rust-ownership-borrowing` skill. Look for unnecessary `clone()`, aliasing
  violations, lifetime annotations that fight the compiler, smart-pointer
  misuse.
- **Error handling** — `docs/rust/error-handling.md` and the
  `rust-error-handling` skill. Look for `unwrap`/`expect`/`panic!` on
  recoverable conditions, swallowed errors, `Box<dyn Error>` in public APIs
  where a concrete type is required.
- **Unsafe safety** — `docs/rust/unsafe-security.md` and the
  `rust-unsafe-review` skill, only if `unsafe` is present. Verify `// SAFETY:`
  comments, `Send`/`Sync` impls, FFI invariants, and consider Miri (see
  `docs/rust/workflows/validation.md`).
- **API design** — `docs/rust/api-design.md` and the `rust-api-design` skill.
  Check naming (`as_`/`to_`/`into_`), trait exposure, `#[non_exhaustive]`,
  semver impact.
- **Documentation** — `docs/rust/documentation-guidelines.md`. Check `///` on
  public items, `# Examples` on non-trivial public functions, `# Safety` on
  `unsafe` items, intra-doc links.

### 8. Use the review checklists

Each topic doc has a `## Review checklist` section. Walk the relevant checklists
explicitly and record which checklist items passed, failed, or were not
applicable. Do not paraphrase checklist items; cite them.

### 9. Report findings with severity levels

Classify every finding under one of these severity levels:

- **blocker** — must be fixed before merge: compile failure, failing test, lint
  treated as error, soundness bug in `unsafe`, missing `// SAFETY:` comment,
  semver-breaking change without intent.
- **major** — should be fixed before merge but may be deferred with owner
  approval: missing tests for new public API, wrong error-type strategy,
  ownership pattern that will cause maintenance pain, missing docs on public
  items.
- **minor** — fix encouraged but not blocking: naming nit, redundant `clone()`,
  missing `# Examples` on a trivial public function, style inconsistency
  rustfmt did not catch.
- **nit** — optional polish: comment wording, import ordering rustfmt did not
  enforce, example simplification.
- **praise** — positive callout: notably clear `// SAFETY:` comment, excellent
  test coverage, idiomatic use of a pattern from `docs/rust/design-patterns.md`.

## Docs consulted

- `docs/rust/ownership-lifetimes.md`
- `docs/rust/error-handling.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/api-design.md`
- `docs/rust/documentation-guidelines.md`
- `docs/rust/lints-clippy.md`
- `docs/rust/style-formatting.md`
- `docs/rust/testing.md`

## Skills loaded

- `.agents/skills/rust-api-design/SKILL.md`
- `.agents/skills/rust-unsafe-review/SKILL.md`
- `.agents/skills/rust-lints-and-clippy/SKILL.md`
- (conditional) `.agents/skills/rust-ownership-borrowing/SKILL.md`
- (conditional) `.agents/skills/rust-error-handling/SKILL.md`

## Commands run (in order)

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --all -- --check
```

## Evidence to report

- One-paragraph summary of the change and its intent (from step 1).
- Pass/fail for each command in steps 3–6 with raw output on failure.
- List of findings, each with: file:line, severity, the checklist item or doc
  rule violated, and a concrete suggested fix.
- Explicit statement of which `## Review checklist` items were walked and their
  pass/fail/N-A status.
- A final verdict: approve, request changes, or block.

## When human judgment is needed

- Deciding whether a `#[allow(...)]` is justified by repo lint policy.
- Judging whether an `unsafe` block's `// SAFETY:` comment is sufficient (this
  is rarely fully mechanical; consult the `rust-unsafe-review` skill and
  `docs/rust/unsafe-security.md`).
- Weighing a semver-breaking change against the PR's stated intent.
- Deciding whether a missing test is a blocker or a major, given the change's
  risk.
- Escalating a soundness concern that the reviewer cannot fully resolve to a
  human `unsafe` reviewer.

## Avoiding scope creep

- Review only the diff. Do not request changes to code the diff does not touch,
  even if it has pre-existing issues; record those as separate follow-ups.
- Do not ask the author to refactor for style preferences beyond what
  `rustfmt` and the repo lint policy enforce.
- Do not combine a review with a parallel implementation task; if the review
  reveals a needed feature, open a separate task under `implementation.md`.
- Keep severity assignments consistent with the definitions above; do not
  inflate a nit to a major to force a change.
