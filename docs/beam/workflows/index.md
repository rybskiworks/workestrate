# BEAM / OTP Workflow Index

## Purpose

This directory contains step-by-step procedural workflows that an AI agent follows for a specific BEAM/OTP task type (implementation, code review, refactoring, debugging, runtime diagnosis, validation). Each workflow is project-independent: it names the docs to consult, the skills to load, the checks to run, and the evidence to report, without duplicating the reference material that lives in the topic docs.

The intended audience is **future AI agents**. Workflows orchestrate procedure; they do not redefine BEAM/OTP semantics.

## How workflows relate to docs and skills

- **Topic docs (`docs/beam/*.md`)** are the source of truth for semantics, conventions, and policy questions. They contain verbatim quotations from official docs, decision tables, review checklists, and "Policy decisions for individual repos" sections. Workflows reference these docs by filename rather than restating their content.
- **Skills (`.agents/skills/beam-*`)** encode the actionable, repeatable procedures an agent follows (writing a `gen_server`, structuring a supervisor, interpreting a crash dump). Skills reference their source doc(s) and do not duplicate reference material.
- **Workflows (this directory)** orchestrate which skills to load and which checks to run for a given task type. A workflow tells an agent: read these docs, load these skills, run these commands, report this evidence.

In short: docs hold semantics, skills hold procedures, workflows hold orchestration.

## Workflow docs

| Workflow file | Purpose | When to use |
|---|---|---|
| `implementation.md` | Write new OTP code: gather context, follow conventions, run checks, report evidence. | When adding a new behaviour module, supervisor, or application. |
| `code-review.md` | Review an OTP diff against the corpus checklists and tooling gates. | When reviewing a PR, patch, or generated change. |
| `refactoring.md` | Refactor OTP code while preserving behavior, verified by tests before and after. | When restructuring supervision trees, migrating gen_fsm→gen_statem, or converting special processes to behaviours. |
| `debugging.md` | Reproduce, isolate, diagnose, and fix a BEAM code defect (crash, hang, wrong output). | When a code-level defect is reported or observed. |
| `runtime-diagnosis.md` | Diagnose live-runtime/operational problems: memory leaks, scheduler pressure, mailbox growth, crash dumps. | When inspecting a running or crashed system for operational issues (distinct from code-defect debugging). |
| `validation.md` | Run the full validation suite (compile, Dialyzer, sys inspection, runtime checks) and report gate status. | Before merge, release, or handoff. |

## Workflow-to-docs-and-skills map

| Workflow | Primary docs consulted | Skills loaded |
|---|---|---|
| `implementation.md` | `overview.md`, `gen-server.md` or `gen-statem.md`, `supervision.md`, `applications.md` | `beam-gen-server` or `beam-gen-statem`, `beam-supervision`, `beam-applications-releases` |
| `code-review.md` | `supervision.md`, `gen-server.md`, `gen-statem.md`, `links-monitors-and-exits.md`, `overview.md` | `beam-supervision`, `beam-gen-server`, `beam-gen-statem`, `beam-errors-failures` |
| `refactoring.md` | `supervision.md`, `gen-server.md`, `gen-statem.md`, `proc-lib-and-sys.md`, `overview.md` | `beam-supervision`, `beam-gen-server`, `beam-gen-statem`, `beam-processes` |
| `debugging.md` | `links-monitors-and-exits.md`, `processes-and-messages.md`, `runtime-debugging.md`, `proc-lib-and-sys.md`, `supervision.md` | `beam-errors-failures`, `beam-processes`, `beam-observability-debugging`, `beam-gen-server` |
| `runtime-diagnosis.md` | `runtime-debugging.md`, `proc-lib-and-sys.md`, `processes-and-messages.md`, `logger-and-config.md` | `beam-observability-debugging`, `beam-processes`, `beam-logger-config` |
| `validation.md` | `gen-server.md`, `supervision.md`, `proc-lib-and-sys.md`, `runtime-debugging.md`, `applications.md`, `releases.md` | `beam-supervision`, `beam-gen-server`, `beam-observability-debugging`, `beam-applications-releases` |

## Repo-specific gates

Workflows are project-independent. They name the checks to run but do not hardcode thresholds. Repo-specific gates — Dialyzer gating, compile warnings-as-errors, supervisor intensity defaults, release mode, logger level — are recorded in each repo's `CONTRIBUTING.md` or in the relevant topic doc's "Policy decisions for individual repos" section. See the consolidated [Open policy decisions](../index.md#open-policy-decisions) summary in the parent index.

When a workflow says "run `rebar3 dialyzer`" or "enable warnings-as-errors", treat that as the default corpus recommendation; the consuming repo may relax or tighten it. Always defer to the repo's recorded policy when one exists.

The specific gates that vary by repo include:

- **Dialyzer gating**: gating step vs advisory, and which warning categories are enabled.
- **Compilation warnings**: whether warnings-as-errors is enforced (`erlc -Werror` / `rebar3 compile` / `mix compile --warnings-as-errors`).
- **Supervisor intensity/period**: default values per supervisor level.
- **Release mode**: embedded vs interactive.
- **Logger level**: default primary and module levels.
- **sys debug in production**: whether sys debug/trace options are permitted at all.

## How to use these workflows

1. Identify the task type (implement, review, refactor, debug, runtime-diagnose, validate).
2. Open the corresponding workflow file in this directory.
3. Follow its sections in order: "What happens first" → "Context gathering" → "Checks run" → "Evidence reported".
4. Load the skills the workflow names; do not load unrelated skills as ceremony.
5. Consult the named topic docs for semantics; do not paraphrase them into the workflow output.
6. Report evidence in the format the workflow specifies, so downstream agents and humans can consume it deterministically.

## Conventions shared by every workflow

Every workflow file in this directory follows the same section shape so an agent can parse them uniformly:

- **Purpose** — what the workflow covers and who it is for.
- **What happens first** — the prerequisite step before any code or tooling is touched.
- **Context gathering** — which docs to read and which skills to load.
- **Docs consulted** — the topic docs referenced, by filename.
- **Skills loaded** — the `.agents/skills/beam-*` skills to load.
- **Checks run** — the `erlc`, `rebar3`, `mix`, or `sys` commands to execute, in order.
- **Evidence reported** — what the agent reports back, and in what shape.
- **When human judgment is needed** — the decision points that exceed agent autonomy.
- **Related docs** / **Related workflows** — cross-references.

## Relationship to the parent index

The parent `docs/beam/index.md` lists these workflow files in its "Workflow map" section and maps skill categories to source docs in its "Skill derivation map" section. This directory is the concrete realization of that plan. If a workflow is added or removed, update both this index and the parent index's workflow map.

## Related docs

- `../index.md` — top-level BEAM/OTP guidance index, including the workflow map and skill derivation map.
- `../source-map.md` — provenance of every source URL in the corpus.
