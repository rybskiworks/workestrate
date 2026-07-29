---
source_url: https://docs.litellm.ai/docs/proxy/rate_limit_tiers
canonical_url: https://docs.litellm.ai/docs/proxy/rate_limit_tiers
title: "✨ Budget / Rate Limit Tiers"
sidebar_section_path: proxy > budgets_rate_limits
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: budgets
---
# ✨ Budget / Rate Limit Tiers

## Headings
- 1. Create a budget
- 2. Assign budget to a key
- 3. Check if budget is enforced on key
- API Reference

## Exact config keys found (full path, verbatim spelling)
- `litellm_budget_table.budget_id` (returned in `/key/generate` response — set on creation)
- `litellm_budget_table.rpm_limit` (per-budget tier)
- `key.budget_id` (assigns a budget tier to a key)
- `tpm_limit` (mentioned as accepted on `/budget/new` per other pages; only `rpm_limit` shown in this page example)

## Exact YAML examples (verbatim — preserve indentation)
None on this page — entire page uses curl JSON only, no YAML config blocks.
(source: https://docs.litellm.ai/docs/proxy/rate_limit_tiers)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `POST /budget/new` — create a budget tier; accepts `budget_id`, `rpm_limit`
- `POST /key/generate` — assign a `budget_id` to a key
- `POST /v1/chat/completions` — to test enforcement

## Exact CLI commands
- none documented on this page

## Requirements
- database: not documented on this page (page does NOT state that a DB is required for budgets/tiers, though other LiteLLM docs do require it; this page omits any explicit statement)
- redis: not documented on this page
- enterprise: required — verbatim: "This is a LiteLLM Enterprise feature. Get a 7 day free trial + get in touch here. See pricing here."
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- none documented beyond Enterprise notice

## Related links
- /docs/proxy/dynamic_rate_limit (prev)
- /docs/proxy/temporary_budget_increase (next)
- API Reference: https://litellm-api.up.railway.app/#/budget%20management

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Budget/Rate Limit Tiers are an ENTERPRISE feature and require a database (budgets stored in `LiteLLM_BudgetTable`). The workestrate runs in-memory (no Postgres, no enterprise license), so rate limit tiers are UNAVAILABLE. Rate limiting in the in-memory deployment is limited to what `router_settings` provides (e.g. `rpm_limit` / `tpm_limit` on model_list entries if supported without DB — inferred, not confirmed on this page).

## Confidence / uncertainty notes
- high confidence on enterprise requirement (verbatim). DB requirement is inferred from the budgets domain (not stated on this page explicitly). The `/budget/new` endpoint likely requires DB (inferred from sibling pages).
