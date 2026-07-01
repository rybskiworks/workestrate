# LiteLLM Troubleshooting

> Synthesized from on-disk corpus under `docs/litellm/extracted/` and
> `docs/litellm/providers/`. No web fetches were performed. Every fact traces
> to a cited corpus file or is marked **(inferred)** /
> **"not documented in fetched source"**.

## Sources

- https://docs.litellm.ai/docs/providers/anthropic — `extracted/p1-provider-anthropic.md`, `providers/anthropic.md`
- https://docs.litellm.ai/docs/providers/openai_compatible — `extracted/p1-provider-openai_compatible.md`, `providers/openai-compatible.md`
- https://docs.litellm.ai/docs/providers/openrouter — `extracted/p1-provider-openrouter.md`, `providers/openrouter.md`
- https://docs.litellm.ai/docs/providers/minimax — `extracted/p1-provider-minimax.md`, `providers/minimax.md`
- https://docs.litellm.ai/docs/providers/moonshot — `extracted/p1-provider-moonshot.md`, `providers/moonshot.md`
- https://docs.litellm.ai/docs/proxy/config_settings — `extracted/p0-config_settings.md`
- https://docs.litellm.ai/docs/proxy/docker_quick_start — `extracted/p0-docker_quick_start.md`
- https://docs.litellm.ai/docs/proxy/deploy — `extracted/p0-deploy.md`
- Real config: `infra/litellm/config.yaml`

## What this area controls

Diagnosis recipes for the most common LiteLLM failure modes in the
workestrator in-memory deployment: provider URL/prefix mismatches, timeout
defaults, retry/fallback not firing, DB-required features failing without a
DB, env-var resolution, port conflicts, and debug logging. Maps to the
existing `debug-litellm-proxy` skill and a general troubleshooting workflow.

## Verified behavior (common issues)

### 1. OpenAI-compatible "Not Found Error"

Verbatim (from `p1-provider-openai_compatible.md`):

> "If you see Not Found Error when testing make sure your api_base has the
> /v1 postfix. Example: `http://vllm-endpoint.xyz/v1`."

> "Do NOT add anything additional to the base url e.g. `/v1/embedding`.
> LiteLLM uses the openai-client to make these calls, and that automatically
> adds the relevant endpoints."

**Real config:** `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1`
— **correct** (includes `/v1`, no appended endpoint path).

**Fix:** ensure `api_base` ends with `/v1` and contains no endpoint suffix.

### 2. `anthropic/` auto-appends `/v1/messages`

Verbatim (from `p1-provider-anthropic.md`):

> "When using a custom API base for Anthropic... LiteLLM automatically
> appends the appropriate suffix (`/v1/messages` or `/v1/complete`) to your
> base URL."

- Without `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`:
  `https://my-proxy.com` → `https://my-proxy.com/v1/messages`.
- With `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`: unchanged.

**Real config resolutions:**

| `model` | `api_base` | Final URL |
| --- | --- | --- |
| `anthropic/kimi-for-coding` | `https://api.kimi.com/coding` | `https://api.kimi.com/coding/v1/messages` |
| `anthropic/MiniMax-M3` | `https://api.minimax.io/anthropic` | `https://api.minimax.io/anthropic/v1/messages` |

The MiniMax final URL matches the MiniMax docs (`https://api.minimax.io/anthropic/v1/messages`).

**Fix:** if Anthropic-prefix calls 404, check whether the suffix was
auto-appended; set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` only if the
upstream already includes `/v1/messages` in `api_base`.

### 3. Provider prefix mismatches

- `minimax/` prefix is the documented pattern for MiniMax
  (`api_base https://api.minimax.io/anthropic/v1/messages`). The repo uses
  `anthropic/MiniMax-M3` — **DEVIATION (flagged)**. It works because the
  MiniMax endpoint speaks the Anthropic Messages API, but it diverges from
  the documented `minimax/` prefix.
- `moonshot/` prefix is for Moonshot's OpenAI-compatible API. The repo uses
  `anthropic/kimi-for-coding` for the Kimi coding endpoint — a different
  integration (Anthropic Messages protocol), not the Moonshot OpenAI-
  compatible API.
- `zai/` vs `openrouter/z-ai/` (hyphenated) — distinct paths. The repo uses
  `openrouter/z-ai/glm-5.1` (OpenRouter nested format), not `zai/` direct.

**Fix:** if a provider call fails with auth/404, verify the prefix matches
the endpoint's protocol. See [providers/README.md](providers/README.md).

### 4. `request_timeout` default 6000s too long

Verbatim (from `p0-config_settings.md`):

> "If not set, the default value is 6000 seconds. [For reference OpenAI
> Python SDK defaults to 600 seconds.]"

**Real config:** sets `request_timeout: 300` (and `timeout: 300` /
`stream_timeout: 300` in `router_settings`).

**Fix:** always set `request_timeout` explicitly; 6000s lets hung requests
block a worker for 100 minutes.

### 5. Retry / fallback not firing

Verbatim (from `p1-routing-fallback_management.md`):

> "Fallbacks are triggered after the configured number of retries fails."

Checklist:

- `num_retries` (real config: `2`; default `3`).
- `allowed_fails` (real config: `3`) — deployment must fail this many times
  before cooldown.
- `cooldown_time` (real config: `60`; default `30`) — seconds a deployment
  is cooled down after `allowed_fails` failures.
- `max_fallbacks` (default `5`) — chains longer than 5 are truncated.
- `retry_policy` keys: `TimeoutErrorRetries`, `RateLimitErrorRetries`,
  `InternalServerErrorRetries`, `AuthenticationErrorRetries`,
  `ContentPolicyViolationErrorRetries`. The real config sets only the first
  three.
- **400/BadRequestErrors are not counted** toward region-outage / fail
  counters; `BadRequestErrorAllowedFails` default is high in
  `allowed_fails_policy`. A steady stream of 400s will not trip fallbacks.

**Fix:** confirm the failure type is one `retry_policy` covers; raise
`allowed_fails` / `num_retries` if legitimate transient errors are
under-failing.

### 6. DB-required features failing in in-memory mode

Without Postgres, these fail or no-op:

- `/key/generate`, `/key/info`, `/key/block`, `/key/unblock`, `/key/update`
- `/user/new`, `/user/info`, `/team/new`, `/team/info`, `/team/update`,
  `/team/member_add`
- `/budget/new` (Enterprise + DB)
- `/daily_metrics` (inferred — spend tracking)
- Dynamic fallback management (`POST /fallback`, `GET /fallback/{model}`,
  `DELETE /fallback/{model}`) — require `STORE_MODEL_IN_DB=True`.
- Admin UI — verbatim "Requires db connected".
- `lite models add`, `lite credentials create`, `lite keys generate`,
  `lite users create` (DB-backed CLI commands).

**Fix:**

- Set `general_settings.disable_spend_logs: true` (real config does) —
  verbatim "turn off writing each transaction to the db".
- Set `general_settings.disable_error_logs: true` — disables error log DB
  writes (independent of `disable_spend_logs`).
- Do **NOT** set `STORE_MODEL_IN_DB` (leave False) — models are static in
  `config.yaml`.
- Set `DISABLE_ADMIN_UI="True"`.
- Use `lite chat completions` / `lite http request` for inference testing
  (these work in-memory).

### 7. Env var not resolving

`os.environ/VAR_NAME` syntax runs `os.getenv("VAR_NAME")` at load time
(verbatim from `p0-docker_quick_start.md`). The env var must be set in the
proxy process environment **before startup**. If empty, `api_key` /
`master_key` resolves to an empty string.

**Real config env vars:** `LITELLM_MASTER_KEY`, `KIMI_CODE_API_KEY`,
`MINIMAX_CODING_API_KEY`, `OPENROUTER_API_KEY`, `NEURALWATT_API_KEY`.

**Fix:** verify each is set in the sandbox `env()`/`secret_env()` before
the proxy starts; an empty `master_key` silently breaks auth, an empty
provider key yields upstream 401s.

### 8. Port 4000 conflicts

Default proxy port is 4000 (docker-compose `ports "4000:4000"`).

**Fix:** use `--port <n>` to change, or remap the sandbox port.

### 9. `--debug` / `--detailed_debug` / `LITELLM_LOG`

Verbatim (from `p0-deploy.md`):

> "Use `--debug` or `--detailed_debug` CLI flags, or set `LITELLM_LOG` env
> var to `INFO`, `DEBUG`, or `ERROR`."

> "WARNING: FOR PROD DO NOT USE `--detailed_debug` it slows down response
> times".

`JSON_LOGS="True"` for JSON logs. `set_verbose` is **DEPRECATED** → use
`LITELLM_LOG`.

**Fix:** use `LITELLM_LOG=INFO` (or `--debug`) in prod; reserve
`--detailed_debug` for reproduction only.

### 10. SSL / self-signed certs

- `litellm_settings.ssl_verify: false` for self-signed certs.
- `--ssl_keyfile_path` + `--ssl_certfile_path` for TLS termination.

### 11. `prometheus.yml` mount error

Verbatim: "must exist as a file before `docker compose up`. If it is
missing, Docker auto-creates it as an empty directory and the Prometheus
container fails to start."

(Same pitfall applies to `config.yaml` mounts — see
[deployment-ops/README.md](deployment-ops/README.md).)

## Config keys / requirements

| Symptom | Key / env to check | Source |
| --- | --- | --- |
| Not Found Error (OpenAI-compat) | `api_base` ends with `/v1`, no endpoint suffix | `p1-provider-openai_compatible.md` |
| Anthropic 404 | `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` | `p1-provider-anthropic.md` |
| Hung requests | `request_timeout` (default 6000), `timeout`, `stream_timeout` | `p0-config_settings.md` |
| Fallback not firing | `num_retries`, `allowed_fails`, `cooldown_time`, `retry_policy`, `max_fallbacks` | `p1-routing-fallback_management.md` |
| DB write errors | `disable_spend_logs: true`, `disable_error_logs: true` | `p0-config_settings.md` |
| Dynamic model mgmt failing | `STORE_MODEL_IN_DB` (leave False) | `p2-deploy-db_info.md` |
| Admin UI broken | `DISABLE_ADMIN_UI="True"` | `p2-admin-ui.md` |
| Empty key / 401 | `os.environ/VAR` resolved at load time | `p0-docker_quick_start.md` |
| Port conflict | `--port` | `p0-deploy.md` |
| Verbose prod logs | `LITELLM_LOG` (not `--detailed_debug`) | `p0-deploy.md` |
| Self-signed cert | `ssl_verify: false` | `p0-config_settings.md` |
| Config mount dir | file must exist before `docker compose up` | `p0-deploy.md` |

## Features that REQUIRE DB/Redis

See [auth-access-budget/README.md](auth-access-budget/README.md) and
[routing/README.md](routing/README.md) for the full DB/Redis-gated feature
lists. In-memory, the recurring failure mode is DB-backed endpoints being
called by tooling that assumes a full deployment.

## Pitfalls

- **`--detailed_debug` in prod slows responses** (verbatim). Use
  `LITELLM_LOG=INFO`.
- **`set_verbose` is deprecated.** Migrate to `LITELLM_LOG`.
- **Empty env vars are silent.** `os.environ/UNSET_VAR` → empty string, not
  an error. Auth and provider keys fail opaquely.
- **400s don't trip fallbacks.** A bad-request loop will not engage
  fallbacks or outage alerts.
- **`STORE_MODEL_IN_DB=True` without a DB breaks model loading.** Leave it
  False in-memory.
- **Config-as-directory mount.** A missing `config.yaml` becomes an empty
  directory and the proxy fails to start.
- **Anthropic suffix auto-append is on by default.** Only disable it when
  `api_base` already includes `/v1/messages`.

## Related schema / workflow

- [`providers/README.md`](providers/README.md) — per-provider prefix/protocol matrix.
- [`endpoints/README.md`](endpoints/README.md) — which endpoints are DB-gated.
- [`routing/README.md`](routing/README.md) — fallback/retry tuning.
- [`auth-access-budget/README.md`](auth-access-budget/README.md) — DB-gated auth/budget features.
- [`deployment-ops/README.md`](deployment-ops/README.md) — health probes, env loading, image choice.
- [`schemas/env-vars.index.json`](schemas/env-vars.index.json) — `LITELLM_LOG`, `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`, `JSON_LOGS`.

## Troubleshooting workflow

General debugging approach (project convention — not upstream):

1. **Reproduce.** Capture the exact request (method, path, `model` field,
   headers minus secret), response status, and full error body. Use
   `lite http request` or `curl` against `:4000`.
2. **Check logs.** Set `LITELLM_LOG=INFO` (not `--detailed_debug` in prod)
   and `JSON_LOGS="True"`. Look for the failing `model_name` and the
   resolved `litellm_params.model` / `api_base`.
3. **Isolate the layer:**
   - **Provider** — does the same call work directly against the upstream
     (`api.kimi.com/coding/v1/messages`, `api.minimax.io/anthropic/v1/messages`,
     `openrouter.ai`, `api.neuralwatt.com/v1`)?
   - **Router** — is the `model` field a configured `model_name`? Is a
     fallback expected but not firing (check `num_retries` /
     `allowed_fails` / `retry_policy`)?
   - **Config** — is `api_base` correct (`/v1` for OpenAI-compat, suffix
     auto-append for `anthropic/`)? Is the env var resolved (non-empty)?
4. **Apply the targeted fix** from the table above.
5. **Verify** with the reproducer, then relax verbosity back to `INFO`.

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

- The workestrator is in-memory: most "feature X is broken" reports are
  actually DB-gated features being invoked. First check
  [auth-access-budget/README.md](auth-access-budget/README.md) → "Features
  that REQUIRE DB/Redis".
- Skill: **`workflow-litellm-debugging`** — now exists at
  `.agents/skills/workflow-litellm-debugging-00-orchestration/` (6-phase
  debugging workflow). It automates steps 1–3 of the workflow above against
  the microsandbox at `:4000`.
- Egress is default-deny (DNS + tcp/443 to `openrouter.ai`,
  `api.kimi.com`, `api.minimax.io`, `api.neuralwatt.com` only). A
  "connection refused" / "DNS resolution failed" error often means the
  upstream host is not in the egress allowlist, not a LiteLLM config bug.
- The `neural` tier has **no fallback** — a Neuralwatt outage yields
  unrecoverable 5xx for that tier. The four `coding*` tiers each have one
  fallback sibling.
- Reproduction tip: `lite http request --model coding --message "ping"`
  exercises the full router → provider path without a DB.
