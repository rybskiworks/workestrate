---
name: validation-elixir-credo
description: |
  Verifies Elixir code passes Credo lint checks. Load after compile validation
  passes. Does NOT cover compilation errors (see validation-elixir-compile),
  type checking (see validation-elixir-dialyzer), or formatting (see
  validation-elixir-format).
metadata:
  org.kind: validation
---

# Validation: Elixir Credo

This gate verifies that the code passes Credo's lint suite with all priorities
enabled. It runs after `validation-elixir-compile` passes and before Dialyzer.

## Triggers

Load this skill when:

- After `validation-elixir-compile` passes.
- As part of the standard validation suite.
- When diagnosing whether a failure is a lint violation vs. a compile/type/test
  failure.

## Command

```bash
mix credo --strict
```

## Pass criteria

- Exit code 0.
- No Credo issues (all priorities included via `--strict`).

## Fail criteria

- Exit code non-zero.
- Credo issues reported (check name, file, line, priority, message).

## Evidence to report

- Exit code.
- Issue count.
- Specific checks triggered (check name, file, line, priority, message).

## Notes

- `--strict` (`--all-priorities`) includes low-priority issues; do not run the
  credo gate without it.
- Credo is a community tool, not an official Elixir tool; it is a
  teaching/consistency linter, NOT a formatter (use `mix format` for
  formatting).
- Check `.credo.exs` (generated via `mix credo.gen.config`) for configured
  checks, priorities, and `strict: true`.
- Inline directives (`# credo:disable-for-next-line`, etc.) should be justified,
  not used as a blanket escape hatch.
- Credo exit statuses are a bitmask per category; `>= 128` indicates a runtime
  error.
- Run inside `just shell` (see `nix-usage` skill).
