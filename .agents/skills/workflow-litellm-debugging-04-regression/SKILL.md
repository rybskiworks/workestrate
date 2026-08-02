---
name: workflow-litellm-debugging-04-regression
description: |
  Use only for the regression phase of the LiteLLM debugging workflow.
  Add/run a check that would have caught the defect (e.g. run
  validation-litellm-config-check). Do not use for reproduction, diagnosis,
  fixing, or final verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(lite:*) Bash(curl:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-debugging
  org.phase: regression
  org.phase_order: "04"
---

## Phase purpose

Add or run a check that would have caught the defect. The check must encode
the reproduction from phase `01-reproduce` so the same defect cannot silently
return.

## Steps to perform

1. Load `litellm-config-anatomy` and `validation-litellm-config-check` (the
   validation procedure with checks (a)–(i), encoded in `check_config.py` +
   the 8 `constraint-litellm-*` skills).
2. Run `validation-litellm-config-check` against the edited `config.yaml`.
   This is the primary regression gate for config-shape defects: it
   cross-checks key existence + section, provider prefix, fallback target
   resolution, deprecated keys, DB/Redis keys in in-memory mode, env var
   resolution, `openai/` `api_base` `/v1`, and `anthropic/` suffix behavior.
3. If the defect is a config-shape defect (the common case), the
   `validation-litellm-config-check` run IS the regression check — confirm
   it FAILS on the pre-fix config (revert temporarily, run, observe FAIL;
   re-apply, run, observe PASS) and record both observations.
4. If the defect is not caught by static config validation (e.g. an env var
   that is correctly named but unset at runtime, or a fallback that does not
   fire because of `retry_policy` semantics), add a targeted runtime check:
   a `lite http request` / `curl` reproducer script asserting the
   post-fix response status and error body, placed under
   `infra/litellm/checks/` (or the project's checks directory). The check
   MUST encode the reproduction from phase `01-reproduce` (same request,
   same `model` field, same assertion that the defect violated).
5. Confirm the check fails before the fix and passes after. Record both
   observations (command + result) in `evidence`.
6. Apply `constraint-litellm-secret-hygiene`: the regression check must not
   embed secret literals; use `os.environ/<VAR>` or `$LITELLM_MASTER_KEY`
   expansion. Keep the check scoped to the defect; do not build a broad
   test-suite expansion as part of a bugfix.

## Docs to consult

- `validation-litellm-config-check` (`.agents/skills/validation-litellm-config-check/`) — checks (a)–(i), encoded in `scripts/check_config.py` + the 8 `constraint-litellm-*` skills.
- `docs/litellm/config/config-validation.md` — startup validation behavior.
- `docs/litellm/troubleshooting.md` — "Troubleshooting workflow" step 5
  ("Verify with the reproducer").

## Operational skills to load

- `litellm-config-anatomy`.

## Constraints to apply

- `constraint-litellm-secret-hygiene` — The regression check must not embed
  secret literals; use `os.environ/<VAR>` or env expansion.
- `constraint-litellm-config-schema` — Any new config assertion must check
  keys against `config-yaml.option-index.json`.

## Validations to run

- `validation-litellm-config-check` — verify the edited config passes static
  validation (and confirm it FAILS on the pre-fix config).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 05-verify`, `next_workflow: null`. Record the regression check
location (file + check name, or "validation-litellm-config-check run") and
its assertion in `tests_run` and `evidence`. Record the fails-before /
passes-after confirmation in `evidence`.
