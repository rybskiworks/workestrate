---
name: validation-nix-flake-check
description: |
  Verifies a Nix flake evaluates cleanly and all `checks` derivations build.
  HOST-GATE: requires nix on the host. Load after any flake.nix or nix/ change
  and before merge. Does NOT cover single-output builds (see
  validation-nix-build), formatting (see validation-nix-format), or purity
  linting (see validation-nix-lint).
metadata:
  org.kind: validation
---

# Validation: Nix Flake Check

This is the primary Nix validation gate. It evaluates every output in the
flake to confirm the attribute tree is well-formed, then builds every
derivation in `checks.<system>`. It does NOT build arbitrary package outputs,
run the purity lint, or check formatting — those are separate gates.

## Triggers

Load this skill when:

- After any change to `flake.nix`, `flake.lock`, or files under `nix/`.
- Before merge of any Nix-touching change.
- As the primary Nix gate in a validation suite (before `validation-nix-build`
  when a full build is also required).
- When diagnosing whether a failure is an eval error vs. a build failure vs. a
  purity violation.

## Command

```bash
nix flake check
```

A faster variant that evaluates the attribute tree without building the
`checks` derivations:

```bash
nix flake check --no-build
```

Use `--no-build` to defer builds to CI when only eval correctness is needed
locally.

## Pass criteria

- Exit code 0.
- All flake outputs evaluate without error.
- All `checks.<system>` derivations build successfully.

## Fail criteria

- Exit code non-zero.
- Evaluation error (malformed flake, missing input, type error in Nix code).
- Build failure in any `checks` derivation.

## Evidence to report

- Exit code.
- Full command output.
- List of failing checks (attribute path, e.g.
  `checks.x86_64-linux.validateConfig`).
- First error message (attribute path, error text).

## Notes

- HOST-GATE: `nix flake check` requires Nix installed on the host. This
  container has no nix; the command semantics are documented, not
  runtime-verified in this environment. Mark the gate
  `not_fully_checkable` when nix is absent.
- `nix flake check` evaluates ALL flake outputs and builds ALL `checks`
  derivations — it is broader than `nix build .#<name>` (which builds a
  single output).
- Use `nix flake check --no-build` to evaluate the attribute tree without
  building checks (faster; defers builds to CI).
- Stage new files (`git add -N`) before running — untracked files are
  invisible to the git-filtered `.#` ref form and cause "not tracked by Git"
  errors.
- NEVER use `--impure`; the project's `validation-nix-lint` gate forbids it
  and it copies the raw working tree into the store.
- The project's `checks` output currently contains only `validateConfig`;
  see `flake.nix` and `docs/nix/validation.md`.
- CI should run `nix flake check` (or `--no-build`) as the primary Nix gate;
  CI should NOT run `nix flake update`.
- `nix flake check` respects the `flake.lock` pins; a stale lockfile can
  cause eval failures that are not code bugs. Run `nix flake lock --no-update`
  to refresh metadata without bumping input revisions if a lockfile drift is
  suspected.
- Evaluation errors surface as `error: ...` with an attribute path trace;
  build failures surface as a derivation name and a build log. Distinguishing
  the two is the first diagnostic step when this gate fails.
- See `nix-usage` skill for the guard inventory and `docs/nix/validation.md`
  for the full validation hierarchy.

## Related skills

- `validation-nix-build` — single-output build gate.
- `validation-nix-format` — Nix formatting gate.
- `validation-nix-lint` — purity lint gate (runs in-container; no nix required).
- `nix-usage` — flake outputs, dev shell, HOST-GATE context.
