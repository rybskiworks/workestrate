---
name: workflow-nix-validation-01-scope
description: |
  Use only for the scope phase of the Nix validation workflow.
  Identify what changed (git diff) and determine which gates apply
  (nix on host? formatter configured? store-delta-check applicable?).
  Do not use for compile, test, format, doc, supply-chain, or final
  reporting.
allowed-tools: Read Bash(git:*) Bash(grep:*) Bash(find:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-validation
  org.phase: scope
  org.phase_order: "01"
---

# Nix Validation Workflow — Phase 01 — Scope

## Phase Purpose

Identify what changed and determine which gates apply. Record applicable vs
not-applicable gates so phases 02-04 run only the relevant gates and mark the
rest "not applicable" or "not configured". Critically, determine whether the
HOST-GATE (nix on host) is satisfied — most Nix gates require nix on the host,
which this container may not have.

## Steps

1. Identify what changed:
   ```sh
   git diff --stat
   git diff --name-only
   ```
2. Determine gate applicability:
   - **HOST-GATE (nix on host)**: check whether `nix` is on PATH. Most Nix gates (`nix flake check`, `nix eval`, `nix build`, `nix flake show`, `just store-audit`) require nix on the host. If nix is absent, mark those gates `not_fully_checkable`. Note: `just lint-nix` runs IN-CONTAINER and requires only `bash`, `awk`, `find` — no nix needed; it can be `validated` in this environment.
   - **Nix formatter**: check whether `nix fmt` / `nixpkgs-fmt` / `alejandra` is configured. This project does NOT configure a Nix formatter — mark the format gate "not configured".
   - **Check derivations**: check whether `checks.<system>` derivations exist. This project has `checks.x86_64-linux.validateConfig` — the test gate applies.
   - **flake.lock changed**: if `flake.lock` changed, the supply-chain gate applies.
   - **Lint applicability**: if `.nix` files, `.sh` scripts, `justfile`, or `docs/**/*.md` changed, the lint-nix gate applies.
3. Record the list of applicable gates and the list of not-applicable/not-configured gates with reasons.
4. Do NOT modify any code, config, or manifest in this phase. Scope only.

## Docs to Consult

- `docs/nix/validation.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix-purity.md`

## Operational Skills to Load

- `nix-usage` — flake outputs, dev shell, runtime context.

## Constraints to Apply

- `constraint-nix-scope-discipline` — validation is a gate, not a fix; do not modify code or config to make a gate applicable/inapplicable.

## Validations to Run

None. This phase identifies which gates apply; it does not run validation gates.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (scope determined) | partial (some gates' applicability unclear).
- `files_touched`: `[]` (scope only; no changes).
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `assumptions`: the list of applicable gates and the list of not-applicable/not-configured gates with reasons; the changed-files list; the HOST-GATE status (nix present/absent).
- `risks`: nix absent on host (most gates `not_fully_checkable`); format gate not configured; `flake.lock` changed (supply-chain gate applies).
- `tests_run`: `["git diff --stat", "git diff --name-only"]`.
- `next_phase`: `02-compile`.
- `next_workflow`: `null`.
- `blockers`: any gate whose applicability could not be determined.
