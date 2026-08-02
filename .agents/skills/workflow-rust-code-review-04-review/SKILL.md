---
name: workflow-rust-code-review-04-review
description: |
  Use only for the review phase of the Rust code-review workflow. Perform the
  human-judgment review using per-doc checklists across ownership, error
  handling, unsafe safety, API design, and documentation. Do not use for
  scoping, analysis, running gates, or issuing a verdict.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-code-review
  org.phase: review
  org.phase_order: "04"
---

# Phase 04: review (Rust code review)

## Phase purpose

Perform the human-judgment review using the per-doc checklists across
ownership, error handling, unsafe safety, API design, and documentation. Walk
each topic doc's `## Review checklist` section explicitly and cite items
rather than paraphrasing.

## Steps to perform

1. **Ownership/borrowing correctness** — consult `docs/rust/ownership-lifetimes.md`
   and the `rust-ownership-borrowing` skill. Look for unnecessary `clone()`,
   aliasing violations, lifetime annotations that fight the compiler, and
   smart-pointer misuse.
2. **Error handling** — consult `docs/rust/error-handling.md` and the
   `rust-error-handling` skill. Look for `unwrap`/`expect`/`panic!` on
   recoverable conditions, swallowed errors, and `Box<dyn Error>` in public
   APIs where a concrete type is required.
3. **Unsafe safety** (only if `unsafe` is present) — consult
   `docs/rust/unsafe-security.md` and the `rust-unsafe-review` skill. Verify
   `// SAFETY:` comments, `Send`/`Sync` impls, and FFI invariants; consider
   Miri (see `docs/rust/workflows/validation.md`).
4. **API design** — consult `docs/rust/api-design.md` and the `rust-api-design`
   skill. Check naming (`as_`/`to_`/`into_`), trait exposure,
   `#[non_exhaustive]`, and semver impact.
5. **Documentation** — consult `docs/rust/documentation-guidelines.md`. Check
   `///` on public items, `# Examples` on non-trivial public functions,
   `# Safety` on `unsafe` items, and intra-doc links.
6. Walk each topic doc's `## Review checklist` section explicitly; record
   pass/fail/N-A per item. **Cite checklist items; do not paraphrase them.**

## Docs to consult

- `docs/rust/ownership-lifetimes.md`
- `docs/rust/error-handling.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/api-design.md`
- `docs/rust/documentation-guidelines.md`

## Operational skills to load

- `rust-ownership-borrowing`
- `rust-error-handling`
- `rust-unsafe-review` (if `unsafe` present)
- `rust-api-design`

## Constraints to apply

- `constraint-rust-ownership` — enforce borrow rules and smart pointer
  selection; flag unnecessary `clone()` and aliasing violations.
- `constraint-rust-error-propagation` — flag `unwrap`/`expect`/`panic!` on
  recoverable conditions and swallowed errors.
- `constraint-rust-unsafe-safety` — applies only if `unsafe` is present:
  verify `// SAFETY:` comments, `Send`/`Sync` impls, and FFI invariants.
- `constraint-rust-api-docs` — verify documentation completeness for the
  public API: `///` on public items, `# Examples`, `# Safety`, intra-doc links.
- `constraint-rust-scope-discipline` — review only the diff; do not request
  changes to untouched code.

## Validations to run

None — this is a manual review phase that uses per-doc checklists. Automated
validations ran in phase 03 (workflow-rust-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-rust-code-review-00-orchestration`. Set:

- `outcome`: `pass` if no findings; `partial` if findings exist but none are
  blockers; `fail` if a blocker-level finding was identified.
- `constraints_applied`: all constraints listed above that applied.
- `risks`: each finding with file:line, the checklist item or doc rule
  violated (cited), and a concrete suggested fix. Severity classification
  happens in phase 05-verdict; here record the raw findings.
- `assumptions`: any N-A checklist items and why.
- `next_phase`: `05-verdict`.
- `next_workflow`: `null`.
