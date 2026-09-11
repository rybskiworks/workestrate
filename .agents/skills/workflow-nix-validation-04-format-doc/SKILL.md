---
name: workflow-nix-validation-04-format-doc
description: |
  Use only for the format and docs phase of the Nix validation workflow.
  Run nix fmt --check (if configured) and verify docs cross-refs. Do not
  use for scoping, compile, or testing.
allowed-tools: Read Bash(nix:*) Bash(grep:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-validation
  org.phase: format-doc
  org.phase_order: "04"
---

# Nix Validation Workflow — Phase 04 — Format & Docs

## Phase Purpose

Run the format gate and verify docs cross-refs. Record pass/fail and evidence
per gate. This project does NOT configure a Nix formatter (`nix fmt` /
`nixpkgs-fmt` / `alejandra`) — mark the format gate "not configured". Docs
cross-refs are verified manually / via `grep`.

## Steps

1. Run the format gate:
   ```sh
   nix fmt --check
   ```
   This project does NOT configure `nix fmt` / `nixpkgs-fmt` / `alejandra`. Mark the format gate "not configured" and record the reason. Map to `validation-nix-format`. Do NOT attempt to run the command if no formatter is configured.
2. Verify docs cross-refs:
   ```sh
   grep -r "docs/nix/" .agents/skills/ docs/ --include="*.md" -l
   ```
   Check that `docs/nix/*.md` references in skills/docs are valid (the referenced files exist). Broken cross-refs are a blocker. Record pass/fail and the list of broken refs or clean.
3. For each gate, record pass/fail and the relevant output (diff or clean; "not configured" for format; broken refs or clean for docs cross-refs).
4. If a gate fails, continue to phase 05 to give a full picture; the overall result will be fail.
5. Do NOT modify code, flake config, or docs to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/nix/validation.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/flake-anatomy.md`

## Operational Skills to Load

- `nix-usage` — formatter configuration status.

## Constraints to Apply

- `constraint-nix-scope-discipline` — do not modify code, flake config, or docs to make a gate pass.

## Validations to Run

- `validation-nix-format` — command: `nix fmt --check` (mark "not configured" — this project does not configure a Nix formatter)
- Docs cross-refs — manual / `grep` verification that `docs/nix/*.md` references are valid

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any gate red) | partial (format gate not configured).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: `["nix fmt --check (not configured)", "docs cross-refs grep verification"]` with pass/fail + evidence per gate (mark format "not configured").
- `risks`: any gate red; format gate not configured (follow-up to configure a formatter).
- `next_phase`: `05-report`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
