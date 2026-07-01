---
source_url: https://docs.litellm.ai/docs/proxy/quick_start
canonical_url: https://docs.litellm.ai/docs/proxy/quick_start
title: CLI - Quick Start | liteLLM
sidebar_section_path: "LiteLLM AI Gateway (Proxy) > Setup & Deployment > CLI - Quick Start"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# CLI - Quick Start | liteLLM

## Headings
- ## Quick Start - LiteLLM Proxy CLI
- ## Quick Start - LiteLLM Proxy + Config.yaml
- ## Using LiteLLM Proxy - Curl Request, OpenAI Package, Langchain
- ## 📖 Proxy Endpoints - Swagger Docs
- ## Debugging Proxy
- ### Test
- ### Supported LLMs
- ### Create a Config for LiteLLM Proxy
- ### Run proxy with config
- ### Set Debug Level using env variables

## Exact config keys found
- `model_list` (section: top-level)
- `model_list[].model_name` (section: model_list)
- `model_list[].litellm_params` (section: model_list)
- `model_list[].litellm_params.model` (section: model_list)
- `model_list[].litellm_params.api_base` (section: model_list)
- `model_list[].litellm_params.api_key` (section: model_list)

## Exact YAML examples (verbatim — preserve indentation exactly)
```yaml
model_list:
  - model_name: gpt-3.5-turbo # user-facing model alias
    litellm_params: # all params accepted by litellm.completion() - https://docs.litellm.ai/docs/completion/input
      model: azure/<your-deployment-name>
      api_base: <your-azure-api-endpoint>
      api_key: <your-azure-api-key>
    - model_name: gpt-3.5-turbo
      litellm_params:
        model: azure/gpt-turbo-small-ca
        api_base: https://my-endpoint-canada-berri992.openai.azure.com/
        api_key: <your-azure-api-key>
    - model_name: vllm-model
      litellm_params:
        model: openai/<your-model-name>
        api_base: <your-vllm-api-base> # e.g. http://0.0.0.0:3000/v1
        api_key: <your-vllm-api-key|none>
```
(source: https://docs.litellm.ai/docs/proxy/quick_start)

## Exact environment variables
- `AWS_ACCESS_KEY_ID` — AWS access key (Bedrock/Sagemaker)
- `AWS_REGION_NAME` — AWS region
- `AWS_SECRET_ACCESS_KEY` — AWS secret key
- `AZURE_API_KEY` — Azure API key
- `AZURE_API_BASE` — Azure API base
- `OPENAI_API_KEY` — OpenAI API key
- `VERTEX_PROJECT` — Vertex AI project
- `VERTEX_LOCATION` — Vertex AI location
- `HUGGINGFACE_API_KEY` — HuggingFace API key (marked #[OPTIONAL])
- `ANTHROPIC_API_KEY` — Anthropic API key
- `TOGETHERAI_API_KEY` — TogetherAI API key
- `REPLICATE_API_KEY` — Replicate API key
- `PALM_API_KEY` — PaLM API key
- `AI21_API_KEY` — AI21 API key
- `COHERE_API_KEY` — Cohere API key
- `LITELLM_LOG` — log level: "INFO" / "DEBUG" / "None"

## Exact provider prefixes found
- `huggingface/` — HuggingFace (e.g. huggingface/bigcode/starcoder)
- `bedrock/` — AWS Bedrock (e.g. bedrock/anthropic.claude-v2)
- `azure/` — Azure OpenAI (e.g. azure/my-deployment-name, azure/gpt-turbo-small-ca)
- `openai/` — OpenAI / OpenAI-compatible (e.g. openai/<your-model-name>)
- `ollama/` — Ollama (e.g. ollama/<ollama-model-name>)
- `vertex_ai/` — Vertex AI (e.g. vertex_ai/gemini-pro)
- `sagemaker/` — SageMaker (e.g. sagemaker/jumpstart-dft-meta-textgeneration-llama-2-7b)
- `vllm/` — vLLM (e.g. vllm/facebook/opt-125m)
- `together_ai/` — TogetherAI (e.g. together_ai/lmsys/vicuna-13b-v1.5-16k)
- `replicate/` — Replicate (e.g. replicate/meta/llama-2-70b-chat:...)
- `petals/` — Petals (e.g. petals/meta-llama/Llama-2-70b-chat-hf)
- `palm/` — PaLM (e.g. palm/chat-bison)

## Exact model strings found
- `huggingface/bigcode/starcoder` — HF model
- `bedrock/anthropic.claude-v2` — Bedrock Claude
- `azure/my-deployment-name` — Azure deployment
- `gpt-3.5-turbo` — OpenAI model (used as model_name alias and direct model)
- `ollama/<ollama-model-name>` — Ollama model
- `vertex_ai/gemini-pro` — Vertex Gemini
- `sagemaker/jumpstart-dft-meta-textgeneration-llama-2-7b` — SageMaker model
- `claude-instant-1` — Anthropic model (no prefix)
- `vllm/facebook/opt-125m` — vLLM model
- `together_ai/lmsys/vicuna-13b-v1.5-16k` — TogetherAI model
- `replicate/meta/llama-2-70b-chat:02e509c789964a7ea8736978a43525956ef40397be9033abf9fd2badfe68c9e3` — Replicate model
- `petals/meta-llama/Llama-2-70b-chat-hf` — Petals model
- `palm/chat-bison` — PaLM model
- `j2-light` — AI21 model (no prefix)
- `command-nightly` — Cohere model (no prefix)

## Exact endpoint paths found (runtime inference API on this page)
- `POST /chat/completions` — primary chat endpoint (curl example)
- `POST /completions` — listed in Proxy Endpoints section
- `POST /embeddings` — listed in Proxy Endpoints section
- `GET /models` — listed in Proxy Endpoints section
- `POST /key/generate` — listed in Proxy Endpoints section (management route)

## Exact CLI commands found
- `uv tool install 'litellm[proxy]'` — install LiteLLM CLI
- `litellm --model huggingface/bigcode/starcoder` — start proxy with single model
- `litellm --model huggingface/bigcode/starcoder --detailed_debug` — start with debug
- `litellm --test` — test command
- `litellm --model bedrock/anthropic.claude-v2` — start with Bedrock
- `litellm --model azure/my-deployment-name` — start with Azure
- `litellm --model gpt-3.5-turbo` — start with OpenAI
- `litellm --model ollama/<ollama-model-name>` — start with Ollama
- `litellm --model openai/<your model name> --api_base <your-api-base>` — start with OpenAI-compatible
- `litellm --model vertex_ai/gemini-pro` — start with Vertex
- `litellm --model huggingface/<your model name> --api_base <your-api-base>` — start with HF TGI
- `litellm --model sagemaker/jumpstart-dft-meta-textgeneration-llama-2-7b` — start with SageMaker
- `litellm --model claude-instant-1` — start with Anthropic
- `litellm --model vllm/facebook/opt-125m` — start with vLLM
- `litellm --model together_ai/lmsys/vicuna-13b-v1.5-16k` — start with TogetherAI
- `litellm --model replicate/meta/llama-2-70b-chat:...` — start with Replicate
- `litellm --model petals/meta-llama/Llama-2-70b-chat-hf` — start with Petals
- `litellm --model palm/chat-bison` — start with PaLM
- `litellm --model j2-light` — start with AI21
- `litellm --model command-nightly` — start with Cohere
- `litellm --config your_config.yaml` — start with config file
- `litellm --model gpt-3.5-turbo --debug` — debug mode
- `litellm --model gpt-3.5-turbo --detailed_debug` — detailed debug mode
- `export LITELLM_LOG=INFO` / `=DEBUG` / `=None` — set log level via env

## Exact API routes (management, on this page)
- `POST /key/generate` — listed in Proxy Endpoints section (key generation)

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: only mentioned in footer promo card (not a hard requirement for quick start)
- admin_ui: linked in sidebar at /docs/proxy/ui but not required by this page

## Deprecations
- none documented on this page

## Caveats / pitfalls
- "Run with `--detailed_debug` if you need detailed debug logs"
- "Ensure you're using openai v1.0.0+" (under Test section)
- "This is **not recommended**. There is duplicate logic as the proxy also uses the sdk, which might lead to unexpected errors." (re: using LiteLLM SDK directly against the proxy)
- "Seel all supported llms" [sic — typo in source] (link to providers)
- Default proxy host/port: http://0.0.0.0:4000
- Swagger reference: https://litellm-api.up.railway.app/

## Related links
- https://docs.litellm.ai/docs/proxy/user_keys (More examples — Making LLM Requests)
- https://docs.litellm.ai/docs/proxy/configs (full config.yaml schema)
- https://docs.litellm.ai/docs/proxy/cli (full CLI flag reference)
- https://docs.litellm.ai/docs/enterprise (enterprise features)
- https://docs.litellm.ai/docs/proxy/ui (Admin UI setup)
- https://docs.litellm.ai/docs/providers (all supported LLMs)
- https://litellm-api.up.railway.app/ (Swagger docs)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
This is the CLI quick start. Relevant to workestrator:
- `litellm --config your_config.yaml` is the primary startup command — matches real config usage.
- `litellm --model <provider/model>` is the single-model quick-start (no config.yaml needed) — NOT used in real config (real config uses config.yaml with model_list).
- `--detailed_debug` / `--debug` flags and `LITELLM_LOG` env var control logging verbosity — relevant for debugging.
- The config.yaml structure shown here (model_list with model_name + litellm_params{model, api_base, api_key}) matches the real config pattern.
- Provider prefix pattern `<provider>/<model>` (e.g. `openai/<model>`, `azure/<deployment>`) is the litellm_params.model format — matches real config.
- `openai/` prefix means "OpenAI-compatible API" — used in real config for OpenAI-compatible endpoints (e.g. openrouter models via openai/ prefix).
- Proxy Endpoints listed: /chat/completions, /completions, /embeddings, /models, /key/generate — the real config primarily uses /chat/completions.

## Confidence / uncertainty notes
- HTTP status 200, no redirect (confirmed by searcher).
- All 46 code blocks captured verbatim (high confidence). Code fences are bare (no language tags).
- Some env-export blocks render on a single line in the markdown source — reproduced as fetched.
- The page contains 5 h3 headings (not 3 as might be assumed).
- Config.yaml structure on this page is minimal (model_list + litellm_params only); full schema is on /docs/proxy/configs and /docs/proxy/config_settings (extracted separately).
