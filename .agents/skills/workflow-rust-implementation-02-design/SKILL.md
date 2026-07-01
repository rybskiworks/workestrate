---
name: workflow-rust-implementation-02-design
description: |
  Use only for the design phase of the Rust implementation workflow. Make API
  design, error type design, and module structure decisions up front. Do not use
  for scoping, implementation, testing, or final verification.
allowed-tools: Read Write Edit Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-implementation
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Rust implementation)

## Phase purpose

Make API design, error type design, and module structure decisions up front so
the implement phase can proceed without mid-flight architectural changes.

## Steps to perform

1. Load `rust-api-design` — naming, trait exposure, semver-sensitive changes.
2. Load `rust-error-handling` — `Result`/`?` patterns, `thiserror`/`anyhow`
   decision, `unwrap`/`expect` policy.
3. Load `rust-cargo-and-deps` for manifest changes, feature flags, and workspace
   layout.
4. Read `docs/rust/modules-visibility.md` for package/crate/module hierarchy,
   visibility grammar, and the library + binary pattern.
5. Decide the module file convention (`foo.rs` vs `foo/mod.rs`) and visibility
   granularity up front; apply it consistently across the implementation.
6. Decide the error strategy:
   - Library crate with a public error enum → `thiserror` +
     `#[non_exhaustive]` + `std::error::Error`.
   - Application/binary boundary → `anyhow::Result` (or repo-approved
     equivalent).
   - Never use `unwrap`/`expect`/`panic!` for expected runtime conditions;
     reserve them for genuine invariants and tests.
7. Conditionally load `rust-async-tokio` if the implementation area is async.

## Docs to consult

- `docs/rust/modules-visibility.md`
- `docs/rust/error-handling.md`
- `docs/rust/api-design.md`

## Operational skills to load

- `rust-api-design`
- `rust-error-handling`
- `rust-cargo-and-deps`
- (conditional) `rust-async-tokio` — only if the implementation area is async.

## Constraints to apply

- `constraint-rust-scope-discipline` — design only what the captured
  requirements demand. Do not introduce speculative abstractions, new feature
  flags, or dependencies beyond what the new code requires.
- `constraint-rust-api-docs` — plan documentation for the public API: decide
  which public items need `///`, `# Examples`, and `# Safety` sections so the
  implement and verify phases can produce complete docs.

## Validations to run

None — validations run in phase 05 (workflow-rust-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-rust-implementation-00-orchestration`. Set:

- `outcome` to `pass` once module convention, visibility granularity, and error
  strategy are decided and recorded.
- `constraints_applied` to include `constraint-rust-scope-discipline` and
  `constraint-rust-api-docs`.
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. error crate choice) needs human
  input — in that case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-scope-discipline
  - constraint-rust-api-docs
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
