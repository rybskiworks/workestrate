---
name: workflow-nix-debugging-02-diagnose
description: |
  Use only for the diagnose phase of the Nix debugging workflow.
  Categorize the defect (eval error / build failure / hash mismatch /
  purity violation / store issue) and identify the root cause. Do not
  use for reproduction, fixing, regression testing, or final
  verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-debugging
  org.phase: diagnose
  org.phase_order: "02"
---

## Phase purpose

Categorize the defect (eval error / build failure / hash mismatch / purity violation / store issue) and identify the root cause. The category determines which operational skills and docs to load.

## Steps to perform

1. Classify the defect into exactly one of:
   - **Eval error** (`nix eval` or `nix build` fails before any build runs; read the error message — `undefined variable`, `infinite recursion encountered`, `value is a function while a set was expected`, `cannot coerce X to a string`, `attribute 'X' missing`);
   - **Build failure** (eval succeeds but the derivation builder returns non-zero — `error: builder for '/nix/store/...drv' failed with exit code N`; inspect with `nix log .#<name>`);
   - **Hash mismatch** (fixed-output derivation hash is wrong — `hash mismatch in fixed-output derivation` with `specified:` and `got:` values);
   - **Purity violation** (sandbox restriction fired — `error: permission denied`, `error: network is unreachable`, or impure path reference caught by `just lint-nix`);
   - **Store issue** (store growth, GC root leak, stale `result*` symlinks, `nix: command not found`, dangling profile — diagnose with `just store-audit` and `just gc`).
2. Load skills and docs matching the category:
   - Eval errors → `nix-language` + `docs/nix/error-handling-and-debugging.md` (common eval errors table); use `nix repl` or `nix eval .#<attr>` to inspect attribute paths; use `builtins.trace` / `lib.debug.traceVal` to instrument evaluation.
   - Build failures → `nix-derivations` + `docs/nix/error-handling-and-debugging.md` (common build errors table); inspect with `nix log .#<name>`, `nix build .#<name> -L`, `nix develop .#<name>` to reproduce interactively; use `--keep-failed` to inspect the failed build directory.
   - Hash mismatches → `nix-usage` + `docs/nix/error-handling-and-debugging.md` (hash mismatch section); use `just update-hashes` to prefetch the real hash; never guess the hash manually.
   - Purity violations → `constraint-nix-purity` + `docs/nix-purity.md`; run `just lint-nix` to identify the violating pattern; consult the anti-accumulation patterns table in `nix-usage`.
   - Store issues → `nix-store-gc` + `docs/nix/store-hygiene-and-gc.md`; run `just store-audit` to identify stale roots and `just gc` to collect.
3. Load `constraint-nix-reproducibility` if the defect is a hash mismatch or flake input pinning issue.
4. Identify the root cause: write a one-paragraph explanation of why the defect occurred. The root cause must explain the mechanism, not just restate the symptom.

## Docs to consult

By category:
- `docs/nix/error-handling-and-debugging.md` (all categories — canonical error reference);
- `docs/nix-purity.md` (purity violations);
- `docs/nix/store-hygiene-and-gc.md` (store issues);
- `docs/nix/language-fundamentals.md` (eval errors — `rec`, `let`, undefined variable);
- `docs/nix/derivations-and-builds.md` (build failures — `mkDerivation`, build phases).

## Operational skills to load

Conditional by category:
- `nix-language` (eval errors);
- `nix-derivations` (build failures);
- `nix-usage` (hash mismatches, project failure modes);
- `nix-store-gc` (store issues).

## Constraints to apply

- `constraint-nix-scope-discipline` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `--impure`, `--fallback`, or hash-guessing workarounds. Diagnosis only; do not begin fixing.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 03-fix`, `next_workflow: null`. Record the defect category and the root-cause paragraph in `evidence` (or `assumptions` if the root cause is provisional). Because the continuation policy is `suggest-next`, surface the category and root cause for confirmation before phase `03-fix` is loaded.
