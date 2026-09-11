---
name: validation-litellm-smoke
description: |
  Verifies the running LiteLLM proxy serves live requests: GET /v1/models
  returns the configured model_name aliases, GET /health/liveliness returns
  200, and POST /v1/chat/completions with a model alias succeeds. Load after
  the startup gate passes. Does NOT cover static config validation (see
  validation-litellm-config-check) or boot validation (see
  validation-litellm-startup).
metadata:
  org.kind: validation
---

# Validation: LiteLLM Smoke

This is the live runtime gate. It sends real HTTP requests to a running
LiteLLM proxy and verifies that the configured `model_name` aliases are
exposed and that a chat completion round-trips successfully. It confirms what
static + startup validation cannot: that providers are reachable and
credentials are valid at runtime.

## Triggers

Load this skill when:

- After `validation-litellm-startup` passes and the proxy is still running.
- Before declaring a config change fully deployed.
- When diagnosing whether a failure is a provider/credential/runtime error vs.
  a config or startup error.

## Procedure

Assume the proxy is running on `http://localhost:4000` and
`LITELLM_MASTER_KEY` is exported in the shell. Run all three checks:

```bash
BASE=http://localhost:4000
KEY=$LITELLM_MASTER_KEY

# 1. Model aliases are exposed.
curl -s -o /tmp/models.json -w "%{http_code}" \
  "$BASE/v1/models" -H "Authorization: Bearer $KEY"

# 2. Liveliness probe.
curl -s -o /tmp/health.txt -w "%{http_code}" \
  "$BASE/health/liveliness"

# 3. Chat completion round-trip with a model alias.
curl -s -o /tmp/chat.json -w "%{http_code}" \
  "$BASE/v1/chat/completions" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"<model-alias>","messages":[{"role":"user","content":"ping"}],"max_tokens":1}'
```

## Pass criteria

- `GET /v1/models` returns HTTP 200 and the response body lists every
  `model_name` alias defined in `model_list` (e.g. `coding`, `coding.fast`,
  `coding.pro`, `coding.free`, `neural`).
- `GET /health/liveliness` returns HTTP 200.
- `POST /v1/chat/completions` returns HTTP 200 and the response body contains
  a `choices` array with a non-empty `message.content`.

## Fail criteria

- Any of the three requests returns a non-200 status code.
- `/v1/models` response is missing a configured `model_name` alias.
- `/v1/chat/completions` returns an error body (e.g. `AuthenticationError`,
  `NotFoundError`, `BadRequestError`) or an empty `choices` array.

## Evidence to report

- HTTP status code for each of the three requests.
- The `/v1/models` response body (or the list of returned `id` values).
- The `/v1/chat/completions` response body (or the error excerpt).
- The model alias used for the chat completion request.

## Notes

- Requires a running proxy (started per `validation-litellm-startup`) and
  `LITELLM_MASTER_KEY` exported in the environment.
- Requires a KVM-capable host for the Docker path; in this environment (no KVM)
  this gate is `not_fully_checkable` unless the proxy is run via the `litellm`
  CLI inside the devshell (`just shell`) or on a KVM host.
- The chat completion request consumes real provider tokens (use `max_tokens: 1`
  and a cheap/free alias like `coding.free` when possible).
- This gate does not validate config schema or boot — run
  `validation-litellm-config-check` and `validation-litellm-startup` first.
