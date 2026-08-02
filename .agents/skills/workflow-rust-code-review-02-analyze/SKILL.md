---
name: workflow-rust-code-review-02-analyze
description: |
  Use only for the analyze phase of the Rust code-review workflow. Categorize
  the diff by dimension and load the operational review skills that match. Do
  not use for scoping, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-code-review
  org.phase: analyze
  org.phase_order: "02"
---

# Phase 02: analyze (Rust code review)

## Phase purpose

Categorize the diff by dimension and load the operational review skills that
match the categories, so phase 04-review has the right skills and docs ready.

## Steps to perform

1. Categorize the diff by dimension: does it touch ownership/borrowing, error
   handling, `unsafe`, async, FFI, public API, or dependencies? Record the
   categories in `risks`.
2. Load `rust-api-design` — naming, trait exposure, semver-sensitive changes,
   `#[non_exhaustive]`.
3. Load `rust-unsafe-review` — soundness invariants, UB catalog, `// SAFETY:`
   comment requirements, `Send`/`Sync` impls. This is **mandatory if `unsafe`
   is present** in the diff.
4. Load `rust-lints-and-clippy` — lint levels, `#[allow]` justification, clippy
   group policy.
5. Conditionally load `rust-ownership-borrowing` and `rust-error-handling` if
   the diff is large or touches ownership/error paths. When in doubt, load them.
6. Apply `constraint-rust-scope-discipline`: categorization covers only the
   diff; do not expand scope to untouched code.

## Docs to consult

- `docs/rust/lints-clippy.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/api-design.md`

## Operational skills to load

- `rust-api-design`
- `rust-unsafe-review` (if `unsafe` present)
- `rust-lints-and-clippy`
- `rust-ownership-borrowing` (conditional)
- `rust-error-handling` (conditional)

## Constraints to apply

- `constraint-rust-scope-discipline` — categorization covers only the diff; do
  not expand scope to untouched code.

## Validations to run

None — validations run in phase 03 (workflow-rust-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-rust-code-review-00-orchestration`. Set:

- `outcome`: `pass` if categorization completed and skills loaded; `partial`
  if a conditional skill was deliberately not loaded (record why in
  `assumptions`).
- `constraints_applied`: `constraint-rust-scope-discipline`.
- `risks`: the dimensions the diff touches.
- `assumptions`: any conditional-load decisions and their rationale.
- `next_phase`: `03-check`.
- `next_workflow`: `null`.
