---
name: workflow-litellm-debugging-05-verify
description: |
  Use only for the verify phase of the LiteLLM debugging workflow.
  Run validation-litellm-startup + validation-litellm-smoke and report the
  root cause, fix, and regression check. Do not use for reproduction,
  diagnosis, fixing, or regression testing.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(lite:*) Bash(curl:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-debugging
  org.phase: verify
  org.phase_order: "05"
---

## Phase purpose

Run the startup and smoke validation gates and report the root cause, fix,
and regression check. This is the final phase.

## Steps to perform

1. Run `validation-litellm-startup`: start the proxy with
   `litellm --config <path>` (or the project's startup command) and confirm
   it reaches the success log. Verbatim from
   `docs/litellm/config/config-validation.md`:
   > "LiteLLM validates `config.yaml` against an internal Pydantic schema at
   > startup. On invalid config, the proxy fails to start."
   Success marker (verbatim): "Loaded config YAML (api_key and
   environment_variables are not shown): { ... }". A Pydantic
   `ValidationError` traceback is a FAIL.
2. Run `validation-litellm-smoke`: exercise the router → provider path with
   the reproducer from phase `01-reproduce` (e.g.
   `lite http request --model <tier> --message "ping"` or the equivalent
   `curl` against `:4000`). Confirm the response status and body match the
   post-fix expectation (no "Not Found Error", no 401, no fallback 500, no
   DB-feature error for in-memory-expected endpoints).
3. Run validation skills: `validation-litellm-startup`;
   `validation-litellm-smoke`. (The `validation-litellm-config-check` gate
   was run in phase `04-regression`; do not re-run unless the config changed
   since.)
4. Report all of the following (aggregate from prior phases):
   - the reproduction command and the original error body (from phase
     `01-reproduce`, secrets redacted);
   - the defect category (from phase `02-diagnose`);
   - the root cause: a one-paragraph explanation of why the defect occurred
     (with corpus citation);
   - the fix: what changed and why it addresses the root cause (from phase
     `03-fix`);
   - the regression check: location, assertion, and fails-before /
     passes-after confirmation (from phase `04-regression`);
   - pass/fail for `validation-litellm-startup` and
     `validation-litellm-smoke`.
5. Relax verbosity back to `LITELLM_LOG=INFO` after reproduction (verbatim
   from `docs/litellm/troubleshooting.md` "Troubleshooting workflow" step 5:
   "Verify with the reproducer, then relax verbosity back to `INFO`.").
   Do not leave `--detailed_debug` on.

## Docs to consult

None new. Findings reference docs cited in earlier phases
(`docs/litellm/troubleshooting.md`,
`docs/litellm/config/config-validation.md`,
`docs/litellm/schemas/config-yaml.option-index.json`,
`docs/litellm/schemas/provider-fields.index.json`,
`docs/litellm/schemas/env-vars.index.json`).

## Operational skills to load

None new.

## Constraints to apply

- `constraint-litellm-secret-hygiene` — Verify only the reported defect's
  fix; do not echo secrets in the verification report; redact all keys.
- `constraint-litellm-config-schema` — Do not make additional config changes
  to make a gate pass; if a gate fails, go back to phase `03-fix`.

## Validations to run

- `validation-litellm-startup` (`litellm --config <path>` reaches the
  "Loaded config YAML" log; no Pydantic `ValidationError`).
- `validation-litellm-smoke` (the reproducer request returns the post-fix
  status/body).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the
verification-phase extra fields:
- `validations_run` — list of validation skills executed
  (`validation-litellm-startup`, `validation-litellm-smoke`);
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — reproduction command, original error (redacted), category,
  root cause, fix, regression check location/assertion, per-gate pass/fail;
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated
  and why (e.g. upstream provider unreachable due to egress allowlist, not a
  config bug).

Set `next_phase: null`, `next_workflow: null`. Unless the fix revealed a need
for new config keys or a new provider entry, in which case set
`next_workflow: workflow-litellm-implementation` (the debugging workflow is
complete; the authoring work is a separate workflow).
