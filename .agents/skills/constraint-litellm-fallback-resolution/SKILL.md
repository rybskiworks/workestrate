---
name: constraint-litellm-fallback-resolution
description: |
  Enforces that every router_settings.fallbacks source model_name and every
  target in its list resolves to a model_name defined in model_list. Load when
  authoring or reviewing router_settings.fallbacks in a LiteLLM config.yaml. Does
  NOT cover fallback management API endpoints (DB-required) or schema validity of
  other router_settings keys (see constraint-litellm-config-schema).
metadata:
  org.kind: constraint
---

# Constraint: Fallbacks Resolve To Defined model_name Entries

This constraint enforces that every model referenced in
`router_settings.fallbacks` (both the source `model_name` key and every target in
its fallback list) exists as a `model_name` in `model_list`. A fallback to an
undefined model is a silent misconfiguration: the router will attempt to fall
back to a deployment that does not exist, producing a routing error at the moment
a fallback is actually needed.

## Triggers

Load this skill when:

- Adding or editing `router_settings.fallbacks` (or `default_fallbacks`,
  `context_window_fallbacks`, `content_policy_fallbacks`) in `config.yaml`.
- Adding or removing a `model_list` entry and checking fallback impact.
- Triaging a "model not found during fallback" routing error.

## Rules

1. `router_settings.fallbacks` is a `List[Dict[str, List[str]]]` (verbatim from
   `extracted/p0-config_settings.md`: "Fallback model for all errors"). Example
   form: `[{"claude-2": ["my-fallback-model"]}]`.
2. For each fallback dict, the key (the source `model_name`) MUST exist as a
   `model_name` in `model_list`. (Validation rule verbatim from
   `extracted/p1-routing-fallback_management.md`: "Model Existence".)
3. For each fallback dict, every entry in the target list MUST exist as a
   `model_name` in `model_list`. (Validation rule verbatim: "Fallback Model
   Existence".)
4. A model MUST NOT fall back to itself. (Validation rule verbatim: "No
   Self-Fallback".)
5. The target list MUST NOT contain duplicates. (Validation rule verbatim: "No
   Duplicates".)
6. The same resolution rules apply to `default_fallbacks`,
   `context_window_fallbacks`, and `content_policy_fallbacks` (all documented
   with the same `List[Dict[str, List[str]]]` shape in `option-index.json` under
   `router_settings`).

## References

- Schema: `docs/litellm/schemas/config-yaml.option-index.json` (`fallbacks`,
  `default_fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`
  under `section: "router_settings"`).
- Docs: `docs/litellm/extracted/p1-routing-fallback_management.md` (verbatim
  validation rules: "Model Existence", "Fallback Model Existence", "No
  Self-Fallback", "No Duplicates").
- Docs: `docs/litellm/extracted/p0-config_settings.md` (fallbacks shape and
  examples).

## Out of scope

- The dynamic fallback management API endpoints (`POST /fallback`, etc.) — those
  require a database (`STORE_MODEL_IN_DB=True`) and are unavailable in-memory; see
  `constraint-litellm-in-memory-no-db`.
- `router.max_fallbacks` count tuning.
- Retry / timeout settings in `router_settings`.

## Violation examples

### Target model not in model_list

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: os.environ/OPENAI_API_KEY
router_settings:
  fallbacks:
    - gpt-4o: ["claude-opus"]   # FORBIDDEN: claude-opus is not a model_name in model_list
```

### Source model not in model_list

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: os.environ/OPENAI_API_KEY
router_settings:
  fallbacks:
    - gpt-5: ["gpt-4o"]         # FORBIDDEN: gpt-5 is not a model_name in model_list
```

### Self-fallback

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: os.environ/OPENAI_API_KEY
router_settings:
  fallbacks:
    - gpt-4o: ["gpt-4o"]        # FORBIDDEN: No Self-Fallback
```

## How to check

Run `validation-litellm-config-check` (check c: every `router_settings.fallbacks`
source key and target list entry resolves to a `model_name` in `model_list`;
also checks `default_fallbacks`, `context_window_fallbacks`,
`content_policy_fallbacks`; enforces No Self-Fallback and No Duplicates). The
check builds the `model_name` set from `model_list` and walks the fallback
graphs.
