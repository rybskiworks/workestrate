# Discovery Source Map

**Generated**: 2026-06-27

## Discovery Sources and Coverage

| Source | URL | HTTP Status | URLs/Routes Contributed | Notes |
|---|---|---|---|---|
| sitemap.xml | https://docs.litellm.ai/sitemap.xml | 200 | 1084 URLs | Authoritative full URL inventory. Flat sitemap (not index). ~139KB. |
| llms.txt | https://docs.litellm.ai/llms.txt | 200 | 52 URLs | Hand-curated index. Focuses on overview, enterprise, completion, embedding, observability, release notes. Omits proxy docs, providers, endpoints. |
| llms-full.txt | https://docs.litellm.ai/llms-full.txt | 200 | ~52 pages (same set as llms.txt) | Large aggregated doc. Page set mirrors llms.txt. Body content not extracted (Phase 3). |
| sidebar (primary roots) | https://docs.litellm.ai/docs/simple_proxy, /docs/supported_endpoints, /docs/proxy/configs, /docs/providers | 200 (all 4) | ~190 sidebar links | Docusaurus v3.8.1 global sidebar. Reveals real section hierarchy. All sidebar URLs are a subset of sitemap. |
| openapi.json | https://www.litellm.org/openapi.json | 200 | 669 routes (491 paths) | Standard OpenAPI 3.x. Top-level keys: openapi, info, paths, components. 92 tags. |
| source_repo (github) | https://github.com/BerriAI/litellm-docs | 200 | 751 docs/ files | Docusaurus 3 source repo. sidebars.js at root. docs/llm_provider/ does NOT exist (404). |

## Docs Framework

- **Framework**: Docusaurus v3.8.1
- **Site URL**: https://docs.litellm.ai
- **Source repo**: https://github.com/BerriAI/litellm-docs (separate from main code repo BerriAI/litellm)
- **Sidebar config**: `sidebars.js` at repo root (47,512 bytes)
- **Release notes sidebar**: `sidebars-release-notes.js` at repo root
- **Docusaurus config**: `docusaurus.config.js` at repo root
- **llms.txt**: Hand-curated (not auto-generated); contains only 52 URLs vs 1084 in sitemap

## GitHub Source-Repo Paths Observed

### docs/ directory structure (top-level, 96 files at root)

Key subdirectories:
| Path | File Count | Notes |
|---|---|---|
| docs/ (root) | 96 | Flat .md files (providers, endpoints, etc.) |
| docs/providers/ | 199 | Provider docs (flat + subdirs for azure, azure_ai, gemini, etc.) |
| docs/proxy/ | 137 | Proxy server docs |
| docs/proxy/guardrails/ | 49 | Guardrail provider docs |
| docs/proxy/ui/ | 4 | Admin UI docs |
| docs/observability/ | 49 | Observability/logging integrations |
| docs/completion/ | 32 | SDK completion docs |
| docs/tutorials/ | 65 | Tutorial pages |
| docs/pass_through/ | 15 | Pass-through endpoint docs |
| docs/search/ | 15 | Search provider docs |
| docs/secret_managers/ | 9 | Secret manager docs |
| docs/troubleshoot/ | 10 | Troubleshooting docs |
| docs/caching/ | 3 | Caching docs |
| docs/anthropic_unified/ | 3 | Anthropic unified API docs |
| docs/learn/ | 4 | Quickstart guides |
| docs/extras/ | 5 | Contributing/extras |
| docs/integrations/ | 5 | Community integrations |
| docs/projects/ | 29 | Community project pages |

### Notable: docs/llm_provider/ does NOT exist

The `docs/llm_provider/` directory returns HTTP 404 from the GitHub API. Provider docs
are flat at `docs/providers/*.md` (152 files) plus top-level `docs/*.md` files. Any
reference to `docs/llm_provider/` is outdated.

## Coverage Notes

- The sitemap is the most comprehensive source (1084 URLs).
- The sidebar reveals the real section hierarchy but its URLs are a subset of the sitemap.
- llms.txt is hand-curated and covers only 52 URLs — a 4.8% coverage of the sitemap.
- The openapi.json provides 669 API routes not present in the docs sitemap (these are API endpoints, not documentation pages).
- The GitHub tree confirms the docs file structure but some files may not be in the sitemap (e.g., draft/unpublished pages).
