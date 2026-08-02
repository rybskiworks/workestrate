# Gleam Workflow Index

## Purpose

This directory contains step-by-step procedural workflows that an AI agent follows for a specific Gleam task type (implementation, code review, refactoring, debugging, validation). Each workflow is project-independent: it names the docs to consult, the skills to load, the checks to run, and the evidence to report, without duplicating the reference material that lives in the topic docs.

The intended audience is **future AI agents**. Workflows orchestrate procedure; they do not redefine Gleam semantics.

## How workflows relate to docs and skills

- **Topic docs (`docs/gleam/*.md`)** are the source of truth for semantics, conventions, and policy questions. They contain verbatim quotations from official docs, decision tables, review checklists, and "Policy decisions for individual repos" sections. Workflows reference these docs by filename rather than restating their content.
- **Skills (`gleam-language`, `gleam-otp-interop`, `gleam-packages-ffi`)** encode the actionable, repeatable procedures an agent follows (naming a function, structuring an actor, declaring an external, publishing a package). Skills reference their source doc(s) and do not duplicate reference material.
- **Workflows (this directory)** orchestrate which skills to load and which checks to run for a given task type. A workflow tells an agent: read these docs, load these skills, run these commands, report this evidence.

In short: docs hold semantics, skills hold procedures, workflows hold orchestration.

## BEAM bridge

Gleam compiles to two targets: Erlang/BEAM and JavaScript. On the **Erlang target**, Gleam shares the BEAM runtime with Erlang and Elixir: actors map to BEAM processes, supervision maps to OTP supervisors, and runtime debugging uses BEAM tooling. When a workflow touches runtime behavior on the Erlang target — process lifecycle, supervision, message passing, runtime debugging — consult the corresponding `docs/beam/` doc:

- Supervision / restart strategies / child specs → `docs/beam/supervision.md`
- Processes / messages / links / monitors → `docs/beam/processes-and-messages.md`
- Runtime debugging / `sys` / tracing / observer → `docs/beam/runtime-debugging.md`
- Exit reasons / error propagation / let-it-crash → `docs/beam/links-monitors-and-exits.md`
- OTP behaviours (gen_server, gen_statem, gen_event) → `docs/beam/otp-behaviours.md`, `docs/beam/gen-server.md`

On the **JavaScript target**, none of the BEAM docs apply; use JavaScript tooling instead (see `javascript-target.md`).

## Workflow docs

| Workflow file | Purpose | When to use |
|---|---|---|
| `implementation.md` | Write new Gleam code: gather context, follow conventions, run checks, report evidence. | When adding a new feature, module, or function. |
| `code-review.md` | Review a Gleam diff against the corpus checklists and tooling gates. | When reviewing a PR, patch, or generated change. |
| `refactoring.md` | Refactor Gleam code while preserving behavior, verified by `gleam test` before and after. | When restructuring without changing external behavior. |
| `debugging.md` | Reproduce, isolate, diagnose, and fix a Gleam defect. | When a defect is reported or observed. |
| `validation.md` | Run the full validation suite (`gleam check`, `gleam format --check`, `gleam test`, per-target) and report gate status. | Before merge, release, or handoff. |

## Workflow-to-docs-and-skills map

| Workflow | Primary docs consulted | Skills loaded |
|---|---|---|
| `implementation.md` | `language-fundamentals.md`, `types-records-and-patterns.md`, `functions-pipelines-and-use.md`, `result-option-and-errors.md`, `project-structure-and-cli.md`, `conventions-patterns-antipatterns.md` | `gleam-language`, `gleam-packages-ffi` |
| `code-review.md` | `conventions-patterns-antipatterns.md`, `result-option-and-errors.md`, `externals-and-ffi.md`, `types-records-and-patterns.md` | `gleam-language`, `gleam-otp-interop` |
| `refactoring.md` | `stdlib.md`, `result-option-and-errors.md`, `conventions-patterns-antipatterns.md` | `gleam-language` |
| `debugging.md` | `result-option-and-errors.md`, `erlang-interop.md`, `otp-actors-and-supervision.md` | `gleam-otp-interop` |
| `validation.md` | `validation.md`, `testing.md`, `project-structure-and-cli.md`, `package-management-and-publishing.md` | `gleam-packages-ffi` |

## Repo-specific gates

Workflows are project-independent. They name the checks to run but do not hardcode thresholds. Repo-specific gates — whether `gleam format --check` gates CI, whether both targets must pass `gleam test`, whether package-interface export is required, whether an SBOM is produced — are recorded in each repo's `CONTRIBUTING.md` or in the relevant topic doc's "Policy decisions for individual repos" section.

When a workflow says "run `gleam test`", treat that as the corpus recommendation; the consuming repo may require per-target runs (`gleam test --target erlang`, `gleam test --target javascript`). Always defer to the repo's recorded policy when one exists.

The specific gates that vary by repo include:

- **Format enforcement**: whether `gleam format --check` gates CI.
- **Target coverage**: whether both Erlang and JavaScript targets must compile and test, or only the project's primary target.
- **Type-check strictness**: `gleam check` is non-configurable and always strict; there is no Dialyzer equivalent for Gleam.
- **Package-interface export**: whether `gleam docs build` / package interface generation is a release gate.
- **SBOM / supply chain**: whether a dependency audit or SBOM is produced at release time.

## How to use these workflows

1. Identify the task type (implement, review, refactor, debug, validate).
2. Open the corresponding workflow file in this directory.
3. Follow its sections in order: "What happens first" → "Context gathering" → "Checks run" → "Evidence reported".
4. Load the skills the workflow names; do not load unrelated skills as ceremony.
5. Consult the named topic docs for semantics; do not paraphrase them into the workflow output.
6. On the Erlang target, consult the BEAM bridge docs named above when the workflow touches runtime/process/supervision concerns.
7. Report evidence in the format the workflow specifies, so downstream agents and humans can consume it deterministically.

## Conventions shared by every workflow

Every workflow file in this directory follows the same section shape so an agent can parse them uniformly:

- **Purpose** — what the workflow covers and who it is for.
- **What happens first** — the prerequisite step before any code or tooling is touched.
- **Context gathering** — which docs to read and which skills to load.
- **Docs consulted** — the topic docs referenced, by filename.
- **Skills loaded** — the `gleam-*` skills to load.
- **Checks run** — the `gleam` commands to execute, in order.
- **Process** — the step-by-step procedure.
- **Runtime/debugging hooks** — where relevant; notes the BEAM bridge on the Erlang target.
- **Evidence reported** — what the agent reports back, and in what shape.
- **Common mistakes** — anti-patterns to avoid.
- **When to escalate to a human policy decision** — the decision points that exceed agent autonomy.
- **Related docs** / **Related workflows** — cross-references.

## Relationship to the parent index

The parent `docs/gleam/index.md` lists these workflow files in its workflow map. This directory is the concrete realization of that plan. If a workflow is added or removed, update both this index and the parent index's workflow map.

## Related docs

- `../index.md` — top-level Gleam guidance index, including the workflow map.
- `../source-map.md` — provenance of every quotation and decision in the corpus (if present).
- `../../beam/` — BEAM runtime docs consulted via the BEAM bridge on the Erlang target.
