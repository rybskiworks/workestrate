---
name: workflow-litellm-config-change-05-verify
description: |
  Use only for the verify phase of the LiteLLM config-change workflow. Run
  `validation-litellm-startup` (proxy boots, "Loaded config YAML"),
  `validation-litellm-smoke` (GET /v1/models, /health/liveliness, POST
  /v1/chat/completions with model=<alias>), and
  `validation-litellm-no-hardcoded-secrets`. Return the verification handoff.
  Do not use for scoping, design, implementation, or static validation.
allowed-tools: Read Grep Bash
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-change
  org.phase: verify
  org.phase_order: "05"
---

# Workflow: LiteLLM Config Change — 05 Verify

Run the runtime validation gate on the edited config. This phase starts the
proxy, smoke-tests the inference endpoints, and scans for hardcoded secrets.
It returns the verification handoff with extra fields.

## Required source files to read first

- `infra/litellm/config.yaml` — the edited config.
- `docs/litellm/config/config-validation.md` — startup validation behavior and the "Loaded config YAML" log.
- `docs/litellm/schemas/endpoints.index.json` — inference endpoints with `auth_required` and `model_alias_behavior`.

## Exact procedure

1. **Run `validation-litellm-startup`.** Start the proxy with the edited config and confirm it boots:
   ```bash
   litellm --config infra/litellm/config.yaml
   ```
   - PASS: the startup log reaches `"Loaded config YAML (api_key and environment_variables are not shown): { ... }"`.
   - FAIL: a Pydantic `ValidationError` traceback (the proxy will not start). Note: `api_key` and `environment_variables` are redacted in the startup log (security).
2. **Run `validation-litellm-smoke`.** Against the running proxy (default `:4000`), confirm the inference endpoints respond. The smoke test requires a KVM host; if `/dev/kvm` is unavailable, mark `not_fully_checkable` and defer to a KVM-capable host.
   ```bash
   curl -s http://localhost:4000/v1/models
   curl -s http://localhost:4000/health/liveliness
   curl -s http://localhost:4000/v1/chat/completions -H "content-type: application/json" \
     -d '{"model":"<alias>","messages":[{"role":"user","content":"ping"}]}'
   ```
   - PASS: `/v1/models` lists the edited alias; `/health/liveliness` returns healthy; `POST /v1/chat/completions` with `model=<alias>` returns a completion. The `model` field in the request is the `model_name` alias, NOT `litellm_params.model`.
3. **Run `validation-litellm-no-hardcoded-secrets`.** Scan `infra/litellm/config.yaml` for secret literals:
   ```bash
   grep -nE 'sk-[A-Za-z0-9]{16,}|Bearer [A-Za-z0-9]' infra/litellm/config.yaml
   ```
   - PASS: no hardcoded secret literal; every secret uses `os.environ/<VAR>`; `master_key` resolves to `os.environ/LITELLM_MASTER_KEY`.
   - FAIL: any hardcoded secret literal.

## Constraints to apply

(Constraints were applied in earlier phases. This phase confirms `constraint-litellm-secret-hygiene` via the secret scan and confirms the config boots under `constraint-litellm-config-schema` / `constraint-litellm-in-memory-no-db` via successful startup.)

## Validations to run

- `validation-litellm-startup` — proxy boots, "Loaded config YAML" log.
- `validation-litellm-smoke` — GET `/v1/models`, `/health/liveliness`, POST `/v1/chat/completions` with `model=<alias>`.
- `validation-litellm-no-hardcoded-secrets` — no hardcoded secret literals.

## Handoff (verification phase — extra fields)

Return the verification handoff. Set `next_phase: null` (terminal) and `next_workflow: null` (cross-workflow chaining is decided by the orchestrator).

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-secret-hygiene
assumptions: []
risks: []
tests_run:
  - validation-litellm-startup
  - validation-litellm-smoke
  - validation-litellm-no-hardcoded-secrets
tests_needed: []
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
# verification-phase extra fields:
validations_run:
  - validation-litellm-startup: pass|fail
  - validation-litellm-smoke: pass|fail|not_fully_checkable
  - validation-litellm-no-hardcoded-secrets: pass|fail
constraints_checked:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-secret-hygiene
evidence:
  - "Loaded config YAML (api_key and environment_variables are not shown): { ... }"
  - "GET /v1/models -> 200, alias <alias> present"
  - "GET /health/liveliness -> healthy"
  - "POST /v1/chat/completions model=<alias> -> 200 completion"
  - "secret scan: no hardcoded literals"
failures: []
not_fully_checkable:
  - validation-litellm-smoke (if /dev/kvm unavailable — defer to KVM host)
```

If `validation-litellm-smoke` could not run (no KVM host), set `outcome: partial`, add the smoke test to `not_fully_checkable`, and set `handoff_requires_hil: true` with `hil_reason: "smoke test requires KVM host; runtime behavior not confirmed"`.

## Anti-hallucination

The startup log string, endpoints, and `model`-alias behavior cited above trace to `docs/litellm/config/config-validation.md`, `docs/litellm/schemas/endpoints.index.json`, and `docs/litellm/config/model-list.md`. Mark any inferred fact.
