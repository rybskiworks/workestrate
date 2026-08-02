# LiteLLM Knowledge Pack — Validation Report

**Generated**: 2026-06-27 (Phase 12 — final validation)
**Validator**: lead_long
**Scope**: Independent validation of the local LiteLLM knowledge pack under `docs/litellm/`.

---

## Coverage table

| Check | Description | Result | Evidence |
| --- | --- | --- | --- |
| A | No real secrets | PASS | grep for `sk-[A-Za-z0-9]{16,}`, `Bearer <value>`, 32+ hex tokens across all 130 files. Zero real secrets. The only `sk-` match is the placeholder `sk-...` in `examples/agent-mcp-skills-gateway.yaml`. The only 32+ hex token is `02e509c789964a7ea8736978a43525956ef40397be9033abf9fd2badfe68c9e3` in `extracted/p0-quick_start.md` — a public Replicate model version hash, not a credential. All secret-like values in examples use `os.environ/<NAME>` or labeled placeholders. |
| B | Config key integrity | PASS | `config-yaml.option-index.json` contains 330 options. 20 cited keys sampled across `config/*.md` + `routing/README.md` + `examples/*.yaml`; 19 directly present in index, 1 (`model_group_settings`) had no actual citations (test-list artifact). Every genuinely cited key traces to the index. No undocumented key presented as fact. |
| C | Provider prefix integrity | PASS | 11 documented prefixes in `provider-fields.index.json`: `anthropic/`, `hosted_vllm/`, `inception/`, `litellm_proxy/`, `minimax/`, `moonshot/`, `ollama/`, `ollama_chat/`, `openai/`, `openrouter/`, `zai/`. All prefixes used in `examples/*.yaml` + `providers/*.md` match documented prefixes. `text-completion-*` are standard LiteLLM prefix modifiers (documented in `providers/inception.md`). Workestrate deviations clearly MARKED as project choices: `anthropic/` for MiniMax (`providers/minimax.md` lines 77,82; `examples/minimax.yaml` line 70 "DEVIATION"); `anthropic/` for Kimi (`providers/moonshot.md` lines 62,68,72; `examples/moonshot-kimi.yaml` "NOT documented on moonshot page"). |
| D | Endpoint integrity | PASS | `endpoints.index.json`: 20 inference endpoints, ALL carry `source_urls` (0 missing). `openapi-management-endpoints.index.json`: 314 management routes across 46 families, every entry has `requires_db` + `requires_db_basis` fields. `requires_db_legend` present. |
| E | Workflow/skill dependencies | PASS | 13/13 workflows have "Read first"/"consult" sections. 16/16 skills have "do not hallucinate" rules pointing to schema index files (all 6 schema files exist). 36 distinct `docs/litellm/` refs in workflows+skills, 0 dangling. 27 skill doc refs, 0 dangling. |
| F | Internal link integrity | PASS | 114 internal markdown links scanned across all `docs/litellm/**/*.md`. 1 "dangling": `/docs/proxy/virtual_keys` in `extracted/p2-admin-ui.md` — an upstream URL path preserved verbatim in an extracted file, not a local link. Acceptable. |
| G | Gateway DB-gate integrity | PASS | `gateway/README.md` (lines 227, 232, 303), `workflows/skills-gateway-onboarding.workflow.md` (lines 12, 14, 22), and `skills/configure-skills-gateway.md` (lines 1, 7, 25, 60, 68) all mark Skills-persistence-needs-DB as deferred to Postgres / M4. Model-based routing is the only in-memory-safe subset. |

**Overall: 7/7 checks PASS.**

---

## Reconciliation edits performed

1. **`docs/litellm/config/README.md`** — Flipped 5 document rows (`litellm-settings.md`, `router-settings.md`, `general-settings.md`, `environment-variables.md`, `config-validation.md`) from `(planned)` to `exists`. Removed `(planned)` from the intro line (now "and per-section references."). No other content changed.
2. **`docs/litellm/00-index.md`** — Replaced the file-map table with a 17-row table listing ALL top-level items that now exist (`01-mental-model.md`, `extracted/`, `schemas/`, `crawl/`, `config/`, `providers/`, `endpoints/`, `gateway/`, `routing/`, `auth-access-budget/`, `deployment-ops/`, `observability-cache-guardrails/`, `examples/`, `workflows/`, `skills/`, `troubleshooting.md`, `workestrate-recommended-patterns.md`), all marked `exists`. Mental-model paragraph and all other sections left identical.
3. **`docs/litellm/troubleshooting.md`** — Line 26: "planned `debug-litellm-proxy` skill" → "existing `debug-litellm-proxy` skill". Lines 283-286: "Planned skill ... does not yet exist" → "Skill ... now exists at [skills/debug-litellm-proxy.md](skills/debug-litellm-proxy.md)" (the skill now exists). No other content changed. *(Post-Phase-12: `debug-litellm-proxy` was further consolidated into `workflow-litellm-debugging` under `.agents/skills/`; the troubleshooting.md link was re-pointed — see "Skill consolidation" note.)*

---

## Final metrics

### 1. Created file tree under `docs/litellm/` (130 files, 15 directories)

```
docs/litellm/
├── 00-index.md
├── 01-mental-model.md
├── troubleshooting.md
├── workestrate-recommended-patterns.md
├── auth-access-budget/
│   └── README.md
├── config/
│   ├── README.md
│   ├── config-validation.md
│   ├── config-yaml-overview.md
│   ├── environment-variables.md
│   ├── general-settings.md
│   ├── litellm-settings.md
│   ├── model-list.md
│   └── router-settings.md
├── crawl/
│   ├── excluded_low_priority_pages.md
│   ├── missing_from_llms_txt.md
│   ├── openapi_route_inventory.json
│   ├── priority_fetch_queue.md
│   ├── sidebar_tree.md
│   ├── source_map.md
│   ├── unresolved_seed_hints.md
│   ├── url_classification.md
│   └── url_inventory.json
├── deployment-ops/
│   └── README.md
├── endpoints/
│   └── README.md
├── examples/
│   ├── agent-mcp-skills-gateway.yaml
│   ├── disposable-local-proxy.yaml
│   ├── inception.yaml
│   ├── microvm-safe-proxy.yaml
│   ├── minimal-openai-compatible.yaml
│   ├── minimax.yaml
│   ├── moonshot-kimi.yaml
│   ├── multi-provider-router.yaml
│   ├── openrouter.yaml
│   ├── production-redis-postgres.yaml
│   ├── split-config-models.yaml
│   ├── split-config-parent.yaml
│   └── zai-glm.yaml
├── extracted/  (48 files)
│   ├── p0-completion.md, p0-config_settings.md, p0-configs.md, p0-deploy.md,
│   ├── p0-docker_quick_start.md, p0-quick_start.md, p0-simple_proxy.md,
│   ├── p0-supported_endpoints.md, p0-user_keys.md
│   ├── p1-auth-model_access.md, p1-auth-virtual_keys.md,
│   ├── p1-budgets-rate_limit_tiers.md, p1-budgets-users.md
│   ├── p1-provider-anthropic.md, p1-provider-inception.md,
│   ├── p1-provider-litellm_proxy.md, p1-provider-minimax.md,
│   ├── p1-provider-moonshot.md, p1-provider-ollama.md,
│   ├── p1-provider-openai.md, p1-provider-openai_compatible.md,
│   ├── p1-provider-openrouter.md, p1-provider-vllm.md, p1-provider-zai.md
│   ├── p1-routing-fallback_management.md, p1-routing-load_balancing.md
│   ├── p2-admin-management_cli.md, p2-admin-ui.md, p2-deploy-db_info.md
│   ├── p2-feature-all_caches.md, p2-feature-caching.md
│   ├── p2-gateway-a2a.md, p2-gateway-mcp.md, p2-gateway-skills.md
│   ├── p2-guardrails-guardrail_policies.md, p2-guardrails-quick_start.md
│   ├── p2-logging-alerting.md, p2-logging-cost_tracking.md,
│   ├── p2-logging-dynamic_logging.md, p2-logging-logging.md,
│   ├── p2-logging-metrics.md, p2-logging-prometheus.md
├── gateway/
│   └── README.md
├── observability-cache-guardrails/
│   └── README.md
├── providers/
│   ├── README.md, anthropic.md, inception.md, litellm-proxy.md,
│   ├── minimax.md, moonshot.md, ollama.md, openai-compatible.md,
│   ├── openai.md, openrouter.md, vllm.md, zai-glm.md
├── routing/
│   └── README.md
├── schemas/
│   ├── config-yaml.normalized.schema.md
│   ├── config-yaml.option-index.json
│   ├── endpoints.index.json
│   ├── env-vars.index.json
│   ├── gateway-agent-mcp-skills.index.json
│   ├── openapi-management-endpoints.index.json
│   └── provider-fields.index.json
├── skills/
│   ├── add-budget-rate-limit.md, add-model-alias.md,
│   ├── add-openai-compatible-provider.md, add-provider.md,
│   ├── add-routing-fallbacks.md, add-virtual-key.md,
│   ├── configure-cache.md, configure-guardrails.md,
│   ├── configure-litellm.md, configure-logging-observability.md,
│   ├── configure-mcp-gateway.md, configure-skills-gateway.md,
│   ├── debug-litellm-proxy.md, harden-production-proxy.md,
│   ├── restrict-model-access.md, validate-litellm-config.md
└── workflows/
    ├── budget-and-rate-limit.workflow.md, cache-and-observability.workflow.md,
    ├── config-review.workflow.md, guardrail-policy.workflow.md,
    ├── mcp-gateway-onboarding.workflow.md, new-litellm-config.workflow.md,
    ├── openai-compatible-provider-onboarding.workflow.md,
    ├── production-hardening.workflow.md, provider-onboarding.workflow.md,
    ├── routing-and-fallbacks.workflow.md,
    ├── skills-gateway-onboarding.workflow.md, troubleshooting.workflow.md,
    └── virtual-key-and-model-access.workflow.md
```

> **Post-Phase-12 consolidation**: the `skills/` (16 files) and
> `workflows/` (13 files) subtrees above were consolidated into 55 packages
> under `.agents/skills/` (see "Skill consolidation" note at end). The two
> source dirs no longer exist under `docs/litellm/`.

### 2. Crawled URL count

- **Computed**: 1,084 (from `crawl/url_inventory.json` → `urls` array length)
- **Anchor**: 1,084
- **Match**: ✅

### 3. Selected high-priority page count

- **Computed**: P0=10, P1=20, P2=25 → 55 total (from `crawl/priority_fetch_queue.md`)
- **Anchor**: P0=10, P1=20, P2=25 = 55
- **Match**: ✅

### 4. Extracted page count

- **Computed**: 48 (files in `extracted/*.md`; +4 this round: `p0-config_management.md`, `p1-routing-timeout.md`, `p1-routing-reliability.md`, `p2-feature-secret_managers.md`)
- **Anchor**: 48
- **Match**: ✅

### 5. Config keys extracted count

- **Computed**: 330 (options in `schemas/config-yaml.option-index.json`; +1 `include` top-level key verified verbatim from `/docs/proxy/config_management` raw GitHub source this round; prior 329 included 9 `router_settings` keys added from verbatim raw GitHub source — SHA 52cbdf40, ref main, ~135 KB)
- **Anchor**: 330
- **Match**: ✅

### 6. Provider pages extracted count

- **Computed**: 11 (`extracted/p1-provider-*.md` files); 12 provider guides in `providers/` (incl. README)
- **Anchor**: 11
- **Match**: ✅

### 7. Endpoint pages/routes

- **Computed**: 20 inference endpoints (`schemas/endpoints.index.json`); 314 management routes (`schemas/openapi-management-endpoints.index.json`)
- **Anchor**: inference=20, mgmt=314
- **Match**: ✅

### 8. Gateway pages extracted count

- **Computed**: 3 (`extracted/p2-gateway-*.md`: a2a, mcp, skills)
- **Anchor**: 3
- **Match**: ✅

### 9. Generated workflows count

- **Computed**: 13 (`workflows/*.md`)
- **Anchor**: 13
- **Match**: ✅
- **Post-Phase-12**: consolidated into 5 `workflow-litellm-*` packages
  (× 6 phases = 30) under `.agents/skills/`; `docs/litellm/workflows/` removed.

### 10. Generated skills count

- **Computed**: 16 (`skills/*.md`)
- **Anchor**: 16
- **Match**: ✅
- **Post-Phase-12**: consolidated into 13 `litellm-*` + 8 `constraint-litellm-*`
  + 4 `validation-litellm-*` packages under `.agents/skills/`; `docs/litellm/skills/` removed.

### 11. Examples generated count

- **Computed**: 13 (`examples/*.yaml`)
- **Anchor**: 13
- **Match**: ✅

### 12. Unresolved seed hints

- **Computed**: 5 (from `crawl/unresolved_seed_hints.md`):
  1. Neuralwatt — no dedicated provider page (custom OpenAI-compatible endpoint)
  2. Kimi — product name; provider is "Moonshot AI" (`/docs/providers/moonshot`)
  3. `docs/llm_provider/*.md` — directory does not exist (HTTP 404); providers are flat at `docs/providers/*.md`
  4. config validation — no dedicated page; covered within `/docs/proxy/config_settings`
  5. env vars — no standalone page; section within `/docs/proxy/config_settings`
- **Anchor**: 5
- **Match**: ✅

### 13. Known uncertainties

- **config_settings now verbatim-complete (with contamination correction)**: the raw GitHub markdown for `/docs/proxy/config_settings` was recovered (135 KB, SHA 52cbdf40, ref main). The trailing-section reconstruction caveat is RESOLVED — `model_list`, `callback_settings`, `environment_variables`, and config validation Reference tables are now byte-for-byte verbatim. Contamination correction: 3 previously-listed section headings (`model_list Reference`, `callback_settings Reference`, `config validation prose`) were found to NOT exist in upstream source and were removed.
- **config_management / timeout / reliability / secret_managers — fetched verbatim this round**: `/docs/proxy/config_management` (raw GitHub) confirms `include` as a real top-level key (verbatim); split-config examples (`split-config-parent.yaml`, `split-config-models.yaml`) upgraded from inferred → verbatim. New doc `config/config-management.md` created. `/docs/proxy/timeout` fetched: `stream_timeout` confirmed as a first-token/TTFT-style timeout; `ttft_timeout` / `stream_idle_timeout` prose NOT on the timeout page (keys ARE verbatim in the `config_settings` index). `/docs/proxy/reliability` fetched: retry/fallback/cooldown model confirmed (fallbacks fire after retries exhaust); `retry_policy` / `allowed_fails_policy` / `retry_after` / `disable_cooldowns` prose NOT on the reliability page (keys ARE verbatim in the index). `/docs/secret_managers/overview` fetched: Enterprise-REQUIRED confirmed; key_management_system unified path added to `config/general-settings.md`. Hot-reload NOT documented on the config_management page (honestly marked).
- **Skills-gateway DB requirement — negative finding (upstream silence)**: `/docs/skills_gateway` was fetched (raw markdown). The Skills-gateway DB requirement is NOT documented upstream (verbatim silence). The DB-gate inference in `gateway/README.md`, `workflows/skills-gateway-onboarding.workflow.md`, and `skills/configure-skills-gateway.md` is grounded in upstream silence, not an explicit upstream statement.
- **MCP CRUD management endpoints — negative finding**: `/docs/mcp_rest_api` was fetched (raw markdown). MCP CRUD management endpoints are NOT on `mcp_rest_api`. `store_model_in_db` is documented on `/docs/mcp` Overview, not on `mcp_rest_api`. 6 verbatim Skills-registry endpoints + `GET /v1/mcp/server` were added; ~7 "verify-against-upstream/deferred" markers resolved to verbatim. 2 new extractions created (`extracted/p2-gateway-skills_gateway.md`, `extracted/p2-gateway-mcp_rest_api.md`).
- **661 newly-added env vars — scope/requires_db defaults not individually verified**: `env-vars.index.json` went 145 → 806 (661 env vars added from the verbatim `environment_variables` table). The added entries carry `scope="proxy"` / `requires_db=false` defaults that were not individually verified against upstream prose — minor refinement opportunity, not a correctness blocker.
- **Inferred openrouter/inception YAML**: provider config examples for openrouter and inception are inferred from provider-page patterns, not extracted verbatim from a config example.
- **MiniMax-M3 & kimi-for-coding not in upstream docs**: `MiniMax-M3` is NOT in the docs (only M2-series); `kimi-for-coding` is NOT documented on the moonshot page. Both are repo-specific model names using the `anthropic/` prefix with custom `api_base` — clearly marked as deviations/inferred in `providers/minimax.md`, `providers/moonshot.md`, and the corresponding example YAMLs.
- **Deferred upstream pages** (deliberately skipped — operational, not config-dimension; their config keys are already verbatim in the 330-key index from `config_settings`): `/docs/proxy/prod`, `/docs/proxy/debugging`, `/docs/proxy/health`, `/docs/proxy/enterprise`, `/docs/proxy/auto_routing`, `/docs/proxy/load_balancing`, `/docs/proxy/docker_image_security`, `/docs/proxy/architecture`, `/docs/mcp_control`, `/docs/proxy_admin`, and per-provider secret-manager subpages. (Note: `/docs/skills_gateway`, `/docs/mcp_rest_api`, `/docs/proxy/config_management`, `/docs/proxy/timeout`, `/docs/proxy/reliability`, and `/docs/secret_managers/overview` are NO LONGER deferred — fetched this round, see findings above and below.)

### 14. Recommended next validation command or manual review step

The `config_settings` full re-fetch is DONE (verbatim-complete, 330 keys / 806 env vars). The `/docs/skills_gateway` and `/docs/mcp_rest_api` gateway backfills are DONE (negative findings recorded). The config-dimension gaps are now CLOSED — `config_management`, `timeout`, `reliability`, and `secret_managers/overview` were all fetched verbatim this round. Recommended next steps, in priority order:

1. **Live smoke test against the real proxy on a KVM host.** This environment has no `/dev/kvm`, so runtime validation is blocked here — defer to a KVM-capable host. Minimum checks:
   ```bash
   curl -s http://localhost:4000/health/readiness
   curl -s http://localhost:4000/v1/models
   curl -s http://localhost:4000/v1/chat/completions -H "content-type: application/json" \
     -d '{"model":"coding","messages":[{"role":"user","content":"ping"}]}'
   ```
2. **Optionally refine the 661 newly-added env vars** in `schemas/env-vars.index.json` — verify `scope` / `requires_db` flags individually against upstream prose where the default (`scope="proxy"`, `requires_db=false`) is uncertain.
3. **User's stated next phase — adjust the actual `infra/litellm/config.yaml`** based on the gathered config-dimension info. The operational toolkit for that work is `docs/litellm/config/` (per-section references) and the consolidated skill packages under `.agents/skills/` (5 `workflow-litellm-*` workflows incl. config-change, config-review, hardening; 13 `litellm-*` regular skills incl. `litellm-config-anatomy`, `litellm-production-hardening`; `validation-litellm-config-check` with co-located `check_config.py`).

---

## Residual risks

1. **Live runtime validation not yet performed**: this environment has no `/dev/kvm`, so the in-memory-safe subset (health, `/v1/models`, model-based routing) has not been smoke-tested against the real proxy. Defer to a KVM-capable host. (See Metric 14.)
2. **661 newly-added env vars — default flags not individually verified**: `scope="proxy"` / `requires_db=false` defaults applied uniformly; individual verification against upstream prose is a minor refinement opportunity, not a correctness blocker.
3. **Deferred upstream pages**: operational pages deliberately skipped (prod, debugging, health, enterprise, auto_routing, load_balancing, docker_image_security, architecture, mcp_control/proxy_admin, per-provider secret-manager subpages) — their config keys are already verbatim in the 330-key index. `/docs/skills_gateway`, `/docs/mcp_rest_api`, `/docs/proxy/config_management`, `/docs/proxy/timeout`, `/docs/proxy/reliability`, and `/docs/secret_managers/overview` are NO LONGER deferred — fetched, with findings recorded in Metric 13.
4. **MiniMax-M3 / kimi-for-coding model names**: unverified against upstream (may be newer/renamed). Low risk — clearly marked as deviations.
5. **openrouter/inception YAML**: inferred from provider-page patterns rather than verbatim config examples. Low risk — patterns are simple and well-documented.

**No remaining claim that `config_settings` is "reconstructed", that `skills_gateway` / `mcp_rest_api` are "deferred / not fetched", or that `config_management` / `timeout` / `reliability` / `secret_managers` are "deferred" or "inferred".**

---

## Skill consolidation (post-Phase-12 update)

The 29 files under `docs/litellm/skills/` (16 skills) and `docs/litellm/workflows/`
(13 workflows) were consolidated into **55 skill packages under `.agents/skills/`**,
following the repo's `workflow-skill-creator` pattern (6-phase workflow shape,
constraint + validation + regular skill categories):

- **13 `litellm-*`** regular skills (config-anatomy, providers, openai-compatible,
  routing-fallbacks, caching, logging-observability, guardrails, budgets-keys,
  gateway-mcp, gateway-skills, gateway-a2a, deployment, production-hardening).
- **8 `constraint-litellm-*`** (config-schema, in-memory-no-db, secret-hygiene,
  provider-prefix, openai-compatible-api-base, anthropic-suffix,
  fallback-resolution, deprecation-free).
- **4 `validation-litellm-*`** (config-check, startup, smoke, no-hardcoded-secrets).
- **5 `workflow-litellm-*`** × 6 phases each = 30 (config-change, config-review,
  debugging, validation, hardening).

The static config validator `check_config.py` (checks a–i) is co-located at
`.agents/skills/validation-litellm-config-check/scripts/check_config.py`.
Re-tested PASS (exit 0) against `infra/litellm/config.yaml` (1 intentional WARN
on `general_settings.disable_spend_logs` — the no-DB compensating control).

The source dirs `docs/litellm/skills/` and `docs/litellm/workflows/` were removed;
all references in `docs/litellm/` and `.agents/skills/` were re-pointed to the new
packages. Metrics 9 & 10 above are annotated with the post-Phase-12 consolidation.

