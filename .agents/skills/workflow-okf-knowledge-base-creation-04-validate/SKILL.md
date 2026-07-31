---
name: workflow-okf-knowledge-base-creation-04-validate
description: |
  Use only for the validate phase of the OKF knowledge base creation workflow.
  Run okf-validate, check cross-reference integrity, provenance completeness,
  skill coverage, and depth targets. Do not use for scoping, design,
  implementation, or final verification.
allowed-tools: Read Write Edit Bash(git:*) Bash(python3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: okf-knowledge-base-creation
  org.phase: validate
  org.phase_order: "04"
---

# Phase 04: validate (OKF knowledge base creation)

## Phase purpose

Run OKF validation checks against the bundle: conformance, cross-reference
integrity, provenance completeness, skill coverage, and depth targets.
Produce a validation report with pass/fail per check.

## Steps to perform

1. Load `okf-validate` skill — for OKF bundle conformance checking.
2. Run `okf-validate` against the bundle. This checks OKF §9 conformance:
   (a) every non-reserved `.md` has parseable YAML frontmatter, (b) every
   frontmatter has a non-empty `type` field, (c) reserved filenames
   (`index.md`, `log.md`) follow their defined structure.
3. Check cross-reference integrity. Scan all markdown links in concept docs;
   verify every absolute (`/`-prefixed) and relative link target resolves to
   an existing `.md` file in the bundle. Record broken links. (Per OKF §5.3,
   consumers MUST tolerate broken links, but for a freshly created bundle all
   links SHOULD resolve.)
4. Check provenance completeness. Verify every topic doc has at least one
   crawl source recorded in `source-map.md`. Record any docs missing
   provenance.
5. Check skill coverage. Verify every topic doc that the phase 02 skill
   derivation map designates for skill derivation has a corresponding skill
   created. Record any missing skills.
6. Check depth targets. Compare actual counts (topic doc count, skill count,
   workflow count) against the targets recorded in phase 01. Record any
   shortfalls.
7. Produce a validation report with pass/fail per check. Fix failures at the
   root cause before proceeding to phase 05. Do not suppress failures by
   deleting docs or removing links.

## Docs to consult

- `okf/SPEC.md` (sections 5, 9)

## Operational skills to load

- `okf-validate`

## Constraints to apply

- OKF conformance — fix any conformance violation (missing frontmatter,
  missing `type`, malformed reserved files) at the root cause.
- Scope discipline — fix only the new bundle's contribution to any failure.
  If a pre-existing bundle issue is found, surface it as a follow-up.

## Validations to run

- `okf-validate` — runs OKF conformance checks (frontmatter parseable,
  `type` non-empty, reserved filenames structured correctly).
- Cross-reference integrity check — all markdown links resolve to existing
  files.
- Provenance completeness check — all topic docs have crawl sources in
  `source-map.md`.
- Skill coverage check — all designated docs have corresponding skills.
- Depth target check — actual counts meet phase 01 targets.

## Handoff output

Return the handoff YAML schema defined in
`workflow-okf-knowledge-base-creation-00-orchestration`. Set:

- `outcome` to `pass` once all validation checks pass.
- `constraints_applied` to include OKF conformance and scope discipline.
- `tests_run` to list each validation check with its pass/fail.
- `next_phase: 05-verify`.
- `blockers: []` unless a validation failure reveals a design or
  implementation defect that needs an earlier phase to revisit — in that case
  set `outcome: fail` and record the failing check.

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
  - name: okf-validate
    result: pass|fail
  - name: cross-reference-integrity
    result: pass|fail
  - name: provenance-completeness
    result: pass|fail
  - name: skill-coverage
    result: pass|fail
  - name: depth-target
    result: pass|fail
tests_needed:
  - ...
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
