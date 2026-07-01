---
name: workflow-litellm-debugging-02-diagnose
description: |
  Use only for the diagnose phase of the LiteLLM debugging workflow.
  Categorize the defect (provider-prefix wrong, api_base missing /v1 or
  appended path, anthropic/ double-suffix, dangling fallback target,
  deprecated key, DB/Redis key in in-memory mode, misspelled env var,
  hardcoded secret) and identify the root cause. Do not use for reproduction,
  fixing, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(litellm:*) Bash(lite:*) Bash(curl:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-debugging
  org.phase: diagnose
  org.phase_order: "02"
---

## Phase purpose

Categorize the defect and identify the root cause. The category determines
which operational skills, constraints, and docs to load in phase `03-fix`.

## Steps to perform

1. Load `litellm-config-anatomy` (the diagnose skill) and read
   `docs/litellm/troubleshooting.md` and
   `docs/litellm/config/config-validation.md`.
2. Classify the defect into exactly one of the following categories (each
   grounded in `docs/litellm/troubleshooting.md`):
   - **Provider prefix wrong** — the `litellm_params.model` prefix does not
     match the endpoint's protocol (verbatim: "if a provider call fails with
     auth/404, verify the prefix matches the endpoint's protocol."). Example
     deviation flagged in the corpus: `anthropic/MiniMax-M3` vs the documented
     `minimax/` prefix.
   - **`api_base` missing `/v1` or appended path** (OpenAI-compatible) —
     verbatim: "If you see Not Found Error when testing make sure your
     api_base has the /v1 postfix. Example: `http://vllm-endpoint.xyz/v1`."
     and "Do NOT add anything additional to the base url e.g.
     `/v1/embedding`."
   - **`anthropic/` double-suffix** — LiteLLM auto-appends `/v1/messages`
     (verbatim: "LiteLLM automatically appends the appropriate suffix
     (`/v1/messages` or `/v1/complete`) to your base URL."). A 404 results if
     `api_base` already ends in `/v1/messages` and
     `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` is not set.
   - **Dangling fallback target** — `router_settings.fallbacks` references a
     `model_name` not in `model_list`; the proxy 500s on failover. Or fallback
     not firing because the failure type is not in `retry_policy` (verbatim:
     "400/BadRequestErrors are not counted toward region-outage / fail
     counters").
   - **Deprecated key** — e.g. `set_verbose` (verbatim: "`set_verbose` is
     **DEPRECATED** → use `LITELLM_LOG`.").
   - **DB/Redis key in in-memory mode** — a `requires_db` / `requires_redis`
     key set without the backing service (verbatim: "`STORE_MODEL_IN_DB=True`
     without a DB breaks model loading. Leave it False in-memory.").
   - **Misspelled env var** — `os.environ/<VAR>` where `<VAR>` is unset or
     misspelled resolves to empty/`None` (verbatim: "Empty env vars are
     silent. `os.environ/UNSET_VAR` → empty string, not an error.").
   - **Hardcoded secret** — a literal key in `config.yaml` instead of
     `os.environ/<VAR>` (per `validate-litellm-config.md` safety note:
     "Flag any hardcoded secret literal (not using `os.environ/`) as FAIL —
     secrets must never appear in `config.yaml`.").
3. Load the relevant provider/routing skill matching the category:
   - provider-prefix or `api_base` defect → the provider skill for the
     affected prefix (e.g. `openai-compatible`, `anthropic`, `openrouter`);
   - fallback/routing defect → the routing skill (`routing/README.md` and
     `docs/litellm/config/router-settings.md`).
4. Identify the root cause: write a one-paragraph explanation of why the
   defect occurred. The root cause must explain the mechanism, not just
   restate the symptom. Cite the corpus file + field that defines the rule
   (e.g. "`provider-fields.index.json` OpenAI-Compatible caveats").
5. Apply `constraint-litellm-config-schema`: confirm the offending key's
   existence and `section` placement against
   `docs/litellm/schemas/config-yaml.option-index.json`. Diagnosis only; do
   not begin fixing.

## Docs to consult

- `docs/litellm/troubleshooting.md` — failure-mode catalog + "Config keys /
  requirements" table.
- `docs/litellm/config/config-validation.md` — validation pitfalls.
- `docs/litellm/schemas/config-yaml.option-index.json` — authoritative key
  existence + `section` + `deprecated` / `requires_db` / `requires_redis`
  flags.
- `docs/litellm/schemas/provider-fields.index.json` — `litellm_prefix`,
  `api_base_behavior`, caveats.
- `docs/litellm/schemas/env-vars.index.json` — env var names,
  `requires_db`, `deprecated`.

## Operational skills to load

- `litellm-config-anatomy` (always, for diagnose).
- The relevant provider/routing skill (conditional by category).

## Constraints to apply

- `constraint-litellm-config-schema` — Confirm the offending key exists in
  `config-yaml.option-index.json` under the correct `section`; diagnosis
  only, do not begin fixing.
- `constraint-litellm-secret-hygiene` — Do not echo secret values while
  inspecting config; redact in handoffs.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 03-fix`, `next_workflow: null`. Record the defect category and
the root-cause paragraph (with corpus citation) in `evidence` (or
`assumptions` if the root cause is provisional). Because the continuation
policy is `suggest-next`, surface the category and root cause for confirmation
before phase `03-fix` is loaded.
