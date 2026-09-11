---
type: Workflow
resource: https://nix.dev/
title: Nix Validation Workflow
description: Step-by-step workflow for running the full Nix validation gate suite — scope, compile, lint-test, format-doc, report.
tags: [nix, workflow, validation]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Validation Workflow

## Purpose

Process for running the full validation gate suite on Nix code: flake check,
eval, lint, tests, format, and supply chain. The workflow reports pass/fail
per gate with evidence. It is the final gate before merge/release. This
workflow IS the validation — it runs all gates and reports; it does not modify
code.

## When to use

Use this workflow as the final validation before merge or release, or as a
periodic health gate on the whole flake. Do not use it as a substitute for
`implementation.md`, `code-review.md`, `refactoring.md`, or `debugging.md`;
run it after one of those workflows has produced code. If a gate fails, hand
off to the appropriate workflow (`debugging.md` for a defect,
`implementation.md` for missing checks) to fix, then re-run validation.

## Order of operations

Run each gate in order. Record pass/fail and the relevant output for each. If
a gate fails, you may continue to the remaining gates to give a full picture,
but the overall result is fail.

### 1. Flake check

```sh
nix flake check
```

`nix flake check` evaluates all outputs and runs `checks`. A failure here is a
blocker. Consult `docs/nix/flake-anatomy.md` and the
`.agents/skills/validation-nix-flake-check/SKILL.md` skill.

### 2. Eval check

```sh
nix eval .#workestrate.meta.description
nix flake show
```

`nix eval` confirms key attribute paths evaluate; `nix flake show` lists all
outputs. A failure here is a blocker. Consult
`.agents/skills/validation-nix-eval/SKILL.md`.

### 3. Lint check

```sh
just lint-nix
```

`just lint-nix` runs the repo's Nix linter (statix / deadnix / nixfmt as
configured). Any warning treated as error is a blocker. Consult
`docs/nix/conventions-and-style.md` and the
`.agents/skills/validation-nix-lint/SKILL.md` skill.

### 4. Test suite

```sh
nix build .#checks.x86_64-linux.<name>
```

Build the `checks` outputs directly to confirm each check builds and passes.
All checks must pass. Consult `docs/nix/testing.md` and the
`.agents/skills/validation-nix-test/SKILL.md` skill.

### 5. Store audit

```sh
just store-audit
```

`just store-audit` confirms no `result*` symlink leaks and GC roots are clean.
A failure here is a blocker. Consult `docs/nix/store-hygiene-and-gc.md`.

### 6. Format check

```sh
nix fmt --check
```

Formatting must be clean across the flake (only if `nix fmt` is configured;
otherwise record "not configured" and skip). Consult
`docs/nix/conventions-and-style.md` and the
`.agents/skills/validation-nix-format/SKILL.md` skill.

### 7. Supply chain

```sh
just store-audit
```

Confirm flake inputs are pinned and no unpinned `builtins.fetch*` calls exist.
Consult `docs/nix/supply-chain-security.md` and the
`.agents/skills/validation-nix-supply-chain/SKILL.md` skill.

### 8. Report

Report pass/fail per gate with evidence for each.

## Docs consulted

- `docs/nix/flake-anatomy.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/testing.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/supply-chain-security.md`

## Skills loaded

- `.agents/skills/validation-nix-flake-check/SKILL.md`
- `.agents/skills/validation-nix-eval/SKILL.md`
- `.agents/skills/validation-nix-lint/SKILL.md`
- `.agents/skills/validation-nix-test/SKILL.md`
- `.agents/skills/validation-nix-format/SKILL.md`
- `.agents/skills/validation-nix-supply-chain/SKILL.md`

## Commands run (in order)

```sh
nix flake check
nix eval .#workestrate.meta.description
nix flake show
just lint-nix
nix build .#checks.x86_64-linux.<name>
just store-audit
nix fmt --check
```

## Evidence to report

A per-gate table:

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Flake check | `nix flake check` | pass/fail | error count or clean |
| Eval | `nix eval .#workestrate.meta.description` | pass/fail | value or error |
| Flake show | `nix flake show` | pass/fail | output list or error |
| Lint | `just lint-nix` | pass/fail | warning count or clean |
| Tests | `nix build .#checks.x86_64-linux.<name>` | pass/fail | built path or error |
| Store audit | `just store-audit` | pass/fail | leak count or clean |
| Format | `nix fmt --check` | pass/fail/N-A | diff or clean |
| Supply chain | input pinning audit | pass/fail | unpinned input count or clean |

Plus an overall verdict: pass (all applicable gates green) or fail (any
applicable gate red), with the list of failing gates.

## When human judgment is needed

- Deciding whether a supply-chain advisory is exploitable in this repo's usage
  (consult `docs/nix/supply-chain-security.md` and the
  `validation-nix-supply-chain` skill; escalate if unclear).
- Deciding whether a `nix flake check` failure caused by an environment
  dependency (for example, a missing builder) is a blocker or an environment
  issue.
- Choosing whether to block on an unpinned input that the repo policy does not
  explicitly address.
- Deciding whether a store-audit `result*` leak is a blocker or a stale
  artifact.
- Judging whether a format check skip (no `nix fmt` configured) is acceptable
  for a given change.

## Avoiding scope creep

- Validation is a gate, not a fix. If a gate fails, record the failure and hand
  off to the appropriate workflow (`debugging.md`, `implementation.md`,
  `refactoring.md`); do not fix in the validation pass.
- Do not add tests, checks, or lint suppressions during validation; those are
  implementation changes that must go through their own workflow and re-run
  validation.
- Do not change `flake.nix`, `nix/devshells/default.nix`, or lint config to
  make a gate pass; policy changes are separate decisions.
- Run only the gates applicable to the flake. If a gate is not configured (for
  example, no `nix fmt`), record "not applicable" or "not configured" rather
  than setting it up as part of validation.
- Keep the report to the per-gate table and overall verdict; do not expand it
  into a code review or implementation summary.
