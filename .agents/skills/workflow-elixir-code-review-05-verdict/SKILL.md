---
name: workflow-elixir-code-review-05-verdict
description: |
  Use only for the verdict phase of the Elixir code-review workflow. Classify
  every finding under a severity level and issue a final verdict. Do not use
  for scoping, analysis, running gates, or manual review.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-code-review
  org.phase: verdict
  org.phase_order: "05"
---

# Phase 05: verdict (Elixir code review)

## Phase purpose

Classify every finding under a severity level, record the checklist items
walked and their status, and issue a final verdict. If the review surfaces a
defect, hand off to `workflow-elixir-debugging`; do not fix in the review pass.

## Steps to perform

1. Classify every finding under exactly one severity level:
   - **blocker** — must fix before merge: compile failure, failing test,
     swallowed error, broken OTP pattern, missing tests for new behavior,
     unsafe NIF invariant violation.
   - **major** — should fix before merge but may be deferred with owner
     approval: missing typespecs per repo policy, wrong error strategy,
     missing docs on public items, weak assertions.
   - **minor** — fix encouraged but not blocking: naming nit, pipe-usage
     inconsistency, missing `@doc` example on a trivial public function, style
     inconsistency `mix format` did not enforce.
   - **nit** — optional polish: comment wording, import ordering `mix format` did
     not enforce, example simplification.
   - **praise** — positive callout: excellent test coverage, idiomatic pattern
     use, clear supervised-process boundary.
2. For each finding record: `file:line`, `severity`, the checklist item or
   doc rule violated (cited, not paraphrased), and a concrete suggested fix.
3. State explicitly which `## Review checklist` items were walked and their
   pass/fail/N-A status.
4. Issue a final verdict: `approve`, `request changes`, or `block`.
5. If the review surfaces a defect, set `next_workflow: workflow-elixir-debugging`
   (do not fix in the review pass). Otherwise set `next_workflow: null`.

## Docs to consult

None new. Findings reference the docs cited in phase 04-review.

## Operational skills to load

None new.

## Constraints to apply

- `constraint-elixir-style` — findings cover only the diff; pre-existing
  issues in untouched code are recorded as separate follow-ups, not as review
  findings.

## Validations to run

None — this is a reporting phase. Automated validations ran in phase 03
(workflow-elixir-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-elixir-code-review-00-orchestration`, including the verification-phase
extra fields. Set:

- `outcome`: `pass` if verdict is `approve`; `partial` if `request changes`;
  `fail` if `block`.
- `constraints_applied`: `constraint-elixir-style` (plus any constraints
  applied in phase 04 that produced findings).
- `risks`: the final classified findings list (file:line, severity, cited
  checklist item, suggested fix).
- `blockers`: every finding classified `blocker`.
- `next_phase`: `null` (this is the final phase).
- `next_workflow`: `workflow-elixir-debugging` if a defect was found, else
  `null`.

Include the verification-phase extra fields:

```yaml
validations_run:
  - validation-elixir-compile
  - validation-elixir-credo
  - validation-elixir-dialyzer
  - validation-elixir-test
  - validation-elixir-format
constraints_checked:
  - constraint-elixir-style
  - constraint-beam-failure
  - constraint-beam-supervision
  - constraint-beam-process-isolation
  - constraint-elixir-otp-api
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

`not_fully_checkable` records any checklist item or correctness concern that
could not be fully resolved mechanically (e.g. a supervised-process boundary
whose correctness requires a human OTP reviewer), with a reason and a
suggested escalation path.
