---
name: workflow-litellm-validation-05-report
description: |
  Use only for the report phase of the LiteLLM validation workflow.
  Aggregate all gate results into a per-gate table and report the
  overall verdict. Do not use for scoping, config-check, startup, or
  smoke.
allowed-tools: Read
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-validation
  org.phase: report
  org.phase_order: "05"
---

# LiteLLM Validation Workflow — Phase 05 — Report

## Phase Purpose

Aggregate all gate results into a per-gate table. If KVM was absent (determined
in phase 01), note that the smoke gate was `not_fully_checkable` and defer to a
KVM-capable host. Report the overall verdict: pass (all applicable gates green)
or fail (any applicable gate red). If a gate failed, hand off to the appropriate
fixing workflow — do NOT fix in the validation pass.

## Steps

1. Aggregate the gate results from phases 02–04 into a per-gate table:

   | Gate | Command | Result | Evidence |
   |---|---|---|---|
   | Config check (a–i) | `python .agents/skills/validation-litellm-config-check/scripts/check_config.py --config infra/litellm/config.yaml` | pass/fail | offending key/path or clean |
   | No hardcoded secrets | `validation-litellm-no-hardcoded-secrets` scan | pass/fail | hardcoded secret location or clean |
   | Startup | `litellm --config infra/litellm/config.yaml` | pass/fail | "Loaded config YAML..." log or Pydantic `ValidationError` traceback |
   | Smoke — `/v1/models` | `curl /v1/models` | pass/fail/N-A | HTTP status + body summary |
   | Smoke — `/health/liveliness` | `curl /health/liveliness` | pass/fail/N-A | HTTP status |
   | Smoke — `/v1/chat/completions` | `curl /v1/chat/completions (model=<alias>)` | pass/fail/N-A | HTTP status + body summary |

   Mark `not_fully_checkable` gates (e.g., smoke when KVM is absent or the proxy
   did not start) and N-A constraints (e.g., `constraint-litellm-in-memory-no-db`
   on a DB-backed deployment).
2. If KVM was absent, note the deferral (verbatim from the validation report):
   "this environment has no `/dev/kvm`, so runtime validation is blocked here —
   defer to a KVM-capable host."
3. Compute the overall verdict:
   - **pass** — all applicable gates green.
   - **fail** — any applicable gate red.
4. If any applicable gate failed, hand off to the appropriate fixing workflow —
   do NOT fix in the validation pass:
   - Config defect → LiteLLM implementation/harden workflow
   - Runtime/startup defect → LiteLLM debugging workflow
   Then re-run validation after the fix.
5. Keep the report to the per-gate table and overall verdict; do not expand it
   into a code review or implementation summary.

## Docs to Consult

- `docs/litellm/config/config-validation.md`
- `docs/litellm/validation-report.md` (KVM-gating precedent)

## Operational Skills to Load

None. This phase aggregates results; it does not run gates or load operational
skills.

## Constraints to Apply

- `constraint-litellm-config-schema` — validation is a gate, not a fix; do not
  modify config; hand off failures to fixing workflows.

## Validations to Run

None. This is a reporting phase — it aggregates all gate results from phases
02–04; it does not run validation gates itself.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any applicable gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-litellm-config-schema"]`
- `tests_run`: the full list of gate commands run across phases 02–04 with
  pass/fail per gate.
- `validations_run`: `["validation-litellm-config-check", "validation-litellm-startup", "validation-litellm-smoke", "validation-litellm-no-hardcoded-secrets"]`
- `constraints_checked`: `["constraint-litellm-config-schema", "constraint-litellm-in-memory-no-db", "constraint-litellm-secret-hygiene", "constraint-litellm-provider-prefix", "constraint-litellm-openai-compatible-api-base", "constraint-litellm-anthropic-suffix", "constraint-litellm-fallback-resolution", "constraint-litellm-deprecation-free"]`
- `evidence`: the per-gate table; the overall verdict; the KVM-deferral note if
  smoke was `not_fully_checkable`.
- `failures`: the list of failing applicable gates with evidence.
- `not_fully_checkable`: gates marked N-A or `not_fully_checkable` — the smoke
  gate when KVM is absent or the proxy did not start; the
  `constraint-litellm-in-memory-no-db` check on a DB-backed deployment.
- `next_phase`: stop.
- `next_workflow`: `null` (if pass) | LiteLLM implementation/harden (config
  defect) | LiteLLM debugging (runtime/startup defect) (if any applicable gate
  failed).
- `blockers`: the failing gates if `outcome: fail`.
