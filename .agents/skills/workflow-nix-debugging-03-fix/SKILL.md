---
name: workflow-nix-debugging-03-fix
description: |
  Use only for the fix phase of the Nix debugging workflow.
  Implement the minimal root-cause fix. Do not use for reproduction,
  diagnosis, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-debugging
  org.phase: fix
  org.phase_order: "03"
---

## Phase purpose

Implement the minimal root-cause fix. Do not suppress the symptom. The fix must address the mechanism identified in phase `02-diagnose`.

## Steps to perform

1. Fix the root cause, not the symptom. Concretely by category:
   - **Eval error** → fix the scope/type/attribute issue per `nix-language` and `docs/nix/error-handling-and-debugging.md` (use `rec`, `let`, `builtins.toString`, correct attribute path) rather than adding `--impure` or `--fallback`.
   - **Build failure** → fix the missing dependency, wrong phase command, or build script bug per `nix-derivations` and `docs/nix/derivations-and-builds.md`; add the missing dependency to `nativeBuildInputs` or `buildInputs`; write only to `$out`/`$TMPDIR`; no network in `buildPhase`.
   - **Hash mismatch** → run `just update-hashes` to prefetch the real hash and inline the `got:` value into the matching `nix/packages/*.nix` file; never guess the hash manually.
   - **Purity violation** → fix the impure path reference per `constraint-nix-purity` and `docs/nix-purity.md`; use native `.#` refs, filtered `builtins.path { path = ...; filter = ...; }`, or `cleanSourceWith` with a `filter =` field; never use `--impure` or `builtins.getFlake (toString ./.)`.
   - **Store issue** → remove stale `result*` symlinks and `/tmp/*.tar.gz` out-links; run `just gc` to collect unreachable paths; re-pin the devshell gcroot if stale after a `flake.lock` change.
2. Apply `constraint-nix-purity`: ensure the fix does not introduce impure path references, unfiltered `builtins.path`, or `--impure` invocations.
3. If the fix touches a fixed-output derivation, apply `constraint-nix-reproducibility`: ensure fetcher hashes are correct and flake inputs remain pinned.
4. Apply `constraint-nix-scope-discipline`: fix only the reported defect; record adjacent issues as follow-ups; do not add features; do not suppress with `--impure`, `--fallback`, or hash-guessing workarounds.

## Docs to consult

By category:
- `docs/nix/error-handling-and-debugging.md` (all categories);
- `docs/nix/language-fundamentals.md` (eval errors);
- `docs/nix/derivations-and-builds.md` (build failures);
- `docs/nix-purity.md` (purity violations);
- `docs/nix/store-hygiene-and-gc.md` (store issues).

## Operational skills to load

Conditional by category:
- `nix-language` (eval errors);
- `nix-derivations` (build failures);
- `nix-usage` (hash mismatches);
- `nix-store-gc` (store issues).

## Constraints to apply

- `constraint-nix-scope-discipline` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `--impure`, `--fallback`, or hash-guessing workarounds.
- `constraint-nix-purity` — Ensure the fix does not introduce impure path references, unfiltered `builtins.path`, or `--impure` invocations.
- `constraint-nix-reproducibility` — If the fix touches a FOD or flake input: ensure fetcher hashes are correct and flake inputs remain pinned.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 04-regression`, `next_workflow: null`. Record the fix — what changed and why it addresses the root cause — in `evidence`. List every file changed in `files_touched`. List the constraints actually applied in `constraints_applied`.
