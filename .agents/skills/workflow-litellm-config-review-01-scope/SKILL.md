---
name: workflow-litellm-config-review-01-scope
description: |
  Use only for the scope phase of the LiteLLM config-review workflow. Capture the
  diff's stated intent + risk areas and identify the top-level sections + model_name
  aliases touched. Do not use for analyzing, running checks, manual review, or
  issuing a verdict.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-review
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (LiteLLM config review)

## Phase purpose

Understand the `config.yaml` diff, capture the change's stated intent, and
identify the top-level sections and `model_name` aliases touched — the inputs
that drive which docs and skills to consult in later phases.

## Steps to perform

1. Read the PR/change description and the full `config.yaml` diff.
2. Record: which top-level sections changed (`model_list`, `litellm_settings`,
   `callback_settings`, `general_settings`, `router_settings`, `credential_list`,
   `environment_variables`, `include`) and what the stated intent is.
3. For every `model_list` entry touched, record the `model_name` alias and the
   `litellm_params.model` value (the provider-prefixed string). These aliases
   drive fallback-resolution and routing analysis in later phases.
4. Identify risk areas that will drive which docs and skills to consult in
   02-analyze and 04-review: provider-prefix changes, `api_base` additions,
   `router_settings.fallbacks` edits, secret-bearing keys (`api_key`,
   `master_key`, `credential_values`), DB/Redis-requiring keys (`database_url`,
   `store_model_in_db`, `redis_*`), and deprecated keys (e.g. `set_verbose`).
5. Apply `constraint-litellm-config-schema`: scope the section/keys touched
   against `docs/litellm/schemas/config-yaml.option-index.json` (the
   authoritative key + `section` index). This is a read-only review — do not
   request changes to untouched config; pre-existing issues in untouched keys are
   recorded as separate follow-ups, not as review findings.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json` — authoritative for
  top-level key existence and `section` placement.
- `docs/litellm/config/config-yaml-overview.md` — top-level structure and
  precedence rule.

## Operational skills to load

None mandatory in this phase (regular skills load in 02-analyze).

## Constraints to apply

- `constraint-litellm-config-schema` — scope the touched sections/keys against
  the schema index; review only the diff; do not request changes to untouched
  config.

## Validations to run

None — validations run in phase 03 (workflow-litellm-config-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-litellm-config-review-00-orchestration`. Set:

- `outcome`: `pass` if the diff and intent were captured; `partial` if the change
  description is missing and intent had to be inferred.
- `files_touched`: one entry per changed file (typically
  `infra/litellm/config.yaml`) with a short `change` summary.
- `constraints_applied`: `constraint-litellm-config-schema`.
- `assumptions`: any inferred intent.
- `risks`: the risk areas identified in step 4 (provider prefix, api_base,
  routing/fallback, secret hygiene, DB/Redis keys, deprecations).
- `next_phase`: `02-analyze`.
- `next_workflow`: `null`.
