---
name: workflow-nix-debugging-01-reproduce
description: |
  Use only for the reproduce phase of the Nix debugging workflow.
  Reproduce the issue reliably and capture the exact command, input,
  environment, and full eval or build error output. Do not use for
  diagnosis, fixing, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-debugging
  org.phase: reproduce
  org.phase_order: "01"
---

## Phase purpose

Reproduce the issue reliably and capture the exact command, input, environment, and full eval or build error output. This phase is reproduction only — do not begin diagnosis or fixing here.

## Steps to perform

1. Reproduce the issue reliably. Capture:
   - the exact command that triggers the defect (`nix build .#<name>`, `nix eval .#<attr>`, `nix develop .#<name>`, `nix flake check`, etc.);
   - the input that triggers it (flake output name, attribute path, system);
   - the environment (Nix version, flake state, working directory, whether inside the devshell (`just shell`), `HOME`/`TMPDIR` settings, git tracking state of new files);
   - the full error message (eval error with file/line/column, build failure with exit code, hash mismatch with expected/got hashes, or wrong-output observation).
2. If the issue cannot be reproduced, record what is known and what reproduction attempts were made. Do NOT proceed to a fix on an unreproducible report. Set `outcome: fail` and `blockers: ["issue not reproduced"]` in the handoff.
3. Apply `constraint-nix-scope-discipline`: this phase is reproduction only. Do not start fixing adjacent issues observed while reproducing; record them as follow-ups.
4. Stage new files (`git add -N`) before eval — untracked files are invisible to `.#` refs (classic flakes gotcha).

## Docs to consult

None mandatory in this phase. Docs are loaded in phase `02-diagnose` based on the defect category.

## Operational skills to load

None mandatory in this phase.

## Constraints to apply

- `constraint-nix-scope-discipline` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `--impure`, `--fallback`, or hash-guessing workarounds. This phase is reproduction only.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 02-diagnose`, `next_workflow: null`, `handoff_requires_hil: false`. Record the reproduction command and the original error output in `evidence` (or `assumptions` if the reproduction is partial). If reproduction failed, set `outcome: fail`, `blockers: ["issue not reproduced"]`, and stop.
