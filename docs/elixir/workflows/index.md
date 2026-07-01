# Elixir Workflow Index

## Purpose

This directory contains step-by-step procedural workflows that an AI agent follows for a specific Elixir task type (implementation, code review, refactoring, debugging, validation). Each workflow is project-independent: it names the docs to consult, the skills to load, the checks to run, and the evidence to report, without duplicating the reference material that lives in the topic docs.

The intended audience is **future AI agents**. Workflows orchestrate procedure; they do not redefine Elixir semantics.

## How workflows relate to docs and skills

- **Topic docs (`docs/elixir/*.md`)** are the source of truth for semantics, conventions, and policy questions. They contain verbatim quotations from official docs, decision tables, review checklists, and "Policy decisions for individual repos" sections. Workflows reference these docs by filename rather than restating their content.
- **Skills (`.agents/skills/elixir-*`)** encode the actionable, repeatable procedures an agent follows (naming a function, structuring a supervisor, writing an ExUnit case). Skills reference their source doc(s) and do not duplicate reference material.
- **Workflows (this directory)** orchestrate which skills to load and which checks to run for a given task type. A workflow tells an agent: read these docs, load these skills, run these commands, report this evidence.

In short: docs hold semantics, skills hold procedures, workflows hold orchestration.

## Workflow docs

| Workflow file | Purpose | When to use |
|---|---|---|
| `implementation.md` | Write new Elixir code: gather context, follow conventions, run checks, report evidence. | When adding a new feature, module, or function. |
| `code-review.md` | Review an Elixir diff against the corpus checklists and tooling gates. | When reviewing a PR, patch, or generated change. |
| `refactoring.md` | Refactor Elixir code while preserving behavior, verified by tests before and after. | When restructuring without changing external behavior. |
| `debugging.md` | Reproduce, isolate, diagnose, and fix an Elixir crash, hang, or wrong-output bug. | When a defect is reported or observed. |
| `validation.md` | Run the full validation suite (format, compile, credo, dialyzer, test, coverage) and report gate status. | Before merge, release, or handoff. |

## Workflow-to-docs-and-skills map

| Workflow | Primary docs consulted | Skills loaded |
|---|---|---|
| `implementation.md` | `language-fundamentals.md`, `core-modules.md`, `naming-conventions.md`, `otp-supervision.md` (if OTP), `mix-project-structure.md` | `elixir-coding`, `elixir-otp` (if applicable), `elixir-project-setup` |
| `code-review.md` | `naming-conventions.md`, `language-fundamentals.md`, `core-modules.md`, `otp-supervision.md`, `testing-exunit.md`, `typespecs-and-dialyzer.md` | `elixir-coding`, `elixir-static-analysis`, `elixir-otp` (if OTP code) |
| `refactoring.md` | `core-modules.md`, `otp-supervision.md`, `language-fundamentals.md` | `elixir-coding`, `elixir-static-analysis` |
| `debugging.md` | `error-handling.md`, `beam-otp-internals.md`, `concurrency-processes.md`, `configuration-and-runtime.md` | `elixir-error-handling`, `elixir-otp` (if OTP issue) |
| `validation.md` | `testing-exunit.md`, `typespecs-and-dialyzer.md`, `static-analysis-credo.md`, `mix-project-structure.md` | `elixir-testing`, `elixir-static-analysis` |

## Repo-specific gates

Workflows are project-independent. They name the checks to run but do not hardcode thresholds. Repo-specific gates — coverage thresholds, whether Credo runs in `--strict` mode, whether Dialyzer is a gate or advisory, whether `--warnings-as-errors` is enforced — are recorded in each repo's `CONTRIBUTING.md` or in the relevant topic doc's "Policy decisions for individual repos" section. See the consolidated [Open policy decisions](../index.md#open-policy-decisions) summary in the parent index.

When a workflow says "run `mix credo --strict`", treat `--strict` as the default corpus recommendation; the consuming repo may relax or tighten it. Always defer to the repo's recorded policy when one exists.

The specific gates that vary by repo include:

- **Format enforcement**: whether `mix format --check-formatted` gates CI.
- **Credo mode**: `--strict` vs default, and which `--min-priority` gates the build.
- **Dialyzer gating**: gating step vs advisory, and which warning categories are enabled.
- **Compilation warnings**: whether `--warnings-as-errors` is enforced.
- **Coverage threshold**: the percentage `mix test --cover` must meet, if any.
- **Typespec requirement**: whether `@spec` is required on all public functions or only at library boundaries.

## How to use these workflows

1. Identify the task type (implement, review, refactor, debug, validate).
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
- **Skills loaded** — the `.agents/skills/elixir-*` skills to load.
- **Checks run** — the `mix` commands to execute, in order.
- **Evidence reported** — what the agent reports back, and in what shape.
- **When human judgment is needed** — the decision points that exceed agent autonomy.
- **Related docs** / **Related workflows** — cross-references.

## Relationship to the parent index

The parent `docs/elixir/index.md` lists these workflow files in its "Workflow map" section and maps skill categories to source docs in its "Skill derivation map" section. This directory is the concrete realization of that plan. If a workflow is added or removed, update both this index and the parent index's workflow map.

## Related docs

- `../index.md` — top-level Elixir guidance index, including the workflow map and skill derivation map.
- `../source-map.md` — provenance of every quotation and decision in the corpus.
- `../../beam/index.md` — BEAM/OTP runtime reference corpus; consult `docs/beam/` when Elixir-level docs are insufficient and the underlying Erlang/OTP runtime semantics are needed.
