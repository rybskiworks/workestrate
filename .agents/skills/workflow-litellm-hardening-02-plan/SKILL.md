---
name: workflow-litellm-hardening-02-plan
description: |
  Use only for the plan phase of the LiteLLM hardening workflow. List the
  hardening edits, each tied to a verbatim config key + source. Do not use for
  scoping, implementation, validation, or verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(curl:*) Bash(git:*) Bash(python3:*) Bash(yq:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-hardening
  org.phase: plan
  org.phase_order: "02"
---

# Phase 02: plan (LiteLLM hardening)

## Phase purpose

Turn the scope-phase gap table into an ordered edit list. Every edit MUST be
tied to a verbatim config key (full_path from
`docs/litellm/schemas/config-yaml.option-index.json`) and the docs page that
documents it. No edit may introduce a key not present in the option-index.

## Steps to perform

1. Read the scope-phase handoff (the gap table).
2. For each `gap` row, define the exact edit: section, key, value, and the
   verbatim source citation.
3. Order edits by section (`general_settings`, `router_settings`,
   `litellm_settings`) to minimize diff churn.
4. For each edit, record the anti-hallucination citation: verbatim `full_path`
   + `default` + `requires_db`/`requires_redis` + source page.
5. Mark every deployment-specific recommendation `[WORKESTRATE NOTE]`.
6. Produce the edit list (below) as the plan contract for the implement phase.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/config/general-settings.md`
- `docs/litellm/config/router-settings.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/examples/microvm-safe-proxy.yaml`

## Operational skills to load

- `litellm-production-hardening` — for the recommended value at each gap.

## Constraints to apply

- `constraint-litellm-config-schema` — every edit key must exist verbatim in
  the option-index; values must match the documented type.
- `constraint-litellm-in-memory-no-db` — DB-required keys set `true` are
  hygiene, not functional; mark `[WORKESTRATE NOTE]`. Do NOT plan any
  `database_url` / `store_model_in_db` / Redis keys.
- `constraint-litellm-deprecation-free` — do not plan deprecated keys
  (`set_verbose`, `disable_copilot_system_to_assistant`).
- `constraint-litellm-fallback-resolution` — verify every fallback target in
  `router_settings.fallbacks` resolves to a `model_name` in `model_list`; do
  not plan fallback edits that break resolution.

## Validations to run

None — validations run in phases 04 and 05.

## Edit list (each tied to a verbatim config key + source)

| # | Section | Verbatim key (full_path) | Planned value | requires_db | Source page | Citation |
| --- | --- | --- | --- | --- | --- | --- |
| E1 | `litellm_settings` | `litellm_settings.user_url_validation` | `true` | — | config_settings | option-index[88]; SSRF guard for user-supplied URLs |
| E2 | `general_settings` | `general_settings.background_health_checks` | `true` | false | config_settings | option-index[184]; example_yaml: `background_health_checks: true` |
| E3 | `general_settings` | `general_settings.health_check_interval` | `300` | false | config_settings | option-index[185]; default 300 |
| E4 | `general_settings` | `general_settings.disable_error_logs` | `true` | true | config_settings, db_info | option-index[153]; `[WORKESTRATE NOTE]` in-memory hygiene |
| E5 | `general_settings` | `general_settings.disable_spend_updates` | `true` | true | config_settings | option-index[152]; `[WORKESTRATE NOTE]` in-memory hygiene |
| E6 | `general_settings` | `general_settings.disable_adding_master_key_hash_to_db` | `true` | true | config_settings | option-index[157]; `[WORKESTRATE NOTE]` secret hygiene |
| E7 | `general_settings` | `general_settings.disable_reset_budget` | `true` | true | config_settings | option-index[156]; `[WORKESTRATE NOTE]` in-memory hygiene |
| E8 | `general_settings` | `general_settings.disable_master_key_return` | `true` | false | config_settings | option-index[154]; secret hygiene — never return master key |
| E9 | `general_settings` | `general_settings.max_request_size_mb` | `10` | false | config_settings | option-index[215]; resource limit |
| E10 | `general_settings` | `general_settings.max_response_size_mb` | `10` | false | config_settings | option-index[216]; resource limit |
| E11 | `general_settings` | `general_settings.max_parallel_requests` | `50` | false | config_settings | option-index[180]; default 0 (unlimited) → bounded |
| E12 | `general_settings` | `general_settings.global_max_parallel_requests` | `100` | false | config_settings | option-index[181]; default 0 (unlimited) → bounded |
| E13 | `litellm_settings` | `litellm_settings.json_logs` | `true` | — | config_settings | option-index[58]; log hygiene |
| E14 | `litellm_settings` | `litellm_settings.redact_user_api_key_info` | `true` | — | config_settings, logging | option-index[53]; log hygiene |
| E15 | `litellm_settings` | `litellm_settings.turn_off_message_logging` | `true` | — | config_settings, logging | option-index[50]; privacy |
| E16 | `router_settings` | `router_settings.enable_pre_call_checks` | `true` | false | config_settings | option-index[281]; default false; "Required for model_info.max_input_tokens enforcement" |
| E17 | `general_settings` | `general_settings.allowed_ips` | (decision) | false | config_settings | option-index[222]; `[WORKESTRATE NOTE]` — see decision below |

### E17 decision: allowed_ips

`general_settings.allowed_ips` (option-index[222], source: config_settings) is
NOT set in the current config. The `docs/litellm/deployment-ops/README.md`
Workestrate notes state: "LiteLLM-level `allowed_ips` is **not** set (network
policy enforces it)" — the microsandbox runtime enforces default-deny egress at
the network layer. Two options:

- **Option A (defense-in-depth):** set `allowed_ips` to the microsandbox
  internal CIDR (e.g. `["127.0.0.1"]` plus the sandbox orchestrator range).
  `[WORKESTRATE NOTE]` — value is deployment-specific, not from upstream docs.
- **Option B (document the control):** leave `allowed_ips` unset and record in
  the handoff that network policy is the primary IP control.

The implement phase applies the chosen option. Record the decision in
`assumptions`.

### Already-satisfied items (no edit)

Rows 1–4 from the scope table (`request_timeout: 300`, retry/fallback,
`master_key: os.environ/...`, `disable_spend_logs: true`) are already satisfied
— no edit planned. The implement phase MUST NOT regress them.

## Handoff output

Return the handoff YAML schema defined in
`workflow-litellm-hardening-00-orchestration`. Set:

- `outcome` to `pass` once every edit is tied to a verbatim key + source.
- `constraints_applied` to include `constraint-litellm-config-schema`,
  `constraint-litellm-in-memory-no-db`, `constraint-litellm-deprecation-free`,
  `constraint-litellm-fallback-resolution`.
- `next_phase: 03-implement`.
- `blockers: []` unless the E17 decision is unresolved (then `blockers` lists
  it and `handoff_requires_hil: true`).

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-deprecation-free
  - constraint-litellm-fallback-resolution
assumptions:
  - "E17 decision: <Option A or B>"
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
