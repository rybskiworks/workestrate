---
name: workflow-okf-knowledge-base-creation-02-design
description: |
  Use only for the design phase of the OKF knowledge base creation workflow.
  Design the bundle directory structure, OKF type taxonomy, cross-reference
  graph, skill derivation map, workflow map, and index.md structure. Do not use
  for scoping, implementation, validation, or verification.
allowed-tools: Read Write Edit Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: okf-knowledge-base-creation
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (OKF knowledge base creation)

## Phase purpose

Design the bundle directory structure, OKF type taxonomy, cross-reference
graph, skill derivation map, workflow map, and index.md structure so the
implement phase can proceed without mid-flight structural changes.

## Steps to perform

1. Load `okf-author` — understand frontmatter fields, body conventions, and
   the `type` field semantics.
2. Design the bundle directory structure. Follow OKF §3: a directory tree of
   markdown files with optional `index.md` and `log.md` at any level. Decide
   the top-level directories (e.g. `tables/`, `datasets/`, `references/`,
   `playbooks/`) based on the scoped domain.
3. Design the OKF type taxonomy. Decide which `type` values to use (e.g.
   `BigQuery Table`, `API Endpoint`, `Metric`, `Playbook`, `Reference`).
   Type values are not registered centrally; pick descriptive,
   self-explanatory values per OKF §4.1.
4. Design the cross-reference graph. Decide which concept docs link to which
   others using absolute (bundle-relative, `/`-prefixed) or relative markdown
   links per OKF §5. Record the link map as a list of source→target pairs.
5. Design the skill derivation map. Decide which topic docs derive into
   operational skills (e.g. a `rust-ownership` topic doc → a
   `rust-ownership-borrowing` skill). Record the doc→skill mapping.
6. Design the workflow map. Decide which workflows to create (e.g.
   implementation, debugging, validation workflows per language/domain).
   Record the workflow list with their phase counts.
7. Design the `index.md` structure. Follow OKF §6: directory listings with
   sections grouping concepts under headings, entries including the
   description from the linked concept's frontmatter. Decide which
   directories get an `index.md`.
8. Decide whether the bundle root `index.md` declares `okf_version: "0.1"`
   per OKF §11.

## Docs to consult

- `okf/SPEC.md` (sections 3–6, 11)

## Operational skills to load

- `okf-author`

## Constraints to apply

- Scope discipline — design only what the captured requirements demand. Do
  not introduce speculative directories, types, or cross-references beyond
  what the scoped domain requires.
- OKF conformance — ensure the design satisfies OKF §9 conformance: every
  non-reserved `.md` will have parseable YAML frontmatter with a non-empty
  `type`; reserved filenames follow their structure.

## Validations to run

None — validations run in phase 04 (workflow-okf-knowledge-base-creation-04-validate).

## Handoff output

Return the handoff YAML schema defined in
`workflow-okf-knowledge-base-creation-00-orchestration`. Set:

- `outcome` to `pass` once directory structure, type taxonomy,
  cross-reference graph, skill derivation map, workflow map, and index.md
  structure are decided and recorded.
- `constraints_applied` to include scope discipline and OKF conformance.
- `next_phase: 03-implement`.
- `blockers: []` unless a structural decision needs human input.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - scope-discipline
  - okf-conformance
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
