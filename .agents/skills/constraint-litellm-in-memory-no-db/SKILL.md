---
name: constraint-litellm-in-memory-no-db
description: |
  Enforces that an in-memory LiteLLM deployment (no Postgres, no Redis) uses no
  config key or env var that requires a database or Redis. Load when authoring or
  reviewing a config.yaml or env for an in-memory / DB-less LiteLLM proxy
  deployment. Does NOT cover schema validity of keys (see
  constraint-litellm-config-schema) or secret hygiene (see
  constraint-litellm-secret-hygiene).
metadata:
  org.kind: constraint
---

# Constraint: In-Memory Deployment Has No DB/Redis Dependencies

This constraint enforces that a LiteLLM proxy deployed in-memory (without
PostgreSQL or Redis) does not reference any config option flagged
`requires_db: true` or `requires_redis: true`, and does not set the
`DATABASE_URL`, `STORE_MODEL_IN_DB`, or any `REDIS_*` environment variable. The
sole intentional compensating control is `general_settings.disable_spend_logs:
true`, which prevents the proxy from attempting spend-log DB writes against a
non-existent database.

## Triggers

Load this skill when:

- Authoring or reviewing a config.yaml for an in-memory / DB-less deployment.
- Reviewing the proxy environment (env vars) for an in-memory deployment.
- Triaging DB connection errors at proxy startup in a deployment with no DB.

## Rules

1. No config key with `requires_db: true` (per
   `config-yaml.option-index.json`) may be set, EXCEPT
   `general_settings.disable_spend_logs: true`, which is the documented
   compensating control that disables spend-log DB writes (verbatim from
   `extracted/p2-deploy-db_info.md`: "set to `True` to disable writing spend logs
   to DB").
2. No config key with `requires_redis: true` (per
   `config-yaml.option-index.json`) may be set.
3. The env var `DATABASE_URL` MUST NOT be set (verbatim from
   `env-vars.index.json`: "Required for virtual keys, spend tracking, teams,
   users, Admin UI" — all unavailable without a DB).
4. The env var `STORE_MODEL_IN_DB` MUST NOT be set / MUST remain false (verbatim
   from `extracted/p2-deploy-db_info.md`: models are defined statically in
   `model_list`).
5. No `REDIS_*` env var (`REDIS_URL`, `REDIS_HOST`, `REDIS_PORT`,
   `REDIS_PASSWORD`, `REDIS_USERNAME`, `REDIS_SSL`, `REDIS_CLUSTER_NODES`,
   `REDIS_SENTINEL_NODES`, `REDIS_SERVICE_NAME`, `REDIS_CONNECTION_POOL_KWARGS`)
   may be set.
6. `general_settings.disable_spend_logs: true` SHOULD be set in an in-memory
   deployment to suppress DB write attempts (high-confidence recommendation from
   `extracted/p2-deploy-db_info.md`).

## References

- Schema: `docs/litellm/schemas/config-yaml.option-index.json`
  (`options[].requires_db`, `options[].requires_redis` flags).
- Schema: `docs/litellm/schemas/env-vars.index.json` (`DATABASE_URL`,
  `STORE_MODEL_IN_DB`, `REDIS_*` entries with `requires_db` flags).
- Docs: `docs/litellm/extracted/p2-deploy-db_info.md` (verbatim
  `disable_spend_logs` / `disable_error_logs` YAML and DB-requirement quotes).

## Out of scope

- Whether a key is schema-valid — see `constraint-litellm-config-schema`.
- Whether spend logging integrations (Prometheus, S3, Langfuse) are configured —
  those work without a DB via callbacks.
- Admin UI disablement (`DISABLE_ADMIN_UI`) — related hardening, not a DB
  dependency of the config keys themselves.

## Violation examples

### DB-requiring key in in-memory config

```yaml
general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  # disable_spend_logs: true   # FORBIDDEN (omitted): no compensating control for spend-log DB writes
  database_url: postgresql://user:pass@host/db   # FORBIDDEN: requires_db, no DB exists
```

### Redis-requiring key in in-memory config

```yaml
litellm_settings:
  cache: true          # FORBIDDEN: caching requires_redis (no Redis in-memory)
  cache_params:
    type: redis        # FORBIDDEN: requires_redis
```

### DB env var set in in-memory deployment

```env
DATABASE_URL=postgresql://user:pass@host/db   # FORBIDDEN: in-memory has no Postgres
STORE_MODEL_IN_DB=True                        # FORBIDDEN: models are static in model_list
REDIS_URL=redis://localhost:6379              # FORBIDDEN: in-memory has no Redis
```

## How to check

Run `validation-litellm-config-check` (check e: no `requires_db: true` key
except `disable_spend_logs: true`; check f: no `requires_redis: true` key and no
`DATABASE_URL` / `STORE_MODEL_IN_DB` / `REDIS_*` env vars). The check cross-
references the option index flags against the config and env.
