# ADR 0012: Odysseus/opencode nix derivations as prerequisite

**Status:** Accepted
**Date:** 2026-07-18

## Context

**Verified gap**: `flake.nix:77` has the comment `# Future:
workestrator-odysseus = ...; workestrator-opencode = ...;`. Odysseus and
opencode have NO nix derivations — they are devshell-only builds
(`nix/devshells/default.nix:225-232`):
- odysseus: `python3.12 -m pip install --only-binary=:all:
  --break-system-packages --target ./.deps -r requirements.lock|requirements.txt`
- opencode: `HUSKY=0 bun install`

Pi and tempest have nix derivations (`pi.nix`, `pi-bun.nix`, `tempest.nix`).
The data-driven migration requires all workloads to have nix build recipes
for the `nix-layered` image path to work uniformly.

## Options considered

1. **Defer odysseus/opencode derivations** — keep them as devshell-only builds.
   Rejected: the `nix-layered` image recipe requires a nix-built `binary`
   input; devshell-only builds can't feed `buildLayeredImage`.
2. **Create odysseus/opencode nix derivations as Phase 0a prerequisite** —
   `nix/packages/odysseus.nix` + `nix/packages/opencode.nix`. Selected.

## Decision

Create `.#odysseus-built` and `.#opencode-built` nix derivations as a Phase 0a
prerequisite. These follow the pattern of `pi.nix` and `tempest.nix`:
- `odysseus.nix`: `buildPythonApplication` or `stdenv.mkDerivation` that
  installs Python deps into `.deps/` and produces the app tree.
- `opencode.nix`: `buildBunPackage` or `stdenv.mkDerivation` that runs
  `bun install` and produces the app tree.

Update `flake.nix:77` to remove the "Future:" comment and add the derivations
to `workload-images` if they get nix-layered images (currently they use
registry images, so this may be deferred — but the derivations must exist for
`local_build` recipe parameterization).

## Consequences

- All 4 agents (pi, odysseus, opencode, tempest) have nix derivations.
- The `local_build` recipe for odysseus (`pip-install`) and opencode
  (`bun-install`) can reference the nix derivation as the canonical source.
- `flake.nix:77` "Future:" comment resolved.

## Rejected why

Deferring: the `nix-layered` image recipe requires nix-built binary input;
devshell-only builds can't feed it.
