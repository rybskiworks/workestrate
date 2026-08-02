---
source_url: https://docs.litellm.ai/docs/proxy/deploy
canonical_url: https://docs.litellm.ai/docs/proxy/deploy
title: Docker, Helm, Terraform | liteLLM
sidebar_section_path: "LiteLLM AI Gateway (Proxy) > Setup & Deployment > Docker, Helm, Terraform"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Docker, Helm, Terraform | liteLLM

## Headings
- ## Quick Start
- ## Deployment Options
- ## Deploy with Database
- ## Deploy with Redis
- ## Deploy with Database + Redis
- ## (Non Root) - without Internet Connection
- ## Advanced Deployment Settings
- ## Platform-specific Guide
- ## Extras
- ## Deployment FAQ
- ### Verify Docker image signatures
- ### Docker Run
- ### Docker Run - CLI Args
- ### Use litellm as a base image
- ### Build from published LiteLLM packages
- ### Terraform
- ### Kubernetes
- ### Helm Chart
- ### Make LLM API Requests

## Exact config keys found
- `model_list` (section: top-level — in config.yaml examples)
- `model_list[].model_name` (section: model_list)
- `model_list[].litellm_params` (section: model_list)
- `model_list[].litellm_params.model` (section: model_list)
- `model_list[].litellm_params.api_base` (section: model_list)
- `model_list[].litellm_params.api_key` (section: model_list)
- `model_list[].litellm_params.rpm` (section: model_list)
- `router_settings` (section: top-level)
- `router_settings.redis_host` (section: router_settings)
- `router_settings.redis_password` (section: router_settings)
- `router_settings.redis_port` (section: router_settings)
- `general_settings` (section: top-level)
- `general_settings.block_robots` (section: general_settings — Enterprise Feature)

## Exact YAML examples (verbatim — preserve indentation exactly)
```yaml
model_list:
  - model_name: azure-gpt-4o
    litellm_params:
      model: azure/<your-azure-model-deployment>
      api_base: os.environ/AZURE_API_BASE # runs os.getenv("AZURE_API_BASE")
      api_key: os.environ/AZURE_API_KEY # runs os.getenv("AZURE_API_KEY")
      api_version: "2025-01-01-preview"
```
(source: https://docs.litellm.ai/docs/proxy/deploy)

Redis config (verbatim):
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/<your-deployment-name>
      api_base: <your-azure-endpoint>
      api_key: <your-azure-api-key>
      rpm: 6      # Rate limit for this deployment: in requests per minute (rpm)
  - model_name: gpt-4o
    litellm_params:
      model: azure/gpt-4o-ca
      api_base: https://my-endpoint-canada-berri992.openai.azure.com/
      api_key: <your-azure-api-key>
      rpm: 6
router_settings:
  redis_host: <your redis host>
  redis_password: <your redis password>
  redis_port: 1992
```
(source: https://docs.litellm.ai/docs/proxy/deploy)

block_robots (verbatim — Enterprise Feature):
```yaml
general_settings:
  block_robots: true
```
(source: https://docs.litellm.ai/docs/proxy/deploy)

docker-compose.yml (verbatim):
```yaml
version: "3.9"
services:
  litellm:
    build:
      context: .
      args:
        target: runtime
    image: docker.litellm.ai/berriai/litellm:latest
    ports:
      - "4000:4000" # Map the container port to the host, change the host port if necessary
    volumes:
      - ./litellm-config.yaml:/app/config.yaml # Mount the local configuration file
    # You can change the port or number of workers as per your requirements or pass any new supported CLI argument. Make sure the port passed here matches with the container port defined above in `ports` value
    command: [ "--config", "/app/config.yaml", "--port", "4000", "--num_workers", "8" ]
# ...rest of your docker-compose config if any
```
(source: https://docs.litellm.ai/docs/proxy/deploy)

## Exact environment variables
- `LITELLM_MASTER_KEY` — required; must start with `sk-`; proxy admin key
- `LITELLM_SALT_KEY` — cannot be changed after adding a model; encrypts LLM API Key credentials
- `DATABASE_URL` — required when deploying with database (postgresql://user:password@host:port/dbname)
- `AZURE_API_KEY` — Azure API key
- `AZURE_API_BASE` — Azure API base
- `OPENAI_API_KEY` — OpenAI API key
- `LITELLM_LOG` — "DEBUG" / "INFO" / "ERROR"
- `SERVER_ROOT_PATH` — custom server root path (e.g. "/api/v1")
- `KEEPALIVE_TIMEOUT` — keepalive timeout in seconds (default 5)
- `MAX_REQUESTS_BEFORE_RESTART` — restart workers after N requests (default disabled)
- `LITELLM_LOCAL_MODEL_COST_MAP` — "True" to disable pulling live model prices
- `LITELLM_CONFIG_BUCKET_TYPE` — "gcs" for GCS bucket config loading
- `LITELLM_CONFIG_BUCKET_NAME` — bucket name for config loading
- `LITELLM_CONFIG_BUCKET_OBJECT_KEY` — object key for config loading
- `AWS_WEB_IDENTITY_TOKEN` — IAM token for RDS auth
- `AWS_ROLE_NAME` — AWS role ARN for RDS auth
- `AWS_SESSION_NAME` — AWS session name for RDS auth
- `DATABASE_USER` / `DATABASE_PORT` / `DATABASE_HOST` / `DATABASE_NAME` / `DATABASE_SCHEMA` — RDS connection params
- `GRACEFUL_SHUTDOWN_TIMEOUT` — referenced in drain endpoint context

## Exact provider prefixes found
- `azure/` — Azure OpenAI (e.g. azure/<your-azure-model-deployment>, azure/gpt-4o-ca)

## Exact model strings found
- `azure-gpt-4o` — model_name alias
- `gpt-4o` — model_name alias
- `azure/gpt-4o-ca` — Azure deployment

## Exact endpoint paths found (runtime inference API on this page)
- `POST /chat/completions` — tested against http://0.0.0.0:4000/chat/completions
- `POST /v1/chat/completions` — Cloud Run example (https://...a.run.app/v1/chat/completions)
- `GET /health/liveliness` — k8s livenessProbe
- `GET /health/readiness` — k8s readinessProbe
- `GET /robots.txt` — returns "User-agent: * / Disallow: /" when block_robots: true

## Exact CLI commands found
- `docker pull docker.litellm.ai/berriai/litellm:latest` — pull main image
- `uv tool install 'litellm[proxy]'` — install LiteLLM CLI
- `docker run docker.litellm.ai/berriai/litellm:latest --config your_config.yaml` — run with config
- `docker run docker.litellm.ai/berriai/litellm:latest --port 8002 --num_workers 8` — run with port + workers
- `docker run docker.litellm.ai/berriai/litellm:latest --ssl_keyfile_path ssl_test/keyfile.key --ssl_certfile_path ssl_test/certfile.crt` — run with SSL
- `docker run docker.litellm.ai/berriai/litellm:latest --run_hypercorn` — run with Hypercorn (HTTP/2)
- `litellm --config config.yaml --port 4000 --run_granian --num_workers 4` — run with Granian (BETA)
- `docker run docker.litellm.ai/berriai/litellm:latest --keepalive_timeout 75` — set keepalive timeout
- `docker run docker.litellm.ai/berriai/litellm:latest --max_requests_before_restart 10000` — restart workers after N requests
- `litellm --config /path/to/config.yaml --iam_token_db_auth` — IAM-based DB auth
- `cosign verify --key <url> ghcr.io/berriai/litellm:<release-tag>` — verify image signature
- `helm pull oci://docker.litellm.ai/berriai/litellm-helm` — pull Helm chart (OCI)
- `helm install lite-helm ./litellm-helm` — install Helm chart
- `terraform init && terraform plan && terraform apply` — Terraform ECS deploy
- `eksctl create cluster --name=litellm-cluster --region=us-west-2 --node-type=t2.small` — EKS cluster create
- `kubectl create configmap litellm-config --from-file=proxy_config.yaml` — create k8s configmap
- `kubectl apply -f kub.yaml && kubectl apply -f service.yaml` — apply k8s manifests

## Exact API routes (management, on this page)
- not documented on this page (deployment page; management routes are on other pages)

## Requirements
- database: PostgreSQL required for "Deploy with Database" / "Deploy with Database + Redis" paths. "Need a postgres database (e.g. Supabase, Neon, etc)". "Currently, PostgreSQL is our primary supported database for production deployments." MySQL was dropped. YugabyteDB is a Postgres-wire-compatible drop-in replacement.
- redis: required when load balancing across multiple litellm containers. Required at high traffic (1000+ RPS) to prevent database connection exhaustion and deadlocks. Settings: `general_settings.use_redis_transaction_buffer: true`, `litellm_settings.cache: true`, `litellm_settings.cache_params.type: redis`.
- enterprise: `block_robots` (general_settings) is an enterprise-only feature. Enterprise features: SSO/SAML, audit logs, spend tracking, multi-team management, guardrails.
- admin_ui: not directly documented on this page (sidebar references /docs/proxy/ui but not expanded here).
- Production hardware: "Production requires at least 4 CPU cores and 8 GB RAM."

## Deprecations
- none explicitly marked deprecated on this page (Helm Chart and Granian are marked BETA, not deprecated)

## Caveats / pitfalls
- "[BETA] Helm Chart is BETA. If you run into an issues/have feedback please let us know"
- "Beta feature--run_granian is in beta. Uvicorn is still the default server."
- "WARNING: FOR PROD DO NOT USE `--detailed_debug` it slows down response times"
- "To avoid issues with predictability, difficulties in rollback, and inconsistent environments, use versioning or SHA digests (for example, litellm:v1.89.4 or litellm@sha256:...) instead of litellm:latest."
- "By default prisma generate downloads prisma's engine binaries. This might cause errors when running without internet connection." (non-root image)
- "SSL: Both `--ssl_certfile_path` and `--ssl_keyfile_path` are required when enabling TLS with Granian."
- "--max_requests_before_restart (use Gunicorn if you need per-request worker recycling)" — NOT supported with Granian
- "--ciphers (Hypercorn only)" — NOT supported with Granian
- "There are **no limits** on the number of users, keys, or teams you can create on LiteLLM OSS."
- "If you expect high traffic (1000+ requests per second), Redis is required to prevent database connection exhaustion and deadlocks."
- "Defaults to 5 seconds. Between requests, connections must receive new data within this period or be disconnected." (keepalive_timeout)
- "Defaults to disabled when unset." (max_requests_before_restart)
- LITELLM_SALT_KEY: "you cannot change this after adding a model"
- LITELLM_MASTER_KEY: "must start with `sk-`"

## Related links
- https://docs.litellm.ai/docs/proxy/quick_start (CLI Quick Start)
- https://docs.litellm.ai/docs/proxy/cli (CLI Arguments)
- https://docs.litellm.ai/docs/proxy/docker_image_security (Docker Image Security)
- https://docs.litellm.ai/docs/proxy/prod (Best Practices for Production)
- https://docs.litellm.ai/docs/proxy/health (Health Checks)
- https://docs.litellm.ai/docs/proxy/microservices_helm (Microservices Helm)
- https://github.com/BerriAI/litellm-ecs-deployment (Terraform ECS)
- https://litellm-api.up.railway.app/ (Swagger)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
This is the production deployment guide. Relevant to workestrate:
- Docker image: `docker.litellm.ai/berriai/litellm:latest` (main) or `docker.litellm.ai/berriai/litellm-database:latest` (with DB). The real config is in-memory (no Postgres), so the main image suffices.
- `LITELLM_MASTER_KEY` must start with `sk-` — used as master_key in real config (via os.environ/).
- `LITELLM_SALT_KEY` encrypts LLM API key credentials — cannot be changed after adding a model. Not currently used in real config (in-memory, no DB).
- Redis is NOT required for single-instance / low-traffic deployments. The real config has no redis_* settings — consistent with single-instance in-memory deployment.
- Redis IS required at 1000+ RPS or multi-instance. If workestrate scales, add `router_settings.redis_host/password/port`.
- `--drop_params` CLI flag corresponds to `litellm_settings.drop_params: True` (used in real config).
- `LITELLM_LOCAL_MODEL_COST_MAP="True"` disables pulling live model prices — relevant for air-gapped/offline deployments.
- Health endpoints: `GET /health/liveliness` and `GET /health/readiness` — for k8s probes.
- `block_robots` (general_settings) is enterprise-only — not used in real config.
- Production minimum: 4 CPU cores + 8 GB RAM.

## Confidence / uncertainty notes
- HTTP status 200, canonical URL confirmed, no redirect (high confidence).
- All 53 code blocks captured verbatim. Some blocks contain source typos (e.g. `"<object_key>>` double-`>`) — preserved verbatim.
- Docker image names/tags are verbatim from the page.
- The page is large; the searcher captured all sections including Deployment FAQ.
