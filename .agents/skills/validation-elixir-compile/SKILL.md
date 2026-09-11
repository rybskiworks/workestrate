---
name: validation-elixir-compile
description: |
  Verifies Elixir code compiles without warnings. Load as the minimum validation
  gate after any code change. Does NOT cover lint (see validation-elixir-credo),
  type checking (see validation-elixir-dialyzer), formatting (see
  validation-elixir-format), or tests (see validation-elixir-test).
metadata:
  org.kind: validation
---

# Validation: Elixir Compile

This is the minimum validation gate. It verifies that the project compiles with
warnings treated as errors. It does not run Credo, Dialyzer, formatting, or
tests — those are separate gates.

## Triggers

Load this skill when:

- After any Elixir code change, before claiming completion.
- As the first gate in a validation suite (before credo, dialyzer, format, test).
- When diagnosing whether a failure is a compile error vs. a lint/type/test
  failure.

## Command

```bash
mix compile --warnings-as-errors
```

## Pass criteria

- Exit code 0.
- No compiler errors or warnings (warnings are treated as errors).

## Fail criteria

- Exit code non-zero.
- Compiler errors or warnings reported.

## Evidence to report

- Exit code.
- Warning/error count.
- First few messages (file, line, message).

## Notes

- `--warnings-as-errors` promotes warnings to failures; do not run the compile
  gate without it.
- This is the minimum gate; it does not replace credo, dialyzer, format, or test.
- Run inside `just shell` (see `nix-usage` skill) so the correct Erlang/Elixir
  toolchain is used.
- For umbrella projects, `mix compile` compiles all apps; `--warnings-as-errors`
  applies across the umbrella.
- Malformed typespecs often surface as compiler warnings here.
