---
name: validation-elixir-test
description: |
  Verifies all Elixir tests pass (ExUnit). Load after compile, credo, and
  dialyzer validation pass. Does NOT cover compile or lint validation.
metadata:
  org.kind: validation
---

# Validation: Elixir Test

This gate verifies that all ExUnit tests pass. It runs after
`validation-elixir-compile`, `validation-elixir-credo`, and
`validation-elixir-dialyzer` pass.

## Triggers

Load this skill when:

- After `validation-elixir-compile` (and ideally credo/dialyzer) pass.
- As part of the standard validation suite.
- After changing code that affects behavior, fixtures, or doctests.

## Command

```bash
mix test
mix test --cover
```

Run `mix test` for the test suite; add `--cover` when coverage is required.

## Pass criteria

- All tests pass.
- Exit code 0.

## Fail criteria

- Any test fails.
- Exit code non-zero.

## Evidence to report

- Exit code.
- Passed / failed / skipped counts.
- Failing test names and their output (assertion message, file, line).

## Notes

- ExUnit runs tests concurrently by default; tag tests `async: false` when they
  share mutable state (e.g. a shared ETS table or a named process).
- Doctests (`doctest Module` in a test module) run as part of `mix test`; a
  doctest failure is distinguishable by its `doctest` location.
- `--cover` generates a coverage report; configure the adapter in `mix.exs`
  (`test_coverage: [tool: ExCoveralls]` etc.).
- For flaky/timing tests, run with `--seed 0` (deterministic) or repeat to
  surface races.
- For a single file: `mix test path/to/file_test.exs`; for a single test:
  `mix test path/to/file_test.exs:line`.
- Run inside `nix develop` (see `nix-usage` skill).
