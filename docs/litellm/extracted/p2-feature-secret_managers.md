---
source_url: https://docs.litellm.ai/docs/secret_managers/overview
canonical_url: https://docs.litellm.ai/docs/secret_managers/overview
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/secret_managers/overview.md
title: Secret Managers Overview
sidebar_section_path: secret_managers
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: secret_managers
---
# Secret Managers Overview

## Headings
- Secret Managers Overview
- Supported Secret Managers
- All Secret Manager Settings
- Team-Level Secret Manager Settings

## Exact config keys found (full path, verbatim spelling)
- `general_settings.key_management_system` (general) — verbatim: "REQUIRED". Enum value shown on this page: `"aws_secret_manager"`. (The config_settings Reference table documents the enum as `google_kms | azure_kms`; this page's unified example uses `aws_secret_manager`. The full supported-provider list is: AWS KMS, AWS Secret Manager, Azure Key Vault, CyberArk Conjur, Google Secret Manager, Google KMS, Hashicorp Vault.)
- `general_settings.key_management_settings` (general) — dict; container for the sub-keys below.
- `general_settings.key_management_settings.store_virtual_keys` (general) — verbatim: "OPTIONAL. Defaults to False, when True will store virtual keys in secret manager".
- `general_settings.key_management_settings.prefix_for_stored_virtual_keys` (general) — verbatim: "OPTIONAL. If set, this prefix will be used for stored virtual keys in the secret manager" (example: `"litellm/"`).
- `general_settings.key_management_settings.access_mode` (general) — verbatim: "OPTIONAL. Literal["read_only", "write_only", "read_and_write"]. Defaults to "read_only"".
- `general_settings.key_management_settings.hosted_keys` (general) — verbatim: "OPTIONAL. Specify which env keys you stored on AWS" (example: `["litellm_master_key"]`).
- `general_settings.key_management_settings.primary_secret_name` (general) — verbatim: "OPTIONAL. Read multiple keys from one JSON secret on AWS Secret Manager" (example: `"litellm_secrets"`).

## Keys NOT documented on this page (documented elsewhere — config_settings)
The following general_settings keys are present in `schemas/config-yaml.option-index.json` but are NOT described on the /docs/secret_managers/overview page:
- `general_settings.use_google_kms` (bool) — documented in config_settings Reference table; not on this page. (This page uses the unified `key_management_system` key instead.)
- `general_settings.use_azure_key_vault` (bool) — documented in config_settings Reference table; not on this page. (This page uses the unified `key_management_system` key instead.)

## Exact YAML examples (verbatim — preserve indentation)
All Secret Manager Settings (verbatim):
```yaml
general_settings:
  key_management_system: "aws_secret_manager" # REQUIRED
  key_management_settings:  

    # Storing Virtual Keys Settings
    store_virtual_keys: true # OPTIONAL. Defaults to False, when True will store virtual keys in secret manager
    prefix_for_stored_virtual_keys: "litellm/" # OPTIONAL. If set, this prefix will be used for stored virtual keys in the secret manager
    
    # Access Mode Settings
    access_mode: "write_only" # OPTIONAL. Literal["read_only", "write_only", "read_and_write"]. Defaults to "read_only"
    
    # Hosted Keys Settings
    hosted_keys: ["litellm_master_key"] # OPTIONAL. Specify which env keys you stored on AWS

    # K/V pairs in 1 AWS Secret Settings
    primary_secret_name: "litellm_secrets" # OPTIONAL. Read multiple keys from one JSON secret on AWS Secret Manager
```

## Exact environment variables
- none documented on this overview page (provider-specific env vars live on the per-provider subpages: ./aws_kms, ./aws_secret_manager, ./azure_key_vault, ./cyberark, ./google_secret_manager, ./google_kms, ./hashicorp_vault)

## Exact endpoint paths / API routes (runtime + management)
- none documented on this overview page (team-level secret manager settings are configured via the Admin UI Teams page, not a documented API route on this page)

## Requirements
- database: not documented on this overview page
- redis: not documented on this overview page
- enterprise: **REQUIRED** — verbatim admonition: "✨ This is an Enterprise Feature" with links to Enterprise Pricing and a free-trial contact. All secret-manager functionality is Enterprise-gated.
- admin_ui: team-level secret manager settings are configured via the Admin UI Teams page (Create Team → Additional Settings → Secret Manager Settings panel, provider-specific JSON).

## Deprecations
- none documented on this page

## Caveats / pitfalls
- Enterprise license is REQUIRED for ALL secret-manager functionality (verbatim: "✨ This is an Enterprise Feature").
- `key_management_system` is REQUIRED (verbatim comment in YAML: `# REQUIRED`).
- `access_mode` defaults to `"read_only"` (verbatim) — to store virtual keys you must set `access_mode` to `"write_only"` or `"read_and_write"` AND `store_virtual_keys: true`.
- `store_virtual_keys` defaults to `False` (verbatim) — virtual keys are NOT stored in the secret manager unless explicitly enabled.
- Team-level secret manager settings require provider-specific JSON pasted into the Admin UI (verbatim: "JSON is required today, but we plan to add a more UI-friendly editor").
- The `use_google_kms` and `use_azure_key_vault` boolean keys (in config_settings) appear to be the older/provider-specific spellings; this overview page standardizes on `key_management_system` + `key_management_settings`. Both spellings exist in the corpus — treat `key_management_system` as the current unified path.

## Related links
- ./aws_kms (AWS Key Management Service)
- ./aws_secret_manager (AWS Secret Manager)
- ./azure_key_vault (Azure Key Vault)
- ./cyberark (CyberArk Conjur)
- ./google_secret_manager (Google Secret Manager)
- ./google_kms (Google Key Management Service)
- ./hashicorp_vault (Hashicorp Vault)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- The workestrator does NOT use any secret manager. `general_settings.key_management_system`, `key_management_settings`, `use_google_kms`, and `use_azure_key_vault` are all unset. API keys are resolved via `os.environ/` at config load time (e.g. `master_key: os.environ/LITELLM_MASTER_KEY`). Secret-manager integration is Enterprise-gated and therefore unavailable without a `litellm_license`. No action needed for the in-memory deployment.

## Confidence / uncertainty notes
- high confidence on Enterprise requirement, `key_management_system` (REQUIRED), and all `key_management_settings` sub-keys (verbatim YAML + prose). The relationship between `key_management_system` (unified, this page) and `use_google_kms`/`use_azure_key_vault` (config_settings) is inferred from the corpus: this overview page does not mention the two boolean keys, while config_settings lists them. Both are real per the option-index; the unified `key_management_system` path is the current documented approach.
