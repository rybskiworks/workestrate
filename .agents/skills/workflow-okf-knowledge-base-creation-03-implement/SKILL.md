---
name: workflow-okf-knowledge-base-creation-03-implement
description: |
  Use only for the implement phase of the OKF knowledge base creation workflow.
  Create the bundle directory structure, crawl files, topic docs, index.md
  files, source-map.md, and log.md. Do not use for scoping, design, validation,
  or verification.
allowed-tools: Read Write Edit Bash(git:*) Bash(python3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: okf-knowledge-base-creation
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (OKF knowledge base creation)

## Phase purpose

Create all bundle files on disk following the design from phase 02: directory
structure, crawl files, topic docs, index.md files, source-map.md, and log.md.

## Steps to perform

1. Load `okf-author` — for concept document authoring (frontmatter + body).
2. Load `okf-extract` — for source extraction/crawling into crawl files.
3. Create the bundle directory structure per the phase 02 design. Use
   `mkdir -p` for each top-level and nested directory.
4. Create crawl files (using `okf-extract`). For each source identified in
   phase 01, extract content into crawl files that capture the raw source
   material. Store crawl files in a `references/` or `_crawl/` subdirectory
   as appropriate.
5. Create topic docs (using `okf-author`). For each concept in the scope,
   write a markdown file with:
   - YAML frontmatter with at minimum `type` (required per OKF §4.1), plus
     recommended `title`, `description`, `resource`, `tags`, `timestamp`.
   - Markdown body using structural markdown (headings, lists, tables, fenced
     code blocks) per OKF §4.2.
   - Cross-links to related concepts using absolute (`/`-prefixed)
     bundle-relative links per OKF §5.1.
   - `# Citations` section where claims reference external sources per
     OKF §8.
6. Create `index.md` files per the phase 02 design. Follow OKF §6: no
   frontmatter (except optional `okf_version` at bundle root), sections
   grouping concepts under headings, entries with descriptions from linked
   concept frontmatter.
7. Create `source-map.md` — a mapping of each concept doc to its crawl
   source(s) for provenance traceability.
8. Create `log.md` at the bundle root. Follow OKF §7: flat list of
   date-grouped entries, newest first, ISO 8601 `YYYY-MM-DD` date headings.
   Record the bundle initialization.

## Docs to consult

- `okf/SPEC.md` (sections 3–8)

## Operational skills to load

- `okf-author`
- `okf-extract`

## Constraints to apply

- OKF conformance — every non-reserved `.md` file must have parseable YAML
  frontmatter with a non-empty `type` field. Reserved filenames (`index.md`,
  `log.md`) must follow their defined structure.
- Scope discipline — implement only the concepts, directories, and
  cross-references captured in phases 01 and 02. Record related concepts as
  follow-ups rather than folding them in.
- Provenance — every topic doc must trace to at least one crawl source
  recorded in `source-map.md`.

## Validations to run

None — validations run in phase 04 (workflow-okf-knowledge-base-creation-04-validate).

## Handoff output

Return the handoff YAML schema defined in
`workflow-okf-knowledge-base-creation-00-orchestration`. Set:

- `outcome` to `pass` once all files are created on disk: directory
  structure, crawl files, topic docs, index.md files, source-map.md, log.md.
- `constraints_applied` to include OKF conformance, scope discipline, and
  provenance.
- `files_touched` to list every file created.
- `next_phase: 04-validate`.
- `blockers: []` unless a source is inaccessible or a concept cannot be
  authored.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - okf-conformance
  - scope-discipline
  - provenance
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
