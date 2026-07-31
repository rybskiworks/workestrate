---
name: validation-nix-lint
description: |
  Verifies Nix code and scripts are free of purity violations via the project's
  check-nix-paths.sh guard (6 checks). Runs in-container — no nix required. Load
  after editing .nix files, .sh scripts, the justfile, or docs/**/*.md, and
  before merge. Does NOT cover compilation (see validation-nix-flake-check),
  builds (see validation-nix-build), or formatting (see validation-nix-format).
metadata:
  org.kind: validation
---

# Validation: Nix Lint (Purity)

This gate runs the project's active purity-enforcement guard,
`scripts/check-nix-paths.sh`, via `just lint-nix`. It scans `.nix` files,
`.sh` scripts, the `justfile`, and `docs/**/*.md` for six classes of impurity
violations that would copy unbounded data into the Nix store or break
reproducibility. Unlike the other Nix validation gates, this one runs entirely
in-container — it requires only `bash`, `awk`, and `find`, no `nix` binary.

## Triggers

Load this skill when:

- After editing any `.nix` file, `.sh` script under `scripts/`, the
  `justfile`, or `docs/**/*.md`.
- Before merge of any Nix-adjacent change.
- As part of the standard `just verify` pipeline (it is wired in).
- When diagnosing whether a store-growth regression traces to an impure
  pattern.

## Command

```bash
just lint-nix
```

This runs `./scripts/check-nix-paths.sh` from the repo root.

## Pass criteria

- Exit code 0.
- No purity violations found across all 6 checks.
- The script prints `OK: no nix-purity violations (scanned N nix files + M
  shell/justfile files + K docs files).`

## Fail criteria

- Exit code 1.
- One or more violations reported, each of the form
  `<file>:<line>: <description>: <source line>`.
- The 6 violation classes are:
  1. `nix ... --impure` or `nix-shell ... --impure` invocations in `.nix`,
     `.sh`, or `justfile`.
  2. `builtins.getFlake` combined with `toString` on the same line (impure
     runtime path fetch).
  3. `builtins.path { ... }` without a `filter =` field in the following 15
     lines (unbounded store copy).
  4. `cleanSourceWith { ... }` without a `filter =` field in the following 15
     lines (same problem via lib helper).
  5. Bare repo-root path literals (`../`) in nix assignments outside
     `src =`/`lockFile =`/`path =` fields.
  6. Impure-pattern references in `docs/**/*.md`: `getFlake ... toString`,
     `nix eval --impure <arg>`, `toString ./.`.

## Evidence to report

- Exit code.
- Full script output (the violation list with file:line:description:source).
- Violation count.
- Per-violation: check number (1-6), file, line, the offending source line.

## Notes

- This gate runs IN-CONTAINER — it requires only `bash`, `awk`, and `find`.
  No `nix` binary is needed. This is the one Nix validation gate that can be
  `validated` (not `not_fully_checkable`) in this environment.
- The guard is a static heuristic, NOT a full eval-purity prover. Check 5
  does NOT catch bare `src = ./.` — rule 1 in `docs/nix-purity.md` remains
  authoritative even when the guard passes.
- Two allowlist mechanisms suppress false positives:
  - Per-line: append `# allow: <reason>` to any line to skip it in every
    scanned file type. Use sparingly — every allowlist entry is a documented
    purity exception.
  - File-level (docs only): the `DOCS_ALLOWLIST` array in
    `check-nix-paths.sh` names whole documents where impure patterns
    legitimately appear in narrative prose (e.g. `docs/nix-purity.md`). Add
    a file here ONLY when the document's purpose is to discuss/forbid the
    pattern, not to invoke it.
- Wired into `just verify` (the full pre-merge gate) and `just lint-nix`
  (standalone).
- The guard skips its own source (`scripts/check-nix-paths.sh`) and skips
  comment lines (first non-space char is `#`).
- See `docs/nix/validation.md` "check-nix-paths.sh — the 6 purity checks" for
  the full table and `docs/nix-purity.md` for the purity rules.

## Related skills

- `validation-nix-flake-check` — full-flake eval + checks gate (HOST-GATE).
- `validation-nix-build` — single-output build gate (HOST-GATE).
- `validation-nix-format` — formatting gate (not configured).
- `nix-usage` — purity rules, guard inventory, anti-accumulation patterns.
