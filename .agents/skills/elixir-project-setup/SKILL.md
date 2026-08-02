---
name: elixir-project-setup
description: |
  Operational guide for Mix projects — mix.exs structure, directory conventions,
  dependencies, aliases, releases, archives, custom tasks. Load when creating or
  modifying Mix projects or building releases. Does NOT cover runtime configuration
  (see elixir-config) or dependency auditing workflows (see docs/elixir/dependencies-and-packages.md).
---

# Mix Project Setup and Build

## Triggers

Load this skill when:

- Creating a new Mix project.
- Modifying `mix.exs`.
- Managing dependencies.
- Building releases.
- Adding custom Mix tasks.
- Configuring compilation/formatting paths.

## References

This skill is an index into the docs corpus. Read the relevant docs file for
full detail; the canonical source URLs are listed in each file's
`## Sources used` section.

- `docs/elixir/mix-project-structure.md`
  - https://hexdocs.pm/mix/Mix.html
  - https://hexdocs.pm/mix/Mix.Project.html
  - https://hexdocs.pm/mix/Mix.Task.html
  - https://hexdocs.pm/mix/Mix.Tasks.Format.html
  - https://hexdocs.pm/mix/Mix.Tasks.Release.html
  - https://hexdocs.pm/mix/Mix.Tasks.Release.Init.html
  - https://hexdocs.pm/mix/Mix.Release.html
  - https://hexdocs.pm/mix/Mix.Tasks.New.html
  - https://hexdocs.pm/mix/Mix.Tasks.Help.html
  - https://hexdocs.pm/mix/Mix.Tasks.Clean.html
  - https://hexdocs.pm/mix/Mix.Tasks.Run.html
  - https://hexdocs.pm/mix/Mix.Tasks.Cmd.html
  - https://hexdocs.pm/mix/Mix.Tasks.Do.html
  - https://hexdocs.pm/mix/Mix.Tasks.Xref.html
  - https://hexdocs.pm/mix/Mix.Tasks.Archive.html
  - https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html
  - https://hexdocs.pm/elixir/introduction-to-mix.html
  - https://hexdocs.pm/elixir/Config.html
  - https://hexdocs.pm/elixir/Config.Provider.html
  - https://hexdocs.pm/elixir/Code.html
  - https://hexdocs.pm/elixir/Version.html
  - https://hexdocs.pm/elixir/OptionParser.html
- `docs/elixir/dependencies-and-packages.md`
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.html
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html
  - https://hexdocs.pm/mix/Mix.Tasks.Deps.Tree.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Audit.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Outdated.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html
  - https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html
- `docs/beam/applications.md` — `.app` resource file, application callback, env precedence, start types.
- `docs/beam/releases.md` — boot scripts, code replacement, `release_handler`, `heart`, appup cookbook.

## Key Rules

- `mix.exs` defines a module `use Mix.Project` with `project/0` (returns
  keyword list: `:app`, `:version`, `:elixir`, `:deps`, `:elixirc_paths`,
  etc.) and `application/0` (returns keyword list:
  `:extra_applications`, `:mod`, `:env`).
- Minimal project: `[app: :my_app, version: "0.1.0"]`. `mix new my_app`
  scaffolds; `--sup` adds a supervision tree; `--umbrella` for umbrella
  projects.
- Directory conventions: `lib/` for compiled source (default
  `elixirc_paths: ["lib"]`); `test/` for `*_test.exs`; `config/` for
  `config.exs`/`runtime.exs`; `mix.exs` at root; `mix.lock` for locked
  dependency versions.
- Environments: `:dev`, `:test`, `:prod`; `Mix.env/0` reads current; `MIX_ENV`
  env var sets it.
- Dependencies in `deps/0`: `{:plug, "~> 1.14"}`,
  `{:git_dep, git: "https://...", tag: "..."}`,
  `{:path_dep, path: "../path_dep"}`. Options: `:only`, `:runtime`,
  `:optional`, `:override`.
- `mix deps.get` fetches; `mix deps.compile` compiles; `mix deps.clean`
  removes; `mix deps.tree` visualizes; `mix deps.update pkg` updates (respects
  requirements); `mix deps.unlock --check-unused` CI gate for stale lockfile
  entries.
- `mix.lock` pins exact versions; commit it for applications, debate for
  libraries.
- Aliases in `project/0`:
  `aliases: [test: ["ecto.create --quiet", "ecto.migrate", "test"]]`.
- Custom Mix tasks: `use Mix.Task`, `@shortdoc`, `@recursive`,
  `@preferred_cli_env`, implement `run/1`.
- Releases: `mix release` (Elixir >= 1.9); `mix release.init` generates
  `rel/env.sh.eex`/`rel/vm.args.eex` templates. Release = self-contained
  deployable artifact with BEAM + app code + runtime config.
- `Config.Provider` for runtime config injection in releases; `runtime.exs`
  evaluated at boot.
- `mix format` + `.formatter.exs` (`:inputs`, `:subdirectories`,
  `:import_deps`, `:export`); `mix format --check-formatted` in CI.
- `mix compile --warnings-as-errors` in CI.
- `mix xref` for cross-reference analysis (`calls`, `graph`, `warnings` modes).
- Archives (`mix archive.install`) and escripts (`mix escript.build`) for
  distributable scripts.

## Quick Commands

```bash
mix new my_app                            # scaffold project
mix new my_app --sup                      # with supervision tree
mix deps.get                              # fetch dependencies
mix deps.compile                          # compile dependencies
mix deps.tree                             # dependency graph
mix deps.update --all                     # update all (respect requirements)
mix deps.unlock --check-unused            # CI: fail on stale lockfile
mix release                               # build release
mix release.init                          # generate release templates
mix format --check-formatted              # CI formatting gate
mix compile --warnings-as-errors          # CI warnings gate
mix xref warnings                         # cross-ref warnings
```

## Anti-patterns

- Not committing `mix.lock` for applications (reproducibility).
- `MIX_ENV=prod mix release` without `--env` (prefer explicit).
- Heavy work in `mix.exs` `project/0` (it runs at compile time).
- Missing `:elixirc_paths` for non-standard layouts.
- Using umbrella projects unnecessarily (adds complexity).
- Not running `mix format --check-formatted` in CI.

## Related Skills

- elixir-config
- elixir-static-analysis
- elixir-docs-publishing
