# Deployment and Runtime

Purpose

Synthesise Gleam's official deployment and runtime guidance. This doc covers how Gleam compiles, what it runs on, and the two documented container-first deployment paths: a single Linux server with Podman/systemd/Caddy, and Fly.io. It explicitly contrasts Gleam's `erlang-shipment` format with traditional BEAM OTP releases.

Sources used

- `/home/node/Development/ai-workbench/docs/gleam/.crawl/12-deploy-linux.md` — <https://gleam.run/deployment/linux-server/> (Gleam v1.12.0, Erlang/OTP 28.0.2.0)
- `/home/node/Development/ai-workbench/docs/gleam/.crawl/13-deploy-fly.md` — <https://gleam.run/deployment/fly/> (Gleam v1.8.0, Erlang/OTP 27.1.1.0)
- `/home/node/Development/ai-workbench/docs/gleam/.crawl/11-faq.md` — <https://gleam.run/frequently-asked-questions/> (versionless)

Related BEAM guidance

- `../beam/releases.md` — traditional OTP releases (`rebar3 release`, `relup`, `sys.config`, boot scripts). Gleam's `erlang-shipment` deliberately does not use these mechanics.
- `../beam/runtime-environment.md` — BEAM runtime environment / VM assumptions. The Erlang/OTP runtime is still present inside the container and BEAM supervision still applies once the VM starts.
- `../beam/logger-and-config.md` — config via `sys.config` / application env. Gleam shipment deployments do not use `sys.config`; configuration is injected as environment variables and consumed directly by the application.

Core guidance

- Gleam compiles to Erlang or JavaScript.
- Gleam is production-ready: "Gleam is a production-ready programming language and the Erlang and JavaScript runtimes it runs on are extremely mature and battle-tested. Gleam is ready for mission critical workloads."
- Both documented deployment guides target the Erlang runtime exclusively.
- Type-safe message passing is provided by libraries (`gleam_erlang`, `gleam_otp`), not by the core language, so Gleam can interoperate with both Erlang and JavaScript despite their incompatible concurrency systems.
- Hot code reloading works via the usual Erlang mechanisms, but upgrades cannot be type-checked. Gleam OTP libraries are optimised for type safety rather than upgrades and use records rather than atom modules, so state upgrade callbacks may be more complex.
- All Gleam data structures are immutable with structural sharing. For mutation needs (caching etc.) use a database or Erlang's ETS.
- Division by zero returns 0. Gleam does not implicitly throw exceptions and the BEAM has no infinity value. The standard library provides Result-returning division alternatives.

Practical rules

Runtime target assumptions

- Target Erlang for the documented deployment paths; the runtime is Erlang/OTP inside a container.
- Bind the application to `0.0.0.0`, not `127.0.0.1`, so the reverse proxy / platform router can reach it.
- Note the port; the guides use port `8000`.
- The Gleam compiler is only required at build time. The production image contains the Erlang/OTP runtime and the compiled shipment.

Build and ship

- Use `gleam export erlang-shipment` to produce `build/erlang-shipment/`, which contains `entrypoint.sh` and the compiled application.
- This is Gleam's own shipment format, not a traditional OTP release. There are no `.rel`, `relup`, or boot scripts.
- Pin toolchain versions in the Dockerfile:
  - Linux server guide: `ARG GLEAM_VERSION=v1.12.0`, `ARG ERLANG_VERSION=28.0.2.0`
  - Fly.io guide: `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine`, `erlang:27.1.1.0-alpine`
- Build the container image on CI (e.g. GitHub Actions on git tags matching `v*`) and push to a registry.

Runtime and operations

- Run as a non-root `webapp` user.
- Minimum 250MB memory; the Linux guide uses 1 vCPU / 1GB.
- HTTPS termination happens outside the BEAM app: Caddy on the Linux server, Fly's proxy on Fly.io.
- Configuration is injected as environment variables, not `sys.config` / `config.exs` / `runtime.exs`.
- Redeployment is a full container replacement, not an in-place hot upgrade.

Review checklist

- [ ] App binds to `0.0.0.0` on the expected port.
- [ ] Dockerfile pins `GLEAM_VERSION` and `ERLANG_VERSION`.
- [ ] Final stage creates and uses a non-root `webapp` user.
- [ ] Final stage copies from `build/erlang-shipment` and sets `ENTRYPOINT ["/app/entrypoint.sh"]` / `CMD ["run"]`.
- [ ] Linux server: `/etc/containers/systemd/webapp.container` sets `PublishPort`, health check, `Restart=always`, and `Environment=` for config.
- [ ] Linux server: Caddy terminates HTTPS and reverse-proxies `localhost:8000`.
- [ ] Linux server: SSH password auth disabled, firewall restricted to ssh/http/https, `unattended-upgrades` enabled.
- [ ] Fly.io: `flyctl launch` used for first deploy and `flyctl deploy` for subsequent deploys.
- [ ] No reliance on `sys.config`, `relup`, or traditional OTP release mechanics.

Implementation checklist

1. Ensure the app binds `0.0.0.0` (Mist/Wisp: `mist.bind("0.0.0.0")`).
2. Write `Dockerfile` with multi-stage build:
   - `gleam` stage from `ghcr.io/gleam-lang/gleam:${GLEAM_VERSION}-scratch` or copy binary from `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine`.
   - `build` stage from `erlang:${ERLANG_VERSION}-alpine`, run `gleam export erlang-shipment`.
   - `final` stage from `erlang:${ERLANG_VERSION}-alpine`, create `webapp` user, copy `/app/build/erlang-shipment`, set entrypoint and default command.
3. Add `healthcheck.sh` for Linux server deployments.
4. Configure CI to build and push the image on git tags.
5. Linux server: install Podman and Caddy, write `/etc/containers/systemd/webapp.container`, run `systemctl daemon-reload && systemctl start webapp`.
6. Fly.io: install Flyctl, run `fly auth login`, then `flyctl launch` and `flyctl deploy`.

Validation hooks

- `gleam build --target erlang`
- `gleam export erlang-shipment` produces `build/erlang-shipment/` with `entrypoint.sh`
- `docker build` succeeds and the final image runs as non-root
- `healthcheck.sh` uses `wget --spider http://127.0.0.1:8000` (Linux server)
- Container health-check failures terminate and restart the container via systemd

Examples

Mist/Wisp bind to all interfaces

```gleam
let assert Ok(_) =
  wisp_mist.handler(router, secret_key_base)
  |> mist.bind("0.0.0.0")
  |> mist.port(8000)
  |> mist.start_http
```

Multi-stage Dockerfile (Linux server style)

```dockerfile
ARG GLEAM_VERSION=v1.12.0
ARG ERLANG_VERSION=28.0.2.0

FROM ghcr.io/gleam-lang/gleam:${GLEAM_VERSION}-scratch AS gleam
FROM erlang:${ERLANG_VERSION}-alpine AS build
COPY --from=gleam /bin/gleam /bin/gleam
COPY . /app/
RUN cd /app && gleam export erlang-shipment

FROM erlang:${ERLANG_VERSION}-alpine
RUN addgroup -S webapp && adduser -S webapp -G webapp
COPY --from=build /app/build/erlang-shipment /app
USER webapp
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["run"]
```

Podman Quadlet unit

```ini
[Container]
Image=ghcr.io/example-org/example-app:production
PublishPort=8000:8000
HealthCmd=sh -c /app/healthcheck.sh
HealthInterval=30s
HealthTimeout=5s
HealthRetries=3
Environment=DATABASE_URL=postgres://...
AutoUpdate=registry

[Service]
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

healthcheck.sh

```sh
#!/bin/sh
wget --spider -q http://127.0.0.1:8000
```

Fly.io deploy commands

```sh
fly auth login
flyctl launch
flyctl open
flyctl deploy
```

Common mistakes

- Binding to `127.0.0.1` instead of `0.0.0.0`; the published port / Fly proxy cannot reach the app.
- Treating `gleam export erlang-shipment` as a traditional OTP release. It has no `.rel`, `relup`, or `sys.config`.
- Expecting hot-code upgrades across container redeploys. Each deploy replaces the container.
- Running the container as root.
- Not pinning `GLEAM_VERSION` and `ERLANG_VERSION` in the Dockerfile.
- Assuming the Fly.io guide documents secrets or `FLY_*` environment variables; it does not.
- Using `gleam export javascript-package` or a Node/Bun runtime for the documented Fly.io path; the guide is Erlang-only.

Strict vs contextual guidance

Strict

- Bind the app to `0.0.0.0`.
- Run the final container as a non-root user.
- Pin Gleam and Erlang/OTP versions in the Dockerfile.
- Linux server: disable SSH password authentication, restrict the firewall to ssh/http/https, and enable unattended security updates.
- Use `gleam export erlang-shipment` for production container images.

Contextual

- Linux server vs Fly.io is a hosting choice; the container build is the same concept but wired to Podman/systemd/Caddy or to Fly's platform.
- HTTPS termination may be Caddy (self-managed) or Fly's edge proxy.
- `AutoUpdate=registry` is optional; without it, redeploys require explicit pulls/restarts.
- Volume mounts and environment variables are project-specific.

Policy decisions for individual repos

- Container registry: GitHub Container Registry (`ghcr.io`) vs Fly's built-in registry.
- Auto-update policy: enable `AutoUpdate=registry` + `podman-auto-update.service` or use explicit redeploys.
- HTTPS termination: Caddy on a single server vs Fly's managed TLS.
- Health-check strategy: HTTP endpoint, interval, timeout, retry count.
- Hosting model: single Linux server vs managed PaaS.

Related docs

- `gleam-toml-and-targets.md` — target selection (Erlang vs JavaScript)
- `project-structure-and-cli.md` — `gleam export` commands
- `javascript-target.md` — JavaScript target; note that both deployment guides are Erlang-only
- `otp-actors-and-supervision.md` — BEAM supervision inside the VM
- `package-management-and-publishing.md` — Hex packages and dependencies

Related skills

- `gleam-packages-ffi` — runtime/shipment and FFI considerations when integrating with Erlang or JavaScript packages
