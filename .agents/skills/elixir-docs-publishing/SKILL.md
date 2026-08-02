---
name: elixir-docs-publishing
description: |
  Operational guide for Elixir documentation and Hex publishing — @moduledoc/@doc/@typedoc
  patterns, ExDoc configuration, doctests, Hex publish/retire workflow. Load when writing
  module documentation, configuring ExDoc, or publishing to Hex. Does NOT cover typespec
  syntax (see elixir-static-analysis) or testing patterns (see elixir-testing).
---

# Documentation and Hex Publishing

## Triggers

Load this skill when:

- Writing or reviewing `@moduledoc`/`@doc`/`@typedoc`.
- Adding doctests.
- Configuring ExDoc.
- Publishing or retiring Hex packages.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/documentation-and-publishing.md`
  - https://hexdocs.pm/elixir/writing-documentation.html
  - https://hexdocs.pm/elixir/Module.html
  - https://hexdocs.pm/ex_unit/ExUnit.DocTest.html
  - https://hexdocs.pm/ex_doc/readme.html
  - https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html
  - https://hexdocs.pm/ex_doc/ExDoc.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.User.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Build.html
  - https://hex.pm/docs/publish

## Key Rules

- Documentation is a first-class contract. Public modules, functions, macros,
  callbacks, and types MUST have `@moduledoc`/`@doc`/`@typedoc`. Private
  functions should NOT have `@doc` (compiler warns and discards).
- `@moduledoc`: string/heredoc, `false` (hides from ExDoc but does NOT make
  module private), or keyword metadata (`since:`, `authors:`). Place inside
  `defmodule`, before functions.
- `@doc`: place immediately before the FIRST clause of multi-clause functions
  (docs are per name/arity, not per clause). String/heredoc, `false`, or
  metadata (`since:`).
- `@typedoc`: documents `@type`/`@opaque`. Place before the type definition.
- `@deprecated "message"` marks deprecated functions;
  `@moduledoc deprecated: "..."` for modules.
- Use `~S"""` heredocs for docs to avoid interpolation of `#{}` in example
  code.
- Doctests: `doctest Module` in a test module runs examples in
  `@doc`/`@moduledoc`. Options: `:only`, `:except`, `:import`, `:async`. Use
  `...>` for multiline continuation. Keep doctests reproducible (no
  timestamps/random).
- ExDoc: add `{:ex_doc, "~> 0.30", only: :dev, runtime: false}`. Configure in
  `mix.exs` `project/0`:
  `docs: [main: "MyApp", extras: ["README.md"], groups_for_modules: [...], source_ref: "v1.0.0"]`.
  Run `mix docs`.
- Hex publish workflow:
  1. Authenticate: `mix hex.user auth`.
  2. Ensure `:package` metadata in `mix.exs` `project/0`:
     `package: [licenses: ["MIT"], links: %{"GitHub" => "..."}]`.
  3. `mix hex.publish` — builds tarball + docs, uploads. Packages are IMMUTABLE
     once published (cannot overwrite a version; must publish a new version).
  4. `mix hex.publish --revert VERSION` — revert within 24h (if no dependents).
- Retirement: `mix hex.retire PACKAGE VERSION REASON`. Reasons: `:security`,
  `:deprecated`, `:invalid`, `:renamed`.
- `mix hex.build` builds tarball without publishing; `mix hex.audit` checks
  for retired deps.
- Documentation vs code comments: `@doc`/`@moduledoc`/`@typedoc` for API
  consumers (extracted by tools); `#` comments for implementers.

## Quick Commands

```bash
mix docs                                   # generate ExDoc HTML
mix hex.user auth                          # authenticate with Hex
mix hex.build                              # build tarball (no publish)
mix hex.publish                            # publish package + docs
mix hex.publish --revert 1.0.0             # revert within 24h
mix hex.retire my_pkg 1.0.0 security      # retire a version
mix hex.audit                              # check for retired deps
```

## Anti-patterns

- `@doc` on private functions (compiler warns/discards).
- `@moduledoc false` thinking it makes a module private (it only hides docs).
- `@doc` on individual clauses of multi-clause functions (place before first
  clause only).
- Publishing without `:package` metadata (`:licenses`, `:links`).
- Doctests with non-reproducible output.
- Using `"""` instead of `~S"""` for docs with `#{}` in examples (unwanted
  interpolation).
- Forgetting to retire deprecated/insecure package versions.
- Publishing over an existing version (impossible — Hex is immutable).

## Related Skills

- elixir-testing
- elixir-static-analysis
- elixir-project-setup
