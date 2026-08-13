# ADR 0028: Location-independent execution (CWD-independent root resolution)

**Status:** Accepted
**Date:** 2026-08-13
**References:** ADR 0023 (single tool home), ADR 0017 (synthetic reference),
spec 17 (`docs/validation-and-improvements/06-improvements/17-config-repo-directory-mode.md`,
F1/F2/F3 path-resolution family),
spec 21 §7 (`docs/validation-and-improvements/06-improvements/21-image-build-lifecycle.md`),
`docs/migration/20-target-system-spec.md` §"Path resolution in mounts".

## Context

`workestrate` must execute correctly from ANY working directory. Today a
nix-layered workload (`workload exec prime --replace`) fails when the CLI is
run from a directory that is not the tool checkout:

```
workload 'prime' uses nix-layered image, which requires a flake project root:
resolved project root '/home/rybski/Development/agent-workbench' does not
contain flake.nix. Set AGENTCTL_ROOT or run from the workbench root directory.
```

Root cause: the "flake project root" that nix-layered images require at
sandbox-build time is resolved by `project_root()` — `AGENTCTL_ROOT` env →
`CARGO_MANIFEST_DIR` walk-up → **current working directory** — and hard-errors
when the resolved root lacks `flake.nix` (`config/mod.rs:117-146`). CWD is the
wrong origin for a workload whose flake root is already knowable: directory-
mode config repos carry `flake.nix` at their own root, and the registry knows
each config repo's path (`config-repos/<name>` for managed clones, the local-
path `url` for `config new` repos).

Precedent (spec 17 F1): mount hosts and seed files already resolve against the
**declaring config layer's content root**, with the flake project root as a
lazy, gated fallback (`microsandbox/mounts.rs` `MountRoots` F1/F2;
`microsandbox/workload/config.rs` seed root). The image/flake-root resolution
must follow the same rule.

## Decision

1. **Flake/image-build roots resolve from the declaring config repo**, which
   the registry knows. A nix-layered workload's flake root = the nearest
   `flake.nix` ancestor of the declaring layer's directory
   (`find_flake_root`), which for directory-mode repos is the config repo
   root. `repo_identity_for` (`images/repo_key.rs`) already implements this
   for the image-record identity; the sandbox-build F2 gate (the
   nix-layered-image / local_build / relative-build-path-mount requirement)
   must use the same declaring-layer-derived root.
2. **CWD is never the resolution origin for a workload's flake root — unless
   the CWD IS the declaring repo** (the declaring layer dir is under the cwd
   ancestor). The `project_root()` CWD tier remains only for tool-relative
   fixtures (reference config) and as a last-resort fallback for synthetic
   workloads with no declaring layer, and even then it must not hard-error
   before the declaring-repo root is tried.
3. **`AGENTCTL_ROOT` becomes an explicit override, not a requirement.** It may
   pin the tool checkout for tool-relative fixtures; it is never required for
   config-repo-derived roots. The error text "Set AGENTCTL_ROOT or run from
   the workbench root directory" must not appear when the declaring repo
   provides the root.

## Consequences

- `workload up` / `exec` / `build` work from any CWD (e.g.
  `~/Development/agent-workbench`), as long as the declaring config repo is
  registered and contains `flake.nix` at its root.
- The failure mode narrows to the honest one: a nix-layered workload whose
  declaring repo genuinely lacks `flake.nix` (spec 21 §7 hard error naming
  the repo) — never a CWD artifact.
- Mount/seed F1 content-root semantics are unchanged; only the F2 gate's root
  source changes.

## Acceptance criteria

1. `workestrate --home <dev-home> workload exec prime` (and `plan` / `build`)
   succeeds from any CWD — including a directory that is neither the tool
   checkout nor a config repo — with no `AGENTCTL_ROOT` set.
2. From a CWD inside the declaring repo, behavior is unchanged.
3. A nix-layered workload declared by a flake-less repo still fails with the
   spec 21 §7 error naming the repo (single target) / skip-with-note (batch)
   — no CWD wording.
4. `AGENTCTL_ROOT` continues to pin the tool checkout where tool-relative
   fixtures need it (reference config), and no longer appears in
   declaring-repo-derived failure paths.
5. Unit tests: the F2 gate resolves the declaring-repo flake root with cwd set
   outside the repo; the CWD-fallback regression is removed.
