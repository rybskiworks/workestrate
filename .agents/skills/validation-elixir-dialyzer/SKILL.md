---
name: validation-elixir-dialyzer
description: |
  Verifies Elixir code passes Dialyzer success-typing analysis. Load after
  compile and credo validation pass. Does NOT cover compilation errors (see
  validation-elixir-compile) or lint (see validation-elixir-credo).
metadata:
  org.kind: validation
---

# Validation: Elixir Dialyzer

This gate verifies that the code passes Dialyzer's success-typing analysis via
Dialyxir. It runs after `validation-elixir-compile` and `validation-elixir-credo`
pass. Dialyzer is sound (no false positives): if it reports a problem, the code
really can fail.

## Triggers

Load this skill when:

- After `validation-elixir-compile` and `validation-elixir-credo` pass.
- As part of the standard validation suite.
- When `@spec`/`@type`/`@callback` annotations change.

## Command

```bash
mix dialyzer --plt   # build/update the PLT (cache in CI; exits 0)
mix dialyzer          # run the analysis
```

Run `--plt` once to warm the cache, then `mix dialyzer` to analyse.

## Pass criteria

- Exit code 0 for `mix dialyzer`.
- No Dialyzer warnings (e.g. `:no_return`, `:pattern_match`, `:unknown_function`,
  `:contract_*`, `:unmatched_return`).

## Fail criteria

- Exit code non-zero for `mix dialyzer`.
- Dialyzer warnings reported (warning atom, file, line, message).

## Evidence to report

- Exit code for each command.
- Warning count.
- Specific warnings (warning atom, file, line, message).

## Notes

- Dialyzer uses success typings (a conservative approximation); it has no false
  positives but can have false negatives.
- The PLT (Persistent Lookup Table) caches the analysis of OTP/Elixir
  stdlib/deps; cache it in CI keyed on OTP+Elixir version and `mix.lock`. The
  first run is slow.
- `:unknown_function` usually means an incomplete PLT — add the app via
  `plt_add_apps`/`plt_add_deps` in `mix.exs`, do not just ignore it.
- `mix dialyzer` halts the VM with a non-zero exit status when warnings are
  found; use `--ignore-exit-status` ONLY for advisory "show warnings" jobs. Do
  NOT use the removed `--halt-exit-status`.
- Do NOT use removed/deprecated flags: `:race_conditions` (removed 1.3.0),
  `--halt-exit-status` (removed 1.0.0-rc.7), `plt_add_deps: :transitive`/
  `:project` (deprecated; use `:app_tree`/`:apps_direct`).
- For new projects, treat Dialyzer as advisory until the baseline is clean, then
  promote it to a gate.
- Run inside `just shell` (see `nix-usage` skill).
