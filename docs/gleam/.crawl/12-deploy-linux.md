# Crawl: deployment/linux-server/
- seed_url: https://gleam.run/deployment/linux-server/
- canonical_url: https://gleam.run/deployment/linux-server/
- family: Gleam official docs
- fetch: 200
- gleam_version: v1.12.0 (referenced in Dockerfile `ARG GLEAM_VERSION=v1.12.0`); Erlang `ARG ERLANG_VERSION=28.0.2.0`
- feeds_docs: deployment-and-runtime.md

## Purpose
Step-by-step guide for deploying a Gleam backend web application (Erlang target) to a single Linux server. The app runs under systemd inside a Linux container (Podman), with Caddy as the HTTPS reverse proxy. Targets Ubuntu LTS (other distros possible with adjustments).

## Build/release steps
This guide does NOT use rebar3 releases, `relup`, or `sys.config`-style BEAM releases. Instead it uses Gleam's own shipment mechanism:

1. Application listens on `0.0.0.0` (Mist/Wisp: `mist.bind("0.0.0.0")`), note the port (guide uses 8000).
2. Create `healthcheck.sh` (wget spider to `http://127.0.0.1:8000`).
3. Create a multi-stage `Dockerfile`:
   - Stage 1 "gleam": `FROM ghcr.io/gleam-lang/gleam:${GLEAM_VERSION}-scratch` — provides the `gleam` binary only.
   - Stage 2 "build": `FROM erlang:${ERLANG_VERSION}-alpine`; copies gleam binary in, `COPY . /app/`, then `RUN cd /app && gleam export erlang-shipment`.
   - Stage 3 "final": `FROM erlang:${ERLANG_VERSION}-alpine`; creates a `webapp` system user/group; `COPY --from=build /app/build/erlang-shipment /app`; `ENTRYPOINT ["/app/entrypoint.sh"]`; `CMD ["run"]`.
4. Build on CI (GitHub Actions) on git tags matching `v*`; `docker build` then push to `ghcr.io/<org>/<repo>:<tag>`.

Key point: the "release" is `gleam export erlang-shipment` producing `/app/build/erlang-shipment` (containing `entrypoint.sh`). This is Gleam's own shipment format, not an OTP release artifact (no `.rel`/`relup`/boot scripts).

## Running as a service (systemd)
The app is NOT run as a bare systemd service. Instead Podman's systemd generator is used:

- File: `/etc/containers/systemd/webapp.container` (Podman Quadlet-style unit).
- `[Unit] Description=...; After=local-fs.target`
- `[Container] Image=ghcr.io/gleam-lang/example:production; PublishPort=8000:8000; HealthCmd=sh -c /app/healthcheck.sh; HealthInterval=30s; HealthTimeout=5s; HealthRetries=3`
- `[Service] Restart=always; RestartSec=5`
- `[Install] WantedBy=multi-user.target default.target`
- Manage with `systemctl daemon-reload && systemctl start webapp`; status `systemctl status webapp`; logs `journalctl -xeu webapp`.
- Health-check failures cause Podman to terminate the container; systemd `Restart=always` replaces it.
- `AutoUpdate=registry` + `podman-auto-update.service` enables automatic image pulls on new pushes.

## Runtime assumptions (Erlang/OTP)
- Server (or container) must have Erlang/OTP installed. The final container image is `erlang:${ERLANG_VERSION}-alpine` — Erlang/OTP is the runtime, Gleam is only present at build time.
- No separate Erlang install on the host server itself: the host only needs Podman + Caddy. Erlang lives inside the container.
- Minimum 250MB memory recommended; guide uses 1 vCPU / 1GB.
- App must bind `0.0.0.0` (not just localhost) to be reachable via the published port.
- NIFs requiring a C compiler need extra packages in the Dockerfile.

## Env-var config
- Environment variables are injected via the Podman systemd unit using `Environment=KEY=value` syntax in the `[Container]` section of `/etc/containers/systemd/webapp.container`.
- Build-time metadata (`GIT_SHA`, `BUILD_TIME`) is baked into the image via `ARG`/`ENV` in the Dockerfile.
- No mention of `sys.config` / `config.exs` / runtime.exs-style config; env vars are consumed by the Gleam app code directly (the guide does not prescribe how the app reads them).
- Volumes can be mounted with `Volume=/path/on/server:/path/in/container:rw,z`.

## Operational notes + relationship to BEAM releases
- This is a container-first deployment, not a traditional BEAM release deployment. There is no `rebar3 release`, no `relup`, no hot code upgrade, no `sys.config`. The unit of deployment is a container image tag.
- "Release" = `gleam export erlang-shipment` → a directory (`build/erlang-shipment`) containing `entrypoint.sh` and the compiled `.beam` files + runtime bootstrap, runnable on an Erlang/OTP image. This is closer to a "shipped Ebin + start script" than an OTP release.
- Redeploy = push a new git tag → CI builds new image → (optional `AutoUpdate=registry`) Podman pulls and restarts. No in-place upgrade path; each deploy is a full container replacement.
- HTTPS termination is handled by Caddy (automatic TLS), not by the BEAM app. Caddy reverse-proxies `localhost:8000`.
- Security hardening covered: disable SSH password auth, ufw firewall (ssh/http/https only), `unattended-upgrades` for Ubuntu security patches.
- Observability: `systemctl status webapp`, `journalctl -xeu webapp`, Podman health checks.
- Relationship to BEAM release behavior: deliberately abstracted away. Gleam's `erlang-shipment` gives a runnable artifact on an OTP runtime without exposing release/relup mechanics. Operators get systemd-managed container restart semantics (Restart=always) instead of supervisor-only restart semantics; BEAM supervision still applies inside the VM once started.

## Strict rules
- App MUST bind `0.0.0.0` (not localhost) for the published port to reach it.
- SSH password authentication MUST be disabled; SSH keys only.
- Firewall MUST restrict to ssh/http/https.
- Container runs as non-root `webapp` user.
- Health check script exit 0 = pass; any non-zero = fail; consecutive failures mark container unhealthy.
- Replace `example.gleam.run`, port `8000`, image `ghcr.io/gleam-lang/example:production`, and the `IMAGE_ID` in CI with your own values.

## Verbatim quotes
- "This guide will take you through the process of deploying a Gleam backend web application to a single Linux server. The application will be run by systemd in a Linux container, and Caddy will be used to handle HTTPS."
- "Ensure your application is listening on `0.0.0.0`."
- "RUN cd /app && gleam export erlang-shipment"
- "COPY --from=build /app/build/erlang-shipment /app"
- "ENTRYPOINT ["/app/entrypoint.sh"]" / "CMD ["run"]"
- "The `/etc/containers/systemd/webapp.container` file creates a systemd service, so `systemctl` can be used to manage the application container."
- "Environment variables can be added using the `Environment=KEY=value` syntax."
- "If you start the `podman-auto-update.service` systemd service and add the `AutoUpdate=registry` configuration then new versions of the image will automatically be downloaded and deployed after they are pushed to the container registry."
- "The logs can be viewed with `journalctl -xeu webapp`."

## Version notes
- Dockerfile pins: `ARG ERLANG_VERSION=28.0.2.0`, `ARG GLEAM_VERSION=v1.12.0` (as of fetch date 2026-06-25). Edit these `FROM`/`COPY` lines to pick other versions.
- GitHub Actions: `actions/checkout@v4`; builds on `ubuntu-latest`; pushes to `ghcr.io`.
- No version of Caddy/Podman pinned; installed via `apt install --yes podman caddy` on Ubuntu LTS.
- Guide targets Ubuntu LTS (most recent); other distros may need command adjustments.

## Discovered links

### Relevant (crawl later)
(none — this page is terminal for the deployment topic; remaining links are nav, community, or external tooling docs.)

### Skipped
- `/` — site root / nav
- `/case-studies` — marketing
- `/community` — community page
- `/documentation` — docs index (already crawled as 01-documentation.md)
- `/install` — install page (already crawled as 10-install.md)
- `/news` — news
- `/roadmap` — roadmap
- `/sponsor` — sponsor
- `/images/lucy/lucy.svg` — asset
- `/styles/main.css?v=...` — stylesheet
- https://caddyserver.com/ — external tooling (Caddy reverse proxy)
- https://discord.gg/Fm8Pwmy — community
- https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html — external tooling (Podman Quadlet unit reference)
- https://github.com/gleam-lang — GitHub org
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md — CoC
- https://gleam.run/feed.xml — RSS feed
- https://gleamweekly.com/ — community newsletter
- https://packages.gleam.run / https://packages.gleam.run/ — package registry
- https://playground.gleam.run — playground
- https://shop.gleam.run/en-gbp — shop
- https://tour.gleam.run — language tour (already crawled as 02-tour-everything.md)
- https://www.vultr.com/?ref=9694426 — server provider referral
