---
name: workflow-litellm-config-review-05-verdict
description: |
  Use only for the verdict phase of the LiteLLM config-review workflow. Classify
  every finding under a severity level and issue a final verdict. Do not use for
  scoping, analysis, running checks, or manual review.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-review
  org.phase: verdict
  org.phase_order: "05"
---

# Phase 05: verdict (LiteLLM config review)

## Phase purpose

Classify every finding under a severity level, record the constraints checked
and their status, and issue a final verdict. If the review surfaces a defect,
hand off to `workflow-litellm-config-change`; **do not fix in the review pass** —
this workflow is read-only.

## Steps to perform

1. Classify every finding under exactly one severity level:
   - **blocker** — must fix before merge: a `validation-litellm-config-check`
     FAIL (checks a–i), a hardcoded secret (`validation-litellm-no-hardcoded-secrets`
     FAIL), a `requires_db`/`requires_redis` key in in-memory mode, a dangling
     fallback target, an unknown provider prefix, an `openai/` `api_base` missing
     `/v1` or missing `api_key`.
   - **major** — should fix before merge but may be deferred with owner approval:
     `anthropic/` `api_base` pre-including `/v1/messages` without the
     disable-suffix env var, an overlap key set in both `litellm_settings` and
     `router_settings` without a comment, a deprecated key (`set_verbose`) still
     in use.
   - **minor** — fix encouraged but not blocking: a WARN from check (d)/(g), a
     project-defined env var not documented in `env-vars.index.json`, a missing
     `model_info` field.
   - **nit** — optional polish: comment wording, key ordering, example
     simplification.
   - **praise** — positive callout: notably clean fallback graph, correct
     `hosted_vllm/` choice for a keyless endpoint.
2. For each finding record: `file:line`/key-path, `severity`, the constraint and
   schema index field or doc rule violated (cited, not paraphrased), and a
   concrete suggested fix.
3. State explicitly which of the 8 constraints were walked and their
   pass/fail/N-A status.
4. Issue a final verdict: `approve`, `request-changes`, or `reject`.
5. If the review surfaces a defect (any blocker or a config defect requiring a
   fix), set `next_workflow: workflow-litellm-config-change-00-orchestration`
   (do not fix in the review pass — this workflow is read-only). Otherwise set
   `next_workflow: null`.

## Docs to consult

None new. Findings reference the docs and schema index files cited in phase
04-review.

## Operational skills to load

None new.

## Constraints to apply

- `constraint-litellm-config-schema` — findings cover only the diff; pre-existing
  issues in untouched config are recorded as separate follow-ups, not as review
  findings.

## Validations to run

None — this is a reporting phase. Automated validations ran in phase 03
(workflow-litellm-config-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-litellm-config-review-00-orchestration`, including the
verification-phase extra fields. Set:

- `outcome`: `pass` if verdict is `approve`; `partial` if `request-changes`;
  `fail` if `reject`.
- `constraints_applied`: `constraint-litellm-config-schema` (plus any of the 8
  constraints applied in phase 04 that produced findings).
- `risks`: the final classified findings list (key-path, severity, cited
  constraint + schema field, suggested fix).
- `blockers`: every finding classified `blocker`.
- `next_phase`: `null` (this is the final phase).
- `next_workflow`: `workflow-litellm-config-change-00-orchestration` if a defect
  was found, else `null`.

Include the verification-phase extra fields:

```yaml
validations_run:
  - validation-litellm-config-check
  - validation-litellm-no-hardcoded-secrets
constraints_checked:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-secret-hygiene
  - constraint-litellm-provider-prefix
  - constraint-litellm-openai-compatible-api-base
  - constraint-litellm-anthropic-suffix
  - constraint-litellm-fallback-resolution
  - constraint-litellm-deprecation-free
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

`not_fully_checkable` records any constraint item that could not be fully
resolved mechanically (e.g. an `os.environ/<VAR>` whose runtime injection cannot
be confirmed by static validation, or a project-defined env var not in
`env-vars.index.json`), with a reason and a suggested escalation path.
