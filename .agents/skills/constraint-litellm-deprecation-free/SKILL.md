---
name: constraint-litellm-deprecation-free
description: |
  Enforces that no deprecated LiteLLM config key or env var is used, flagging each
  with its documented replacement. Load when authoring or reviewing a config.yaml
  or proxy env for deprecated settings. Does NOT cover schema validity of
  non-deprecated keys (see constraint-litellm-config-schema) or secret hygiene
  (see constraint-litellm-secret-hygiene).
metadata:
  org.kind: constraint
---

# Constraint: No Deprecated Keys or Env Vars

This constraint enforces that a LiteLLM `config.yaml` and proxy environment use
no key or env var marked `deprecated: true` in the option index or env-vars
index. Each violation is flagged with its documented replacement so the author
can migrate. Deprecated settings may be removed in future LiteLLM versions,
silently ignored, or carry security/behavior caveats.

## Triggers

Load this skill when:

- Authoring or editing a LiteLLM `config.yaml` or proxy env.
- Reviewing a PR that touches logging verbosity or auth settings.
- Migrating an older config forward to a current LiteLLM version.

## Rules

1. `litellm_settings.set_verbose` MUST NOT be used. It is `deprecated: true` in
   `config-yaml.option-index.json` with `replacement: "LITELLM_LOG / --debug"`.
   Use the `LITELLM_LOG` env var (`INFO`/`DEBUG`/`ERROR`) or the `--debug` CLI
   flag instead.
2. The env vars `LITELLM_SET_VERBOSE` and `SET_VERBOSE` MUST NOT be set. Both are
   `deprecated: true` in `env-vars.index.json` with `replacement: "LITELLM_LOG"`.
   Use `LITELLM_LOG` instead.
3. `litellm_settings.disable_copilot_system_to_assistant` MUST NOT be used. It is
   `deprecated: true` in `config-yaml.option-index.json` (verbatim from
   `extracted/p0-config_settings.md`: "DEPRECATED - GitHub Copilot API supports
   system prompts.").
4. `general_settings.allow_user_auth` MUST NOT be used. It is `deprecated: true`
   in `config-yaml.option-index.json` (verbatim from
   `extracted/p0-config_settings.md`: "(Deprecated) old approach for user
   authentication.").
5. Every violation MUST be reported with its documented replacement so the author
   can migrate, not merely rejected.

## References

- Schema: `docs/litellm/schemas/config-yaml.option-index.json` (`deprecated: true`
  entries: `set_verbose`, `disable_copilot_system_to_assistant`,
  `allow_user_auth`, with `replacement` fields).
- Schema: `docs/litellm/schemas/env-vars.index.json` (`LITELLM_SET_VERBOSE`,
  `SET_VERBOSE` — `deprecated: true`, `replacement: "LITELLM_LOG"`).
- Docs: `docs/litellm/extracted/p0-config_settings.md` (verbatim deprecation
  notes for each key).

## Out of scope

- Whether a non-deprecated key is schema-valid — see
  `constraint-litellm-config-schema`.
- Whether a replacement env var (e.g. `LITELLM_LOG`) is itself set correctly.
- DB/Redis dependency flags — see `constraint-litellm-in-memory-no-db`.

## Violation examples

### Deprecated set_verbose config key

```yaml
litellm_settings:
  set_verbose: true              # FORBIDDEN: deprecated; use LITELLM_LOG=DEBUG env or --debug flag
```

### Deprecated SET_VERBOSE env var

```env
SET_VERBOSE=True                # FORBIDDEN: deprecated; use LITELLM_LOG=DEBUG
LITELLM_SET_VERBOSE=True        # FORBIDDEN: deprecated; use LITELLM_LOG=DEBUG
```

### Deprecated disable_copilot_system_to_assistant

```yaml
litellm_settings:
  disable_copilot_system_to_assistant: false   # FORBIDDEN: deprecated (GitHub Copilot API supports system prompts)
```

### Deprecated allow_user_auth

```yaml
general_settings:
  allow_user_auth: true         # FORBIDDEN: deprecated old user-auth approach
```

## How to check

Run `validation-litellm-config-check` (check d: no config key with
`deprecated: true` in `config-yaml.option-index.json` is used; no env var with
`deprecated: true` in `env-vars.index.json` is set; each violation is reported
with its `replacement`). The check cross-references the `deprecated` and
`replacement` fields from both indexes.
