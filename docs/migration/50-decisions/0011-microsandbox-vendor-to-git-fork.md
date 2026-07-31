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

## Addendum (2026-07-30): carrier decision reversed

The fork-as-dependency-carrier plan in the Decision section is **REVERSED**.
A consumed git-fork dependency is maintenance rot: rebase-per-release churn
and fork accumulation.

**Interim:** the current nix-side patch apparatus
(`fetchCrate` + `nix/packages/microsandbox-filesystem-agentd.patch` +
`nix/packages/microsandbox-filesystem-patched.nix` + the devshell
`_setup_vendor_link` vendor-symlink machinery) stays **UNCHANGED** until
upstream lands the fix.

**The fork** (`github.com/georgrybski/microsandbox`) exists **ONLY** as a
transient vehicle to open the upstream PR — it is never consumed as a
dependency. The earlier-prepared `.tmp/microsandbox-fork` 0.5.6+patch branch
(`448937d4`) is moot as a dependency source (kept as reference only).

**End-state:** upstream merges the MSB_HOME-parity fix for
`crates/filesystem/build.rs` (mirroring their own #704, which merged the
identical pattern into the SDK crate) → bump the microsandbox pin to the
release carrying it → delete the entire patch apparatus:
`nix/packages/microsandbox-filesystem-patched.nix`, the patch file, the
devshell `_setup_vendor_link` hook, the `agentctl.nix` vendor staging + the
`control/agentctl/.cargo/config.toml` `[patch.crates-io]` path entry, the
justfile `vendor-unlock`/`vendor-lock` recipes, and the `.gitignore` vendor
line.

Cross-references: spec 09 option 1 (upstream PR, IN-FLIGHT — branch
`fix/filesystem-msb-home-agentd-staging` on `georgrybski/microsandbox`; PR
draft at `.tmp/msb-upstream/PR.md`) at
[../../validation-and-improvements/06-improvements/09-microsandbox-agentd-offline-build.md](../../validation-and-improvements/06-improvements/09-microsandbox-agentd-offline-build.md).
This is consistent with ADR 0025's spirit of pinning to immutable upstream
releases/revs rather than maintaining mutable carrier artifacts (no fork
carrier).
