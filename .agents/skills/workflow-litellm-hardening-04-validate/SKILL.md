---
name: workflow-litellm-hardening-04-validate
description: |
  Use only for the validate phase of the LiteLLM hardening workflow. Run
  `validation-litellm-config-check` against the edited config. Do not use for
  scoping, planning, implementation, or final verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(curl:*) Bash(git:*) Bash(python3:*) Bash(yq:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-hardening
  org.phase: validate
  org.phase_order: "04"
---

# Phase 04: validate (LiteLLM hardening)

## Phase purpose

Run `validation-litellm-config-check` against the edited
`infra/litellm/config.yaml`. This is the config-level gate; it does not start
the proxy or make upstream calls. A failure here stops the workflow and returns
to phase 03 for a fix.

## Steps to perform

1. Read the implement-phase handoff (the edited config path).
2. Run `validation-litellm-config-check` against `infra/litellm/config.yaml`.
3. If it fails, capture the exact error, stop, and surface a handoff with
   `outcome: fail` and `next_phase: 03-implement` so the root cause can be
   fixed. Do not attempt the fix in this phase.
4. If it passes, record the raw output and proceed to phase 05.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json` — for interpreting
  unknown-key errors.
- `docs/litellm/config/general-settings.md`, `router-settings.md`,
  `litellm-settings.md` — for interpreting type/value errors.

## Operational skills to load

None mandatory. The validation skill `validation-litellm-config-check` is loaded
directly as a validation (below), not as an operational skill.

## Constraints to apply

- `constraint-litellm-config-schema` — a config-check failure on an unknown key
  or wrong type is a schema violation; route back to phase 03, do not paper over
  it.
- `constraint-litellm-deprecation-free` — if the checker flags a deprecated
  key, remove it rather than silencing the warning.

## Validations to run

- `validation-litellm-config-check` — the LiteLLM config validator against
  `infra/litellm/config.yaml`. This is the sole gate in this phase.

## Handoff output

Return the handoff YAML schema defined in
`workflow-litellm-hardening-00-orchestration`. Set:

- `outcome` to `pass` only when `validation-litellm-config-check` passes.
- `constraints_applied` to include `constraint-litellm-config-schema` and
  `constraint-litellm-deprecation-free` (if a deprecated key was flagged).
- `validations_run` to list `validation-litellm-config-check` with pass/fail.
- `evidence` to include the raw validator output.
- `next_phase: 05-verify` on pass; `next_phase: 03-implement` on fail.
- `blockers: []` on pass; the failure detail on fail.

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: validation-litellm-config-check
    covers: "config schema + key validity"
tests_needed:
  - "validation-litellm-startup (phase 05)"
  - "validation-litellm-smoke (phase 05)"
  - "validation-litellm-no-hardcoded-secrets (phase 05)"
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-litellm-config-check
    gate: "litellm config-check infra/litellm/config.yaml"
    result: pass|fail
constraints_checked:
  - constraint-litellm-config-schema
evidence:
  - gate: "litellm config-check infra/litellm/config.yaml"
    output: ...
failures: []
not_fully_checkable: []
```
