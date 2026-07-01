---
name: workflow-litellm-hardening-05-verify
description: |
  Use only for the verify phase of the LiteLLM hardening workflow. Run
  `validation-litellm-startup`, `validation-litellm-smoke`, and
  `validation-litellm-no-hardcoded-secrets`, then report. Do not use for
  scoping, planning, implementation, or config-check validation.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(curl:*) Bash(git:*) Bash(python3:*) Bash(yq:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-hardening
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (LiteLLM hardening)

## Phase purpose

Run the verification gate suite — startup, smoke, and secret scan — and report
the root-cause-to-verification evidence chain. This is the terminal phase; it
does not introduce new behavior.

## Steps to perform

1. Read the validate-phase handoff (config-check passed).
2. Run the gates in this order, stopping and surfacing a failure if any fails:
   - `validation-litellm-startup` — start the proxy with the hardened config
     and confirm it reaches a healthy state (`GET /health/liveliness` returns
     200). Use the main in-memory image; no DB.
   - `validation-litellm-smoke` — make a minimal authenticated request against
     the `coding` completion model (or `/health/readiness`) and confirm a
     non-error response. Use `os.environ/LITELLM_MASTER_KEY` for auth.
   - `validation-litellm-no-hardcoded-secrets` — scan
     `infra/litellm/config.yaml` for any `api_key` / `master_key` value that is
     NOT `os.environ/...`; confirm all secrets use indirection.
3. Apply `constraint-litellm-secret-hygiene` to interpret the secret-scan result.
4. Report: the scope-phase gap table (which gaps were closed), files modified,
   raw output/pass-fail of each gate, and any deferred decisions (e.g. E17).

## Docs to consult

- `docs/litellm/deployment-ops/README.md` — health endpoints, image, env loading.
- `docs/litellm/schemas/config-yaml.option-index.json` — for interpreting any
  startup warning about a key.

## Operational skills to load

None mandatory. The three validation skills are loaded directly as validations
(below).

## Constraints to apply

- `constraint-litellm-secret-hygiene` — the secret-scan gate enforces this; any
  hardcoded secret is a hard failure.
- `constraint-litellm-config-schema` — any startup warning about an unknown
  key is a schema regression; route back to phase 03.
- `constraint-litellm-in-memory-no-db` — confirm startup used the main
  in-memory image (no `database_url`, no DB errors in startup log).
- `constraint-litellm-deprecation-free` — confirm no deprecated keys triggered
  startup warnings.
- `constraint-litellm-fallback-resolution` — confirm every fallback target
  resolved at startup (no "model not found" errors for fallback model names).

## Validations to run

Run these validation skills in gate order. Each maps to a runtime check.

- `validation-litellm-startup` — proxy starts healthy with the hardened config.
- `validation-litellm-smoke` — a minimal authenticated request succeeds.
- `validation-litellm-no-hardcoded-secrets` — no secret literals in config.

## Handoff output

Return the handoff YAML schema defined in
`workflow-litellm-hardening-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when all three gates pass.
- `constraints_applied` to include `constraint-litellm-secret-hygiene`,
  `constraint-litellm-config-schema`, `constraint-litellm-in-memory-no-db`,
  `constraint-litellm-deprecation-free`, `constraint-litellm-fallback-resolution`.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output of each gate.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. smoke could not reach a real upstream because
  egress allowlist / env vars not set in the verify environment).
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: "hardened config (E1–E16 applied; E17 per decision)"
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
  - name: validation-litellm-startup
    covers: "proxy starts healthy with hardened config"
  - name: validation-litellm-smoke
    covers: "minimal authenticated request succeeds"
  - name: validation-litellm-no-hardcoded-secrets
    covers: "no secret literals in config"
tests_needed: []
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-litellm-startup
    gate: "litellm --config infra/litellm/config.yaml; GET /health/liveliness == 200"
    result: pass|fail
  - validation: validation-litellm-smoke
    gate: "authenticated request to coding model / readiness probe"
    result: pass|fail
  - validation: validation-litellm-no-hardcoded-secrets
    gate: "scan config for non-os.environ secret values"
    result: pass|fail
constraints_checked:
  - constraint-litellm-secret-hygiene
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-deprecation-free
  - constraint-litellm-fallback-resolution
evidence:
  - gate: "validation-litellm-startup"
    output: ...
  - gate: "validation-litellm-smoke"
    output: ...
  - gate: "validation-litellm-no-hardcoded-secrets"
    output: ...
failures: []
not_fully_checkable: []
```
