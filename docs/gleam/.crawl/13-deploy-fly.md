# Crawl: deployment/fly/
- seed_url: https://gleam.run/deployment/fly/
- canonical_url: https://gleam.run/deployment/fly/
- family: Gleam official docs
- fetch: 200
- gleam_version: v1.8.0 (referenced in Dockerfile image tag `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine`); Erlang/OTP 27.1.1.0 (base image `erlang:27.1.1.0-alpine`)
- feeds_docs: deployment-and-runtime.md, javascript-target.md

## Purpose
Guide for deploying a Gleam web application to Fly.io using a Dockerfile-based
container build. Walks through preparing the app to bind to `0.0.0.0`, writing a
multi-stage Dockerfile that exports the Erlang shipment, installing/authenticating
the Flyctl CLI, and launching/deploying via `flyctl launch` / `flyctl deploy`.

## Fly.io deploy steps (Dockerfile, fly commands)
1. **Prepare your application** — ensure it listens on `0.0.0.0`. For Mist/Wisp
   apps use `mist.bind("0.0.0.0")` before `mist.start_http`. Note the port
   (guide uses 8000).
2. **Create a Dockerfile** — multi-stage build:
   - Stage `build`: `FROM erlang:27.1.1.0-alpine`, copy the Gleam binary from
     `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine`, copy app source to `/app`,
     run `gleam export erlang-shipment`.
   - Final stage: `FROM erlang:27.1.1.0-alpine`, create a non-root `webapp`
     user/group, `USER webapp`, copy `/app/build/erlang-shipment` from build
     stage, `WORKDIR /app`, `ENTRYPOINT ["/app/entrypoint.sh"]`,
     `CMD ["run"]`.
3. **Set up the Fly.io CLI** — install Flyctl (link to
   https://fly.io/docs/getting-started/installing-flyctl/), then
   `fly auth signup` or `fly auth login`.
4. **Deploy the application** — from the project run `flyctl launch`. The CLI
   prompts for: app name, Fly organisation, region, and whether to provision a
   PostgreSQL database. It then builds via the Dockerfile. Open in a browser
   with `flyctl open`. Subsequent deploys use `flyctl deploy`.

## Target choice (Erlang vs JavaScript)
The guide deploys the **Erlang target** exclusively. The Dockerfile uses
`erlang:27.1.1.0-alpine` as the base image and runs
`gleam export erlang-shipment`, copying the resulting
`/app/build/erlang-shipment` directory into the final image. There is no
JavaScript-target variant, no Node/Bun runtime, and no mention of
`gleam export javascript-package` or `gleam build --target javascript`. The
entrypoint is the generated `entrypoint.sh` from the Erlang shipment.

## Secrets/env
Not covered on this page. The guide does not document `flyctl secrets set`,
`FLY_*` environment variables, or runtime configuration injection. The only
secret-like reference is the `secret_key_base` argument passed to
`wisp_mist.handler` in the prepare-application example, but its provisioning on
Fly is not discussed here.

## Target-specific notes
- Erlang target only; relies on `gleam export erlang-shipment` producing a
  self-contained `/app/build/erlang-shipment` directory with an
  `entrypoint.sh`.
- Base image is Alpine (`erlang:27.1.1.0-alpine`); Gleam binary is copied from
  the official `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine` image rather than
  installed from a package manager.
- Runs as a non-root `webapp` user.
- `CMD ["run"]` passes `run` as the argument to `entrypoint.sh` (the shipment's
  entrypoint interprets this as the Gleam app's main module invocation).
- Port binding to `0.0.0.0` is mandatory because Fly's proxy must reach the
  listener; binding to `127.0.0.1` would be unreachable.
- The guide assumes a Mist/Wisp web server; the `mist.bind` snippet is the only
  framework-specific code shown.

## Strict rules
- Bind the application to `0.0.0.0` (not localhost) so Fly can route traffic.
- Use the Erlang shipment (`gleam export erlang-shipment`) for the container
  build; do not attempt to run Gleam source directly in production.
- Run the container as a non-root user (the Dockerfile creates `webapp`).
- Pin toolchain versions in the Dockerfile (Erlang 27.1.1.0, Gleam v1.8.0) for
  reproducible builds.
- Use `flyctl launch` for first deploy (creates `fly.toml` and provisions
  infra), then `flyctl deploy` for subsequent updates.

## Verbatim quotes
- "Run Gleam all over the world. No ops required."
- "Fly.io is a convenient and easy to use deployment platform. They were also a
  sponsor of the Gleam project, thank you Fly!"
- "Ensure your application is listening on `0.0.0.0`. If you're using Mist or
  Wisp you can do this with the `mist.bind` function, as shown here."
- "Take note of what port your application is starting on. We will be using port
  8000 for the rest of this guide."
- "We can use Fly's support for containers to build the application and prepare
  it for deployment."
- "Follow the instructions here to install Flyctl, the command-line interface
  for the Fly.io platform."
- "From within the project use the Fly CLI to create and run your application on
  their platform."
- "The CLI will ask you a series of questions: What the application should be
  named. What Fly organisation should the application belong to. What region the
  application should be deployed to. Whether you would like a PostgreSQL database
  to go with the application."
- "Once you have answered these it will build the application using the docker
  file. Once deployed you can open it in a web browser by running `flyctl open`."
- "To deploy future versions of the application run `flyctl deploy` after saving
  any changes to the source code."

## Version notes
- Gleam binary image tag: `ghcr.io/gleam-lang/gleam:v1.8.0-erlang-alpine`
  (Gleam v1.8.0).
- Erlang/OTP base image: `erlang:27.1.1.0-alpine` (OTP 27.1.1.0).
- No `gleam_version` meta tag or version banner on the page itself; version
  inferred solely from the Dockerfile image tags.
- Page copyright footer reads "© 2026 Louis Pilfold".

## Discovered links
### Relevant (crawl later)
- https://fly.io/docs/getting-started/installing-flyctl/ — Flyctl install
  instructions (external, Fly docs; crawl only if Fly deployment context is
  expanded).
- https://gleam.run/install — Gleam install page (already crawled as 10-install.md).
- https://gleam.run/documentation — Gleam docs index (already crawled as
  01-documentation.md).

### Skipped
- /, /news, /community, /sponsor, /case-studies, /roadmap — site nav, non-doc.
- /styles/main.css?v=... — stylesheet.
- /images/lucy/lucy.svg — mascot image.
- https://discord.gg/Fm8Pwmy — Discord invite.
- https://github.com/gleam-lang, https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md — GitHub repo / CoC.
- https://gleam.run/feed.xml — RSS feed.
- https://gleamweekly.com/, https://packages.gleam.run/,
  https://packages.gleam.run, https://playground.gleam.run,
  https://tour.gleam.run, https://shop.gleam.run/en-gbp — external Gleam
  properties, not deployment docs.
- #create-a-dockerfile, #deploy-the-application, #prepare-your-application,
  #set-up-the-flyio-cli — in-page anchors.
