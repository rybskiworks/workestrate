---
type: Reference
resource: https://nix.dev/guides/recipes/continuous-integration-github-actions.html
title: CI/CD Integration
description: Nix in CI/CD — GitHub Actions setup, nix flake check as the primary gate, nix build of all outputs, Cachix binary caching, CI workflow patterns, and HOST-NIX gate boundaries for the workestrator project.
tags: [nix, ci, cd, github-actions, cachix, ci-cd]
timestamp: 2026-07-24T02:00:00Z
---

# CI/CD Integration

## Purpose

Source-verified guidance for running Nix in CI/CD. Covers GitHub Actions
setup with `cachix/install-nix-action`, `nix flake check` as the primary CI
gate, `nix build` of all flake outputs, Cachix binary caching, self-hosted
binary caches, CI workflow patterns, and the HOST-NIX / HOST-KVM gate
boundaries that govern what can run in the workestrator container versus
what requires a nix-capable host. Agents who wire up, modify, or debug Nix
CI should follow these rules.

## Sources used

- Crawl file: `docs/nix/.crawl/34-continuous-integration-github-actions.md`
  — https://nix.dev/guides/recipes/continuous-integration-github-actions.html
- Crawl file: `docs/nix/.crawl/19-binary-cache-setup.md`
  — https://nix.dev/tutorials/nixos/binary-cache-setup.html
- Crawl file: `docs/nix/.crawl/63-nix-command-develop.md`
  — https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
- Project file: `justfile` — `verify` and `verify-full` recipes, `lint-nix`,
  `store-audit`, `store-delta-check`
- Project file: `SPEC.md` — HOST-NIX gate definition, milestone M5 (CI/CD)
- Project file: `docs/nix-purity.md` — enforcement context (`just lint-nix`,
  store-growth model)
- Project file: `flake.nix` — `checks.validateConfig`, `.#workestrate`,
  `.#workestrator` outputs

## Core guidance

### GitHub Actions with Nix

`cachix/install-nix-action` is the standard action for installing Nix on
GitHub Actions runners. It installs the Nix package manager and configures
the nixpkgs channel. From crawl 34:

> "Nix lets CI build and cache developer environments for every project on
> every branch using binary caches." [crawl 34]

> "Build time is a key CI metric. Cachix (below) is the most straightforward
> caching option." [crawl 34]

The canonical workflow from crawl 34 (`.github/workflows/test.yml`):

```yaml
name: "Test"
on:
  pull_request:
  push:
jobs:
  tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: cachix/install-nix-action@v25
      with:
        nix_path: nixpkgs=channel:nixos-unstable
    - uses: cachix/cachix-action@v14
      with:
        name: mycache
        # If you chose signing key for write access
        signingKey: '${{ secrets.CACHIX_SIGNING_KEY }}'
        # If you chose API tokens for write access OR if you have a private cache
        authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
    - run: nix-build
    - run: nix-shell --run "echo OK"
```

`DeterminantSystems/nix-installer-action` is an alternative installer action
(from Determinant Systems) that uses the `nix-installer`. It supports flakes
out of the box and is often faster and more robust than
`cachix/install-nix-action`. Use it as a drop-in replacement when the
cachix installer proves unreliable on a runner image.

### nix flake check in CI — the primary gate

`nix flake check` runs all `checks.*` outputs and validates flake
structure. This is the primary CI gate for Nix projects.

`nix flake check` builds all check derivations by default. Use `--no-build`
to only evaluate (faster, catches eval errors but not build failures):

```bash
nix flake check            # eval + build all checks
nix flake check --no-build # eval only (faster)
```

The workestrator flake exposes `checks.x86_64-linux.validateConfig` (from
`flake.nix`), which runs `workestrate validate-config` inside a
`runCommand`. This is the project's current check output — it validates the
reference config TOML against the `workestrate` binary built from the same
flake.

### nix build .#<name> in CI — building all outputs

`nix build .#workestrate` builds the CLI. `nix build .#workestrator` builds
the wrapped binary with `WORKESTRATE_PI_BUILD` baked in (the canonical
runtime entry point).

`nix build .#checks.x86_64-linux.<name>` builds a specific check
derivation.

CI-friendly flags:

- `--no-link` — do not create a `./result` symlink (avoids clutter and
  stale symlinks in CI workspaces).
- `--print-out-paths` — print the output store path(s) to stdout, one per
  line. Essential for CI scripting (e.g. to push to Cachix or upload as an
  artifact).

### Cachix in CI

`cachix/cachix-action` pushes build results to a Cachix binary cache. From
crawl 34:

> "Using Cachix you'll never have to waste time building a derivation twice,
> and you'll share built derivations with all your developers." [crawl 34]

> "After each job, just-built derivations are pushed to your binary cache."
> [crawl 34]

> "Before each job, derivations to be built are first substituted (if they
> exist) from your binary cache." [crawl 34]

Secrets: `CACHIX_SIGNING_KEY` (signing key) and/or `CACHIX_AUTH_TOKEN` (API
token, also needed for private caches). Set as GitHub repository or
organization secrets. [crawl 34]

> "It's recommended to have different binary caches per team, depending on
> who will have write/read access to it." [crawl 34]

### CI workflow patterns

| Stage | Command | Purpose |
|---|---|---|
| Lint | `nix flake check --no-build` + `just lint-nix` | Eval purity + static path guard |
| Build | `nix build .#workestrate --no-link --print-out-paths` | Build the CLI |
| Test | `nix build .#checks.x86_64-linux.validateConfig` | Run flake checks |
| Full | `just verify-full` | All gates + nix build |

`just verify` runs: `toolchain-check`, `check` (fmt + clippy + cargo check),
`test`, `spec-examples`, `litellm-check`, `golden-check`, `schema-check`,
`scaffold-check`, `lint-nix`, `store-audit`, plus
`git diff --exit-code HEAD -- control/agentctl/Cargo.lock`. And
`just verify-full` = `verify` + `nix build .#workestrate`. (Source: justfile.)

### nix profile install — installing tools in CI

`nix profile install nixpkgs#<pkg>` installs a package into the user
profile (persistent). Useful in CI for installing tools that persist across
steps within a job.

Contrast with `nix shell nixpkgs#<pkg>` (temporary, per-command) and
`nix develop -c <cmd>` (devshell-scoped).

### nix develop -c — running commands in devshell

`nix develop -c <command>` runs a command non-interactively in the devshell
environment. From crawl 63:

> "Instead of starting an interactive shell, start the specified command and
> arguments." [crawl 63]

The justfile uses this pattern extensively:

- `nix develop -c python3` (litellm-check fallback)
- `nix develop -c cargo run` (generate-schema)
- `nix develop -c load-images`
- `nix develop -c setup-secrets`
- `nix develop -c scripts/validate-secrets-workflow.sh`

(Source: justfile.)

### Caching strategies

- **GitHub Actions cache** — `actions/cache` or the `cache` input on
  `install-nix-action`. Caches `/nix/store` paths between runs. Limited to
  10 GB per repo.
- **Cachix** — hosted binary cache as a service. Push after build,
  substitute before build. [crawl 34]
- **Self-hosted binary cache** — `nix-serve` / `nix-serve-ng` / `attic` on a
  NixOS machine. [crawl 19] The NixOS module config from crawl 19:

  ```nix
  { config, ... }:

  {
    services.nix-serve = {
      enable = true;
      secretKeyFile = "/var/secrets/cache-private-key.pem";
    };

    services.nginx = {
      enable = true;
      recommendedProxySettings = true;
      virtualHosts.cache = {
        locations."/".proxyPass = "http://${config.services.nix-serve.bindAddress}:${toString config.services.nix-serve.port}";
      };
    };

    networking.firewall.allowedTCPPorts = [
      config.services.nginx.defaultHTTPListenPort
    ];
  }
  ```

- **Self-hosted runners** — runners with a persistent `/nix/store` avoid
  re-downloading; pair with `auto-optimise-store` and periodic
  `nix-collect-garbage`.

### nix-github-actions — generating GitHub Actions matrix from flake outputs

`nix-github-actions` (by nix-community) generates a GitHub Actions matrix
from flake outputs, so each `checks.*` / `packages.*` output runs as a
separate job. Use it as a pattern for splitting flake checks into parallel
CI jobs.

### Hydra — the NixOS CI system

Hydra is the NixOS continuous build system. It builds all attributes of a
flake/nixpkgs evaluation, caches results, and provides a web dashboard. It
is what nixpkgs itself uses. It is the self-hosted, heavyweight option (vs
Cachix which is the lightweight hosted option).

### garnix.io — hosted Nix CI

garnix.io is a hosted CI service for Nix flakes. It builds all flake
outputs and provides GitHub status checks, similar to Hydra but managed.

### HOST-NIX gates — in-container vs host-with-nix

**HOST-NIX** (from SPEC.md): nix builds (`nix build .#workestrate`, image
builds, config-repo flake builds) require a host with nix; the development
container has none.

> "HOST-NIX — nix builds (`nix build .#workestrate`, image builds,
> config-repo flake builds) require a host with nix; this container has
> none." (SPEC.md)

**What runs in-container** (no nix needed): `cargo` commands (via the
relocated `CARGO_TARGET_DIR`), `just check`, `just test`, `just lint-nix`
(bash static guard), `just litellm-check` (python3 fallback),
`just golden-check`, `just schema-check`, `just scaffold-check`,
`just store-audit` (skips when nix absent). Essentially all of
`just verify` EXCEPT the nix build step.

**What requires HOST-NIX**: `nix build .#workestrate`,
`nix build .#workestrator`,
`nix build .#checks.x86_64-linux.validateConfig`, `nix flake check` (needs
nix), `nix develop`, `just verify-full` (adds `nix build`),
`just generate-schema` (uses `nix develop -c`), `just update-hashes` (uses
`nix run`/`nix build`), `just store-delta-check` (uses `nix eval`).

**HOST-KVM** (separate gate): runtime microVM execution (`up`/`exec`/`logs`)
requires `/dev/kvm`; the container has none. This is distinct from
HOST-NIX. (SPEC.md.)

Use the HOST-GATE convention from derivations-and-builds.md: a
`# HOST-GATE:` comment marks commands that can only be verified on a host
with nix.

### The project's current CI state

`just verify` runs locally (in-container where possible). It is the primary
pre-merge gate. (justfile.)

HOST-NIX gates are deferred: `nix build .#workestrate` and
`nix flake check` are not run in the current container. `just verify-full`
(which adds the nix build) is the heaviest validation, run on a
nix-capable host. (justfile, SPEC.md.)

Milestone M5 (from SPEC.md): "Production deployment profiles, CI/CD,
automated updates." — CI/CD is a future milestone. No
`.github/workflows/` CI config exists yet in the repo; this doc is the
design reference for when CI is wired.

`just store-delta-check` is a periodic host/CI check (NOT wired into
`verify`) that measures `/nix/store` growth from one pure eval. (justfile.)

### nix build --no-link --print-out-paths — CI-friendly build output

- `--no-link`: do not create a `./result` symlink (avoids clutter in CI,
  avoids stale symlinks).
- `--print-out-paths`: print the output store path(s) to stdout, one per
  line. Essential for CI scripting (e.g. to push to Cachix or upload as an
  artifact).
- Combined: `nix build .#workestrate --no-link --print-out-paths` — the
  canonical CI build invocation.

The justfile uses `--no-link` in `just update-hashes`:
`nix build .#opencode-built --no-link`. (Source: justfile.)

### nix flake archive --json — CI artifact publishing

`nix flake archive --json` copies all flake closure inputs to the Nix store
and prints the resulting store paths as JSON. Useful in CI for publishing /
flashing a closure to a binary cache or artifact store.

Contrast with `nix build ... --print-out-paths` (single output) vs
`nix flake archive` (entire input closure).

## Practical rules

1. Use `cachix/install-nix-action` (or
   `DeterminantSystems/nix-installer-action`) to install Nix in GitHub
   Actions. [crawl 34]
2. Use `cachix/cachix-action` to push build results to a binary cache; set
   `CACHIX_SIGNING_KEY` or `CACHIX_AUTH_TOKEN` as GitHub secrets. [crawl 34]
3. Run `nix flake check` as the primary CI gate (eval + build all checks).
   Use `--no-build` for eval-only.
4. Build outputs with `nix build .#<name> --no-link --print-out-paths` for
   CI-friendly output (no symlink, machine-readable path).
5. Run `just verify` as the local pre-merge gate; run `just verify-full`
   (adds `nix build .#workestrate`) on a nix-capable host.
6. Mark HOST-NIX-gated commands with `# HOST-GATE:` comments — the
   container has no nix. (derivations-and-builds.md convention.)
7. Use `nix develop -c <cmd>` for non-interactive devshell commands in
   CI/scripts. [crawl 63]
8. Use `nix profile install nixpkgs#<pkg>` for persistent tool installation
   within a CI job; `nix shell` for ephemeral.
9. Cache `/nix/store` via GitHub Actions cache or Cachix to avoid rebuilding
   derivations every run. [crawl 34]
10. Run `just lint-nix` (the static purity guard) in CI — it catches
    `--impure`, unfiltered `builtins.path`, etc. (justfile, nix-purity.md)
11. Run `just store-audit` and `just store-delta-check` periodically on a
    nix-capable host or in CI to catch store-growth regressions. (justfile)
12. For self-hosted binary caches, use `nix-serve` (or
    `nix-serve-ng`/`attic`) behind nginx with a generated signing key pair.
    [crawl 19]

## Review checklist

- [ ] GitHub Actions workflow uses `cachix/install-nix-action` or
      `DeterminantSystems/nix-installer-action`?
- [ ] `cachix/cachix-action` configured with `name:` and a secret
      (`CACHIX_SIGNING_KEY` or `CACHIX_AUTH_TOKEN`)? [crawl 34]
- [ ] `nix flake check` runs in CI (the primary gate)?
- [ ] `nix build` invocations use `--no-link --print-out-paths`?
- [ ] `just verify` passes locally before pushing?
- [ ] `just verify-full` passes on a nix-capable host (HOST-NIX)?
- [ ] HOST-NIX-gated commands marked with `# HOST-GATE:`?
- [ ] `just lint-nix` (purity guard) runs in CI?
- [ ] Binary cache secrets stored as GitHub repo/org secrets (not in-repo)?
      [crawl 34]
- [ ] Self-hosted cache uses a signing key pair (private + public)? [crawl 19]
- [ ] `nix develop -c` used for non-interactive devshell commands?
- [ ] Store-growth checks (`store-audit`, `store-delta-check`) run
      periodically?

## Implementation checklist

- [ ] Create `.github/workflows/<name>.yml` with `actions/checkout@v4`,
      `cachix/install-nix-action`, `cachix/cachix-action`.
- [ ] Add `nix flake check` step.
- [ ] Add `nix build .#workestrate --no-link --print-out-paths` step.
- [ ] Add `just verify` step (requires `just` in the runner — install via
      `nix profile install nixpkgs#just` or the devshell).
- [ ] Set `CACHIX_SIGNING_KEY` / `CACHIX_AUTH_TOKEN` as GitHub secrets.
      [crawl 34]
- [ ] For self-hosted cache: generate key pair with
      `nix-store --generate-binary-cache-key`. [crawl 19]
- [ ] For self-hosted cache: configure `services.nix-serve` +
      `services.nginx` on NixOS. [crawl 19]
- [ ] Configure `substituters` / `trusted-public-keys` in CI nix config to
      pull from the cache.
- [ ] Add `# HOST-GATE:` comments to any step that requires a nix-capable
      host.
- [ ] Wire `just lint-nix` into the CI lint stage.
- [ ] Wire `just store-audit` / `just store-delta-check` into a
      periodic/scheduled job.
- [ ] Pin nixpkgs in the workflow
      (`nix_path: nixpkgs=channel:nixos-unstable` or a flake input).
      [crawl 34]

## Runtime / debugging checklist

- [ ] CI build fails: check `nix log $(nix path-info .#<name>)` for the
      build log.
- [ ] Cache miss (rebuilding everything): verify
      `CACHIX_SIGNING_KEY`/`CACHIX_AUTH_TOKEN` secret is set and the cache
      name matches. [crawl 34]
- [ ] `nix flake check` fails: run `nix flake check --no-build` locally to
      isolate eval vs build failures.
- [ ] `just verify` fails in CI but not locally: check that `just` and the
      devshell tools are installed; check `CARGO_TARGET_DIR` is relocated.
- [ ] Store growth in CI: run `just store-audit` on a self-hosted runner;
      check for `*-source` paths. (nix-purity.md)
- [ ] `nix develop -c <cmd>` fails: ensure the flake is in a git-clean state
      (`git add -N` new files).
- [ ] Self-hosted cache unreachable: `curl http://cache/nix-cache-info`
      should return `StoreDir: /nix/store`. [crawl 19]
- [ ] Signature errors from self-hosted cache: verify the public key is in
      `trusted-public-keys` on the client. [crawl 19]
- [ ] `nix build --print-out-paths` prints nothing: the output may already
      be in the store (cached); check exit code 0.
- [ ] HOST-NIX commands fail in container: expected — the container has no
      nix; run on a host or mark `# HOST-GATE:`.

## Validation hooks

- `nix flake check` — the primary CI gate (eval + build all checks).
- `nix flake check --no-build` — eval-only (faster, no builds).
- `nix build .#workestrate --no-link --print-out-paths` — build the CLI,
  print store path.
- `nix build .#checks.x86_64-linux.validateConfig` — run the project's
  validateConfig check.
- `just verify` — local pre-merge gate (all non-nix gates). (justfile)
- `just verify-full` — `verify` + `nix build .#workestrate` (HOST-NIX).
  (justfile)
- `just lint-nix` — static purity guard (`scripts/check-nix-paths.sh`).
  (justfile, nix-purity.md)
- `just store-audit` — top-20 store paths + source-path gate (skips when
  nix absent). (justfile)
- `just store-delta-check` — periodic `/nix/store` growth assertion
  (HOST-NIX, not in `verify`). (justfile)

> **HOST-GATE:** This container has no nix. The `nix flake check`,
> `nix build`, and `nix develop` commands below are documented Nix
> semantics, not runtime-verified in this environment. Run them on a
> nix-capable host or in CI.

## Examples

### GitHub Actions workflow (from crawl 34, verbatim YAML)

```yaml
name: "Test"
on:
  pull_request:
  push:
jobs:
  tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: cachix/install-nix-action@v25
      with:
        nix_path: nixpkgs=channel:nixos-unstable
    - uses: cachix/cachix-action@v14
      with:
        name: mycache
        # If you chose signing key for write access
        signingKey: '${{ secrets.CACHIX_SIGNING_KEY }}'
        # If you chose API tokens for write access OR if you have a private cache
        authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
    - run: nix-build
    - run: nix-shell --run "echo OK"
```

[crawl 34]

### GitHub Actions workflow for a flake project (workestrator pattern)

A modern workflow using flakes:

```yaml
name: "CI"
on:
  pull_request:
  push:
    branches: [main]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: cachix/install-nix-action@v25
      with:
        nix_path: nixpkgs=channel:nixos-unstable
        extra_nix_config: |
          experimental-features = nix-command flakes
    - uses: cachix/cachix-action@v14
      with:
        name: mycache
        authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
    - run: nix flake check --no-build
    - run: nix build .#workestrate --no-link --print-out-paths
    - run: nix develop -c just verify
```

Note: this is the target pattern for milestone M5; no
`.github/workflows/` exists in the repo yet.

### Cachix setup (secrets) — from crawl 34

Three steps [crawl 34]:

1. Create a binary cache at `app.cachix.org` (fill out the form on the
   create binary cache page).
2. Add `CACHIX_SIGNING_KEY` and/or `CACHIX_AUTH_TOKEN` as GitHub secrets
   (Settings → Secrets on the repository or organization).
3. Add the workflow YAML (the `cachix/cachix-action` step above).

### Self-hosted binary cache (NixOS nix-serve) — from crawl 19

The NixOS module config from crawl 19 (`binary-cache.nix`):

```nix
{ config, ... }:

{
  services.nix-serve = {
    enable = true;
    secretKeyFile = "/var/secrets/cache-private-key.pem";
  };

  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    virtualHosts.cache = {
      locations."/".proxyPass = "http://${config.services.nix-serve.bindAddress}:${toString config.services.nix-serve.port}";
    };
  };

  networking.firewall.allowedTCPPorts = [
    config.services.nginx.defaultHTTPListenPort
  ];
}
```

[crawl 19]

Key generation command [crawl 19]:

```bash
nix-store --generate-binary-cache-key cache.example.com cache-private-key.pem cache-public-key.pem
```

> "You need a pair of private and public keys to ensure that the store
> objects in the cache are authentic." [crawl 19]

> "A binary cache stores pre-built Nix store objects and provides them to
> other machines over the network." [crawl 19]

> "Any machine with a Nix store can be a binary cache for other machines."
> [crawl 19]

Availability check [crawl 19]:

```bash
$ curl http://cache/nix-cache-info
StoreDir: /nix/store
WantMassQuery: 1
Priority: 30
```

### CI-friendly build invocation

```bash
# Build without creating ./result symlink; print store path to stdout
nix build .#workestrate --no-link --print-out-paths

# Build a specific check
nix build .#checks.x86_64-linux.validateConfig --no-link --print-out-paths

# Push to Cachix after build
nix build .#workestrate --no-link --print-out-paths | cachix push mycache
```

### nix develop -c in CI/scripts (workestrator justfile patterns)

```bash
# Run a command in the devshell non-interactively
nix develop -c just verify

# Generate the JSON schema (requires devshell RUSTFLAGS)
nix develop -c cargo run --manifest-path control/agentctl/Cargo.toml -- generate-schema --output schemas/workestrate.schema.json

# Load workload images
nix develop -c load-images
```

(Source: justfile.)

### nix profile install in CI

```bash
# Install just into the runner profile (persists for the job)
nix profile install nixpkgs#just

# Then run verify
just verify
```

## Common mistakes

- Not setting `experimental-features = nix-command flakes` in CI nix config
  — flake commands fail.
- Using `nix-build` (legacy) instead of `nix build` (new CLI) in flake
  projects.
- Forgetting `--no-link` — creates stale `./result` symlinks in CI
  workspaces.
- Not configuring Cachix secrets — builds re-run every time (no
  substitution). [crawl 34]
- Running `nix flake check` without `--no-build` when you only want eval
  validation — slow, builds everything.
- Expecting `just verify` to run nix builds — it does not; use
  `just verify-full` for the nix build (HOST-NIX).
- Running HOST-NIX commands in the container — the container has no nix;
  they fail. Mark with `# HOST-GATE:`. (SPEC.md)
- Not pinning nixpkgs in the workflow —
  `nix_path: nixpkgs=channel:nixos-unstable` or a flake input. [crawl 34]
- Hardcoding `CACHIX_SIGNING_KEY` in the workflow YAML instead of using
  GitHub secrets. [crawl 34]
- Self-hosted cache without a signing key pair — clients reject unsigned
  store objects. [crawl 19]
- Not running `just lint-nix` in CI — purity violations (`--impure`,
  unfiltered paths) slip through. (nix-purity.md)
- Not running `git add -N` for new files before `nix flake check` —
  untracked files are invisible to flakes.

## Strict vs contextual guidance

### Strict (always follow)

- Use `cachix/install-nix-action` or
  `DeterminantSystems/nix-installer-action` to install Nix in GitHub
  Actions. [crawl 34]
- Set cache secrets (`CACHIX_SIGNING_KEY`/`CACHIX_AUTH_TOKEN`) as GitHub
  secrets, never in-repo. [crawl 34]
- Run `nix flake check` as the primary CI gate.
- Use `--no-link --print-out-paths` for CI `nix build` invocations.
- Mark HOST-NIX-gated commands with `# HOST-GATE:`.
  (derivations-and-builds.md)
- Run `just lint-nix` (purity guard) in CI. (nix-purity.md)
- Pin nixpkgs in CI workflows. [crawl 34]

### Contextual (depends on the project)

- Cachix (hosted) vs self-hosted binary cache (`nix-serve`/`attic`) —
  depends on team/infra. [crawl 19]
- `nix flake check` vs `nix flake check --no-build` — full vs eval-only,
  depends on CI time budget.
- `nix-github-actions` matrix generation — optional, for parallelizing
  checks.
- Hydra (self-hosted heavyweight) vs garnix.io (hosted) vs Cachix
  (lightweight) — depends on scale.
- GitHub Actions cache vs Cachix — GitHub cache is limited to 10 GB; Cachix
  is purpose-built.
- Self-hosted runners (persistent store) vs GitHub-hosted runners
  (ephemeral) — depends on build frequency.
- `just verify` (in-container) vs `just verify-full` (HOST-NIX) — depends on
  whether the runner has nix.

## Policy decisions for individual repos

- Default to `cachix/install-nix-action` + `cachix/cachix-action` for GitHub
  Actions CI. [crawl 34]
- Require `nix flake check` in CI (the primary gate).
- Require `just lint-nix` in CI (purity guard).
- Require `--no-link --print-out-paths` for all CI `nix build` invocations.
- Mark all HOST-NIX-gated commands with `# HOST-GATE:` comments.
- Decide Cachix (hosted) vs self-hosted cache (`nix-serve`/`attic`) based on
  team needs. [crawl 19]
- Decide GitHub-hosted vs self-hosted runners based on build frequency and
  store-size needs.
- Require `just verify` to pass before merge; require `just verify-full` on
  a nix-capable host for release branches.

## Related docs

- `/docs/nix-purity.md` — Writing and maintaining pure derivations
  (enforcement context, `just lint-nix`, store-growth model)
- `/docs/nix/derivations-and-builds.md` — Derivations and builds (HOST-GATE
  convention, `nix build` semantics)
- `/docs/nix/devshells.md` — Development shells (`nix develop -c`,
  `mkShell`)
- `/docs/nix/flake-anatomy.md` — Flake anatomy (`checks.*`, `packages.*`
  outputs)
- `/docs/nix/nix-commands.md` — Nix CLI commands (`nix build`,
  `nix flake check`, `nix develop`)
- `/docs/nix/source-map.md` — Nix source map (provenance index)
- `/SPEC.md` — HOST-NIX / HOST-KVM gate definitions, milestone M5 (CI/CD)

## Related skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference

# Citations

[1] [Continuous integration with GitHub Actions](https://nix.dev/guides/recipes/continuous-integration-github-actions.html) — crawl file docs/nix/.crawl/34-continuous-integration-github-actions.md
[2] [Setting up an HTTP binary cache](https://nix.dev/tutorials/nixos/binary-cache-setup.html) — crawl file docs/nix/.crawl/19-binary-cache-setup.md
[3] [nix develop command reference](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop) — crawl file docs/nix/.crawl/63-nix-command-develop.md
