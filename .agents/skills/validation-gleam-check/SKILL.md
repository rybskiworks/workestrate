---
name: validation-gleam-check
description: |
  Verifies Gleam code type-checks without errors. Load as the minimum validation
  gate after any code change. Gleam is statically typed, so this is the
  type+compile gate; no Dialyzer is needed. Does NOT cover formatting (see
  validation-gleam-format) or tests (see validation-gleam-test).
metadata:
  org.kind: validation
---

# Validation: Gleam Check

This is the minimum validation gate. Gleam is statically typed, so `gleam check`
performs full type-checking and exhaustiveness verification without code
generation (faster than `gleam build`). It does not run formatting or tests.

## Triggers

Load this skill when:

- After any Gleam code change, before claiming completion.
- As the first gate in a validation suite (before format, test).
- When diagnosing whether a failure is a type error vs. a test failure.

## Command

```bash
gleam check
```

## Pass criteria

- Exit code 0.
- No type errors and no missing/redundant patterns (exhaustiveness is enforced).

## Fail criteria

- Exit code non-zero.
- Type errors or exhaustiveness errors reported.

## Evidence to report

- Exit code.
- Error count.
- Specific errors (file, line, message).

## Notes

- Gleam is statically typed; `gleam check` is the type+compile gate. There is no
  separate Dialyzer step for Gleam (unlike Elixir/Erlang).
- `gleam check` type-checks without codegen; use `gleam build` when you need
  compiled artifacts or to validate a specific target.
- The compiler enforces `snake_case`/`PascalCase` and exhaustiveness; most
  other conventions are review-enforced (see `constraint-gleam-conventions`).
- Run inside `nix develop` (see `nix-usage` skill) so the correct Gleam toolchain
  is used.
