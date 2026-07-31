---
name: workflow-nix-packaging-05-verify
description: |
  Use only for the verify phase of the Nix packaging workflow. Run
  nix flake check, just lint-nix, just verify, and confirm purity. Do not use
  for scoping, design, implementation, or test writing.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-packaging
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Nix packaging)

## Phase purpose

Run `nix flake check`, `just lint-nix`, `just verify`, and confirm purity.
This is the validation phase; it does not introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from the first gate:
   - `nix flake check --no-build` (eval + checks; `--no-build` skips building
     check derivations for speed — drop `--no-build` for full check)
   - `nix build .#<name>` (full build, confirms FOD hashes are correct)
   - `just lint-nix` (static purity guard: checks 1-6 — `--impure`,
     `getFlake`+`toString`, `builtins.path` without `filter`,
     `cleanSourceWith` without `filter`, `../` path literals outside
     escape-hatch fields)
   - `just verify` (full repo gate suite: toolchain-check, check, test,
     lint-nix, store-audit, etc.)
2. Confirm purity:
   - No `src = ./.` without a `filter` (rule 1 in `docs/nix-purity.md` —
     `just lint-nix` does NOT catch bare `src = ./.`).
   - No `--impure` anywhere in the derivation or wrapping scripts.
   - No `lib.fakeHash`/`lib.fakeSha256`/empty hash committed (real
     `sha256-...` only).
   - No network in `buildPhase`/`installPhase` (all deps via FODs).
   - No `builtins.getFlake` + `toString` on this repo.
   - No `../` path literals outside `src =`/`lockFile =`/`path =` escape-hatch
     fields.
   - `pname` + `version` set (not bare `name`).
   - `meta.platforms` set.
   - `runHook preXxx`/`runHook postXxx` present in any custom phase.
3. Confirm store hygiene:
   - No stale `result*` symlinks left from testing.
   - `just store-audit` does not flag new `*-source` paths exceeding the 50M
     threshold (impure-path probe).
4. Run `just gc` if the build churned significant store paths (new
   nixpkgs-dependent builds pull large closures).
5. Document: the packaging surface summary (from phase 01), files
   added/modified, FOD hashes computed, raw output/pass-fail of each gate, and
   any deferred decisions.
6. Report: if `nix` is not available on the host (HOST-GATE), mark the
   build/flake-check gates as `not_fully_checkable` with the reason;
   `just lint-nix` runs in-container without nix and can still pass.

## Docs to consult

- `docs/nix-purity.md`
- `docs/nix/testing.md`
- `docs/nix/flake-anatomy.md`

## Operational skills to load

None required; the validation skills execute the gates. Load
`nix-packaging-recipes` if interpretation of a build failure or hash mismatch
is needed.

## Constraints to apply

- `constraint-nix-purity` — verify all purity invariants listed above.
- `constraint-nix-reproducibility` — verify real FOD hashes, pinned flake
  inputs.
- `constraint-nix-sandbox-safety` — verify no network in build phases.
- `constraint-nix-store-hygiene` — verify no stale GC roots, `--no-link`
  used.
- Stay within the captured requirements. Fix only the new code's contribution
  to any gate failure; surface pre-existing issues as follow-ups.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-nix-flake-check` — `nix flake check` (eval + checks)
- `validation-nix-build` — `nix build .#<name>` (full build, confirms FOD
  hashes)
- `validation-nix-lint` — `just lint-nix` (static purity guard, runs
  in-container)
- `validation-nix-eval` — `nix eval .#<name>.drvPath` (confirms drv
  instantiation; optional sanity check)

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-packaging-00-orchestration`, extended with the verification-phase
extra fields. Set:

- `outcome` to `pass` only when every gate passes and purity is confirmed.
- `constraints_applied` to include `constraint-nix-purity`,
  `constraint-nix-reproducibility`, `constraint-nix-sandbox-safety`,
  `constraint-nix-store-hygiene`, and the scope-discipline principle.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each gate.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. runtime behavior of the packaged binary,
  foreign-function correctness — covered by phase 04 output checks and human
  review).
- `next_phase: null` — this is the terminal phase.
- `next_workflow: null` — no dedicated nix code-review workflow exists yet;
  human review of the derivation recipe and `flake.nix` wiring is recommended
  before merge (the evaluator cannot verify runtime correctness of the
  packaged binary).

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-sandbox-safety
  - constraint-nix-store-hygiene
  - stay within captured requirements (scope discipline)
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-nix-flake-check
    gate: nix flake check
    result: pass|fail
  - validation: validation-nix-build
    gate: nix build .#<name>
    result: pass|fail
  - validation: validation-nix-lint
    gate: just lint-nix
    result: pass|fail
  - validation: validation-nix-eval
    gate: nix eval .#<name>.drvPath
    result: pass|fail
constraints_checked:
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-sandbox-safety
  - constraint-nix-store-hygiene
  - stay within captured requirements (scope discipline)
evidence:
  - gate: nix flake check
    output: ...
  - gate: nix build .#<name>
    output: ...
  - gate: just lint-nix
    output: ...
  - gate: nix eval .#<name>.drvPath
    output: ...
failures: []
not_fully_checkable:
  - "Runtime behavior of the packaged binary and foreign-function correctness
    cannot be verified by the Nix evaluator; covered by phase 04 output checks
    and human review of the derivation recipe and flake.nix wiring before
    merge."
```
