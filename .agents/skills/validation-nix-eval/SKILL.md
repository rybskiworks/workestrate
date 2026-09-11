---
name: validation-nix-eval
description: |
  Verifies flake outputs evaluate correctly and expected attributes are
  present. Load after flake.nix changes or before running builds. Does NOT
  cover building derivations (see validation-nix-test) or supply chain
  pinning (see validation-nix-supply-chain).
metadata:
  org.kind: validation
---

# Validation: Nix Eval

This gate verifies that flake outputs evaluate without errors and that
expected attributes are present in the output tree. It runs after
`flake.nix` changes and before building derivations.

## Triggers

Load this skill when:

- After editing `flake.nix` (inputs, outputs, overlays, modules).
- After adding or renaming a flake output attribute.
- Before running `nix build .#<name>` to confirm the attribute exists.
- As a fast eval-only check before the slower `nix flake check`.

## Command

```bash
nix eval .#workestrate.meta.description
nix eval .#packages.x86_64-linux.workestrate.drvPath
nix flake show
nix flake show --json
```

Run `nix eval` for specific attributes; `nix flake show` for the full
attribute tree.

## Pass criteria

- `nix eval .#<attr>` succeeds and prints the expected value.
- `nix flake show` lists all expected output attributes (packages,
  devShells, checks, lib).
- Exit code 0 for all commands.

## Fail criteria

- Eval error (e.g., infinite recursion, type error, missing input).
- Expected attribute is absent from `nix flake show` output.
- "error: attribute 'X' missing" or similar evaluation failure.
- Non-zero exit code.

## Evidence to report

- Exit code for each command.
- The evaluated value for each `nix eval` invocation.
- The full `nix flake show` attribute tree (or `--json` output).
- The exact eval error message and stack trace on failure.

## Notes

- HOST-GATE: This container has no nix. Run these commands on a
  nix-capable host; the semantics are documented, not runtime-verified
  here.
- NEVER use `--impure` — the project's `lint-nix` guard
  (scripts/check-nix-paths.sh) forbids it, and it copies the raw working
  tree into the store.
- Use `.#` references (git-filtered, pure), NOT `builtins.getFlake` with
  `toString` — the lint-nix guard rejects the latter.
- Stage new files (`git add -N`) before eval — untracked files are
  invisible to `.#` refs.
- There is NO `packages.default` — use `.#workestrate`.
- `nix flake show` evaluates ALL outputs; an eval error in any output
  fails the command.
- `nix eval` does not build; it only forces evaluation of the attribute.
  Use `validation-nix-test` to build derivations.
- For the project's `checks.validateConfig`, eval confirms the attribute
  exists; building it is covered by `validation-nix-test`.
- Run inside `just shell` (see `nix-usage` skill).
- `nix eval` forces evaluation lazily — only the requested attribute is
  forced, not its dependencies. To force deep evaluation of an entire
  attribute set, use `nix eval .#packages.x86_64-linux --apply
  'builtins.deepSeq'` (rarely needed; surfaces hidden eval errors).
- `nix flake show --json` is machine-readable; pipe to `jq` to assert on
  specific output paths (e.g., `jq -e '.packages.x86_64-linux.workestrate'`).
- Eval errors are often caused by missing inputs: if `flake.lock` is stale
  after editing `flake.nix` inputs, run `nix flake lock` to update the lock
  before eval.
- `nix eval` does NOT require building; it is fast and safe to run
  frequently. Use it as the first gate after any `flake.nix` edit.
- `nix eval .#workestrate.meta.description` confirms the package has a
  description; a missing or empty description is a quality signal, not a
  hard failure.
- `nix eval .#packages.x86_64-linux.workestrate.drvPath` confirms the
  derivation path resolves without forcing a full build.
- If `nix flake show` is slow, it is evaluating every output; narrow with
  `nix eval .#<specific-attr>` for faster feedback during iteration.
- Cross-references: `nix-usage` (dev shell, flake basics),
  `validation-nix-test` (build checks), `validation-nix-supply-chain`
  (lockfile pinning), `validation-rust-compile` (Rust compile gate).
- Related docs: `docs/nix/validation.md`, `docs/nix/flake-anatomy.md`.
