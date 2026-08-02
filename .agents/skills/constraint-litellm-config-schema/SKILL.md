---
name: constraint-litellm-config-schema
description: |
  Enforces that every LiteLLM config.yaml key exists in the option index under the
  correct section. Load when authoring, editing, or reviewing a LiteLLM proxy
  config.yaml. Does NOT cover provider-specific field validity (see
  constraint-litellm-provider-prefix), secret hygiene (see
  constraint-litellm-secret-hygiene), or deprecation status (see
  constraint-litellm-deprecation-free).
metadata:
  org.kind: constraint
---

# Constraint: LiteLLM Config Schema (Known Keys Only)

This constraint enforces that every key appearing in a LiteLLM `config.yaml` is a
documented option listed in `docs/litellm/schemas/config-yaml.option-index.json`
under the correct `section`. Unknown keys are rejected as violations — they are
either typos, removed options, or hallucinated settings that LiteLLM will silently
ignore or error on at load time.

## Triggers

Load this skill when:

- Authoring or editing a LiteLLM proxy `config.yaml`.
- Reviewing a PR that changes `config.yaml` or any included child config.
- Triaging a "key not recognized" / silent-ignored-setting symptom.

## Rules

1. Every top-level key MUST be one of: `environment_variables`, `model_list`,
   `litellm_settings`, `callback_settings`, `general_settings`,
   `router_settings`, `credential_list`, `include` (verbatim top-level structure
   in `config-yaml.normalized.schema.md`).
2. Every nested key MUST exist in `config-yaml.option-index.json` under the
   `section` matching its parent path (e.g. a key under `general_settings` must
   appear with `section: "general_settings"`).
3. A key spelled correctly but placed under the wrong `section` is a violation
   (e.g. `fallbacks` is valid under `router_settings`, not under
   `litellm_settings` top-level — though overlap keys are cross-noted in the
   index).
4. Unknown keys (not present in the option index at all) are violations.
5. The `model_list[].litellm_params` dict accepts any key documented with
   `section: "model_list.litellm_params"` plus provider-pass-through params; a
   key absent from the index there is a violation unless it is a documented
   provider pass-through.

## References

- Schema: `docs/litellm/schemas/config-yaml.option-index.json` (`options[].key`,
  `options[].section`).
- Schema: `docs/litellm/schemas/config-yaml.normalized.schema.md` (top-level
  structure, section precedence rules).

## Out of scope

- Whether a provider model prefix is valid — see
  `constraint-litellm-provider-prefix`.
- Whether a secret is hardcoded — see `constraint-litellm-secret-hygiene`.
- Whether a key is deprecated — see `constraint-litellm-deprecation-free`.
- Whether a DB/Redis-requiring key is used in an in-memory deployment — see
  `constraint-litellm-in-memory-no-db`.

## Violation examples

### Unknown top-level key

```yaml
litellm_settings:
  drop_params: true
general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
model_settings:        # FORBIDDEN: not a documented top-level key
  timeout: 30
```

### Known key under wrong section

```yaml
general_settings:
  num_retries: 3        # FORBIDDEN: num_retries belongs in router_settings
```

### Hallucinated nested key

```yaml
router_settings:
  fallback_strategy: latency   # FORBIDDEN: not in option-index.json
```

## How to check

Run `validation-litellm-config-check` (check a: unknown/misspelled keys not in
`config-yaml.option-index.json` under the correct section). The check loads the
option index, walks the config tree, and flags any key whose `section`-qualified
path is absent.
