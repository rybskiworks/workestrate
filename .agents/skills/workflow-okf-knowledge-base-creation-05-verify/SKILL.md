---
name: workflow-okf-knowledge-base-creation-05-verify
description: |
  Use only for the verify phase of the OKF knowledge base creation workflow.
  Run full OKF validation, verify corpus integration, skill registration,
  workflow chaining, and clean up temporary files. Do not use for scoping,
  design, implementation, or validation.
allowed-tools: Read Write Edit Bash(git:*) Bash(python3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: okf-knowledge-base-creation
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (OKF knowledge base creation)

## Phase purpose

Run full OKF validation, verify corpus integration (languages.md updated,
cross-refs to sibling corpora), verify skill registration (all skills
discoverable), verify workflow chaining (all workflows reference correct
next_workflow), and clean up temporary files. This is the final verification
phase; it does not introduce new content.

## Steps to perform

1. Run full OKF validation (re-run `okf-validate`) against the complete
   bundle to confirm no regressions from phase 04 fixes.
2. Verify corpus integration. If the bundle is part of a larger knowledge
   workspace, verify that the workspace's corpus index (e.g. `languages.md`
   or equivalent) has been updated to reference the new bundle. Verify
   cross-references to sibling corpora resolve.
3. Verify skill registration. Confirm all skills created during this workflow
   are discoverable — i.e. each skill directory has a valid `SKILL.md` with
   correct frontmatter and is listed in the skill registry / available
   skills.
4. Verify workflow chaining. Confirm all workflows created reference the
   correct `next_workflow` values per the orchestration phase's workflow
   chaining table. Confirm handoff schemas are consistent across phases.
5. Clean up temporary files. Remove cloned source repos, crawl caches, or
   other temporary artifacts that were used during implementation but should
   not ship with the bundle. Do NOT remove `source-map.md` or `log.md` —
   these are part of the bundle.
6. Report: the scope document (from phase 01), files created, validation
   results (from phase 04), verification results, and any deferred items.

## Docs to consult

- `okf/SPEC.md` (sections 3, 6, 7, 9, 11)

## Operational skills to load

- `okf-validate`

## Constraints to apply

- OKF conformance — verify the final bundle state satisfies OKF §9
  conformance.
- Scope discipline — fix only the new bundle's contribution to any
  verification failure. Surface pre-existing workspace issues as follow-ups.

## Validations to run

- `okf-validate` — full OKF conformance re-check.
- Corpus integration check — workspace index references the new bundle;
  sibling cross-refs resolve.
- Skill registration check — all skills discoverable with valid SKILL.md
  frontmatter.
- Workflow chaining check — all workflows reference correct next_workflow
  values.
- Cleanup check — no temporary files remain in the bundle directory.

## Handoff output

Return the handoff YAML schema defined in
`workflow-okf-knowledge-base-creation-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when every verification check passes.
- `constraints_applied` to include OKF conformance and scope discipline.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each check.
- `failures` to list any check that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. corpus integration requires manual review of
  sibling cross-refs).
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - okf-conformance
  - scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: okf-validate
    gate: okf-validate
    result: pass|fail
  - validation: corpus-integration
    gate: corpus-integration-check
    result: pass|fail
  - validation: skill-registration
    gate: skill-registration-check
    result: pass|fail
  - validation: workflow-chaining
    gate: workflow-chaining-check
    result: pass|fail
  - validation: cleanup
    gate: cleanup-check
    result: pass|fail
constraints_checked:
  - okf-conformance
  - scope-discipline
evidence:
  - gate: okf-validate
    output: ...
  - gate: corpus-integration-check
    output: ...
  - gate: skill-registration-check
    output: ...
  - gate: workflow-chaining-check
    output: ...
  - gate: cleanup-check
    output: ...
failures: []
not_fully_checkable: []
```
