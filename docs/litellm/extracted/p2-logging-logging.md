---
source_url: https://docs.litellm.ai/docs/proxy/logging
canonical_url: https://docs.litellm.ai/docs/proxy/logging
title: "Logging"
sidebar_section_path: proxy > logging (Logging, Alerting, Metrics)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# Logging

## Headings
- Getting the LiteLLM Call ID
- Logging Features
  - Redact Messages, Response Content
  - Redacting UserAPIKeyInfo
  - Disable Message Redaction
  - Turn off all tracking/logging
  - Dynamically Disable specific callbacks
  - Conditional Logging by Virtual Keys, Teams
- What gets logged?
- Langfuse
- OpenTelemetry
- Google Cloud Storage Buckets
- Google Cloud Storage - PubSub Topic
- Deepeval
- s3 Buckets
- AWS SQS
- Azure Blob Storage
- Datadog
- Azure Sentinel
- Lunary
- MLflow
- Custom Callback Class [Async]
- Custom Callback APIs [Async]
- Langsmith
- Arize AI
- Langtrace
- Galileo

## Exact config keys found (full path, verbatim spelling)
- `litellm_settings.success_callback` (litellm_settings) — list of success callback names (alt: `callbacks`)
- `litellm_settings.failure_callback` (litellm_settings) — list of failure callback names
- `litellm_settings.callbacks` (litellm_settings) — preferred; supports python `file.instance`, `s3://`, `gcs://` URLs
- `litellm_settings.turn_off_message_logging` (litellm_settings) — bool; redact messages
- `litellm_settings.redact_user_api_key_info` (litellm_settings) — bool
- `litellm_settings.global_disable_no_log_param` (litellm_settings) — bool; turn off all tracking/logging
- `litellm_settings.langfuse_default_tags` (litellm_settings) — list
- `litellm_settings.forward_traceparent_to_llm_provider` (litellm_settings) — bool
- `litellm_settings.s3_callback_params` (litellm_settings) — sub-keys: `s3_bucket_name`, `s3_region_name`, `s3_aws_access_key_id`, `s3_aws_secret_access_key`, `s3_path`, `s3_endpoint_url`, `s3_use_virtual_hosted_style`, `s3_strip_base64_files`, `s3_use_team_prefix`, `s3_use_key_prefix`
- `litellm_settings.aws_sqs_callback_params` (litellm_settings) — sub-keys: `sqs_queue_url`, `sqs_region_name`, `sqs_strip_base64_files`, `s3_use_team_prefix`, `s3_use_key_prefix`
- `callback_settings.otel.message_logging` (callback_settings) — sub-key per callback name
- `environment_variables` (top-level config key) — map for adding env vars via config

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Redact Messages, Response Content (Global)
model_list:
 - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
litellm_settings:
  success_callback: ["langfuse"]
  turn_off_message_logging: True # 👈 Key Change
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# Redacting UserAPIKeyInfo
litellm_settings: 
  callbacks: ["langfuse"]
  redact_user_api_key_info: true
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# Turn off all tracking/logging
litellm_settings:
  global_disable_no_log_param: True
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# Langfuse
model_list:
 - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
litellm_settings:
  success_callback: ["langfuse"]
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# OpenTelemetry
litellm_settings:
  callbacks: ["otel"]
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# OpenTelemetry - Redacting Messages, Response Content
litellm_settings:
  callbacks: ["otel"]
## 👇 Key Change
callback_settings:
  otel:
    message_logging: False
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# s3 Buckets
model_list:
 - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
litellm_settings:
  success_callback: ["s3_v2"]
  s3_callback_params:
    s3_bucket_name: logs-bucket-litellm
    s3_region_name: us-west-2
    s3_aws_access_key_id: os.environ/AWS_ACCESS_KEY_ID
    s3_aws_secret_access_key: os.environ/AWS_SECRET_ACCESS_KEY
    s3_path: my-test-path # [OPTIONAL] set path in bucket you want to write logs to
    s3_endpoint_url: https://s3.amazonaws.com  # [OPTIONAL]
    s3_use_virtual_hosted_style: false # [OPTIONAL]
    s3_strip_base64_files: false # [OPTIONAL]
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# Custom Callback Class
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
litellm_settings:
  callbacks: custom_callbacks.proxy_handler_instance
```
(source: https://docs.litellm.ai/docs/proxy/logging)

```yaml
# Langsmith
litellm_settings:
  success_callback: ["langsmith"]
environment_variables:
  LANGSMITH_API_KEY: "lsv2_pt_xxxxxxxx"
  LANGSMITH_PROJECT: "litellm-proxy"
  LANGSMITH_BASE_URL: "https://api.smith.langchain.com" # (Optional)
```
(source: https://docs.litellm.ai/docs/proxy/logging)

## Exact environment variables
- `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY` / `LANGFUSE_HOST` — Langfuse
- `OTEL_TRACER_NAME` / `OTEL_SERVICE_NAME` / `OTEL_EXPORTER` / `OTEL_ENDPOINT` / `OTEL_HEADERS` — OpenTelemetry
- `GCS_BUCKET_NAME` / `GCS_PATH_SERVICE_ACCOUNT` / `GCS_PUBSUB_TOPIC_ID` / `GCS_PUBSUB_PROJECT_ID` — GCS
- `CONFIDENT_API_KEY` — Deepeval
- `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION_NAME` — S3
- `AZURE_STORAGE_ACCOUNT_NAME` / `AZURE_STORAGE_FILE_SYSTEM` / `AZURE_STORAGE_ACCOUNT_KEY` / `AZURE_STORAGE_TENANT_ID` / `AZURE_STORAGE_CLIENT_ID` / `AZURE_STORAGE_CLIENT_SECRET` — Azure Blob
- `LUNARY_PUBLIC_KEY` — Lunary
- `GENERIC_LOGGER_ENDPOINT` / `GENERIC_LOGGER_HEADERS` — Custom API callback
- `LANGSMITH_API_KEY` / `LANGSMITH_PROJECT` / `LANGSMITH_BASE_URL` — Langsmith
- `ARIZE_SPACE_KEY` / `ARIZE_API_KEY` / `ARIZE_ENDPOINT` / `ARIZE_HTTP_ENDPOINT` — Arize
- `LANGTRACE_API_KEY` — Langtrace
- `GALILEO_API_KEY` / `GALILEO_PROJECT_ID` / `GALILEO_LOG_STREAM_ID` / `GALILEO_BASE_URL` / `GALILEO_USERNAME` / `GALILEO_PASSWORD` — Galileo

## Exact endpoint paths / API routes (runtime + management)
- `POST /chat/completions` — OpenAI-compatible chat completions
- `POST /v1/chat/completions` — OpenAI v1-compatible chat completions

## Exact CLI commands
- `litellm --config config.yaml` — Start proxy with config
- `litellm --config config.yaml --debug` — Start proxy with debug logging
- `litellm --config config.yaml --detailed_debug` — Start proxy with detailed debug
- `litellm --test` — Make a test request after start

## Requirements
- database: not documented on this page (logging callbacks do not specify DB requirements)
- redis: not documented on this page
- enterprise: required for GCS Buckets, GCS PubSub, Azure Blob Storage, Custom Callback APIs, Dynamically Disable specific callbacks — verbatim: "✨ This is an enterprise feature" (each)
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "Dynamic request message redaction is in BETA."
- "Only use this for self hosted LLMs, this can cause Bedrock, VertexAI calls to fail" (for `forward_traceparent_to_llm_provider: True`)
- "Note: OTLP gRPC requires `grpcio`. Install via `uv add "litellm[grpc]"` (or `grpcio`)."
- "if both team alias and key alias are enabled then the path becomes `my-test-path/my-team-alias/my-key-alias/...`"
- Supported callback names (verbatim from YAML): `langfuse`, `otel`, `gcs_bucket`, `gcs_pubsub`, `deepeval`, `s3_v2`, `aws_sqs`, `azure_storage`, `lunary`, `langsmith`, `arize`, `langtrace`, `generic_api`, `datadog`

## Related links
- /docs/proxy/team_logging
- /docs/proxy/logging_spec
- /docs/observability/opentelemetry_integration
- /docs/observability/datadog
- /docs/observability/azure_sentinel
- /docs/observability/mlflow
- /docs/observability/custom_callback
- /docs/enterprise
- /docs/proxy/dynamic_logging

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Logging callbacks (`litellm_settings.success_callback` / `failure_callback` / `callbacks`) work WITHOUT a database — they send logs to external services (Langfuse, OTEL, S3, etc.). The workestrate can use `callbacks: ["otel"]` or `success_callback: ["langfuse"]` etc. in-memory. `turn_off_message_logging` and `global_disable_no_log_param` work without DB. `redact_user_api_key_info` works without DB. `general_settings.disable_spend_logs` (from db_info page) is separate — it disables DB spend log writes (relevant since no DB). Custom callback classes (`callbacks: custom_callbacks.proxy_handler_instance`) work without DB. Note: `s3_v2` callback requires AWS creds but no DB/Redis.

## Confidence / uncertainty notes
- high confidence on callback config keys and callback names (verbatim YAML). DB requirement is "not documented" — callbacks are external integrations that don't need LiteLLM's DB (inferred high confidence). Enterprise requirements are verbatim.
