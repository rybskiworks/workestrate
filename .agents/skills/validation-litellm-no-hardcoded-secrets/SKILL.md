---
name: validation-litellm-no-hardcoded-secrets
description: |
  Verifies a LiteLLM config.yaml contains no hardcoded secret literals: every
  secret-like value uses os.environ/<VAR> indirection. Load after any config
  edit, alongside validation-litellm-config-check. Does NOT cover config
  schema validation, provider-prefix rules, or runtime startup (see
  validation-litellm-config-check, validation-litellm-startup).
metadata:
  org.kind: validation
---

# Validation: LiteLLM No Hardcoded Secrets

This is the secret-hygiene gate. It greps the config for hardcoded secret
literals that should never appear in `config.yaml`. Every secret-like value
must use `os.environ/<VAR>` indirection so that real keys are injected at
runtime, never committed. This complements `validation-litellm-config-check`
check (g), which verifies that `os.environ/<VAR>` references resolve to known
env vars; this gate catches the opposite failure — a literal secret where
indirection should be.

## Triggers

Load this skill when:

- After any edit to `infra/litellm/config.yaml` or a `docs/litellm/examples/`
  config, before claiming completion.
- Alongside `validation-litellm-config-check` as part of the static validation
  suite.
- When reviewing a config diff for secret leakage.

## Command

```bash
# Flag any secret-like literal that is NOT os.environ/<VAR>.
grep -nE 'api_key|master_key|password|database_url|salt_key' <config.yaml> \
  | grep -vE 'os\.environ/' \
  | grep -vE '^\s*#'

# Flag raw OpenAI-style key literals (sk- followed by 16+ word chars).
grep -nE 'sk-[A-Za-z0-9_-]{16,}' <config.yaml>

# Flag Bearer token literals.
grep -nE 'Bearer\s+[A-Za-z0-9._-]{8,}' <config.yaml>

# Flag 32+ char hex tokens that are not os.environ/.
grep -nE '[A-Fa-f0-9]{32,}' <config.yaml> | grep -vE 'os\.environ/'
```

If all four greps return no matches, the gate passes.

## Pass criteria

- Zero hardcoded secret literals found.
- Every `api_key`, `master_key`, `password`, `database_url`, and `salt_key`
  value uses `os.environ/<VAR>` indirection.
- No `sk-<16+>` literals, no `Bearer <value>` literals, and no 32+ char hex
  tokens that are not `os.environ/<VAR>`.

## Fail criteria

- Any secret-like key (`api_key`, `master_key`, `password`, ...) has a literal
  value instead of `os.environ/<VAR>`.
- Any `sk-<16+>` literal, `Bearer <value>` literal, or 32+ char hex token is
  found outside an `os.environ/` reference.

## Evidence to report

- The grep command(s) run and their exit codes.
- Any matching lines (file, line number, content — redact the secret value in
  the report if it must be shown).
- A statement that `master_key` resolves to
  `os.environ/LITELLM_MASTER_KEY` (confirmed or not).

## Notes

- `master_key` MUST resolve to `os.environ/LITELLM_MASTER_KEY` — a hardcoded
  master key is always a FAIL.
- This gate is read-only; it greps but never modifies the config.
- The grep patterns are intentionally broad; review any matches manually — a
  32+ char hex string inside a URL path (not a secret) is a known false
  positive source. Use judgement, but never dismiss a real `sk-` or `Bearer`
  literal.
- This gate complements `validation-litellm-config-check` check (g): (g)
  verifies `os.environ/<VAR>` references resolve to known env vars; this gate
  verifies no literal secrets bypass that indirection. Run both.
