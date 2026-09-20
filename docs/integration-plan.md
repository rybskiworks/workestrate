# workestrate integration plan

## Current state

- **LiteLLM**: Running on KVM, accepts requests, routes to providers. The primary `chat` model (OpenRouter/gpt-4o) falls back to MiniMax-M3 — likely an API key or model name issue.
- **Pi**: TypeScript monorepo (Node 22, 4 workspace packages). Cloned at `agents/pi`. Needs `pi-provider-litellm` extension to connect to LiteLLM. Currently uses `node:24-bookworm-slim` base image.
- **Odysseus**: Python app (FastAPI + SQLAlchemy + chromadb + many deps). Cloned at `agents/odysseus` (on `dev` branch). agentctl already generates `settings.json` pointing at LiteLLM. Currently uses `python:3.12-slim` base image.

## LiteLLM configuration

### Issues identified

1. **OpenRouter primary not responding**: The `chat` role falls back to `chat-fallback` (MiniMax-M3). Check:
   - Is `OPENROUTER_API_KEY` valid and non-empty in `.env.enc`?
   - Is `openrouter/openai/gpt-4o` the correct model name? (OpenRouter's naming may have changed)
   - Test directly: `curl https://openrouter.ai/api/v1/models` with the key

2. **No timeout configuration**: Default `request_timeout` is 6000 seconds (100 min). In a Microsandbox with limited egress, a hung upstream blocks the sandbox for too long.

3. **No explicit retry policy**: Default is 3 retries. No per-error-type tuning.

4. **No cooldown configuration**: A failing upstream is retried without explicit cooldown.

5. **Spend/error logs write nowhere**: No DB, so `disable_spend_logs` should be explicit.

### Recommended config changes

```yaml
litellm_settings:
  drop_params: true
  request_timeout: 300
  force_ipv4: true

router_settings:
  fallbacks:
    - chat: [chat-fallback]
    - coding: [coding-fallback]
    - reasoning: [reasoning-fallback]
  num_retries: 2
  timeout: 300
  stream_timeout: 300
  allowed_fails: 3
  cooldown_time: 60
  retry_policy:
    TimeoutErrorRetries: 2
    RateLimitErrorRetries: 3
    InternalServerErrorRetries: 2

general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  completion_model: chat
  disable_spend_logs: true
```

### Role naming

Consider renaming roles from generic (`chat`, `coding`, `reasoning`) to occupation-based names that better reflect what each role does in the workestrate context. For example:
- `chat` -> something describing its actual use
- `coding` -> the coding agent role
- `reasoning` -> the reasoning/planning role

This is a naming decision — the underlying routing stays the same. Clients (Pi, Odysseus) would need to request the new names.

### Config management

- `include` directive available for splitting config by provider when it grows
- Hot reload: appears to require proxy restart (not clearly documented)
- Config lives at `infra/litellm/config.yaml`, mounted read-only into the sandbox

### Caching

In-memory caching works without DB:
```yaml
litellm_settings:
  cache: true
  cache_params:
    type: local
    ttl: 600
```
Low priority for M1 — evaluate after basic integration works.

### Skills gateway

The Skills Gateway is a central registry for Claude Code skills/plugins. API endpoints:
- `POST /claude-code/plugins` — register a skill
- `GET /public/skill_hub` — list public skills

**Requires DB for persistence** — skills registered at runtime are lost on restart. Skip for M1. Revisit when Postgres is added (M4).

### Security assessment

| Feature | Works without DB | Notes |
|---|---|---|
| TLS in-transit | Yes | On by default (httpx/aiohttp) |
| Master key auth | Yes | Single token for all requests |
| IP allowlist | Yes | `general_settings.allowed_ips` |
| SSRF protection | Yes | `litellm_settings.user_url_validation` (default true) |
| At-rest encryption | No | Requires DB (NaCl SecretBox) |
| Virtual keys | No | Requires DB |
| RBAC | No | Requires DB |
| Spend tracking | No | Requires DB |

For M1: master key + network policy (Microsandbox default-deny) is the security model. This is adequate for a single-host local setup.

## Pi integration

### pi-provider-litellm

`pi-provider-litellm` (by balcsida) is a Pi extension that connects Pi to a self-hosted LiteLLM proxy.

**Installation**: `pi install npm:pi-provider-litellm` or clone into Pi's extension directory.

**Configuration**:
- Environment variables: `LITELLM_BASE_URL` and `LITELLM_API_KEY`
- Or interactive: `/login litellm` in Pi's TUI
- Provider discovers models via LiteLLM's `/v1/models` endpoint

**Dependencies**: Node.js >= 22, peer deps on `@earendil-works/pi-ai` and `@earendil-works/pi-coding-agent`.

### Required changes in agentctl

1. **Add pi-provider-litellm to the Pi sandbox plan**: Either install it at image build time or mount it into the sandbox.

2. **Pi gets the provider URL + key from the seeded `models.json`, NOT env vars**: Pi does NOT need `LITELLM_BASE_URL`/`LITELLM_API_KEY` env vars. The provider URL and key come from the seeded `models.json` (`[[seed_files]]` with `template = true`): `depends_on.litellm` injects `LITELLM_ADDR` (guest-visible `host.microsandbox.internal:4000`), which the seed template renders into the `baseUrl` (`http://host.microsandbox.internal:4000/v1`); the seed's `apiKey` token `$${LITELLM_MASTER_KEY}` renders to the literal `${LITELLM_MASTER_KEY}`, which pi expands to the `$MSB_LITELLM_MASTER_KEY` placeholder, substituted by the host egress proxy.

3. **Seed `models.json` via `seed_files template = true`** (authoritative delivery path): the pi workload's `[[seed_files]]` renders `models.json` at seed time (rendered `baseUrl` + `$${…}`-escaped `apiKey`; acceptance is B13.2 in `docs/validation-and-improvements/05-host-validation.md`). No env vars are involved.

4. **Build and link Pi**: The Pi monorepo needs to be built (`npm run build`) before it can run. Either build in the Nix image or pre-build and copy the artifacts.

### Pi tech stack notes

- Node.js 22 (CI uses Node 24)
- TypeScript 5.9, ESM
- 4 workspaces: agent, ai, coding-agent, tui
- Build order: tui -> ai -> agent -> coding-agent
- The `pi` command runs the coding-agent package in RPC mode
- Biome for linting, esbuild for bundling

## Odysseus integration

> The full companion-services (chromadb/searxng/ntfy) gap analysis (docs/odysseus-full-capability.md) moved to the user's personal fleet, as it is personal-workload content rather than generic tooling.

### Current state

agentctl already generates `data/settings.json` at sandbox startup:
```json
{
  "providers": {
    "litellm": {
      "base_url": "http://host.microsandbox.internal:4000/v1",
      "api_key": "${LITELLM_MASTER_KEY}",
      "model": "chat"
    }
  }
}
```

### Required changes

1. **Verify settings.json is sufficient**: Odysseus may need additional config fields or a different format. Check Odysseus's source for how it reads provider settings.

2. **Verify dependencies**: Odysseus has ~30 Python dependencies (FastAPI, SQLAlchemy, chromadb-client, fastembed, etc.). The `python:3.12-slim` base image needs all of these installed. Consider:
   - Pre-built Nix image with all deps
   - Or a `requirements.txt` install step at container start
   - `fastembed` pulls ONNX models — may need special handling for offline/restricted-network environments

   > **Update (vendor strategy):** Odysseus Python deps are now vendored into `agents/odysseus/build/.deps` by the `nix develop` shell using Nix's `python3.12` interpreter (`pip install --target`). The `python:3.12-slim` microVM imports them via `PYTHONPATH=/app/.deps` and runs `python -m uvicorn app:app --host 0.0.0.0 --port 7000` (not bare `uvicorn`, whose console-script shebang is absent in the slim image). An optional lock step — `cd agents/odysseus/repo && python3.12 -m piptools compile requirements.txt -o requirements.lock` — produces a pinned `requirements.lock` that the build prefers when present.

3. **Database**: Odysseus uses SQLite (`sqlite:///app/data/app.db`). Ensure the data directory is writable and persistent.

### Odysseus tech stack notes

- Python 3.12, FastAPI + Uvicorn
- ~30 dependencies including heavy ones (numpy, chromadb-client, fastembed)
- RAG/embeddings via local ONNX models (fastembed)
- Calendar integration (caldav, icalendar)
- MCP server support
- On `dev` branch (not main)

## Nix Docker images

### Approach

Replace Docker Hub base images (`node:24-bookworm-slim`, `python:3.12-slim`) with Nix-built OCI images for full reproducibility.

**Key tools** (from `pkgs.dockerTools`):
- `buildLayeredImage` — multi-layer image with Nix store paths (best caching)
- `streamLayeredImage` — stream to stdout for piping (avoids store bloat)
- `pullImage` — fetch and pin existing Docker image by digest (migration path)

### Pi image strategy

1. Start with `pkgs.nodejs_22` (or `nodejs_24`) in a minimal Nix image
2. Pre-build Pi from source: `npm run build` in the Nix derivation
3. Install `pi-provider-litellm` as part of the build
4. Set `config.Cmd = [ "pi" "--mode" "rpc" ]`
5. Pin nixpkgs commit in `flake.nix`

Example structure:
```nix
pkgs.dockerTools.buildLayeredImage {
  name = "workestrate-pi";
  contents = [ pkgs.nodejs_22 pi-built pi-provider-litellm ];
  config = {
    Cmd = [ "pi" "--mode" "rpc" ];
    WorkingDir = "/app";
  };
}
```

### Odysseus image strategy

1. Start with `pkgs.python312` in a minimal Nix image
2. Install all dependencies via `python312Packages` or a `requirements.txt` derivation
3. Handle `fastembed` (ONNX models) — may need pre-download or special setup
4. Set `config.Cmd = [ "uvicorn" "app:app" "--host" "0.0.0.0" "--port" "7000" ]`

Challenge: Odysseus has ~30 deps, some not in nixpkgs. Options:
- Use `python312Packages.buildPythonPackage` for missing ones
- Use `pip` inside a Nix derivation (less reproducible)
- Use `poetry2nix` if Odysseus migrates to Poetry

### Microsandbox compatibility

Nix-built images produce standard OCI tarballs. Microsandbox can consume them via:
```bash
nix-build pi-image.nix | docker load
# or import directly if Microsandbox supports OCI tarball import
```

The image must be available in Microsandbox's image store before the sandbox can use it.

### Migration path

1. Phase 1: Keep using Docker Hub images, verify everything works
2. Phase 2: Build Nix images, test locally
3. Phase 3: Switch agentctl plans to use Nix-built image names
4. Phase 4: Pin everything in flake.nix

## Prioritized action items

### P0: Fix LiteLLM primary model
- Debug why OpenRouter falls back to MiniMax
- Check API key validity, model name correctness
- Test with a direct curl to OpenRouter

### P1: LiteLLM config hardening
- Add timeouts, retry policy, cooldown settings
- Disable spend logs
- Consider renaming roles

### P2: Pi integration
- Install pi-provider-litellm in the Pi sandbox
- Configure LITELLM_BASE_URL and LITELLM_API_KEY
- Verify Pi can reach LiteLLM and list models
- Test a chat completion through Pi

### P3: Odysseus integration
- Verify settings.json format is correct for Odysseus
- Ensure all Python deps are available
- Test Odysseus can reach LiteLLM
- Test a chat completion through Odysseus

### P4: Nix images
- Build Pi Nix image
- Build Odysseus Nix image
- Test with Microsandbox
- Switch agentctl plans to Nix image names

### P5: Advanced LiteLLM features
- In-memory caching
- Config splitting with `include`
- Skills gateway (requires DB — defer to M4)
- Virtual keys (requires DB — defer to M4)
- At-rest encryption (requires DB — defer to M4)

## Open questions

1. **OpenRouter model name**: Is `openrouter/openai/gpt-4o` still the correct identifier? Has OpenRouter changed their naming?
2. **Pi build reproducibility**: Can `npm run build` run in a Nix sandbox? Does it need network access?
3. **fastembed ONNX models**: Do they need to be pre-downloaded for offline Microsandbox use?
4. **Odysseus branch**: It's on `dev` — should we track `main` or `dev`?
5. **Image store**: How does Microsandbox discover/load custom OCI images? Is `docker load` sufficient?
6. **Config hot reload**: Does LiteLLM pick up config changes without restart?
