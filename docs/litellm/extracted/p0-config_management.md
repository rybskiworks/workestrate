---
source_url: https://docs.litellm.ai/docs/proxy/config_management
canonical_url: https://docs.litellm.ai/docs/proxy/config_management
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_management.md
title: File Management | liteLLM
sidebar_section_path: "Config.yaml > File Management"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# File Management | liteLLM

## Headings
- # File Management (h1)
- ## `include` external YAML files in a config.yaml (h2)
- ## Examples using `include` (h2)

## Exact config keys found
- `include` — top-level key; takes a single file or a list of external YAML files to merge into the parent config.yaml. (verbatim: "You can use `include` to include external YAML files in a config.yaml.")

## Exact YAML examples (verbatim — preserve indentation exactly)

### Parent config (`parent_config.yaml`) — verbatim
```yaml
include:
  - model_config.yaml # 👈 Key change, will include the contents of model_config.yaml

litellm_settings:
  callbacks: ["prometheus"] 
```

### Included child config (`model_config.yaml`) — verbatim
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_base: https://exampleopenaiendpoint-production.up.railway.app/
  - model_name: fake-anthropic-endpoint
    litellm_params:
      model: anthropic/fake
      api_base: https://exampleanthropicendpoint-production.up.railway.app/
```

### Include a single file — verbatim
```yaml
include:
  - model_config.yaml
```

### Include multiple files — verbatim
```yaml
include:
  - model_config.yaml
  - another_config.yaml
```

## Exact environment variables
- not documented on this page

## Exact provider prefixes found
- not documented on this page (the verbatim child example uses `openai/` and `anthropic/` prefixes illustratively)

## Exact model strings found
- `openai/gpt-4o` (in verbatim child example)
- `anthropic/fake` (in verbatim child example)

## Exact endpoint paths found (runtime inference API on this page)
- not documented on this page

## Exact CLI commands found
- `litellm --config parent_config.yaml --detailed_debug` (verbatim — standard proxy start command; NOT a config-management-specific flag)

## Exact API routes (management, on this page)
- not documented on this page

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- not documented on this page

## Caveats / pitfalls
- The page is short (~1260 bytes, ~30 lines). It documents ONLY the `include` directive for merging external YAML files into a parent config.yaml.
- The page does NOT document: hot-reload / config-reload behavior, config file rotation, config-management-specific CLI flags, or DB/Redis/Enterprise requirements for `include`. Those topics are NOT on this page (and were not found verbatim here).
- The child file (`model_config.yaml`) in the verbatim example contains ONLY a top-level `model_list:` — confirming a child file may contain a single top-level section without a wrapper.
- The verbatim start command `litellm --config parent_config.yaml --detailed_debug` is the standard proxy start command (the `--config` flag selects the parent file; `--detailed_debug` is a verbosity flag). Neither is specific to `include` — the `include` directive is resolved at config-load time by pointing `--config` at the parent file.
- Hot-reload behavior is NOT documented on this page. Whether the proxy picks up `include`d file changes without restart is NOT stated verbatim here. (Cross-ref: config_settings page documents startup-time Pydantic validation; config is loaded at startup.)

## Related links
- https://docs.litellm.ai/docs/proxy/config_settings (top-level YAML schema + Caveats)
- https://docs.litellm.ai/docs/proxy/configs (Overview — model_list, providers, credentials)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
The real config at `infra/litellm/config.yaml` is a single monolithic file (no `include`). The `include` directive is available but unused in workestrator. If adopted, a split config would separate `model_list` (per-provider child files) from `general_settings`/`router_settings`/`litellm_settings` (parent). NOTE: in the workestrator microsandbox the config is mounted **read-only** at `/app/config.yaml`, so even if `include` were used, hot-reload (if it existed) would be blocked by the read-only mount — config changes require a sandbox restart regardless. Hot-reload is NOT documented upstream on this page in any case.

## Confidence / uncertainty notes
- HTTP status 200 confirmed via raw GitHub fetch (raw.githubusercontent.com/.../config_management.md, 1260 bytes).
- Raw markdown fetched verbatim-complete (no truncation); content cross-checked against the rendered docs.litellm.ai page.
- The `include` key, its list-of-strings shape, and all four YAML examples are byte-for-byte verbatim from the raw source.
- Hot-reload, rotation, and config-management CLI flags are confidently NOT on this page (the page is only ~30 lines and was read in full).
