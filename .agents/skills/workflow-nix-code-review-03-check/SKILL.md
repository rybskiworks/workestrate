---
name: workflow-nix-code-review-03-check
description: |
  Use only for the check phase of the Nix code-review workflow. Run the
  automated gates (just lint-nix, nix flake check, nix build) and record
  pass/fail. Do not use for scoping, analysis, manual review, or issuing a
  verdict.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-code-review
  org.phase: check
  org.phase_order: "03"
---

# Phase 03: check (Nix code review)

## Phase purpose

Run the automated gates (`just lint-nix`, `nix flake check`, `nix build`) and
record their pass/fail status. A clean lint is the baseline; a lint failure is
a blocker that stops the workflow. The nix-gated checks require nix on the
host; if nix is unavailable, run the in-container gate and mark the rest
`not_fully_checkable`.

## Steps to perform

1. Run `just lint-nix` (backed by `scripts/check-nix-paths.sh`). This is the
   in-container purity gate and runs without nix. A failure is a blocker:
   report it, set `outcome: fail`, and stop — do not proceed to 04-review
   against code that violates purity invariants. The guard catches: (1)
   `nix ... --impure`, (2) `builtins.getFlake` + `toString` on the same line,
   (3) `builtins.path { ... }` without `filter =`, (4) `cleanSourceWith { ... }`
   without `filter =`, (5) `../` path literals outside `src =`/`lockFile =`/
   `path =` escape-hatch fields. Review any `# allow: <reason>` allowlist
   comments for justification.
2. Run `nix flake check --no-build` (evaluation gate; does not build checks
   derivations). If nix is unavailable on the host, skip and record in
   `not_fully_checkable` with reason "nix not on PATH". An eval failure is a
   blocker.
3. Run `nix flake check` (full check: evaluates and builds all `checks`
   derivations). If nix is unavailable, skip and record in
   `not_fully_checkable`. A failure is a blocker.
4. Run `nix build .#<package>` for each package output touched by the diff
   (e.g. `nix build .#workestrate`). If nix is unavailable, skip and record in
   `not_fully_checkable`. A build failure is a blocker. Use
   `--no-link --print-out-paths` to avoid leaving `result` symlinks (store
   hygiene).
5. Run the validation skills: `validation-nix-lint`,
   `validation-nix-flake-check`, `validation-nix-build`, `validation-nix-test`,
   `validation-nix-supply-chain` (if flake.lock changed). Each validation skill
   wraps the corresponding command above and records structured pass/fail
   evidence.
6. Apply `constraint-nix-scope-discipline`: gate failures are reported only
   for the diff under review; do not request fixes for pre-existing failures
   in untouched code (record them as follow-ups).

## Docs to consult

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/testing.md`
- `docs/nix/validation.md`
- `docs/nix/store-hygiene-and-gc.md`

## Operational skills to load

- `nix-testing`
- `nix-store-gc` (for `--no-link` discipline awareness)

## Constraints to apply

- `constraint-nix-scope-discipline` — gate failures are reported only for the
  diff under review; do not request fixes for pre-existing failures in
  untouched code (record them as follow-ups).
- `constraint-nix-store-hygiene` — use `--no-link --print-out-paths` for build
  gates; do not leave `result*` symlinks that pin closures.

## Validations to run

- `validation-nix-lint` — `just lint-nix` (in-container, no nix required)
- `validation-nix-flake-check` — `nix flake check` (HOST-GATE: requires nix)
- `validation-nix-build` — `nix build .#<package>` (HOST-GATE: requires nix)
- `validation-nix-test` — `nix flake check` checks derivations (HOST-GATE)
- `validation-nix-supply-chain` — flake input pinning audit (if flake.lock changed)

## Handoff output

Return the handoff YAML block per the schema in
`workflow-nix-code-review-00-orchestration`. Set:

- `outcome`: `pass` if all available gates are green; `fail` if any gate
  failed (especially lint or eval); `partial` if a nix-gated check was skipped
  because nix is unavailable (record in `not_fully_checkable`).
- `constraints_applied`: `constraint-nix-scope-discipline`,
  `constraint-nix-store-hygiene`.
- `tests_run`: one entry per gate with command and pass/fail, e.g.
  `just lint-nix: pass`, `nix flake check --no-build: pass`,
  `nix flake check: pass`, `nix build .#workestrate --no-link: pass`.
- `blockers`: any gate failure, with the failing command.
- `assumptions`: any skipped gate and the reason (e.g. "nix not on PATH").
- `next_phase`: `04-review` (or `null` if a blocker stopped the workflow).
- `next_workflow`: `null`.
