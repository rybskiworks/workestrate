---
name: workflow-litellm-validation-03-startup
description: |
  Use only for the startup phase of the LiteLLM validation workflow.
  Run validation-litellm-startup (proxy boots; "Loaded config YAML (api_key
  and environment_variables are not shown)"). Do not use for scoping,
  config-check, smoke, or final reporting.
allowed-tools: Read Bash(litellm:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-validation
  org.phase: startup
  org.phase_order: "03"
---

# LiteLLM Validation Workflow — Phase 03 — Startup

## Phase Purpose

Run the startup gate: boot the proxy with the config and confirm it reaches the
"Loaded config YAML (api_key and environment_variables are not shown)" log. This
validates the config against LiteLLM's internal Pydantic schema at startup.
Record pass/fail and evidence.

## Steps

1. Start the proxy with the config:
   ```sh
   litellm --config infra/litellm/config.yaml
   ```
   Per `docs/litellm/config/config-validation.md` (verbatim): "LiteLLM validates
   `config.yaml` against an internal Pydantic schema at startup. On invalid
   config, the proxy fails to start."
2. Watch for the success log (verbatim from config-validation.md):
   > "Loaded config YAML (api_key and environment_variables are not shown): { ... }"
   Note: `api_key` and `environment_variables` are redacted in the startup log
   (security). They are parsed but not echoed.
3. If the proxy emits a Pydantic `ValidationError` traceback, the gate is fail —
   the proxy will not start. Capture the traceback as evidence.
4. Map to `validation-litellm-startup`. Record pass/fail and the relevant log
   line or traceback.
5. If phase 02 was red, still attempt startup to capture the Pydantic
   `ValidationError` traceback as corroborating evidence, but the overall result
   is already fail.
6. If the gate fails, continue to phase 04 only if the proxy is running; otherwise
   skip smoke and mark it `not_fully_checkable` (proxy not running).
7. Do NOT modify config to make the gate pass. Record the failure and hand off to
   a fixing workflow in phase 05.
8. Shut down the proxy cleanly after capturing the log (so phase 04 can start a
   controlled smoke run, or so the gate leaves no orphan process).

## Docs to Consult

- `docs/litellm/config/config-validation.md`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`

## Operational Skills to Load

- `validation-litellm-startup` — proxy boot and the "Loaded config YAML" log check.

## Constraints to Apply

- `constraint-litellm-config-schema` — do not modify config to make the gate pass.
- `constraint-litellm-secret-hygiene` — confirm `api_key` and
  `environment_variables` are redacted in the startup log (not echoed).

## Validations to Run

- `validation-litellm-startup` — command: `litellm --config infra/litellm/config.yaml`; success signal: "Loaded config YAML (api_key and environment_variables are not shown)"

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (proxy reached the "Loaded config YAML" log) | fail (Pydantic
  `ValidationError` traceback; proxy did not start).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-litellm-config-schema", "constraint-litellm-secret-hygiene"]`
- `tests_run`: `["litellm --config infra/litellm/config.yaml"]` with pass/fail + the log line or traceback.
- `risks`: proxy did not start (smoke will be `not_fully_checkable`); secrets
  echoed in the log (secret-hygiene violation).
- `next_phase`: `04-smoke` (if proxy running) | `05-report` (if proxy not running).
- `next_workflow`: `null`.
- `blockers`: the Pydantic `ValidationError` traceback if `outcome: fail`.
