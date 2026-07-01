---
name: workflow-litellm-validation-02-config-check
description: |
  Use only for the config-check phase of the LiteLLM validation workflow.
  Run validation-litellm-config-check (the checks a–i; the co-located script
  .agents/skills/validation-litellm-config-check/scripts/check_config.py).
  Do not use for scoping, startup, smoke, or final reporting.
allowed-tools: Read Bash(python:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-validation
  org.phase: config-check
  org.phase_order: "02"
---

# LiteLLM Validation Workflow — Phase 02 — Config Check

## Phase Purpose

Run the static config-check gate against `infra/litellm/config.yaml` using the
corpus under `docs/litellm/schemas/` and `docs/litellm/config/config-validation.md`.
This gate CHECKS all 8 LiteLLM constraints. Record pass/fail and evidence per
check (a–i). A failure here is a blocker for the overall verdict, but continue
to the remaining gates to give a full picture.

## Steps

1. Run the config-check gate via the co-located script:
   ```sh
   python .agents/skills/validation-litellm-config-check/scripts/check_config.py \
     --config infra/litellm/config.yaml
   ```
   The script runs the checks a–i against the config and the on-disk corpus.
   Map to `validation-litellm-config-check`.
2. Run the secret-hygiene gate (part of the config-check family):
   map to `validation-litellm-no-hardcoded-secrets`. This scans the config for
   hardcoded secrets (api keys, tokens) that should be sourced from
   `os.environ/<VAR>` instead.
3. For each check (a–i), record pass/fail and the relevant output (the offending
   key/path or clean).
4. If a gate fails, continue to the remaining gates (phase 03, 04) to give a full
   picture; the overall result will be fail.
5. Do NOT modify config to make a gate pass. Record the failure and hand off to a
   fixing workflow in phase 05.

## Docs to Consult

- `docs/litellm/config/config-validation.md`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/provider-fields.index.json`
- `docs/litellm/schemas/env-vars.index.json`

## Operational Skills to Load

- `validation-litellm-config-check` — the co-located `check_config.py` script
  and the checks a–i.

## Constraints to Apply

This phase CHECKS all 8 LiteLLM constraints via the config-check gate:
- `constraint-litellm-config-schema`
- `constraint-litellm-in-memory-no-db`
- `constraint-litellm-secret-hygiene`
- `constraint-litellm-provider-prefix`
- `constraint-litellm-openai-compatible-api-base`
- `constraint-litellm-anthropic-suffix`
- `constraint-litellm-fallback-resolution`
- `constraint-litellm-deprecation-free`

## Validations to Run

- `validation-litellm-config-check` — command: `python .agents/skills/validation-litellm-config-check/scripts/check_config.py --config infra/litellm/config.yaml` (the checks a–i)
- `validation-litellm-no-hardcoded-secrets` — scans config for hardcoded secrets

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all checks a–i green) | fail (any check red) | partial (a
  constraint marked N-A per phase 01, e.g. in-memory-no-db on a DB-backed deploy).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: the 8 constraint names listed above (with N-A noted).
- `tests_run`: `["python .agents/skills/validation-litellm-config-check/scripts/check_config.py --config infra/litellm/config.yaml"]` with pass/fail + evidence per check (a–i).
- `risks`: any check red; a deprecated key still accepted by the schema.
- `next_phase`: `03-startup`.
- `next_workflow`: `null`.
- `blockers`: the failing check and evidence if `outcome: fail`.
