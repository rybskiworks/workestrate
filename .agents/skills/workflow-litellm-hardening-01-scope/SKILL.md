---
name: workflow-litellm-hardening-01-scope
description: |
  Use only for the scope phase of the LiteLLM hardening workflow. Assess the
  current config against the hardening baseline and identify gaps tied to
  verbatim config keys. Do not use for planning, implementation, validation, or
  verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(curl:*) Bash(git:*) Bash(python3:*) Bash(yq:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-hardening
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (LiteLLM hardening)

## Phase purpose

Assess `infra/litellm/config.yaml` against the production-hardening baseline and
identify gaps. Each gap MUST be tied to a verbatim config key from
`docs/litellm/schemas/config-yaml.option-index.json` and the docs page that
documents it. This phase produces the gap table that the plan phase turns into
edits and that the verify phase checks against.

## Steps to perform

1. Load the `litellm-production-hardening` skill for the hardening baseline.
2. Read `infra/litellm/config.yaml` (the current deployment config).
3. Read the corpus pages that define the hardening keys:
   - `docs/litellm/config/general-settings.md`
   - `docs/litellm/config/router-settings.md`
   - `docs/litellm/config/litellm-settings.md`
   - `docs/litellm/deployment-ops/README.md`
4. Cross-reference each baseline item against
   `docs/litellm/schemas/config-yaml.option-index.json` to capture the verbatim
   `full_path`, `default`, `requires_db`/`requires_redis` flags, and `source_urls`.
5. For each baseline item, classify the current config state as `satisfied` or
   `gap`, recording the verbatim key + source. Mark any deployment-specific
   recommendation `[WORKESTRATOR NOTE]`.
6. Record the gap table (below) as the scope contract for the rest of the
   workflow.

## Docs to consult

- `docs/litellm/deployment-ops/README.md`
- `docs/litellm/config/general-settings.md`
- `docs/litellm/config/router-settings.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/examples/microvm-safe-proxy.yaml`
- `docs/litellm/examples/production-redis-postgres.yaml`

## Operational skills to load

- `litellm-production-hardening` — the hardening baseline checklist. Load at the
  start of this phase; it defines which config keys constitute a hardened
  deployment.

## Constraints to apply

- `constraint-litellm-config-schema` — every gap must trace to a verbatim key
  in the option-index; no invented keys.
- `constraint-litellm-in-memory-no-db` — DB-required keys
  (`requires_db=true`) must be assessed for the in-memory (no-Postgres)
  deployment: setting them `true` is hygiene to suppress DB write attempts, not
  a functional requirement. Mark each such case `[WORKESTRATOR NOTE]`.
- `constraint-litellm-deprecation-free` — do not flag as gaps any deprecated
  keys (e.g. `set_verbose`); recommend the documented replacement instead.

## Validations to run

None — validations run in phases 04 and 05.

## Gap assessment table (baseline → current state)

Each row ties a hardening baseline item to a verbatim config key (full_path from
`config-yaml.option-index.json`) and the docs page that documents it. Fill the
`current` and `state` columns from `infra/litellm/config.yaml`.

| # | Baseline item | Verbatim key (full_path) | default | requires_db | Source page | Current config value | State |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | request_timeout 6000s → 300 | `litellm_settings.request_timeout` | 6000 | — | config_settings, configs | `300` | satisfied |
| 2 | retry/fallback present | `router_settings.num_retries` (default 3), `router_settings.fallbacks`, `router_settings.retry_policy` | 3 / — / — | false | config_settings, configs | num_retries:2, fallbacks set, retry_policy set | satisfied |
| 3 | master_key via os.environ/ | `general_settings.master_key` (security_risk: "Admin secret; must start with 'sk-'. Use os.environ/ resolution.") | — | false | config_settings, configs, deploy, docker_quick_start, virtual_keys, users | `os.environ/LITELLM_MASTER_KEY` | satisfied |
| 4 | disable_spend_logs | `general_settings.disable_spend_logs` (workestrator_recommendation: "Set to true in real config; compensates for no DB") | — | true | config_settings, db_info | `true` | satisfied |
| 5 | SSRF user_url_validation | `litellm_settings.user_url_validation` | — | — | config_settings | NOT set | **gap** |
| 6 | allowed_ips | `general_settings.allowed_ips` | — | false | config_settings | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 7 | background_health_checks | `general_settings.background_health_checks` | — | false | config_settings | NOT set | **gap** |
| 8 | health_check_interval | `general_settings.health_check_interval` | 300 | false | config_settings | NOT set | **gap** |
| 9 | disable_error_logs | `general_settings.disable_error_logs` | — | true | config_settings, db_info | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 10 | disable_spend_updates | `general_settings.disable_spend_updates` | — | true | config_settings | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 11 | disable_adding_master_key_hash_to_db | `general_settings.disable_adding_master_key_hash_to_db` | — | true | config_settings | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 12 | disable_reset_budget | `general_settings.disable_reset_budget` | — | true | config_settings | NOT set | **gap** `[WORKESTRATOR NOTE]` |
| 13 | disable_master_key_return | `general_settings.disable_master_key_return` | — | false | config_settings | NOT set | **gap** |
| 14 | max_request_size_mb | `general_settings.max_request_size_mb` | — | false | config_settings | NOT set | **gap** |
| 15 | max_response_size_mb | `general_settings.max_response_size_mb` | — | false | config_settings | NOT set | **gap** |
| 16 | max_parallel_requests | `general_settings.max_parallel_requests` | 0 | false | config_settings | NOT set | **gap** |
| 17 | global_max_parallel_requests | `general_settings.global_max_parallel_requests` | 0 | false | config_settings | NOT set | **gap** |
| 18 | json_logs | `litellm_settings.json_logs` | — | — | config_settings | NOT set | **gap** |
| 19 | redact_user_api_key_info | `litellm_settings.redact_user_api_key_info` | — | — | config_settings, logging | NOT set | **gap** |
| 20 | turn_off_message_logging | `litellm_settings.turn_off_message_logging` | — | — | config_settings, logging | NOT set | **gap** |
| 21 | enforce_user_param | `general_settings.enforce_user_param` | — | false | config_settings | NOT set | **gap** (optional) |
| 22 | reject_clientside_metadata_tags | `general_settings.reject_clientside_metadata_tags` | — | false | config_settings | NOT set | **gap** (optional) |
| 23 | enable_pre_call_checks | `router_settings.enable_pre_call_checks` (common_misconfiguration: "Required for model_info.max_input_tokens enforcement. Default: false.") | false | false | config_settings | NOT set | **gap** |

> `[WORKESTRATOR NOTE]` rows: deployment-specific. Rows 9–12 are DB-required
> keys (`requires_db=true`); in the in-memory (no-Postgres) deployment, setting
> them `true` is hygiene to suppress DB write attempts — the same pattern already
> used for `disable_spend_logs` (row 4). Row 6 (`allowed_ips`): the microsandbox
> runtime enforces default-deny egress at the network layer, so LiteLLM-level
> `allowed_ips` is a defense-in-depth overlay, not the primary control.

## Handoff output

Return the handoff YAML schema defined in
`workflow-litellm-hardening-00-orchestration`. Set:

- `outcome` to `pass` once the gap table is filled and every gap is tied to a
  verbatim key + source.
- `constraints_applied` to include `constraint-litellm-config-schema`,
  `constraint-litellm-in-memory-no-db`, `constraint-litellm-deprecation-free`.
- `next_phase: 02-plan`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: read-only assessment (no edits in scope phase)
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-deprecation-free
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 02-plan
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
