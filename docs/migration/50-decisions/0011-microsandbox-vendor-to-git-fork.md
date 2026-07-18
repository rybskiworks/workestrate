# ADR 0011: Microsandbox vendor → git-fork dependency

**Status:** Accepted
**Date:** 2026-07-18

## Context

The microsandbox-filesystem crate is currently vendored via a symlink:
`control/agentctl/vendor/microsandbox-filesystem-0.5.6` →
`${microsandbox-filesystem-patched}` (a nix derivation that applies
`microsandbox-filesystem-agentd.patch`). The devshell refreshes the symlink
on entry (`nix/devshells/default.nix:99-125`). `just vendor-unlock`/
`vendor-lock` recipes manage manual editing.

This mechanism is fragile (symlink breakage after GC, manual refresh) and
couples the build to a patched-vendor workflow.

## Options considered

1. **Keep vendor symlink** — status quo. Rejected: fragile; symlink breakage;
  manual refresh.
2. **Git-fork dependency** — fork the microsandbox-filesystem crate, apply the
  patch on the fork, consume the fork as a Cargo dependency via
  `[patch.crates-io]` pointing at the fork's git URL. Selected.

## Decision

Migrate the vendor symlink to a git-fork dependency. The fork
(`github:georgrybski/microsandbox-filesystem` or similar) carries the
`agentd` patch. `Cargo.toml` uses `[patch.crates-io]` to point at the fork.
`nix/packages/agentctl.nix:41-47` preBuild is simplified (no vendor symlink
staging). `nix/packages/microsandbox-filesystem-patched.nix` and the patch
file are removed.

This follows the existing **fork-carries-compat** policy documented in
`SPEC.md:71-75` and `README.md:372-374`: "nix-build compatibility (patches,
lockfile, committed catalogs) lives on the agent fork, not as nix-side
patches in this repo."

## Consequences

- No more vendor symlink; no `vendor-unlock`/`vendor-lock` recipes.
- The patch is reviewed and versioned on the fork (git history).
- `nix/packages/agentctl.nix` preBuild is simpler.
- `just vendor-unlock`/`vendor-lock` removed from justfile.

## Rejected why

Vendor symlink: fragile; manual refresh; symlink breakage after GC.
