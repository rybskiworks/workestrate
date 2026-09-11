---
name: workflow-litellm-hardening-03-implement
description: |
  Use only for the implement phase of the LiteLLM hardening workflow. Apply the
  planned edits to `infra/litellm/config.yaml` under the active constraints. Do
  not use for scoping, planning, validation, or verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(curl:*) Bash(git:*) Bash(python3:*) Bash(yq:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-hardening
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (LiteLLM hardening)

## Phase purpose

Apply the plan-phase edit list to `infra/litellm/config.yaml`. Every edit must
match a verbatim config key from the option-index. This phase does not invent
keys, regress already-satisfied items, or introduce secrets.

## Steps to perform

1. Read the plan-phase handoff (the edit list + E17 decision).
2. Read `infra/litellm/config.yaml`.
3. Apply edits grouped by section, in this order: `general_settings`,
   `router_settings`, `litellm_settings`. Preserve existing keys; only add the
   planned keys.
4. For each edit, verify the key exists verbatim in
   `docs/litellm/schemas/config-yaml.option-index.json` before writing.
5. Do NOT touch `model_list` (no model changes in this workflow).
6. Do NOT regress the already-satisfied items: `request_timeout: 300`,
   `num_retries`/`fallbacks`/`retry_policy`, `master_key: os.environ/...`,
   `disable_spend_logs: true`, `force_ipv4: true`, `drop_params: true`.
7. Apply the active constraints (below) during the edit.
8. Record the exact diff in `files_touched`.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json` — verify each key.
- `docs/litellm/examples/microvm-safe-proxy.yaml` — the in-memory shape.

## Operational skills to load

- `litellm-production-hardening` — for the recommended value at each edit.

## Constraints to apply

- `constraint-litellm-secret-hygiene` — all `api_key` and `master_key` values
  MUST remain `os.environ/...`; no hardcoded secrets. New keys must not introduce
  secret literals.
- `constraint-litellm-config-schema` — every written key must exist verbatim in
  the option-index with the documented type.
- `constraint-litellm-in-memory-no-db` — do NOT add `database_url`,
  `store_model_in_db`, `redis_*`, `cache`/`cache_params`, or any
  `requires_db`/`requires_redis` key as a functional dependency. DB-required
  hygiene keys (`disable_error_logs`, `disable_spend_updates`,
  `disable_adding_master_key_hash_to_db`, `disable_reset_budget`) are set `true`
  to suppress DB write attempts — mark `[WORKESTRATE NOTE]` in comments.
- `constraint-litellm-deprecation-free` — do NOT add `set_verbose` (deprecated;
  use `LITELLM_LOG`) or `disable_copilot_system_to_assistant` (deprecated).
- `constraint-litellm-fallback-resolution` — do NOT modify
  `router_settings.fallbacks`; verify every fallback target still resolves to a
  `model_name` in `model_list` after the edit.

## Validations to run

None in this phase — `validation-litellm-config-check` runs in phase 04. A
lightweight YAML-parse sanity check (`python3 -c "import yaml; yaml.safe_load(...)"`)
is permitted to avoid handing off a syntactically broken file, but it is not a
gate.

## Expected resulting shape

After the edits, `infra/litellm/config.yaml` should contain (additions marked
`# [HARDENING]`; existing keys unchanged):

```yaml
general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY          # existing
  completion_model: coding                           # existing
  disable_spend_logs: true                           # existing
  background_health_checks: true                    # [HARDENING] E2
  health_check_interval: 300                         # [HARDENING] E3
  disable_error_logs: true                           # [HARDENING] E4 [WORKESTRATE NOTE]
  disable_spend_updates: true                        # [HARDENING] E5 [WORKESTRATE NOTE]
  disable_adding_master_key_hash_to_db: true         # [HARDENING] E6 [WORKESTRATE NOTE]
  disable_reset_budget: true                         # [HARDENING] E7 [WORKESTRATE NOTE]
  disable_master_key_return: true                    # [HARDENING] E8
  max_request_size_mb: 10                            # [HARDENING] E9
  max_response_size_mb: 10                           # [HARDENING] E10
  max_parallel_requests: 50                          # [HARDENING] E11
  global_max_parallel_requests: 100                  # [HARDENING] E12
  # allowed_ips: [...]                               # [HARDENING] E17 — per plan decision

router_settings:
  fallbacks: ...                                     # existing (unchanged)
  num_retries: 2                                      # existing
  timeout: 300                                        # existing
  stream_timeout: 300                                 # existing
  allowed_fails: 3                                    # existing
  cooldown_time: 60                                   # existing
  retry_policy: ...                                   # existing
  enable_pre_call_checks: true                        # [HARDENING] E16

litellm_settings:
  drop_params: true                                   # existing
  request_timeout: 300                                # existing
  force_ipv4: true                                    # existing
  user_url_validation: true                           # [HARDENING] E1
  json_logs: true                                     # [HARDENING] E13
  redact_user_api_key_info: true                      # [HARDENING] E14
  turn_off_message_logging: true                     # [HARDENING] E15
```

## Handoff output

Return the handoff YAML schema defined in
`workflow-litellm-hardening-00-orchestration`. Set:

- `outcome` to `pass` once all planned edits are applied and the file parses as
  YAML.
- `constraints_applied` to include all five constraints listed above.
- `files_touched` to list `infra/litellm/config.yaml` with the change summary.
- `next_phase: 04-validate`.
- `blockers: []` unless an edit key failed option-index verification.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: "added E1–E16 hardening keys; E17 per decision; no model_list changes; no secret regressions"
constraints_applied:
  - constraint-litellm-secret-hygiene
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-deprecation-free
  - constraint-litellm-fallback-resolution
assumptions:
  - ...
risks:
  - ...
tests_run:
  - "yaml.safe_load sanity check"
tests_needed:
  - "validation-litellm-config-check (phase 04)"
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
