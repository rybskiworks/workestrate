---
name: validation-litellm-config-check
description: |
  Verifies a LiteLLM proxy config.yaml against the on-disk schema index corpus
  (checks a-i). Load after any config edit, before proxy startup, or as a
  periodic health gate. Does NOT cover runtime startup, live smoke testing, or
  hardcoded-secret grep (see validation-litellm-startup,
  validation-litellm-smoke, validation-litellm-no-hardcoded-secrets).
metadata:
  org.kind: validation
---

# Validation: LiteLLM Config Check

This is the static validation gate for a LiteLLM `config.yaml`. It runs the
co-located `check_config.py` script, which cross-checks every config key,
provider prefix, fallback target, env var, and `api_base` against the schema
index files in `docs/litellm/schemas/`. It is read-only — it never modifies
the config. It does not start the proxy or send live requests.

## Triggers

Load this skill when:

- After any edit to `infra/litellm/config.yaml` or a `docs/litellm/examples/`
  config, before claiming completion.
- As the first gate in the LiteLLM validation suite (before startup and smoke
  tests).
- When diagnosing whether a proxy failure is a config-schema error vs. a
  runtime/startup error.

## Procedure

Run the co-located script:

```bash
python3 .agents/skills/validation-litellm-config-check/scripts/check_config.py \
  --config <path-to-config.yaml> \
  --schemas docs/litellm/schemas \
  --mode in-memory
```

`--schemas` is an alias for `--schemas-dir`. Use `--mode db-backed` when the
deployment has Postgres/Redis (then checks e/f do not flag DB/Redis keys).
Default mode is `in-memory` (the workestrator deployment shape).

The script runs checks (a)-(i), encoded in `scripts/check_config.py` and the
8 `constraint-litellm-*` skills (the source of truth):

- **(a)** every config key exists in `config-yaml.option-index.json` under the
  correct `section` (unknown keys and wrong-section keys are FAIL).
- **(b)** every `litellm_params.model` prefix is a valid provider prefix in
  `provider-fields.index.json`.
- **(c)** every `router_settings.fallbacks` source + target resolves to a
  `model_name` in `model_list`.
- **(d)** deprecated keys are flagged (WARN) with their replacement.
- **(e)** in-memory mode: `requires_db=true` keys are FAIL (except
  `disable_spend_logs: true`, the intentional compensating control → WARN);
  DB-requiring env vars (`DATABASE_URL`/`STORE_MODEL_IN_DB`/...) are FAIL.
- **(f)** `requires_redis=true` keys are FAIL in in-memory mode; `REDIS_*` env
  vars are FAIL.
- **(g)** every `os.environ/<VAR>` resolves to a known env var (WARN if
  unknown, FAIL if a clear misspelling of a built-in); `master_key` must
  resolve to `os.environ/LITELLM_MASTER_KEY`.
- **(h)** `openai/` entries: `api_base` ends with `/v1`, no appended endpoint
  path, `api_key` present.
- **(i)** `anthropic/` entries: `api_base` does not pre-include `/v1/messages`
  unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` is set.

## Pass criteria

- Exit code 0.
- Overall verdict `PASS` (zero FAIL).
- WARNs are acceptable but must be listed in the findings.

## Fail criteria

- Exit code 1.
- Any check result `FAIL` (overall verdict `FAIL`).
- Each FAIL cites the offending key path and the schema index field that
  defines the rule.

## Evidence to report

- Exit code.
- The per-check table (check | result) and the findings detail (check, severity,
  key path, message).
- Overall verdict line (`PASS (N FAIL, M WARN)` or `FAIL (N FAIL, M WARN)`).

## Notes

- Read-only: the script never writes secrets or modifies the config.
- The 8 constraints this skill encodes (across checks a-i) are:
  `constraint-litellm-config-schema` (a), `in-memory-no-db` (e+f),
  `secret-hygiene` (g — `os.environ/` indirection + `master_key` rule; for
  literal-secret grep see `validation-litellm-no-hardcoded-secrets`),
  `provider-prefix` (b), `openai-compatible-api-base` (h),
  `anthropic-suffix` (i), `fallback-resolution` (c), `deprecation-free` (d).
- The script requires Python 3 + PyYAML. Run inside `nix develop` if PyYAML is
  not installed on the host.
- The script is robust to missing sections and unknown keys — it reports them
  rather than crashing.
- Source of truth for the checks: `scripts/check_config.py` + the 8
  `constraint-litellm-*` skills. Do not invent rules beyond checks (a)-(i)
  and the schema index files.
