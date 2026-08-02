# LiteLLM Docs Sidebar Navigation Tree

**Source**: Observed from 4 primary root pages (simple_proxy, supported_endpoints, proxy/configs, providers)
**Framework**: Docusaurus v3.8.1 (single global sidebar, `sidebars.js` at repo root)
**Fetched**: 2026-06-27
**HTTP Status**: All 4 primary roots returned 200 OK

The sidebar is a global left-nav rendered identically across all pages; only the active
page highlight and expanded category differ. Below is the full observed hierarchy.

---

## Top-Level Sidebar Categories

1. **Get Started**
   - Quickstart -> /docs/
   - Models & Pricing -> https://models.litellm.ai (external)
   - Changelog -> /release_notes
2. **LiteLLM Python SDK** (collapsed; caret-only)
   - LiteLLM Python SDK -> /docs/#litellm-python-sdk (anchor-only)
3. **LiteLLM AI Gateway (Proxy)** -> /docs/simple_proxy
4. **Supported Endpoints** -> /docs/supported_endpoints
5. **Supported Models & Providers** -> /docs/providers
6. **Routing & Load Balancing** -> /docs/routing-load-balancing
7. **Benchmarks** -> /docs/benchmarks
8. **Contributing** (collapsed) -> /docs/extras/contributing_code
9. **Extras** (collapsed) -> /docs/sdk_custom_pricing
10. **Troubleshooting** (collapsed) -> /docs/troubleshoot/ui_issues

---

## LiteLLM AI Gateway (Proxy) Sub-Tree

- **Getting Started Tutorial** -> /docs/proxy/docker_quick_start
- **Agent & MCP Gateway** (category) -> /docs/a2a
- **Config.yaml** (category)
  - Overview -> /docs/proxy/configs
  - File Management -> /docs/proxy/config_management
  - config_settings -> /docs/proxy/config_settings
- **Setup & Deployment** (category) -> /docs/proxy/quick_start
- **Demo LiteLLM Cloud** -> https://www.litellm.ai/cloud (external)
- **Admin UI** (category) -> /docs/proxy/ui
- **Architecture** (category) -> /docs/proxy/architecture
- **All Endpoints (Swagger)** -> https://litellm-api.up.railway.app/ (external)
- **Authentication** (category) -> /docs/proxy/virtual_keys
- **Budgets + Rate Limits** (category) -> /docs/proxy/users
- **Caching** -> /docs/proxy/caching
- **Memory Management** -> /docs/proxy/memory
- **Guardrails** (category) -> /docs/proxy/guardrails/quick_start
- **Policies** (category) -> /docs/proxy/guardrails/guardrail_policies
- **Create Custom Plugins** (category) -> /docs/proxy/call_hooks
- **LiteLLM Proxy CLI** -> /docs/proxy/management_cli
- **Load Balancing, Routing, Fallbacks** -> /docs/routing-load-balancing
- **A/B Testing - Traffic Mirroring** -> /docs/traffic_mirroring
- **Logging, Alerting, Metrics** (category) -> /docs/proxy/dynamic_logging
- **Making LLM Requests** (category) -> /docs/proxy/user_keys
- **Model Access** (category) -> /docs/proxy/model_access_guide
- **Secret Managers** (category) -> /docs/secret_managers/overview
- **Spend Tracking** (category) -> /docs/proxy/cost_tracking
- **Cost Optimization** (category) -> /docs/proxy/auto_routing

---

## Supported Endpoints Sub-Tree

- /a2a - A2A Agent Gateway -> /docs/a2a
- /assistants -> /docs/assistants
- /audio/transcriptions -> /docs/audio_transcription
- /audio/speech -> /docs/text_to_speech
- /batches (category) -> /docs/batches
- /containers -> /docs/containers
- /containers/files -> /docs/container_files
- /chat/completions (category) -> /docs/completion
- /completions -> /docs/text_completion
- /converse -> /docs/bedrock_converse
- /embeddings -> /docs/embedding/supported_embedding
- /files (category) -> /docs/files_endpoints
- /fine_tuning (category) -> /docs/fine_tuning
- /evals -> /docs/evals_api
- /generateContent -> /docs/generateContent
- /guardrails/apply_guardrail -> /docs/apply_guardrail
- /invoke -> /docs/bedrock_invoke
- /interactions -> /docs/interactions
- /v1beta/agents (Gemini Managed Agents) -> /docs/managed_agents
- /memory -> /docs/memory_management
- /images/edits -> /docs/image_edits
- Image Generations -> /docs/image_generation
- [BETA] Image Variations -> /docs/image_variations
- /videos -> /docs/videos
- /vector_stores/{vector_store_id}/files -> /docs/vector_store_files
- /vector_stores - Create Vector Store -> /docs/vector_stores/create
- /vector_stores/search - Search Vector Store -> /docs/vector_stores/search
- /mcp - Model Context Protocol (category) -> /docs/mcp
- /v1/messages (category) -> /docs/anthropic_unified/
- Token Counting -> /docs/count_tokens
- /v1/messages/count_tokens -> /docs/anthropic_count_tokens
- /moderations -> /docs/moderation
- /ocr -> /docs/ocr
- Pass-through Endpoints (Anthropic SDK, etc.) (category) -> /docs/pass_through/intro
- /rag/ingest -> /docs/rag_ingest
- /rag/query -> /docs/rag_query
- /realtime -> /docs/realtime
- /realtime - WebRTC Support -> /docs/proxy/realtime_webrtc
- /rerank -> /docs/rerank
- /responses -> /docs/response_api
- Prompt Management with Responses API -> /docs/prompt_management
- /responses/compact -> /docs/response_api_compact
- /search (category) -> /docs/search/
- /skills - Anthropic Skills API -> /docs/skills

---

## Supported Models & Providers Sub-Tree

- Integrate as a Model Provider -> /docs/providers/
- Add OpenAI-Compatible Provider (JSON) -> /docs/providers/json
- Add Model Pricing & Context Window -> /docs/providers/add_model_pricing
- OpenAI (category) -> /docs/providers/openai
- OpenAI (Text Completion) -> /docs/providers/openai_completion
- OpenAI-Compatible Endpoints -> /docs/openai_compatible
- Azure OpenAI (category) -> /docs/providers/azure
- Azure AI (category) -> /docs/providers/azure_ai
- Vertex AI (category) -> /docs/providers/vertex
- Google AI Studio (category) -> /docs/providers/gemini
- Anthropic -> /docs/providers/anthropic
- Tool Search -> /docs/providers/tool_search
- AWS Sagemaker -> /docs/providers/aws_sagemaker
- Bedrock (category) -> /docs/providers/bedrock
- LiteLLM Proxy (LLM Gateway) -> /docs/providers/litellm_proxy
- *(... 100+ additional provider pages, each at /docs/providers/<name>)*

**Note**: The full provider list (175 provider URLs in sitemap) is in url_inventory.json.
Priority providers for this knowledge pack: openai, openai_compatible, openrouter, minimax,
moonshot, inception, zai (Z.AI/Zhipu/GLM), vllm, ollama, litellm_proxy.

---

## Notes

- Collapsed categories (Agent & MCP Gateway, Setup & Deployment, Admin UI, Architecture,
  Authentication, Budgets + Rate Limits, Guardrails, Policies, Create Custom Plugins,
  Logging/Alerting/Metrics, Making LLM Requests, Model Access, Secret Managers, Spend
  Tracking, Cost Optimization) have known caret-href (category landing page) but their
  child pages are only visible when expanded. Child pages were enumerated from the sitemap.
- The sidebar config file is `sidebars.js` at the repo root of github.com/BerriAI/litellm-docs.
- `docs/llm_provider/` does NOT exist in the source repo (HTTP 404); provider docs are flat
  at `docs/*.md` and `docs/providers/*.md`.
