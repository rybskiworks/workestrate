---
name: validation-nix-build
description: |
  Verifies a single Nix flake output builds successfully. HOST-GATE: requires
  nix on the host. Load after adding or modifying a derivation, or before
  release. Does NOT cover full-flake evaluation (see validation-nix-flake-check),
  formatting (see validation-nix-format), or purity linting (see
  validation-nix-lint).
metadata:
  org.kind: validation
---

# Validation: Nix Build

This gate builds a single flake output (derivation) to confirm it produces a
valid store path. It is narrower than `nix flake check` (which evaluates all
outputs and builds all checks) and is used when a specific package must be
confirmed buildable — e.g. before release or after modifying a derivation.

## Triggers

Load this skill when:

- After adding or modifying a derivation in `flake.nix` or `nix/`.
- Before release of a package output.
- When `validation-nix-flake-check` is too broad or too slow and only one
  output needs confirmation.
- As the `verify-full` step (`nix build .#workestrate`) when the nix build
  itself must be confirmed.

## Command

```bash
nix build .#<name> --no-link --print-out-paths
```

Replace `<name>` with the output attribute (e.g. `workestrate`).

- `--no-link` avoids creating a stale `result` symlink (GC-root hygiene).
- `--print-out-paths` prints the resulting store path to stdout for evidence
  capture.

## Pass criteria

- Exit code 0.
- A store path is printed to stdout (the build artifact).
- No build errors or hash mismatches.

## Fail criteria

- Exit code non-zero.
- Build failure (compile error in a buildPhase, missing dependency, sandbox
  network access denied).
- Hash mismatch (fixed-output derivation hash does not match `sha256-...` in
  the nix code).
- Evaluation error (malformed attribute path, missing output).

## Evidence to report

- Exit code.
- The printed store path (on success).
- Build log (capture stderr; use `nix log .#<name>` to retrieve the full log
  of the last build).
- First error message (phase, error text).

## Notes

- HOST-GATE: `nix build` requires Nix on the host. This container has no nix;
  mark the gate `not_fully_checkable` when nix is absent.
- Use `--no-link` to avoid creating a `result` symlink that pins a GC root and
  grows the store indefinitely.
- Use `nix log .#<name>` to retrieve the full build log of the last build of
  an output.
- There is NO `packages.default` in this project — use `.#workestrate` (the
  canonical build target). Using `.#default` fails with "attribute 'default'
  missing".
- Stage new files (`git add -N`) before building — untracked files are
  invisible to the git-filtered `.#` ref form.
- NEVER use `--impure`; it copies the raw working tree into the store (up to
  multi-GB) and is forbidden by `validation-nix-lint`.
- For fixed-output derivations (FODs) with `lib.fakeHash` placeholders, the
  first build fails with a `got: sha256-...` value to inline; see the
  `update-hashes` justfile recipe.
- `just verify-full` runs `just verify` then `nix build .#workestrate` — use
  it when the full pre-merge gate AND the nix build must pass.
- Sandbox builds have no network access by default; a buildPhase that fetches
  from the network (curl, git clone over https) will fail unless the
  derivation is a fixed-output derivation (FOD) or uses `fetchurl`/`fetchgit`
  in the Nix code instead.
- A hash mismatch on a FOD is not a code bug — it means the upstream artifact
  changed or the placeholder was never replaced. Inline the `got:` hash and
  rebuild; do not "fix" the derivation to match a wrong hash.
- See `nix-usage` skill and `docs/nix/validation.md` for the build target
  inventory.

## Related skills

- `validation-nix-flake-check` — full-flake eval + checks gate.
- `validation-nix-format` — formatting gate.
- `validation-nix-lint` — purity lint gate.
- `nix-usage` — build targets, HOST-GATE context.
