---
name: elixir-static-analysis
description: |
  Operational guide for Elixir static analysis — Credo linting and Dialyzer/Dialyxir
  type checking, plus typespec patterns. Load when running linters/type checkers,
  configuring CI quality gates, or writing @spec/@type annotations. Does NOT cover
  runtime debugging or testing.
---

# Static Analysis: Credo and Dialyzer

## Triggers

Load this skill when:

- Running or configuring Credo or Dialyzer.
- Writing `@spec`/`@type`/`@callback`.
- Setting up CI quality gates.
- Interpreting lint/type warnings or inline disable directives.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/static-analysis-credo.md`
  - https://hexdocs.pm/credo/overview.html
  - https://hexdocs.pm/credo/installation.html
  - https://hexdocs.pm/credo/basic_usage.html
  - https://hexdocs.pm/credo/mix_tasks.html
  - https://hexdocs.pm/credo/cli_switches.html
  - https://hexdocs.pm/credo/config_file.html
  - https://hexdocs.pm/credo/check_params.html
  - https://hexdocs.pm/credo/exit_statuses.html
  - https://hexdocs.pm/credo/config_comments.html
  - https://hexdocs.pm/credo/Credo.Check.html
- `docs/elixir/typespecs-and-dialyzer.md`
  - https://hexdocs.pm/elixir/typespecs.html
  - https://hexdocs.pm/dialyxir/Mix.Tasks.Dialyzer.html

## Key Rules

- Credo: add as `{:credo, "~> 1.7", only: [:dev, :test], runtime: false}`. Run
  `mix credo`.
- Credo categories: `:consistency`, `:design`, `:readability`, `:refactor`,
  `:warning`. Each issue has a priority (low/normal/high); default output hides
  low-priority.
- `mix credo --all` shows all issues (no per-category cap); `mix credo --strict`
  / `--all-priorities` includes low-priority.
- `mix credo.gen.config` generates `.credo.exs`; configure checks via `checks:`
  section, enable/disable, override `priority`/`weight`/`params`; `strict: true`
  enables all.
- CLI switches: `--only`, `--ignore`, `--min-priority`, `--format`,
  `--config-file`, `--verbose`, `--watch`.
- Inline directives: `# credo:disable-for-next-line`,
  `# credo:disable-for-lines:5`, `# credo:disable-for-this-file`.
- Exit statuses: bitmask per category (CI can gate on specific categories);
  `>= 128` = runtime error.
- Typespecs: `@type` (public), `@typep` (private), `@opaque` (public, hidden
  structure), `@spec`, `@callback`, `@macrocallback`, `@typedoc`, `@impl`.
- Place `@spec` immediately above the `def`/`defp`/`defmacro`. Types near top
  of module.
- `@spec name(type1, type2) :: return_type`; named args allowed; multiple specs
  per name/arity; `when` constraints for type variables.
- Built-in types: `term()`, `any()`, `atom()`, `binary()`, `boolean()`,
  `float()`, `integer()`, `list()`, `map()`, `nil`, `no_return()`, `pid()`,
  `port()`, `reference()`, `tuple()`, `keyword()`.
- Map types: `%{key => value}`, `%{key: value_type}`. Literal types: integers,
  atoms, lists, tuples.
- Behaviours: `@behaviour`, `@callback`, `@macrocallback`,
  `@optional_callbacks`.
- Pitfall: `string()` = Erlang charlist, NOT Elixir string — use `String.t()`.
- Dialyxir: `{:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}`. Run
  `mix dialyzer`.
- PLT (Persistent Lookup Table): `mix dialyzer --plt` builds; cache in CI.
  Success-typing analysis (not gradual typing) — finds inconsistencies,
  unreachable code, wrong returns.
- `mix dialyzer` flags: `--plt`, `--halt-exit-status`, `--format`. Add apps to
  `plt_add_apps` in `mix.exs`.
- Common Dialyzer warnings: `:unknown_function` (add to `plt_add_apps`),
  `:no_return`, `:no_match`, `:unmatched_return`, `:improper_list`.
- CI integration: run `mix credo --strict`, `mix dialyzer`,
  `mix compile --warnings-as-errors` as gates (decide per-repo which are gating
  vs advisory).

## Quick Commands

```bash
mix credo                                 # default lint
mix credo --strict                        # all priorities
mix credo --all                           # no per-category cap
mix credo.gen.config                      # generate .credo.exs
mix dialyzer                              # type check
mix dialyzer --plt                        # build PLT
mix dialyzer --halt-exit-status           # CI-friendly exit code
mix compile --warnings-as-errors          # warnings as errors
```

## Anti-patterns

- `string()` typespec (use `String.t()`).
- `@spec` not matching actual function behavior (Dialyzer false
  positives/negatives).
- Running Dialyzer without caching PLT in CI (slow).
- `# credo:disable-for-this-file` as a blanket escape hatch without
  justification.
- Treating Credo as a style formatter (it's a teaching/consistency tool; use
  `mix format` for formatting).
- Ignoring `:unknown_function` warnings instead of adding apps to
  `plt_add_apps`.

## Related Skills

- elixir-coding
- elixir-testing
- elixir-project-setup
