---
name: validation-nix-format
description: |
  Verifies Nix code formatting is clean. Load after editing .nix files and
  before merge. NOTE: this project does NOT currently configure a Nix formatter;
  the gate is documented for future adoption. Does NOT cover compilation (see
  validation-nix-flake-check), builds (see validation-nix-build), or purity
  linting (see validation-nix-lint).
metadata:
  org.kind: validation
---

# Validation: Nix Format

This gate verifies that `.nix` files are formatted per the project's configured
formatter. It is a style gate, not a correctness gate. IMPORTANT: this project
(ai-workbench) does NOT currently configure a Nix code formatter — `nix fmt`,
`nixpkgs-fmt`, and `alejandra` are all absent. Nix formatting is enforced by
code review, not tooling. This skill documents the gate for future adoption
and the command to use if a formatter is configured.

## Triggers

Load this skill when:

- After editing any `.nix` file, before merge — IF a formatter is configured.
- When wiring formatting into `just verify` (future adoption).
- When diagnosing whether a diff is a formatting-only change.
- When reviewing a PR that touches `.nix` files and a reviewer asks whether
  formatting was applied.
- When onboarding a contributor who expects a `nix fmt` / `nixpkgs-fmt` gate
  (so the absence is explained, not silently skipped).

## Command

```bash
nix fmt --check
```

Alternative, if `nixpkgs-fmt` is configured instead of the experimental
`nix fmt`:

```bash
nixpkgs-fmt --check
```

## Pass criteria

- Exit code 0.
- No formatting changes needed (all `.nix` files already conform).
- (When no formatter is configured: the gate is `not_validated`, not passed.)

## Fail criteria

- Exit code non-zero.
- One or more `.nix` files require formatting.
- (When no formatter is configured: the gate cannot fail because it cannot
  run; report `not_validated` instead of `failed`.)

## Evidence to report

- Exit code.
- List of unformatted files (the formatter prints each file that needs
  changes).
- A diff of the proposed formatting changes (run `nix fmt` without `--check`
  to see the diff, or `nixpkgs-fmt` to apply).

## Notes

- `nix fmt` is experimental and NOT configured in this project. Running it
  fails or is a no-op. Do not run it unless a formatter is configured in
  `flake.nix` or `formatter.<system>`.
- `nixpkgs-fmt` and `alejandra` are also NOT configured in this project.
- This project enforces Nix formatting by code review, not tooling. If a
  formatter is adopted, add the check to `just verify` and update this skill's
  status.
- This gate is currently `not_validated` by default — there is no command to
  run. Mark it `not_validated` (not `validated`) when reporting.
- If adopting a formatter: add `formatter.<system>.<name>` to `flake.nix`,
  add a `fmt-check` justfile recipe, wire it into `verify`, and update this
  skill to remove the "not configured" notes.
- `alejandra` is the most opinionated formatter (enforces a single canonical
  style); `nixpkgs-fmt` is closer to the nixpkgs house style. Either is
  acceptable; pick one and apply it to the whole tree in a single commit to
  avoid noisy diffs later.
- When adopting a formatter, run it once across all `.nix` files in a
  dedicated formatting commit (no logic changes) so future diffs stay clean
  and reviewable.
- A formatting gate is a style gate, not a correctness gate. A clean
  `nix fmt --check` does NOT imply the flake evaluates or builds — always run
  `validation-nix-flake-check` or `validation-nix-build` for correctness.
- See `docs/nix/validation.md` "Format checking" section for the policy
  decision.

## Related skills

- `validation-nix-flake-check` — full-flake eval + checks gate.
- `validation-nix-build` — single-output build gate.
- `validation-nix-lint` — purity lint gate (active; runs in-container).
- `nix-usage` — formatter policy, HOST-GATE context.
