---
name: workflow-litellm-validation-01-scope
description: |
  Use only for the scope phase of the LiteLLM validation workflow.
  Identify what changed (git diff) and determine which gates apply
  (in-memory vs DB-backed; KVM available for smoke?). Do not use for
  config-check, startup, smoke, or final reporting.
allowed-tools: Read Bash(git:*) Bash(grep:*) Bash(find:*) Bash(test:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-validation
  org.phase: scope
  org.phase_order: "01"
---

# LiteLLM Validation Workflow — Phase 01 — Scope

## Phase Purpose

Identify what changed and determine which gates apply. Record applicable vs
not-applicable gates so phases 02–04 run only the relevant gates and mark the
rest "not applicable" or "not fully checkable".

## Steps

1. Identify what changed:
   ```sh
   git diff --stat
   git diff --name-only
   ```
   Focus on `infra/litellm/config.yaml` and any `docs/litellm/` corpus changes.
2. Determine gate applicability:
   - **In-memory vs DB-backed**: inspect the config for `database_url` and
     `store_model_in_db`. The normalized schema records both verbatim
     (`database_url` — string, sources: config_settings, configs,
     docker_quick_start, virtual_keys; `store_model_in_db` — bool, source:
     config_settings). If `database_url` is absent, the deployment is
     in-memory; the `constraint-litellm-in-memory-no-db` check applies in
     phase 02. If `database_url` is set, the deployment is DB-backed and the
     in-memory-no-db constraint is not applicable (mark N-A with reason).
   - **KVM availability for smoke**: check for `/dev/kvm`. The validation
     report records verbatim: "this environment has no `/dev/kvm`, so runtime
     validation is blocked here — defer to a KVM-capable host." If `/dev/kvm`
     is absent, the phase 04 smoke gate is `not_fully_checkable` and the live
     calls are skipped.
   - **Secret-hygiene gate**: always applicable — `validation-litellm-no-hardcoded-secrets`
     runs as part of the config-check family in phase 02.
3. Record the list of applicable gates and the list of not-applicable /
   not-fully-checkable gates with reasons.
4. Do NOT modify any config or corpus in this phase. Scope only.

## Docs to Consult

- `docs/litellm/config/config-validation.md`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
- `docs/litellm/validation-report.md`

## Operational Skills to Load

None. This phase identifies which gates apply; it does not run validation gates
or load operational skills.

## Constraints to Apply

- `constraint-litellm-config-schema` — validation is a gate, not a fix; do not
  modify config to make a gate applicable/inapplicable.

## Validations to Run

None. This phase identifies which gates apply; it does not run validation gates.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (scope determined) | partial (some gates' applicability unclear).
- `files_touched`: `[]` (scope only; no changes).
- `constraints_applied`: `["constraint-litellm-config-schema"]`
- `assumptions`: the list of applicable gates and the list of
  not-applicable / not-fully-checkable gates with reasons; the changed-files
  list; the in-memory vs DB-backed determination; the KVM-availability
  determination.
- `risks`: KVM absent (smoke `not_fully_checkable`); DB-backed deployment
  (in-memory-no-db constraint N-A).
- `tests_run`: `["git diff --stat", "git diff --name-only", "test -e /dev/kvm"]`.
- `next_phase`: `02-config-check`.
- `next_workflow`: `null`.
- `blockers`: any gate whose applicability could not be determined.
