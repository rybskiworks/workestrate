---
name: workflow-litellm-debugging-01-reproduce
description: |
  Use only for the reproduce phase of the LiteLLM debugging workflow.
  Reproduce the issue reliably and capture the exact request, response
  status, full error body, and proxy log lines. Do not use for diagnosis,
  fixing, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(lite:*) Bash(curl:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-debugging
  org.phase: reproduce
  org.phase_order: "01"
---

## Phase purpose

Reproduce the issue reliably and capture the exact request, response status,
full error body, and proxy log lines. This phase is reproduction only — do not
begin diagnosis or fixing here.

## Steps to perform

1. Reproduce the issue reliably. Capture:
   - the exact request that triggers the defect — method, path, the `model`
     field value, and headers **minus the secret** (`Authorization: Bearer
     $LITELLM_MASTER_KEY` redacted to `Authorization: Bearer <redacted>`);
   - the response status code and full error body;
   - the environment — proxy port (default 4000), `LITELLM_LOG` level,
     `JSON_LOGS`, working directory, and the `config.yaml` path in use;
   - the relevant proxy log lines (the failing `model_name` and the resolved
     `litellm_params.model` / `api_base`).
2. Use `lite http request --model <tier> --message "ping"` or a direct `curl`
   against `:4000` to exercise the full router → provider path. Verbatim
   reproduction tip from `docs/litellm/troubleshooting.md`:
   > "Reproduction tip: `lite http request --model coding --message "ping"`
   > exercises the full router → provider path without a DB."
3. Enable debug logging for the reproduction only. Verbatim from
   `docs/litellm/troubleshooting.md` (section 9):
   > "Use `--debug` or `--detailed_debug` CLI flags, or set `LITELLM_LOG` env
   > var to `INFO`, `DEBUG`, or `ERROR`."
   > "WARNING: FOR PROD DO NOT USE `--detailed_debug` it slows down response
   > times".
   Prefer `LITELLM_LOG=INFO` (or `--debug`); reserve `--detailed_debug` for
   reproduction only.
4. Match the symptom to one of the documented LiteLLM failure modes (from
   `docs/litellm/troubleshooting.md`), recording which mode was reproduced:
   - **"Not Found Error"** from a missing `/v1` on an `openai/`-prefix
     `api_base` (verbatim: "If you see Not Found Error when testing make sure
     your api_base has the /v1 postfix.");
   - **auth fail / 401** — empty `master_key` or empty provider key from an
     unset `os.environ/<VAR>` (verbatim: "an empty `master_key` silently
     breaks auth, an empty provider key yields upstream 401s.");
   - **fallback 500** — a dangling fallback target, or fallback not firing
     because the failure type is not in `retry_policy` (verbatim: "Fallbacks
     are triggered after the configured number of retries fails."; "400/
     BadRequestErrors are not counted toward region-outage / fail counters");
   - **DB-feature error in in-memory mode** — a DB-gated endpoint
     (`/key/generate`, `/user/new`, `/team/new`, `/budget/new`, dynamic
     `POST /fallback`, Admin UI) called without Postgres (verbatim:
     "Dynamic fallback management ... require `STORE_MODEL_IN_DB=True`.";
     "Admin UI — verbatim 'Requires db connected'.");
   - **env var resolving to None** — `os.environ/UNSET_VAR` → empty string,
     not an error (verbatim: "`os.environ/VAR_NAME` syntax runs
     `os.getenv("VAR_NAME")` at load time ... If empty, `api_key` /
     `master_key` resolves to an empty string.").
5. If the issue cannot be reproduced, record what is known and what
   reproduction attempts were made. Do NOT proceed to a fix on an
   unreproducible report. Set `outcome: fail` and
   `blockers: ["issue not reproduced"]` in the handoff.
6. Apply `constraint-litellm-secret-hygiene`: never echo `master_key` or
   provider `api_key` values in logs, handoffs, or `evidence`. Redact all
   secrets. This phase is reproduction only; do not start fixing adjacent
   issues observed while reproducing — record them as follow-ups.

## Docs to consult

- `docs/litellm/troubleshooting.md` — failure-mode catalog and the
  "Troubleshooting workflow" section (steps 1–3).
- `docs/litellm/config/config-validation.md` — startup log shape and
  validation behavior.

## Operational skills to load

None mandatory in this phase. The `litellm-config-anatomy` skill is loaded in
phase `02-diagnose` based on the defect category.

## Constraints to apply

- `constraint-litellm-secret-hygiene` — Never log, echo, or persist
  `master_key` / provider `api_key` values; redact all secrets in handoffs and
  `evidence`; this phase is reproduction only.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 02-diagnose`, `next_workflow: null`, `handoff_requires_hil:
false`. Record the reproduction command, the redacted request, the response
status + error body, and the relevant log lines in `evidence` (or
`assumptions` if the reproduction is partial). If reproduction failed, set
`outcome: fail`, `blockers: ["issue not reproduced"]`, and stop.
