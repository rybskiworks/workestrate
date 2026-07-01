---
name: workflow-gleam-code-review-05-verdict
description: |
  Use only for the verdict phase of the Gleam code-review workflow. Classify
  every finding under a severity level and issue a final verdict. Do not use
  for scoping, analysis, running gates, or manual review.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-code-review
  org.phase: verdict
  org.phase_order: "05"
---

# Phase 05: verdict (Gleam code review)

## Phase purpose

Classify every finding under a severity level, record the checklist items
walked and their status, and issue a final verdict. If the review surfaces a
defect, hand off to `workflow-gleam-debugging`; do not fix in the review pass.

## Steps to perform

1. Classify every finding under exactly one severity level:
   - **blocker** — must fix before merge: compile failure, failing test,
     type error, swallowed `Result`/error, unsound `@external`/FFI,
     `panic`/`let assert` in library code, missing tests for new public
     behavior, breaking public API without intent.
   - **major** — should fix before merge but may be deferred with owner
     approval: wrong `Result`/`Option` strategy, missing type annotations on
     public functions, catch-all pattern disabling exhaustiveness, weak error
     types, missing docs on public items.
   - **minor** — fix encouraged but not blocking: naming nit, redundant
     pipeline, missing example on a trivial public function, style
     inconsistency `gleam format` missed.
   - **nit** — optional polish: comment wording, import ordering `gleam format`
     did not enforce, example simplification.
   - **praise** — positive callout: notably clear error handling, excellent
     test coverage, idiomatic pattern use.
2. For each finding record: `file:line`, `severity`, the checklist item or
   doc rule violated (cited, not paraphrased), and a concrete suggested fix.
3. State explicitly which `## Review checklist` items were walked and their
   pass/fail/N-A status.
4. Issue a final verdict: `approve`, `request changes`, or `block`.
5. If the review surfaces a defect, set `next_workflow: workflow-gleam-debugging`
   (do not fix in the review pass). Otherwise set `next_workflow: null`.

## Docs to consult

None new. Findings reference the docs cited in phase 04-review.

## Operational skills to load

None new.

## Constraints to apply

- Scope discipline — findings cover only the diff; pre-existing issues in
  untouched code are recorded as separate follow-ups, not as review findings.

## Validations to run

None — this is a reporting phase. Automated validations ran in phase 03
(`workflow-gleam-code-review-03-check`).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-gleam-code-review-00-orchestration`, including the verification-phase
extra fields. Set:

- `outcome`: `pass` if verdict is `approve`; `partial` if `request changes`;
  `fail` if `block`.
- `constraints_applied`: scope discipline (plus any constraints applied in
  phase 04 that produced findings).
- `risks`: the final classified findings list (file:line, severity, cited
  checklist item, suggested fix).
- `blockers`: every finding classified `blocker`.
- `next_phase`: `null` (this is the final phase).
- `next_workflow`: `workflow-gleam-debugging` if a defect was found, else
  `null`.

Include the verification-phase extra fields:

```yaml
validations_run:
  - validation-gleam-check
  - validation-gleam-test
  - validation-gleam-format
constraints_checked:
  - constraint-gleam-result
  - constraint-gleam-conventions
  - constraint-beam-supervision
  - constraint-beam-failure
  - constraint-beam-process-isolation
  - scope discipline
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

`constraints_checked` should include the BEAM constraints **only** if the diff
was reviewed on the Erlang target and touched OTP/actors/supervision/interop.
On the JavaScript target, BEAM constraints do not apply.

`not_fully_checkable` records any checklist item or soundness concern that
could not be fully resolved mechanically (e.g. an `@external` FFI boundary
whose data-mapping correctness requires a human OTP/interop reviewer), with a
reason and a suggested escalation path.
