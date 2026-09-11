# Excluded Low-Priority Pages

**Generated**: 2026-06-27
**Total excluded**: 258

These discovered URLs are deliberately excluded from Phase 3 extraction because they are
not relevant to the workestrate LiteLLM proxy knowledge pack.

## Exclusion Categories

### 1. Blog Posts (161 URLs)

Blog posts are announcements, incident reports, and townhall updates. They are not
reference documentation and are excluded from extraction.

- `https://docs.litellm.ai/blog`
- `https://docs.litellm.ai/blog/agent-platform-alpha`
- `https://docs.litellm.ai/blog/agents-are-the-new-llms`
- `https://docs.litellm.ai/blog/akto-partnership`
- `https://docs.litellm.ai/blog/anthropic_advanced_features`
- `https://docs.litellm.ai/blog/anthropic-wildcard-model-access-incident`
- `https://docs.litellm.ai/blog/april-townhall-announcement`
- `https://docs.litellm.ai/blog/april-townhall-updates`
- `https://docs.litellm.ai/blog/archive`
- `https://docs.litellm.ai/blog/authors`
- `https://docs.litellm.ai/blog/ci-cd-v2-improvements`
- `https://docs.litellm.ai/blog/claude_fable_5`
- `https://docs.litellm.ai/blog/claude_opus_4_6`
- `https://docs.litellm.ai/blog/claude_opus_4_7`
- `https://docs.litellm.ai/blog/claude_opus_4_8`
- `https://docs.litellm.ai/blog/claude_sonnet_4_6`
- `https://docs.litellm.ai/blog/claude-code-beta-headers-incident`
- `https://docs.litellm.ai/blog/cleaner-release-versions`
- `https://docs.litellm.ai/blog/componentized-deployment`
- `https://docs.litellm.ai/blog/cve-2026-42208-litellm-proxy-sql-injection`
- *(... +141 more)*

**Reason**: Blog posts are time-sensitive announcements, not reference docs. They may
contain useful context but are not needed for the knowledge pack.

### 2. Tutorial Pages (68 URLs)

Tutorial pages are step-by-step guides for specific integrations and use cases. They are
not reference documentation and are excluded from extraction.

- `https://docs.litellm.ai/docs/tutorials/anthropic_file_usage`
- `https://docs.litellm.ai/docs/tutorials/azure_openai`
- `https://docs.litellm.ai/docs/tutorials/claude_agent_sdk`
- `https://docs.litellm.ai/docs/tutorials/claude_code_beta_headers`
- `https://docs.litellm.ai/docs/tutorials/claude_code_byok`
- `https://docs.litellm.ai/docs/tutorials/claude_code_customer_tracking`
- `https://docs.litellm.ai/docs/tutorials/claude_code_max_subscription`
- `https://docs.litellm.ai/docs/tutorials/claude_code_plugin_marketplace`
- `https://docs.litellm.ai/docs/tutorials/claude_code_prompt_cache_routing`
- `https://docs.litellm.ai/docs/tutorials/claude_code_skills`
- `https://docs.litellm.ai/docs/tutorials/claude_code_websearch`
- `https://docs.litellm.ai/docs/tutorials/claude_desktop_cowork`
- `https://docs.litellm.ai/docs/tutorials/claude_mcp`
- `https://docs.litellm.ai/docs/tutorials/claude_non_anthropic_models`
- `https://docs.litellm.ai/docs/tutorials/claude_responses_api`
- `https://docs.litellm.ai/docs/tutorials/compare_llms`
- `https://docs.litellm.ai/docs/tutorials/compare_llms_2`
- `https://docs.litellm.ai/docs/tutorials/copilotkit_sdk`
- `https://docs.litellm.ai/docs/tutorials/cost_tracking_coding`
- `https://docs.litellm.ai/docs/tutorials/cursor_integration`
- *(... +48 more)*

**Reason**: Tutorials are use-case-specific and not needed for the proxy config reference.
Some tutorials (e.g., /docs/tutorials/fallbacks, /docs/tutorials/model_fallbacks) overlap
with reference docs but are superseded by the P1 routing/fallbacks pages.

### 3. Other Low-Priority (29 URLs)

- `https://docs.litellm.ai/docs/projects/Agent%20Lightning`
- `https://docs.litellm.ai/docs/projects/Codium%20PR%20Agent`
- `https://docs.litellm.ai/docs/projects/dbally`
- `https://docs.litellm.ai/docs/projects/Docq.AI`
- `https://docs.litellm.ai/docs/projects/Elroy`
- `https://docs.litellm.ai/docs/projects/FastREPL`
- `https://docs.litellm.ai/docs/projects/Google%20ADK`
- `https://docs.litellm.ai/docs/projects/GPT%20Migrate`
- `https://docs.litellm.ai/docs/projects/GPTLocalhost`
- `https://docs.litellm.ai/docs/projects/GraphRAG`
- `https://docs.litellm.ai/docs/projects/Harbor`
- `https://docs.litellm.ai/docs/projects/HolmesGPT`
- `https://docs.litellm.ai/docs/projects/Langstream`
- `https://docs.litellm.ai/docs/projects/LiteLLM%20Proxy`
- `https://docs.litellm.ai/docs/projects/llm_cord`
- `https://docs.litellm.ai/docs/projects/mini-swe-agent`
- `https://docs.litellm.ai/docs/projects/openai-agents`
- `https://docs.litellm.ai/docs/projects/OpenInterpreter`
- `https://docs.litellm.ai/docs/projects/Otter`
- `https://docs.litellm.ai/docs/projects/PDL`
- *(... +9 more)*

**Reason**: Community project pages, benchmarks, and other non-reference content.

## Also Excluded (Not in low_priority Classification but Deprioritized)

- **Release notes** (178 URLs): Version-specific changelogs.
  Only the latest stable release notes may be useful for version compatibility info.
- **SDK completion docs** (43 URLs): Python SDK docs, not proxy server docs.
  Only /docs/completion/drop_params is relevant (drop_params is used in config).
- **Observability integrations** (52 URLs):
  Individual third-party logging integration pages. Only the proxy logging/metrics pages are needed.
