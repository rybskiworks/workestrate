---
name: validation-nix-test
description: |
  Verifies all Nix flake checks pass (checks derivations built by nix flake
  check). Load after adding tests or before merge. Does NOT cover eval-only
  validation (see validation-nix-eval) or supply chain pinning (see
  validation-nix-supply-chain).
metadata:
  org.kind: validation
---

# Validation: Nix Test

This gate verifies that all `checks.<system>` derivations build
successfully. It runs after adding test derivations to the flake and before
merge.

## Triggers

Load this skill when:

- After adding or modifying a `checks.<system>.<name>` derivation in
  `flake.nix`.
- After changing code that a check derivation exercises (e.g., config
  validation).
- Before merge, as part of the full validation suite.
- As the build-confirmation step after `validation-nix-eval` passes.

## Command

```bash
nix build .#checks.x86_64-linux.<name>
nix flake check
nix flake check --no-build
```

`nix build .#checks.x86_64-linux.<name>` runs a single check; `nix flake
check` runs all checks AND evaluates all outputs. Use `--no-build` for
eval-only.

## Pass criteria

- All check derivations build successfully (exit code 0).
- `nix flake check` exits 0 (all outputs evaluate AND all checks build).
- The check derivation produces `$out` on success.

## Fail criteria

- Any check derivation fails to build (non-zero exit).
- `nix flake check` reports an evaluation error in any output.
- A `runCommand` check's script exits non-zero.
- Non-zero exit code from any command.

## Evidence to report

- Exit code for each command.
- Names of failing check derivations.
- Build log / error output for failing checks.
- For `nix flake check`, which phase failed (evaluation vs. check build).

## Notes

- HOST-GATE: This container has no nix. Run these commands on a
  nix-capable host; the semantics are documented, not runtime-verified
  here.
- Checks are standard Nix derivations: if they build, the check passes; if
  the build fails, the check fails.
- `nix flake check` does TWO things: (1) evaluates every output, (2)
  builds every `checks.<system>` derivation. An eval error in any output
  (packages, devShells, lib) fails the command even if checks are fine.
- Use `nix build .#checks.x86_64-linux.<name> --no-link` to avoid creating
  a `result` symlink (GC-root hygiene).
- The project's `checks.validateConfig` is a `runCommand` derivation that
  runs `workestrate validate-config` against a TOML config; it produces
  `$out/ok` on success.
- NixOS VM tests (`testers.runNixOSTest`) require Linux + QEMU + KVM; they
  do not run on macOS and need hardware acceleration in CI.
- Successful checks are cached in the Nix store; delete the `result`
  symlink and store path to force a re-run.
- NEVER use `--impure` — the lint-nix guard forbids it.
- Stage new files (`git add -N`) before building — untracked files are
  invisible to `.#` refs.
- The project's `workestrate` Rust package sets `doCheck = false` (tests
  need a running Microsandbox daemon); tests run via `just` recipes
  instead.
- Run inside `just shell` (see `nix-usage` skill).
- `nix flake check --no-build` is the eval-only variant: it evaluates all
  outputs but skips building checks. Use it as a fast pre-check before the
  full `nix flake check` when only `flake.nix` structure changed.
- `nix build .#checks.x86_64-linux.<name>` builds a single check in
  isolation; useful for iterating on one failing check without rebuilding
  the entire check set.
- Check derivations are cached by their store hash; if a check's inputs
  have not changed, Nix skips the rebuild. To force a re-run, use
  `--rebuild` or delete the store path.
- `nix flake check` builds checks in parallel where the dependency graph
  allows; wall-clock time depends on the number of checks and available
  cores.
- Cross-references: `nix-usage` (dev shell, flake basics),
  `validation-nix-eval` (eval-only gate), `validation-nix-supply-chain`
  (lockfile pinning), `validation-rust-test` (Rust test gate),
  `validation-rust-compile` (Rust compile gate).
- Related docs: `docs/nix/testing.md`, `docs/nix/validation.md`.
