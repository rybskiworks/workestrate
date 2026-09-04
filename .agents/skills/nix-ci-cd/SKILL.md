---
name: nix-ci-cd
description: |
  Operational guide for Nix in CI/CD — GitHub Actions with install-nix-action,
  cachix-action, nix flake check as the primary gate, nix build --no-link
  --print-out-paths, CI workflow patterns, Cachix/self-hosted binary caching,
  and HOST-NIX gate boundaries. Load when wiring up, modifying, or debugging
  Nix CI/CD pipelines. Distilled from docs/nix/ci-cd-integration.md and
  docs/nix/validation.md; consult those docs for full detail.
---

# Nix CI/CD Integration

Distilled from [`docs/nix/ci-cd-integration.md`](../../../docs/nix/ci-cd-integration.md).
That doc holds the canonical workflow YAML, Cachix setup, and upstream source
links; this skill is the actionable subset.

## Triggers

Load this skill when:

- Setting up GitHub Actions with Nix (`cachix/install-nix-action`).
- Configuring Cachix binary caching in CI.
- Running `nix flake check` as a CI gate.
- Building flake outputs in CI (`nix build .#workestrate`).
- Debugging CI Nix failures (eval errors, cache misses, HOST-NIX).
- Wiring `just verify` / `just verify-full` into CI.
- Setting up self-hosted binary caches (`nix-serve`, `attic`).

## GitHub Actions Setup

`cachix/install-nix-action` is the standard action for installing Nix on
GitHub Actions runners. `DeterminantSystems/nix-installer-action` is an
alternative (uses the `nix-installer`, supports flakes out of the box, often
faster/more robust). Example workflow:

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
    - run: just verify   # guarded recipes self-enshell with the devenv-root override
```

## nix flake check in CI

`nix flake check` is the primary CI gate — it evaluates all outputs and builds
all `checks.*` derivations. Use `--no-build` for eval-only (faster, catches
eval errors but not build failures). The project exposes
`checks.x86_64-linux.validateConfig`, which runs `workestrate validate-config`
inside a `runCommand`.

## CI-Friendly Build Flags

| Flag | Purpose |
|---|---|
| `--no-link` | Do not create a `./result` symlink (avoids clutter/stale symlinks in CI). |
| `--print-out-paths` | Print output store path(s) to stdout, one per line (machine-readable). |

Combined canonical invocation: `nix build .#workestrate --no-link --print-out-paths`.

## Cachix

`cachix/cachix-action` pushes build results to a Cachix binary cache after each
job and substitutes from it before each build. Secrets: `CACHIX_SIGNING_KEY`
(signing key) and/or `CACHIX_AUTH_TOKEN` (API token, also needed for private
caches). Set as GitHub repository or organization secrets — NEVER in-repo.

## CI Workflow Patterns

| Stage | Command | Purpose |
|---|---|---|
| Lint | `nix flake check --no-build` + `just lint-nix` | Eval purity + static path guard |
| Build | `nix build .#workestrate --no-link --print-out-paths` | Build the CLI |
| Test | `nix build .#checks.x86_64-linux.validateConfig` | Run flake checks |
| Full | `just verify-full` | All gates + nix build |

## just verify vs verify-full

- `just verify` — runs in-container (all non-nix gates): `toolchain-check`,
  `check` (fmt + clippy + cargo check), `test`, `spec-examples`,
  `golden-check`, `schema-check`, `scaffold-check`,
  `lint-nix`, `store-audit`, plus `git diff --exit-code HEAD -- control/agentctl/Cargo.lock`.
- `just verify-full` — `verify` + `nix build .#workestrate` (HOST-NIX).

## HOST-NIX Gates

**Runs in-container** (no nix needed): `cargo` commands (via relocated
`CARGO_TARGET_DIR`), `just check`, `just test`, `just lint-nix` (bash static
guard), `just golden-check`, `just schema-check`,
`just scaffold-check`, `just store-audit` (skips when nix absent). Essentially
all of `just verify` EXCEPT the nix build step.

**Requires HOST-NIX**: `nix build .#workestrate`, `nix build .#workestrator`,
`nix build .#checks.x86_64-linux.validateConfig`, `nix flake check`,
`just shell` (devshell entry), `just verify-full`, `just generate-schema`
(re-execs `nix develop` with the devenv-root override),
`just update-hashes` (uses `nix run`/`nix build`), `just store-delta-check`
(uses `nix eval`).

**HOST-KVM** is a separate gate: runtime microVM execution (`up`/`exec`/`logs`)
requires `/dev/kvm`; the container has none. Distinct from HOST-NIX.

## Caching Strategies

- **GitHub Actions cache** — `actions/cache` or the `cache` input on
  `install-nix-action`. Caches `/nix/store` paths between runs. Limited to 10 GB per repo.
- **Cachix** — hosted binary cache as a service. Push after build, substitute before build.
- **Self-hosted binary cache** — `nix-serve` / `nix-serve-ng` / `attic` on a
  NixOS machine behind nginx with a generated signing key pair.
- **Self-hosted runners** — runners with a persistent `/nix/store` avoid
  re-downloading; pair with `auto-optimise-store` and periodic `nix-collect-garbage`.

## Non-interactive devshell commands in CI

`just shell -c <command>` runs a command non-interactively in the devshell
environment (it wraps `nix develop` with the devenv-root override). The
justfile's guarded recipes self-enshell: when not already inside the devshell
they re-exec `nix develop --override-input devenv-root
"file+file://$HOME/.cache/workestrate/devenv-root/workestrate" -c
just _<name>-inner`; generate-schema runs `cargo run` with that same
override. Bare `nix develop` is not a supported entry (fails the
`devenv.root != ""` assertion under pure eval). For persistent tool
installation within a CI job, use `nix profile install nixpkgs#<pkg>`; for
ephemeral, `nix shell nixpkgs#<pkg>`.

## Practical Rules

1. Use `cachix/install-nix-action` (or `DeterminantSystems/nix-installer-action`) to install Nix in GitHub Actions.
2. Use `cachix/cachix-action` with `CACHIX_SIGNING_KEY`/`CACHIX_AUTH_TOKEN` as GitHub secrets (never in-repo).
3. Run `nix flake check` as the primary CI gate; use `--no-build` for eval-only.
4. Build outputs with `nix build .#<name> --no-link --print-out-paths` for CI-friendly output.
5. Run `just verify` as the local pre-merge gate; `just verify-full` (adds `nix build`) on a nix-capable host.
6. Mark HOST-NIX-gated commands with `# HOST-GATE:` comments — the container has no nix.
7. Use `just shell -c <cmd>` (or the `--override-input devenv-root` form) for
   non-interactive devshell commands in CI/scripts; bare `nix develop` fails
   pure eval on the `devenv.root` assertion.
8. Use `nix profile install nixpkgs#<pkg>` for persistent tool installation; `nix shell` for ephemeral.
9. Cache `/nix/store` via GitHub Actions cache or Cachix to avoid rebuilding every run.
10. Run `just lint-nix` (the static purity guard) in CI — catches `--impure`, unfiltered `builtins.path`.
11. Run `just store-audit` and `just store-delta-check` periodically on a nix-capable host or in CI.
12. For self-hosted binary caches, use `nix-serve` (or `nix-serve-ng`/`attic`) behind nginx with a signing key pair.

## Review Checklist

- [ ] GitHub Actions workflow uses `cachix/install-nix-action` or `DeterminantSystems/nix-installer-action`.
- [ ] `cachix/cachix-action` configured with `name:` and a secret (`CACHIX_SIGNING_KEY` or `CACHIX_AUTH_TOKEN`).
- [ ] `nix flake check` runs in CI (the primary gate).
- [ ] `nix build` invocations use `--no-link --print-out-paths`.
- [ ] `just verify` passes locally before pushing.
- [ ] `just verify-full` passes on a nix-capable host (HOST-NIX).
- [ ] HOST-NIX-gated commands marked with `# HOST-GATE:`.
- [ ] `just lint-nix` (purity guard) runs in CI.
- [ ] Binary cache secrets stored as GitHub repo/org secrets (not in-repo).
- [ ] Self-hosted cache uses a signing key pair (private + public).
- [ ] `just shell -c` / devenv-root-override form used for non-interactive devshell commands (no bare `nix develop`).
- [ ] Store-growth checks (`store-audit`, `store-delta-check`) run periodically.

## Validation Commands

> **HOST-GATE:** This container has no nix. The `nix flake check`, `nix build`,
> and `nix develop` commands below are documented Nix semantics, not
> runtime-verified in this environment. Run them on a nix-capable host or in CI.

```bash
nix flake check                                       # eval + build all checks (primary gate)
nix flake check --no-build                            # eval only (faster)
nix build .#workestrate --no-link --print-out-paths   # build the CLI, print store path
just verify                                           # local pre-merge gate (all non-nix gates)
just verify-full                                      # verify + nix build .#workestrate (HOST-NIX)
just lint-nix                                         # static purity guard
just store-audit                                      # top-20 store paths + source-path warning (informational)
```

## Common Mistakes

1. Not setting `experimental-features = nix-command flakes` in CI nix config — flake commands fail.
2. Using `nix-build` (legacy) instead of `nix build` (new CLI) in flake projects.
3. Forgetting `--no-link` — creates stale `./result` symlinks in CI workspaces.
4. Not configuring Cachix secrets — builds re-run every time (no substitution).
5. Running `nix flake check` without `--no-build` when you only want eval validation — slow, builds everything.
6. Expecting `just verify` to run nix builds — it does not; use `just verify-full` for the nix build (HOST-NIX).
7. Running HOST-NIX commands in the container — the container has no nix; they fail. Mark with `# HOST-GATE:`.
8. Not pinning nixpkgs in the workflow (`nix_path: nixpkgs=channel:nixos-unstable` or a flake input).
9. Hardcoding `CACHIX_SIGNING_KEY` in the workflow YAML instead of using GitHub secrets.
10. Self-hosted cache without a signing key pair — clients reject unsigned store objects.
11. Not running `just lint-nix` in CI — purity violations (`--impure`, unfiltered paths) slip through.
12. Not running `git add -N` for new files before `nix flake check` — untracked files are invisible to flakes.

## Strict Rules

- Use `cachix/install-nix-action` or `DeterminantSystems/nix-installer-action` to install Nix in GitHub Actions.
- Set cache secrets as GitHub secrets, never in-repo.
- Run `nix flake check` as the primary CI gate.
- Use `--no-link --print-out-paths` for all CI `nix build` invocations.
- Mark HOST-NIX-gated commands with `# HOST-GATE:`.
- Run `just lint-nix` (purity guard) in CI.
- Pin nixpkgs in CI workflows.

## Related Docs

- Full reference: [`docs/nix/ci-cd-integration.md`](../../../docs/nix/ci-cd-integration.md) (canonical; workflow YAML + Cachix setup there).
- [`docs/nix/validation.md`](../../../docs/nix/validation.md) — validation gate suite.
- [`docs/nix-purity.md`](../../../docs/nix-purity.md) — enforcement context, `just lint-nix`, store-growth model.
- [`docs/nix/derivations-and-builds.md`](../../../docs/nix/derivations-and-builds.md) — `nix build` semantics, HOST-GATE convention.
- [`SPEC.md`](../../../SPEC.md) — HOST-NIX / HOST-KVM gate definitions, milestone M5 (CI/CD).

## Related Skills

- [`nix-usage`](../nix-usage/SKILL.md) — project flake, dev shell, `just verify` / `verify-full`.
- [`nix-testing`](../nix-testing/SKILL.md) — `nix flake check`, `checks` output, `checkPhase`.
- [`nix-store-gc`](../nix-store-gc/SKILL.md) — `just store-audit` / `store-delta-check` in CI, store hygiene.
