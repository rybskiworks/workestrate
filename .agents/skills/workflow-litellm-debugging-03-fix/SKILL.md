---
name: workflow-litellm-debugging-03-fix
description: |
  Use only for the fix phase of the LiteLLM debugging workflow.
  Implement the minimal root-cause config edit; apply the relevant LiteLLM
  constraints. Do not use for reproduction, diagnosis, regression testing,
  or final verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(lite:*) Bash(curl:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-debugging
  org.phase: fix
  org.phase_order: "03"
---

## Phase purpose

Implement the minimal root-cause config edit. Do not suppress the symptom.
The fix must address the mechanism identified in phase `02-diagnose`.

## Steps to perform

1. Fix the root cause, not the symptom. Concretely by category:
   - **Provider prefix wrong** → correct the `litellm_params.model` prefix to
     match the endpoint's protocol (apply
     `constraint-litellm-provider-prefix`); do not paper over with an
     `api_base` hack.
   - **`api_base` missing `/v1` or appended path** → set `api_base` to end
     with `/v1` and strip any appended endpoint path (apply
     `constraint-litellm-openai-compatible-api-base`); verbatim: "LiteLLM
     uses the openai-client to make these calls, and that automatically adds
     the relevant endpoints."
   - **`anthropic/` double-suffix** → either strip the trailing
     `/v1/messages` from `api_base`, or set
     `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` only if the upstream already
     includes `/v1/messages` (apply `constraint-litellm-anthropic-suffix`);
     verbatim: "set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` only if the
     upstream already includes `/v1/messages` in `api_base`."
   - **Dangling fallback target** → add the missing `model_name` to
     `model_list`, or remove the dangling fallback entry; if fallback is not
     firing, add the missing `retry_policy` key or raise `allowed_fails` /
     `num_retries` (apply `constraint-litellm-fallback-resolution`).
   - **Deprecated key** → replace with the documented replacement (apply
     `constraint-litellm-deprecation-free`); e.g. `set_verbose` →
     `LITELLM_LOG` (verbatim: "`set_verbose` is **DEPRECATED** → use
     `LITELLM_LOG`.").
   - **DB/Redis key in in-memory mode** → remove the `requires_db` /
     `requires_redis` key, or set the compensating control (e.g.
     `disable_spend_logs: true`); leave `STORE_MODEL_IN_DB` False (verbatim:
     "Do **NOT** set `STORE_MODEL_IN_DB` (leave False) — models are static in
     `config.yaml`.").
   - **Misspelled env var** → correct the var name to a known LiteLLM
     built-in or documented project-defined var; confirm it is injected in
     the runtime environment.
   - **Hardcoded secret** → replace the literal with `os.environ/<VAR>` and
     inject the var via the sandbox `env()`/`secret_env()` (apply
     `constraint-litellm-secret-hygiene`).
2. Apply `constraint-litellm-config-schema`: every edited/added key must
   exist in `config-yaml.option-index.json` under the correct `section`.
3. Apply `constraint-litellm-deprecation-free`: do not introduce a
   deprecated key as part of the fix.
4. Apply `constraint-litellm-secret-hygiene`: never write a secret literal
   into `config.yaml`; always use `os.environ/<VAR>`.
5. Keep the edit minimal: fix only the reported defect; record adjacent
   issues as follow-ups; do not refactor surrounding config or add new
   providers as part of a bugfix.

## Docs to consult

- `docs/litellm/troubleshooting.md` — per-category fixes in the "Config keys
  / requirements" table and sections 1–11.
- `docs/litellm/config/config-validation.md` — validation pitfalls to avoid
  reintroducing.
- `docs/litellm/schemas/config-yaml.option-index.json` — key existence +
  `section` for every edit.
- `docs/litellm/schemas/provider-fields.index.json` — `api_base_behavior` and
  caveats for the affected provider.

## Operational skills to load

Conditional by category:
- the relevant provider/routing skill (for prefix / `api_base` / fallback
  fixes);
- `litellm-config-anatomy` (for schema placement).

## Constraints to apply

- `constraint-litellm-config-schema` — Every edited/added key exists in
  `config-yaml.option-index.json` under the correct `section`.
- `constraint-litellm-provider-prefix` — The `litellm_params.model` prefix
  matches the endpoint's protocol.
- `constraint-litellm-openai-compatible-api-base` — `openai/`-prefix
  `api_base` ends with `/v1` and has no appended endpoint path.
- `constraint-litellm-anthropic-suffix` — `anthropic/`-prefix `api_base` does
  not double-append `/v1/messages`.
- `constraint-litellm-fallback-resolution` — Every `fallbacks` source and
  target exists as a `model_name` in `model_list`.
- `constraint-litellm-deprecation-free` — No deprecated key is introduced.
- `constraint-litellm-secret-hygiene` — No secret literal in `config.yaml`;
  always `os.environ/<VAR>`.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 04-regression`, `next_workflow: null`. Record the fix — what
changed and why it addresses the root cause (with corpus citation) — in
`evidence`. List every file changed in `files_touched`. List the constraints
actually applied in `constraints_applied`.
