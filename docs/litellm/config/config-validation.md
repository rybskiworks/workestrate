# Config validation

> Source: https://docs.litellm.ai/docs/proxy/config_settings (+ debugging)
> Raw markdown: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_settings.md
>
> NOTE: The raw markdown does NOT contain a dedicated `### config validation` section.
> The validation prose on this page was derived from page caveats and startup log
> behavior documented across multiple LiteLLM pages, not from a verbatim section.

## Purpose

How LiteLLM validates `config.yaml` at startup, failure behavior, and debugging.

## How validation works

(Verbatim) "LiteLLM validates `config.yaml` against an internal Pydantic schema at startup. On invalid config, the proxy fails to start."

Startup log confirms a successful load:

> "Loaded config YAML (api_key and environment_variables are not shown): { ... }"

Note: `api_key` and `environment_variables` are **redacted** in the startup log (security). They are parsed but not echoed.

## Validation flag

No separate `--validate` flag exists **(not documented in fetched source)**. Validation is performed implicitly at proxy startup against the internal Pydantic schema. To validate a config without serving traffic, start the proxy with `--config <path>` and observe whether it reaches the "Loaded config YAML..." log or emits a Pydantic `ValidationError` traceback.

## Common validation pitfalls

(Each derived from the caveats in `docs/litellm/extracted/p0-config_settings.md`; marked as derived, not verbatim.)

- **Unknown/misspelled keys** *(derived)* — rejected by the Pydantic schema; the proxy fails to start. Watch for typos like `num_retry` vs `num_retries`.
- **Wrong type** *(derived)* — e.g. passing a list where a dict is expected for `fallbacks` (each fallback entry is a dict `{model: [fallbacks]}`), or a string where an int is expected. Rejected by Pydantic.
- **Overlap keys set in both `litellm_settings` and `router_settings`** *(derived)* — not an error; `router_settings` silently wins. Easy to misread which value is active.
- **`enable_pre_call_check` (singular) vs `enable_pre_call_checks` (plural)** *(derived)* — both appear verbatim in the router_settings Reference table; both are valid spellings. Do not assume one is a typo.
- **`os.environ/<VAR>` where VAR is unset** *(derived)* — resolves to empty/`None` at load time. This typically causes a downstream auth failure (e.g. empty `api_key`) rather than a config validation failure, because the value type is still a string.
- **`database_url` set but DB unreachable** *(derived)* — a runtime error, not a validation error. The config shape is valid; the connection is not.
- **`set_verbose` deprecated** *(derived)* — still accepted by the schema but deprecated; replacement is `LITELLM_LOG` / `--debug`.

## How to debug

(Verbatim CLI/env from config_settings + debugging.)

- `litellm --debug`
- `litellm --detailed_debug`
- `export LITELLM_LOG="DEBUG"`
- `export JSON_LOGS="True"`

Debugging docs: https://docs.litellm.ai/docs/proxy/debugging

## Reference

- Authoritative key index: [`config-yaml.option-index.json`](../schemas/config-yaml.option-index.json)
- Normalized schema: [`config-yaml.normalized.schema.md`](../schemas/config-yaml.normalized.schema.md) — the authoritative key reference.

## Workestrator notes

> **PROJECT CONTEXT** — not upstream LiteLLM docs. Describes the workestrator deployment specifically.

The real config at `infra/litellm/config.yaml` is validated at proxy startup against LiteLLM's internal Pydantic schema.

To validate before deploy:

```bash
litellm --config infra/litellm/config.yaml
```

Watch for either:

- the "Loaded config YAML (api_key and environment_variables are not shown): { ... }" log (success), or
- a Pydantic `ValidationError` traceback (failure — the proxy will not start).

The harden/validation workflows (see project harden docs; *(see `docs/litellm/harden/` if present, else not documented in fetched source)*) cover pre-flight config checks.

`set_verbose` is deprecated — use `LITELLM_LOG=DEBUG` instead for the workestrator deployment.
