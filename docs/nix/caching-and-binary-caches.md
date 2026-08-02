---
type: Reference
resource: https://nix.dev/tutorials/nixos/binary-cache.html
title: Caching and Binary Caches
description: Nix binary caches, substituters, trusted-public-keys, Cachix, self-hosted cache servers, closure copying, evaluation caching, flake-level nixConfig, CI caching, and the cache trust model for the ai-workbench flake.
tags: [nix, cache, binary-cache, substituters, cachix]
timestamp: 2026-07-24T01:20:00Z
---

## Overview

A binary cache is a network service that stores pre-built Nix store objects and serves them to other machines so they can substitute (download) pre-built paths instead of building them locally. This document covers the substitution model, the `substituters` / `trusted-public-keys` / `trusted-substituters` configuration surface, Cachix as a hosted service, self-hosted cache servers (`nix-serve`, `harmonia`, `attic`), closure copying with `nix copy`, evaluation caching, the `nix.conf` settings reference, flake-level `nixConfig` caching, CI caching strategies, the signing-key trust model, and when to use or avoid caches. It is the operational companion to [Nix Store and Paths](./nix-store-and-paths.md) (garbage collection, closures) and the [Nix Usage skill](../../.agents/skills/nix-usage/SKILL.md).

## What is a binary cache?

- Verbatim from crawl 19:
  > "A binary cache stores pre-built [Nix store objects] and provides them to other machines over the network." [19]
  > "Any machine with a Nix store can be a binary cache for other machines." [19]
- Substitution is the act of downloading a pre-built store path from a binary cache instead of building it from source. When Nix needs a store path, it queries each configured substituter in priority order; the first that has a signed, valid copy wins.
- The default public binary cache is `https://cache.nixos.org`, signed by `cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=`.
- A binary cache advertises a `nix-cache-info` file describing `StoreDir`, `WantMassQuery`, and `Priority`. Verbatim example from crawl 19:
  > ```
  > StoreDir: /nix/store
  > WantMassQuery: 1
  > Priority: 30
  > ```
  [19]
- Each store object is described by a `.narinfo` file keyed on the store path hash, which lists the NAR file URL, hash, size, references, and `Sig:` lines. Verbatim from crawl 19:
  > "Sig: cache.example.org:GyBFzocLAeLEFd0hr2noK84VzPUw0ArCNYEnrm1YXakdsC5FkO2Bkj2JH8Xjou+wxeXMjFKa0YP2AML7nBWsAg==" [19]
- Caches interact with garbage collection: paths pulled from a cache live in the local store and are subject to the same GC rules as locally-built paths. "Of course, since disk space is not infinite, unused packages should be removed at some point." [61]

## Substituters configuration

- Verbatim from crawl 33:
  > "Nix can be configured to use a binary cache with the `substituters` and `trusted-public-keys` settings, either exclusively or in addition to cache.nixos.org." [33]
- `substituters` is the list of cache URLs Nix will query. `extra-substituters` appends to the system list without replacing it.
- Per-invocation override via command-line flags (verbatim from crawl 33):
  > ```
  > $ nix-build --substituters https://example.org --trusted-public-keys example.org:My56...Q==%
  > ```
  [33]
- Permanently trying a custom cache before the public cache, with a lower `priority` value (verbatim from crawl 33):
  > ```
  > $ echo "extra-substituters = https://example.org?priority=30" >> /etc/nix/nix.conf
  > $ echo "extra-trusted-public-keys = example.org:My56...Q==%" >> /etc/nix/nix.conf
  > ```
  [33]
- To use only the custom cache (verbatim from crawl 33):
  > ```
  > $ echo "substituters = https://example.org" >> /etc/nix/nix.conf
  > $ echo "trusted-public-keys = example.org:My56...Q==%" >> /etc/nix/nix.conf
  > ```
  [33]
- `priority` semantics: a lower priority value means the cache is tried first. `cache.nixos.org` defaults to priority 40; setting a custom cache to `priority=30` makes Nix try it before the public cache.
- On NixOS, configuration is declarative via `nix.settings` (verbatim from crawl 33):
  > ```nix
  > { ... }: {
  >   nix.settings = {
  >     substituters = [ "https://example.org?priority=30" ];
  >     trusted-public-keys = [ "example.org:My56...Q==%" ];
  >   };
  > }
  > ```
  [33]
- The ai-workbench `flake.nix` does NOT currently declare a `nixConfig.substituters` or `nixConfig.extra-substituters` attribute; the project relies on the host's `nix.conf` and `cache.nixos.org`. [flake]

## Trusted public keys

- Binary caches sign each `.narinfo` with a private key; clients verify signatures against configured public keys before substituting a path.
- Key pair generation (verbatim from crawl 19):
  > ```
  > nix-store --generate-binary-cache-key cache.example.com cache-private-key.pem cache-public-key.pem
  > ```
  [19]
- The private key is held by the cache server (configured via `services.nix-serve.secretKeyFile` on NixOS, or `secret-key-files` in `nix.conf` for build machines that sign before uploading). The public key is distributed to all clients in `trusted-public-keys`.
- Trust warning (verbatim from crawl 33):
  > "Nix will accept any requested store object signed with private keys corresponding to the configured public keys. Access to those private keys thus allows substituting arbitrary files into your Nix store. This includes executables that may run with elevated privileges or automatically! Only add public keys you trust unconditionally." [33]
- `require-sigs` (default: true) enforces that every substituted path must carry a valid signature from a trusted key. Disabling it (`require-sigs = false`) allows unsigned substitution and is dangerous — only for isolated, trusted-network experiments.
- Build machines that upload to a cache must sign paths with the private key. Verbatim from crawl 37:
  > "The path to the file containing the private key you just generated must be added to the `secret-key-files` setting for those machines" [37]
  > ```
  > secret-key-files = /etc/nix/key.private
  > ```
  [37]

## Trusted substituters

- `trusted-substituters` is the list of substituter URLs that non-root users are allowed to use without an explicit `--substituters` override. Substituters listed here are queried even when a non-root user requests them, provided their public keys are in `trusted-public-keys`.
- Distinction:
  - `substituters` — the global list, applied for all users (root-configured).
  - `extra-substituters` — appended to `substituters` without replacing the defaults.
  - `trusted-substituters` — the allow-list of caches non-root users may opt into.
- A non-root user passing `--substituters https://my-cache` will be refused unless `https://my-cache` is in `trusted-substituters` (or the user is root). This prevents untrusted users from pulling arbitrary signed paths.
- Typical pattern: root configures `trusted-substituters = https://cache.example.org` and `trusted-public-keys = example.org:...`; any user may then add `https://cache.example.org` to their own `substituters`.

## Cachix

- Cachix is a hosted binary-cache-as-a-service. Verbatim from crawl 19's alternatives list:
  > "[Cachix](https://www.cachix.org): Nix binary cache as a service" [19]
- Verbatim from crawl 34:
  > "Using Cachix you'll never have to waste time building a derivation twice, and you'll share built derivations with all your developers." [34]
  > "After each job, just-built derivations are pushed to your binary cache. Before each job, derivations to be built are first substituted (if they exist) from your binary cache." [34]
- Cache-per-team guidance (verbatim from crawl 34):
  > "It's recommended to have different binary caches per team, depending on who will have write/read access to it." [34]
- Key Cachix commands:
  - `cachix authtoken <token>` — authenticate the local machine for push access.
  - `cachix use <name>` — configure the local `nix.conf` to pull from a Cachix cache (writes `substituters` + `trusted-public-keys`).
  - `cachix watch-exec <name> -- <command>` — runs a command and watches the Nix store, pushing any newly-built paths to the cache in real time (a daemon alternative to the post-build hook).
- Write access uses either a signing key (`CACHIX_SIGNING_KEY`) or an API token (`CACHIX_AUTH_TOKEN`). Verbatim from crawl 34's GitHub Actions setup:
  > ```yaml
  > - uses: cachix/cachix-action@v14
  >   with:
  >     name: mycache
  >     # If you chose signing key for write access
  >     signingKey: '${{ secrets.CACHIX_SIGNING_KEY }}'
  >     # If you chose API tokens for write access OR if you have a private cache
  >     authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
  > ```
  [34]
- The ai-workbench project does not currently wire a Cachix cache into `flake.nix` or CI; builds substitute from `cache.nixos.org` only. [flake]

## Self-hosted binary cache servers

- **`nix-serve`** — the classic NixOS binary cache daemon. Verbatim NixOS module from crawl 19:
  > ```nix
  > { config, ... }:
  > {
  >   services.nix-serve = {
  >     enable = true;
  >     secretKeyFile = "/var/secrets/cache-private-key.pem";
  >   };
  >   services.nginx = {
  >     enable = true;
  >     recommendedProxySettings = true;
  >     virtualHosts.cache = {
  >       locations."/".proxyPass = "http://${config.services.nix-serve.bindAddress}:${toString config.services.nix-serve.port}";
  >     };
  >   };
  >   networking.firewall.allowedTCPPorts = [
  >     config.services.nginx.defaultHTTPListenPort
  >   ];
  > }
  > ```
  [19]
  - Limitation (verbatim): "`nix-serve` doesn't support IPv6 or SSL/HTTPS." [19] nginx is used as a reverse proxy to add IPv6 and TLS.
- **`nix-serve-ng`** — a drop-in Haskell replacement for `nix-serve` (listed in crawl 19's alternatives). [19]
- **`harmonia`** — a high-performance Nix binary cache server (community project, not in the crawl corpus); suitable when `nix-serve` throughput is insufficient.
- **`attic`** — a Nix binary cache server backed by S3-compatible storage. Verbatim from crawl 19:
  > "[attic](https://github.com/zhaofengli/attic): Nix binary cache server backed by an S3-compatible storage provider" [19]
- **S3 Binary Cache Store** — Nix's native S3-backed cache. Verbatim from crawl 19:
  > "The [SSH Store], [Experimental SSH Store], and the [S3 Binary Cache Store] can also be used to serve a cache. There are many commercial providers for S3-compatible storage, for example: Amazon S3, Tigris, Cloudflare R2" [19]
- HTTPS hardening for a public self-hosted cache uses Let's Encrypt via `security.acme` + `enableACME` / `forceSSL` on the nginx virtual host. [19]

## Copying closures to remote stores

- `nix copy --to <store-uri> <paths>` copies store paths (and their closures with `--recursive`) to a remote store. This is the push side of caching — distinct from substitution, which is the pull side.
- Common target URIs:
  - `ssh://host` — copy over SSH to a remote Nix store.
  - `ssh-ng://host` — the newer SSH store protocol.
  - `s3://bucket` — copy to an S3 binary cache.
  - `file:///path` — copy to a local directory store.
- The post-build-hook pattern automates pushing every local build to a cache. Verbatim hook script from crawl 37:
  > ```bash
  > #!/bin/sh
  > set -eu
  > set -f # disable globbing
  > export IFS=' '
  > echo "Uploading paths" $OUT_PATHS
  > exec nix copy --to "s3://example-nix-cache" $OUT_PATHS
  > ```
  [37]
- `$OUT_PATHS` is a space-separated list of store paths provided by Nix to the hook. `set -f` disables globbing because a store path may contain glob characters (though never spaces). [37]
- Caveat (verbatim from crawl 37):
  > "The post-build-hook program runs after each executed build, and blocks the build loop. The build loop exits if the hook program fails." [37]
  > "Concretely, this implementation will make Nix slow or unusable when the network connection is slow or unreliable." [37]
- Enable the hook in `nix.conf` (verbatim from crawl 37):
  > ```
  > post-build-hook = /etc/nix/upload-to-cache.sh
  > ```
  [37]
- `nix copy` is also used for one-off closure migration between machines (e.g., seeding a new build host with the devshell closure).

## Evaluation caching

- `nix build --eval-cache` (and the `--eval-cache` / `--no-eval-cache` flag on `nix eval`, `nix build`, `nix flake check`) controls whether Nix caches flake evaluation results in `~/.cache/nix/eval-cache-v4/`.
- Evaluation caching memoises the result of evaluating flake outputs and attribute lookups, speeding up repeated `nix flake show`, `nix build .#attr`, and `nix develop` invocations against the same flake revision.
- The eval cache is keyed on the flake input hash; a `nix flake update` invalidates it. It is a local, per-user cache — not a binary cache and not shared across machines.
- `nix flake archive --json` and `nix copy` are the mechanisms for sharing evaluated flake closures across machines; the eval cache itself is not portable.
- For the ai-workbench flake, eval-cache is on by default and is safe — the flake is pure (git-filtered `.#` refs), so cached evaluations are reproducible. [flake]

## nix.conf settings reference

| Setting | Purpose | Default |
|---|---|---|
| `substituters` | Global list of binary cache URLs queried for substitution. | `https://cache.nixos.org` |
| `extra-substituters` | Appended to `substituters` without replacing defaults. | (empty) |
| `trusted-substituters` | Caches non-root users may opt into via `--substituters`. | (empty) |
| `trusted-public-keys` | Public keys used to verify substituted paths. | `cache.nixos.org-1:...` |
| `extra-trusted-public-keys` | Appended to `trusted-public-keys`. | (empty) |
| `trusted-binary-caches` | (Legacy) allow-list of cache URLs; superseded by `trusted-substituters` on modern Nix. | (empty) |
| `require-sigs` | Require valid signatures on all substituted paths. | `true` |
| `secret-key-files` | Private key files used to sign paths built locally (for upload). | (empty) |
| `max-substitution-jobs` | Maximum number of parallel substitution (download) jobs. | `4` |
| `post-build-hook` | Program run after each build; receives `$OUT_PATHS`. | (unset) |
| `auto-optimise-store` | Hard-link identical content across store paths (dedup). | `false` |
| `keep-derivations` | Keep build-time dependency derivations alive under GC. | `true` |
| `keep-outputs` | Keep runtime output paths alive under GC. | `false` |
| `experimental-features` | Enables features like `ca-derivations`, `nix-command`, `flakes`. | (varies) |

- `max-substitution-jobs` tunes download parallelism; raising it speeds cache fills on fat pipes but can saturate bandwidth and trigger rate limits on `cache.nixos.org`.
- `auto-optimise-store` addresses content-addressed duplication (vector (c) of the store-growth model) but is not a substitute for purity discipline. [usage]

## Flake-level caching (nixConfig)

- A flake may declare a `nixConfig` attribute whose values are applied to `nix.conf` for that flake's evaluations (subject to the `accept-flake-config` setting — Nix prompts the user unless `accept-flake-config = true` is set globally).
- `nixConfig` is commonly used to advertise a project's binary cache so contributors automatically substitute from it:
  ```nix
  {
    nixConfig = {
      extra-substituters = [ "https://my-cache.cachix.org" ];
      extra-trusted-public-keys = [ "my-cache.cachix.org-1:..." ];
    };
    outputs = { ... }: { ... };
  }
  ```
- Security note: `nixConfig` is advisory and gated by `accept-flake-config` precisely because a malicious flake could otherwise inject an attacker-controlled substituter + public key pair into every evaluator's trust store. Users must opt in.
- The ai-workbench `flake.nix` does NOT declare a `nixConfig` attribute. The flake's `outputs` define `devShells`, `packages`, `lib`, and `apps` only; no cache advertisement is present. [flake]
- To add a project cache, a `nixConfig` block would sit at the top level of `flake.nix` alongside `description` and `inputs`, and the corresponding public key would be published in the project README.

## CI caching strategies

- Verbatim from crawl 34:
  > "Nix lets CI build and cache developer environments for every project on every branch using binary caches." [34]
  > "Build time is a key CI metric. Cachix (below) is the most straightforward caching option." [34]
- **Cachix GitHub Action** — the canonical CI cache. Verbatim workflow from crawl 34:
  > ```yaml
  > name: "Test"
  > on:
  >   pull_request:
  >   push:
  > jobs:
  >     tests:
  >       runs-on: ubuntu-latest
  >       steps:
  >       - uses: actions/checkout@v4
  >       - uses: cachix/install-nix-action@v25
  >         with:
  >           nix_path: nixpkgs=channel:nixos-unstable
  >       - uses: cachix/cachix-action@v14
  >         with:
  >           name: mycache
  >           signingKey: '${{ secrets.CACHIX_SIGNING_KEY }}'
  >           authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
  >       - run: nix-build
  >       - run: nix-shell --run "echo OK"
  > ```
  [34]
- **GitHub Actions native cache** — `actions/cache` can cache `~/.cache/nix` (the eval cache) and the nix store's substituter downloads, but it has size/retention limits and is less effective than a dedicated binary cache for large closures. The crawl corpus notes GitHub Actions caching limits as a consideration. [34]
- **Self-hosted runner with a local cache** — a persistent self-hosted runner keeps `/nix/store` warm across runs, acting as an implicit cache without upload bandwidth. Best when the runner is trusted and long-lived.
- **Post-build hook in CI** — for self-hosted caches, the `post-build-hook` pattern (crawl 37) pushes every build to an S3 cache automatically. [37]
- Secret hygiene in CI: signing keys and auth tokens must come from encrypted repository secrets (`secrets.CACHIX_SIGNING_KEY`, `secrets.CACHIX_AUTH_TOKEN`), never hardcoded. [34]

## Cache security and trust model

- The trust model is asymmetric: clients trust public keys; the cache server holds the corresponding private key. Anyone with the private key can sign paths that all trusting clients will accept.
- Verbatim risk statement from crawl 33:
  > "Nix will accept any requested store object signed with private keys corresponding to the configured public keys. Access to those private keys thus allows substituting arbitrary files into your Nix store. This includes executables that may run with elevated privileges or automatically! Only add public keys you trust unconditionally." [33]
- `require-sigs = true` (default) is the core guard: no signature, no substitution. Disabling it allows unsigned paths and must never be done on a machine that runs untrusted code.
- Key rotation: generate a new key pair, add the new public key to `trusted-public-keys` alongside the old, push newly-signed paths, then remove the old public key once no clients depend on it.
- Private key storage: on NixOS, `services.nix-serve.secretKeyFile` points at a file on disk (e.g., `/var/secrets/cache-private-key.pem`). For build machines, `secret-key-files` lists signing key files. These files must be protected (mode 0600, owned by root, not in git). [19] [37]
- `trusted-substituters` is the user-isolation guard: it bounds which caches non-root users may pull from, preventing a local user from introducing an attacker-controlled cache via `--substituters`.
- Air-gapped / sensitive environments: do not configure external substituters at all; build everything from source with `substituters =` empty and `require-sigs` irrelevant. The closure is reproducible from the flake inputs alone.

## When to use caches

- **CI pipelines** — "Build time is a key CI metric." [34] A binary cache turns repeated CI builds into cheap substitution, often cutting minutes to seconds.
- **Team sharing** — "you'll share built derivations with all your developers." [34] A team cache means a build done by one developer is instantly available to the rest.
- **Reducing build times for large closures** — the ai-workbench devshell pins a Rust/LLVM/GCC toolchain (~5–7 GB). Without a cache, every fresh machine rebuilds or re-downloads it; with `cache.nixos.org` (for nixpkgs paths) and a project cache (for project-specific derivations), the closure substitutes in minutes. [usage]
- **Devshell reload** — `nix-direnv` caches the `nix develop` environment so entering a project directory is instant after the first load. [36] This is local evaluation caching, not a binary cache, but serves the same goal.
- **Cross-machine closure seeding** — `nix copy --to ssh://host` pre-seeds a build host with a closure so its first build does not pay the full fetch cost. [37]

## When NOT to use caches

- **Air-gapped systems** — no network path to a substituter. Configure `substituters =` empty and build from source.
- **Sensitive / high-assurance environments** — "Access to those private keys thus allows substituting arbitrary files into your Nix store. This includes executables that may run with elevated privileges or automatically!" [33] If you cannot unconditionally trust the cache operator, do not add their public key.
- **Reproducibility audits** — when verifying that a derivation builds reproducibly, substitution defeats the purpose. Use `--substituters ""` (empty) to force a from-source build.
- **Untrusted flake `nixConfig`** — a flake's `nixConfig` can inject substituters + public keys. Unless `accept-flake-config` is deliberately enabled and the flake is trusted, refuse the prompt. [flake]
- **Slow or unreliable networks with post-build hooks** — "this implementation will make Nix slow or unusable when the network connection is slow or unreliable." [37] A blocking post-build hook on a flaky link stalls the build loop; use an async upload daemon instead.
- **Disk-constrained CI runners** — pulling a large closure that is then immediately discarded wastes disk and bandwidth; prefer targeted substitution of only the paths the job needs.

## Citations

[19] [Setting up an HTTP binary cache](https://nix.dev/tutorials/nixos/binary-cache-setup.html) — crawl source: `.crawl/19-binary-cache-setup.md`
[33] [Configure Nix to use a custom binary cache](https://nix.dev/guides/recipes/add-binary-cache.html) — crawl source: `.crawl/33-add-binary-cache.md`
[34] [Continuous integration with GitHub Actions](https://nix.dev/guides/recipes/continuous-integration-github-actions.html) — crawl source: `.crawl/34-continuous-integration-github-actions.md`
[36] [Automatic environment activation with direnv](https://nix.dev/guides/recipes/direnv.html) — crawl source: `.crawl/36-direnv.md`
[37] [Setting up post-build hooks](https://nix.dev/guides/recipes/post-build-hook.html) — crawl source: `.crawl/37-post-build-hook.md`
[61] [Nix Manual — Garbage Collection](https://nix.dev/manual/nix/2.34/package-management/garbage-collection) — crawl source: `.crawl/61-nix-garbage-collection.md`
[flake] [flake.nix](../../flake.nix) — project flake
[usage] [Nix Usage Skill](../../.agents/skills/nix-usage/SKILL.md) — project skill
