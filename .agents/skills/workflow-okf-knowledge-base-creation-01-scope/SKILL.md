---
name: workflow-okf-knowledge-base-creation-01-scope
description: |
  Use only for the scope phase of the OKF knowledge base creation workflow.
  Understand requirements — what knowledge domain, what sources, what depth —
  and record the intended corpus structure. Do not use for design,
  implementation, validation, or verification.
allowed-tools: Read Write Edit Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: okf-knowledge-base-creation
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (OKF knowledge base creation)

## Phase purpose

Understand requirements: what knowledge domain, what sources, what depth.
Identify source URLs/repos to crawl. Determine topic doc count target, skill
count target, workflow count target. Record the intended corpus structure
(directory layout, topic list, skill list). This phase produces the scope
document that every later phase is checked against.

## Steps to perform

1. Capture the knowledge domain, the audience (human/agent/both), and the
   purpose of the knowledge base. Write this down before reading any docs.
2. Read the OKF spec (`okf/SPEC.md`) — bundle structure (§3), concept
   documents (§4), cross-linking (§5), index files (§6), log files (§7),
   citations (§8), conformance (§9).
3. Load `okf-author` skill — understand the concept document authoring model
   (frontmatter + body).
4. Load `okf-extract` skill — understand the source extraction/crawling
   model.
5. Identify source URLs and/or repos to crawl. Record each source with its
   URL, expected content type, and expected concept count.
6. Determine depth targets: topic doc count target, skill count target,
   workflow count target. Record these as measurable goals.
7. Record the intended corpus structure: directory layout, topic list
   (concept IDs), skill list, and workflow list. This is the scope contract
   for the rest of the workflow.

## Docs to consult

- `okf/SPEC.md` (sections 3–9)

## Operational skills to load

- `okf-author`
- `okf-extract`

## Constraints to apply

- Scope discipline — stay within the captured domain and depth targets. Do
  not fold in unrelated knowledge domains or speculative concepts. If a
  related concept appears, record it as a follow-up.

## Validations to run

None — validations run in phase 04 (workflow-okf-knowledge-base-creation-04-validate).

## Handoff output

Return the handoff YAML schema defined in
`workflow-okf-knowledge-base-creation-00-orchestration`. Set:

- `outcome` to `pass` once the scope document (domain, sources, depth targets,
  corpus structure) is recorded.
- `constraints_applied` to include scope discipline.
- `next_phase: 02-design`.
- `blockers: []` unless sources are inaccessible or the domain is too broad
  to scope.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 02-design
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
