---
name: constraint-litellm-secret-hygiene
description: |
  Enforces that every LiteLLM secret (api_key, master_key, passwords, tokens) is
  resolved via os.environ/<VAR> indirection and never hardcoded as a literal.
  Load when authoring or reviewing a config.yaml or env for secret handling.
  Does NOT cover which env var names are valid (see env-vars.index.json) or DB
  dependency flags (see constraint-litellm-in-memory-no-db).
metadata:
  org.kind: constraint
---

# Constraint: Secret Hygiene (os.environ/ Indirection Only)

This constraint enforces that no secret material appears as a hardcoded literal
in a LiteLLM `config.yaml`. Every secret-bearing field (`api_key`,
`general_settings.master_key`, `model_info.custom_tokenizer.auth_token`,
`credential_list[].credential_values` containing api_key secrets, and any
password/token field) MUST use the `os.environ/<VAR_NAME>` indirection form,
which runs `os.getenv("<VAR_NAME>")` at config load time. The proxy admin
`master_key` MUST resolve to `os.environ/LITELLM_MASTER_KEY`.

## Triggers

Load this skill when:

- Authoring or editing a LiteLLM `config.yaml` containing `api_key`,
  `master_key`, or any credential/token field.
- Reviewing a PR that touches secret-bearing config lines.
- Triaging a leaked-credential or "key checked into repo" incident.

## Rules

1. Every `model_list[].litellm_params.api_key` MUST use the form
   `os.environ/<VAR>` (verbatim syntax from `extracted/p0-config_settings.md`:
   "us os.environ/<variable name> to pass environment variables ... runs os.getenv
   at load time").
2. `general_settings.master_key` MUST resolve to
   `os.environ/LITELLM_MASTER_KEY` (per `env-vars.index.json`: "Proxy admin
   master key. Must start with 'sk-'.").
3. No secret field may contain a hardcoded literal (e.g. `api_key: sk-abc123`,
   `master_key: sk-1234`). The `option-index.json` `security_risk` field marks
   secret-bearing keys: `api_key` ("Secret credential; use os.environ/
   resolution, never hardcode."), `custom_tokenizer.auth_token` ("Secret
   token."), `credential_list[].credential_values` ("May contain api_key
   secrets.").
4. A placeholder like `api_key: api-key` or `api_key: your-key` is still a
   violation — only `os.environ/<VAR>` is accepted.
5. The `<VAR>` segment must be a non-empty uppercase env var name (no spaces, no
   literal key material after the slash).

## References

- Docs: `docs/litellm/extracted/p0-config_settings.md` (verbatim
  `os.environ/<VAR>` syntax and "runs os.getenv at load time" confirmation;
  `general_settings.master_key` used as `os.environ/...`).
- Schema: `docs/litellm/schemas/env-vars.index.json` (`LITELLM_MASTER_KEY`
  entry).
- Schema: `docs/litellm/schemas/config-yaml.option-index.json`
  (`security_risk` field on `api_key`, `custom_tokenizer.auth_token`,
  `credential_values`).

## Out of scope

- Whether the referenced env var is itself a documented LiteLLM built-in — see
  `env-vars.index.json` (project-defined vars like `KIMI_CODE_API_KEY` are
  allowed as long as they use `os.environ/`).
- Whether a key requires a DB — see `constraint-litellm-in-memory-no-db`.
- Rotation / lifecycle of the env vars themselves.

## Violation examples

### Hardcoded api_key literal

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: openai/gpt-4o
      api_key: sk-proj-abcdef123456   # FORBIDDEN: hardcoded literal secret
```

### Hardcoded master_key

```yaml
general_settings:
  master_key: sk-1234                 # FORBIDDEN: literal; must be os.environ/LITELLM_MASTER_KEY
```

### Placeholder still a literal

```yaml
litellm_params:
  api_key: api-key                    # FORBIDDEN: placeholder, not os.environ/ indirection
```

## How to check

Run `validation-litellm-no-hardcoded-secrets` (scans secret-bearing fields for
non-`os.environ/` values) AND `validation-litellm-config-check` (check g:
`master_key` resolves to `os.environ/LITELLM_MASTER_KEY` and every `api_key` /
token field uses `os.environ/<VAR>`).
