---
source_url: https://docs.litellm.ai/docs/proxy/configs
canonical_url: https://docs.litellm.ai/docs/proxy/configs
title: Overview | liteLLM
sidebar_section_path: "LiteLLM AI Gateway (Proxy) > Config.yaml > Overview"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Overview | liteLLM

## Headings
- ## Quick Start
- ## LLM configs `model_list`
- ## General Settings `general_settings` (DB Connection, etc)
- ## LiteLLM License Key (Enterprise)
- ## Extras
- ### Model-specific params (API Base, Keys, Temperature, Max Tokens, Organization, Headers etc.)
- ### Embedding Models - Use Sagemaker, Bedrock, Azure, OpenAI, XInference
- ### Multiple OpenAI Organizations
- ### Load Balancing
- ### Load API Keys / config values from Environment
- ### Centralized Credential Management
- ### Load API Keys from Secret Managers (Azure Vault, etc)
- ### Set Supported Environments for a model - `production`, `staging`, `development`
- ### Set Custom Prompt Templates
- ### Set custom tokenizer
- ### Configure DB Pool Limits + Connection Timeouts
- ### Cap Idle DB Connections + Pass Extra Prisma URL Params
- ### Disable Server-Side Prepared Statements
- ### Disable Swagger UI
- ### Disable Redoc
- ### Use CONFIG_FILE_PATH for proxy (Easier Azure container deployment)
- ### Providing LiteLLM config.yaml file as a s3, GCS Bucket Object/url

## Exact config keys found
- `model_list` (section: top-level)
- `model_list[].model_name` (section: model_list)
- `model_list[].litellm_params` (section: model_list)
- `model_list[].litellm_params.model` (section: model_list)
- `model_list[].litellm_params.api_base` (section: model_list)
- `model_list[].litellm_params.api_key` (section: model_list)
- `model_list[].litellm_params.api_version` (section: model_list)
- `model_list[].litellm_params.azure_ad_token` (section: model_list)
- `model_list[].litellm_params.seed` (section: model_list)
- `model_list[].litellm_params.max_tokens` (section: model_list)
- `model_list[].litellm_params.temperature` (section: model_list)
- `model_list[].litellm_params.organization` (section: model_list)
- `model_list[].litellm_params.extra_headers` (section: model_list)
- `model_list[].litellm_params.rpm` (section: model_list)
- `model_list[].litellm_params.tpm` (section: model_list)
- `model_list[].litellm_params.aws_region_name` (section: model_list)
- `model_list[].litellm_params.litellm_credential_name` (section: model_list)
- `model_list[].model_info` (section: model_list)
- `model_list[].model_info.version` (section: model_list)
- `model_list[].model_info.supported_environments` (section: model_list)
- `model_list[].model_info.access_groups` (section: model_list)
- `model_list[].model_info.custom_tokenizer` (section: model_list)
- `model_list[].model_info.custom_tokenizer.identifier` (section: model_list)
- `model_list[].model_info.custom_tokenizer.revision` (section: model_list)
- `model_list[].model_info.custom_tokenizer.auth_token` (section: model_list)
- `litellm_settings` (section: top-level)
- `litellm_settings.drop_params` (section: litellm_settings)
- `litellm_settings.success_callback` (section: litellm_settings)
- `litellm_settings.num_retries` (section: litellm_settings)
- `litellm_settings.request_timeout` (section: litellm_settings)
- `litellm_settings.fallbacks` (section: litellm_settings)
- `litellm_settings.context_window_fallbacks` (section: litellm_settings)
- `litellm_settings.allowed_fails` (section: litellm_settings)
- `general_settings` (section: top-level)
- `general_settings.master_key` (section: general_settings)
- `general_settings.alerting` (section: general_settings)
- `general_settings.database_connection_pool_limit` (section: general_settings)
- `general_settings.database_connection_timeout` (section: general_settings)
- `general_settings.database_socket_timeout` (section: general_settings)
- `general_settings.database_connect_timeout` (section: general_settings)
- `general_settings.database_extra_connection_params` (section: general_settings)
- `general_settings.database_disable_prepared_statements` (section: general_settings)
- `router_settings` (section: top-level)
- `router_settings.routing_strategy` (section: router_settings)
- `router_settings.model_group_alias` (section: router_settings)
- `router_settings.num_retries` (section: router_settings)
- `router_settings.timeout` (section: router_settings)
- `router_settings.redis_host` (section: router_settings)
- `router_settings.redis_password` (section: router_settings)
- `router_settings.redis_port` (section: router_settings)
- `credential_list` (section: top-level)
- `credential_list[].credential_name` (section: credential_list)
- `credential_list[].credential_values` (section: credential_list)
- `credential_list[].credential_info` (section: credential_list)

## Exact YAML examples (verbatim — preserve indentation exactly)
```yaml
model_list:
  - model_name: gpt-4o ### RECEIVED MODEL NAME ###
    litellm_params: # all params accepted by litellm.completion() - https://docs.litellm.ai/docs/completion/input
      model: azure/gpt-4o-eu ### MODEL NAME sent to `litellm.completion()` ###
      api_base: https://my-endpoint-europe-berri-992.openai.azure.com/
      api_key: "os.environ/AZURE_API_KEY_EU" # does os.getenv("AZURE_API_KEY_EU")
      rpm: 6      # [OPTIONAL] Rate limit for this deployment: in requests per minute (rpm)
  - model_name: bedrock-claude-v1
    litellm_params:
      model: bedrock/anthropic.claude-instant-v1
  - model_name: gpt-4o
    litellm_params:
      model: azure/gpt-4o-ca
      api_base: https://my-endpoint-canada-berri992.openai.azure.com/
      api_key: "os.environ/AZURE_API_KEY_CA"
      rpm: 6
  - model_name: anthropic-claude
    litellm_params: 
      model: bedrock/anthropic.claude-instant-v1
      ### [OPTIONAL] SET AWS REGION ###
      aws_region_name: us-east-1
  - model_name: vllm-models
    litellm_params:
      model: openai/facebook/opt-125m # the `openai/` prefix tells litellm it's openai compatible
      api_base: http://0.0.0.0:4000/v1
      api_key: none
      rpm: 1440
    model_info: 
      version: 2
    # Use this if you want to make requests to `claude-3-haiku-20240307`,`claude-3-opus-20240229`,`claude-2.1` without defining them on the config.yaml
  # Default models
  # Works for ALL Providers and needs the default provider credentials in .env
  - model_name: "*" 
    litellm_params:
      model: "*"
litellm_settings: # module level litellm settings - https://github.com/BerriAI/litellm/blob/main/litellm/__init__.py
  drop_params: True
  success_callback: ["langfuse"] # OPTIONAL - if you want to start sending LLM Logs to Langfuse. Make sure to set `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` in your env
general_settings: 
  master_key: sk-1234 # [OPTIONAL] Only use this if you to require all calls to contain this key (Authorization: Bearer sk-1234)
  alerting: ["slack"] # [OPTIONAL] If you want Slack Alerts for Hanging LLM requests, Slow llm responses, Budget Alerts. Make sure to set `SLACK_WEBHOOK_URL` in your env
```
(source: https://docs.litellm.ai/docs/proxy/configs)

Load balancing + reliability example (verbatim):
```yaml
model_list:
  - model_name: zephyr-beta
    litellm_params:
        model: huggingface/HuggingFaceH4/zephyr-7b-beta
        api_base: http://0.0.0.0:8001
        rpm: 60      # Optional[int]: When rpm/tpm set - litellm uses weighted pick for load balancing. rpm = Rate limit for this deployment: in requests per minute (rpm).
        tpm: 1000   # Optional[int]: tpm = Tokens Per Minute 
  - model_name: zephyr-beta
    litellm_params:
        model: huggingface/HuggingFaceH4/zephyr-7b-beta
        api_base: http://0.0.0.0:8002
        rpm: 600
  - model_name: zephyr-beta
    litellm_params:
        model: huggingface/HuggingFaceH4/zephyr-7b-beta
        api_base: http://0.0.0.0:8003
        rpm: 60000
  - model_name: gpt-4o
    litellm_params:
        model: gpt-4o
        api_key: <my-openai-key>
        rpm: 200
  - model_name: gpt-3.5-turbo-16k
    litellm_params:
        model: gpt-3.5-turbo-16k
        api_key: <my-openai-key>
        rpm: 100
litellm_settings: 
  num_retries: 3 # retry call 3 times on each model_name (e.g. zephyr-beta)
  request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout 
  fallbacks: [{"zephyr-beta": ["gpt-4o"]}] # fallback to gpt-4o if call fails num_retries
  context_window_fallbacks: [{"zephyr-beta": ["gpt-3.5-turbo-16k"]}, {"gpt-4o": ["gpt-3.5-turbo-16k"]}] # fallback to gpt-3.5-turbo-16k if context window error
  allowed_fails: 3 # cooldown model if it fails > 1 call in a minute.
router_settings: # router_settings are optional
  routing_strategy: simple-shuffle # Literal["simple-shuffle", "least-busy", "usage-based-routing","latency-based-routing"], default="simple-shuffle"
  model_group_alias: {"gpt-4": "gpt-4o"} # all requests with `gpt-4` will be routed to models with `gpt-4o`
  num_retries: 2
  timeout: 30                                  # 30 seconds
  redis_host: <your redis host>                # set this when using multiple litellm proxy deployments, load balancing state stored in redis
  redis_password: <your redis password>
  redis_port: 1992
```
(source: https://docs.litellm.ai/docs/proxy/configs)

Env-loader syntax (verbatim):
```yaml
os.environ/<YOUR-ENV-VAR> # runs os.getenv("YOUR-ENV-VAR")
```
(source: https://docs.litellm.ai/docs/proxy/configs)

Centralized credential management (verbatim):
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/gpt-4o
      litellm_credential_name: default_azure_credential  # Reference credential below
credential_list:
  - credential_name: default_azure_credential
    credential_values:
      api_key: os.environ/AZURE_API_KEY  # Load from environment
      api_base: os.environ/AZURE_API_BASE
      api_version: "2023-07-15"
    credential_info:
      description: "Production credentials for EU region"
      custom_llm_provider: "azure"
```
(source: https://docs.litellm.ai/docs/proxy/configs)

DB pool limits (verbatim):
```yaml
general_settings: 
  database_connection_pool_limit: 10 # sets connection pool per worker for prisma client to postgres db (default: 10, recommended: 10-20)
  database_connection_timeout: 60 # sets a 60s timeout for any connection call to the db 
```
(source: https://docs.litellm.ai/docs/proxy/configs)

## Exact environment variables
- `AZURE_API_KEY_EU` — Azure API key (EU region), loaded via os.environ/
- `AZURE_API_KEY_CA` — Azure API key (Canada region), loaded via os.environ/
- `AZURE_API_BASE` — Azure API base, loaded via os.environ/
- `AZURE_API_KEY` — Azure API key, loaded via os.environ/
- `AZURE_NORTH_AMERICA_API_KEY` — Azure API key (NA region), loaded via os.environ/
- `OPENAI_API_KEY` — OpenAI API key, loaded via os.environ/
- `HUGGINGFACE_API_KEY` — HuggingFace API key, loaded via os.environ/
- `LITELLM_LICENSE` — enterprise license key (export LITELLM_LICENSE="eyJ...")
- `LITELLM_ENVIRONMENT` — set environment for supported_environments (production/staging/development)
- `LITELLM_KEY` — proxy key used in Authorization: Bearer header
- `DATABASE_URL` — postgres database URL (docker run examples)
- `LITELLM_CONFIG_BUCKET_TYPE` — "gcs" for GCS bucket config loading
- `LITELLM_CONFIG_BUCKET_NAME` — bucket name for config loading
- `LITELLM_CONFIG_BUCKET_OBJECT_KEY` — object key for config loading
- `CONFIG_FILE_PATH` — path to config.yaml (easier Azure container deployment)
- `NO_DOCS` — "True" to disable Swagger UI
- `NO_REDOC` — "True" to disable Redoc
- `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY` — Langfuse logging
- `SLACK_WEBHOOK_URL` — Slack alerting

## Exact provider prefixes found
- `azure/` — Azure OpenAI (e.g. azure/gpt-4o-eu, azure/gpt-4o-ca, azure/chatgpt-v-2, azure/azure-embedding-model)
- `bedrock/` — AWS Bedrock (e.g. bedrock/anthropic.claude-instant-v1, bedrock/cohere.command-text-v14, bedrock/amazon.titan-embed-text-v1)
- `openai/` — OpenAI-compatible prefix (e.g. openai/gpt-4o, openai/facebook/opt-125m, openai/*) — "the `openai/` prefix tells litellm it's openai compatible"
- `huggingface/` — HuggingFace (e.g. huggingface/HuggingFaceH4/zephyr-7b-beta, huggingface/microsoft/codebert-base, huggingface/mistralai/Mistral-7B-Instruct-v0.1)
- `sagemaker/` — SageMaker (e.g. sagemaker/berri-benchmarking-gpt-j-6b-fp16)
- `ollama/` — Ollama (e.g. ollama/mistral)
- `xinference/` — Xinference (e.g. xinference/bge-base-en)
- `deepseek/` — DeepSeek (e.g. deepseek/deepseek-chat)

## Exact model strings found
- `gpt-4o` — model_name alias and openai model
- `azure/gpt-4o-eu` — Azure deployment (EU)
- `azure/gpt-4o-ca` — Azure deployment (Canada)
- `bedrock/anthropic.claude-instant-v1` — Bedrock Claude
- `openai/facebook/opt-125m` — vLLM via openai-compatible prefix
- `huggingface/HuggingFaceH4/zephyr-7b-beta` — HF zephyr
- `gpt-3.5-turbo-16k` — OpenAI model
- `deepseek/deepseek-chat` — DeepSeek model
- `text-embedding-ada-002` — OpenAI embedding model

## Exact endpoint paths found (runtime inference API on this page)
- `POST /chat/completions` — curl example to http://0.0.0.0:4000/chat/completions
- `GET /v1/model/info` — curl example to http://0.0.0.0:4000/v1/model/info

## Exact CLI commands found
- `litellm --config /path/to/config.yaml` — start proxy with config
- `litellm --config /path/to/config.yaml --detailed_debug` — start with detailed debug
- `litellm --config config.yaml` — start proxy with config (shorthand)
- `litellm` — start proxy (uses CONFIG_FILE_PATH env var), runs on http://0.0.0.0:4000
- `export LITELLM_LICENSE="eyJ..."` — set enterprise license

## Exact API routes (management, on this page)
- `GET /v1/model/info` — model info (curl example with Authorization: Bearer)

## Requirements
- database: PostgreSQL (via Prisma) — referenced in DB Pool Limits section; DATABASE_URL used in docker run examples. Not required for basic proxy operation; required for virtual keys/spend tracking.
- redis: required only for multi-instance load balancing ("When using multiple LiteLLM Servers / Kubernetes set redis settings `router_settings:redis_host` etc"). Not required for single-instance.
- enterprise: LITELLM_LICENSE required to enable LiteLLM Enterprise features.
- admin_ui: not required on this page (sidebar links to /docs/proxy/ui but page does not require it).

## Deprecations
- none documented on this page

## Caveats / pitfalls
- "When `tpm/rpm` is set + `routing_strategy==simple-shuffle` litellm will use a weighted pick based on set tpm/rpm. In our load tests setting tpm/rpm for all deployments + `routing_strategy==simple-shuffle` maximized throughput"
- "When using multiple LiteLLM Servers / Kubernetes set redis settings `router_settings:redis_host` etc"
- "Look for this line in your console logs to confirm the config.yaml was loaded in correctly." → "LiteLLM: Proxy initialized with Config, Set models:"
- "`database_socket_timeout` is the main knob for capping idle DB connections from LiteLLM."
- "`database_extra_connection_params` is an untyped passthrough — any key you set here **overrides** the LiteLLM-set defaults for that key"
- "The tradeoff is that every query pays the prepare cost instead of amortizing it" (re: database_disable_prepared_statements)
- Include/split-config syntax: NOT DOCUMENTED ON THIS PAGE. The sibling "File Management" page (/docs/proxy/config_management) describes split-config/include syntax.

## Related links
- https://docs.litellm.ai/docs/simple_proxy (LiteLLM AI Gateway landing)
- https://docs.litellm.ai/docs/proxy/config_management (File Management — split config/include syntax)
- https://docs.litellm.ai/docs/proxy/config_settings (config_settings reference)
- https://docs.litellm.ai/docs/completion/input (litellm.completion() params)
- https://docs.litellm.ai/docs/proxy/ui (Admin UI)
- https://docs.litellm.ai/docs/proxy/virtual_keys (Virtual Keys)
- https://docs.litellm.ai/docs/proxy/management_cli (LiteLLM Proxy CLI)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
This is the primary config.yaml structure page. Maps directly to infra/litellm/config.yaml:
- `model_list[].model_name` + `litellm_params{model, api_base, api_key: os.environ/X}` — exact pattern used in real config. The `os.environ/<VAR>` syntax confirmed here: "does os.getenv(...)".
- `general_settings.master_key` — shown as `sk-1234` example; real config uses `os.environ/...` form.
- `litellm_settings.drop_params: True` — documented here verbatim. Used in real config.
- `litellm_settings.num_retries`, `request_timeout`, `fallbacks`, `context_window_fallbacks`, `allowed_fails` — all documented here under litellm_settings. Real config places these under router_settings instead (both valid; router_settings overrides litellm_settings per config_settings page).
- `router_settings.routing_strategy`, `num_retries`, `timeout`, `redis_host/password/port` — documented here. Real config uses num_retries/timeout under router_settings.
- KEY FINDING: This page shows fallbacks/retries/timeouts under BOTH litellm_settings AND router_settings (Block 20). The config_settings page clarifies router_settings overrides litellm_settings for overlapping keys. Real config uses router_settings for these — consistent.
- KEY FINDING: `credential_list` / `litellm_credential_name` is a centralized credential reuse feature (same config, not external file include). Not used in real config but available.
- KEY FINDING: No include/split-config syntax on this page — that's on config_management page (not extracted; deferred).

## Confidence / uncertainty notes
- All 43 code blocks captured verbatim from the fetched page (high confidence).
- The upstream markdown uses bare ``` fences without language tags — reproduced as-is.
- Several upstream code blocks contain source typos (unbalanced quotes, stray backticks, double-`>` chars) — preserved verbatim per anti-hallucination rules.
- HTTP status inferred as 200 from successful full content render (webfetch tool does not expose raw status code).
