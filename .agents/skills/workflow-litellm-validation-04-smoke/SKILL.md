---
name: workflow-litellm-validation-04-smoke
description: |
  Use only for the smoke phase of the LiteLLM validation workflow.
  Run validation-litellm-smoke (GET /v1/models, /health/liveliness,
  POST /v1/chat/completions with model=<alias>). Requires KVM; mark
  not_fully_checkable if absent. Do not use for scoping, config-check,
  startup, or final reporting.
allowed-tools: Read Bash(curl:*) Bash(litellm:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-validation
  org.phase: smoke
  org.phase_order: "04"
---

# LiteLLM Validation Workflow — Phase 04 — Smoke

## Phase Purpose

Run the live smoke gate against the running proxy: GET `/v1/models`,
GET `/health/liveliness`, and POST `/v1/chat/completions` with
`model=<alias>`. Record pass/fail and evidence per call. **Requires KVM**;
if KVM is absent (determined in phase 01), mark the gate
`not_fully_checkable` and skip the live calls.

## Steps

1. Confirm KVM availability (from phase 01). If `/dev/kvm` is absent, mark the
   smoke gate `not_fully_checkable` and skip the live calls. The validation
   report records verbatim: "this environment has no `/dev/kvm`, so runtime
   validation is blocked here — defer to a KVM-capable host." Go to phase 05.
2. Confirm the proxy is running (from phase 03). If phase 03 was red (proxy did
   not start), skip smoke and mark `not_fully_checkable` (proxy not running).
   Go to phase 05.
3. Run the smoke calls (verbatim endpoints from `docs/litellm/schemas/endpoints.index.json`):
   - GET `/v1/models` — purpose (verbatim): "List available models (OpenAI
     /v1/models). Returns configured model_name entries."
   - GET `/health/liveliness` — purpose (verbatim): "Liveness probe. Returns
     200 if the proxy process is alive." (Note verbatim from the index: "both
     /health/liveness and /health/liveliness exist in the OpenAPI — the
     misspelled variant is the one used by workestrate infra.")
   - POST `/v1/chat/completions` with `model=<alias>` — the `model` field maps
     to a configured `model_name` alias (verbatim from the index: "model field
     maps to config model_name, same aliasing as /v1/chat/completions").
   ```sh
   curl -s http://localhost:4000/v1/models
   curl -s http://localhost:4000/health/liveliness
   curl -s http://localhost:4000/v1/chat/completions -H "content-type: application/json" \
     -d '{"model":"<alias>","messages":[{"role":"user","content":"ping"}]}'
   ```
4. For each call, record pass/fail and the HTTP status / response body summary.
5. Map to `validation-litellm-smoke`. If any call fails, continue to phase 05 to
   give a full picture; the overall result will be fail.
6. Do NOT modify config or the proxy to make a call pass. Record the failure and
   hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/litellm/schemas/endpoints.index.json`
- `docs/litellm/validation-report.md` (KVM-gating precedent)

## Operational Skills to Load

- `validation-litellm-smoke` — the three smoke calls and their pass/fail criteria.

## Constraints to Apply

- `constraint-litellm-config-schema` — do not modify config to make a call pass.
- `constraint-litellm-provider-prefix` — smoke validates that aliases route to
  the correct provider-prefixed model.
- `constraint-litellm-fallback-resolution` — smoke with a fallback alias
  validates fallback resolution at runtime.

## Validations to Run

- `validation-litellm-smoke` — commands: `curl /v1/models`, `curl /health/liveliness`, `curl /v1/chat/completions` with `model=<alias>`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all three calls green) | fail (any call red) | partial
  (KVM absent or proxy not running → `not_fully_checkable`).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-litellm-config-schema", "constraint-litellm-provider-prefix", "constraint-litellm-fallback-resolution"]`
- `tests_run`: `["curl /v1/models", "curl /health/liveliness", "curl /v1/chat/completions (model=<alias>)"]` with pass/fail + HTTP status per call.
- `risks`: KVM absent (smoke `not_fully_checkable`); proxy not running (smoke
  `not_fully_checkable`); a call red.
- `next_phase`: `05-report`.
- `next_workflow`: `null`.
- `blockers`: the failing call and evidence if `outcome: fail`.
